-- 0001 Foundation: extensions, roles, tenancy helpers, platform (vendor) tables, audit, outbox.
-- See docs/database/DATA_MODEL.md §1-§5. PostgreSQL 16. Standard features only (TB-06).

CREATE EXTENSION IF NOT EXISTS btree_gist;

-- Roles (group roles, no login; services log in as members). Idempotent.
DO $$
DECLARE r text;
BEGIN
  FOREACH r IN ARRAY ARRAY['app_owner','app_api','app_gateway','app_jobs','app_vendor','app_relay'] LOOP
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = r) THEN
      EXECUTE format('CREATE ROLE %I NOLOGIN NOBYPASSRLS', r);
    END IF;
  END LOOP;
END $$;

CREATE SCHEMA app;
CREATE SCHEMA platform;
CREATE SCHEMA audit;
GRANT USAGE ON SCHEMA app TO app_api, app_gateway, app_jobs, app_vendor, app_relay;

-- ---------------------------------------------------------------------------
-- Tenancy helpers
-- ---------------------------------------------------------------------------

-- Organisation of the current transaction; NULL when not set => RLS matches nothing (fails closed).
CREATE FUNCTION app.current_org() RETURNS uuid
LANGUAGE sql STABLE AS $$
  SELECT nullif(current_setting('app.org_id', true), '')::uuid
$$;

-- Called first in every transaction, with the organisation from the verified token / device.
CREATE FUNCTION app.set_org(p_org uuid) RETURNS void
LANGUAGE sql AS $$
  SELECT set_config('app.org_id', p_org::text, true)
$$;

CREATE FUNCTION app.current_support_grant() RETURNS uuid
LANGUAGE sql STABLE AS $$
  SELECT nullif(current_setting('app.support_grant_id', true), '')::uuid
$$;

CREATE FUNCTION app.forbid_mutation() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'append-only table %.%: % is not allowed', TG_TABLE_SCHEMA, TG_TABLE_NAME, TG_OP
    USING ERRCODE = 'insufficient_privilege';
END $$;

CREATE FUNCTION app.touch_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  NEW.updated_at := now();
  RETURN NEW;
END $$;

-- Tables in context schemas that are deliberately not tenant-scoped, with the reason.
CREATE TABLE app.rls_exempt (
  table_name regclass PRIMARY KEY,
  reason     text NOT NULL
);

-- Registers an org-owned table: forced RLS on org_id, grants, optional append-only triggers.
CREATE FUNCTION app.register_tenant_table(p_table regclass, p_append_only boolean DEFAULT false)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format('ALTER TABLE %s ENABLE ROW LEVEL SECURITY', p_table);
  EXECUTE format('ALTER TABLE %s FORCE ROW LEVEL SECURITY', p_table);
  EXECUTE format(
    'CREATE POLICY tenant_isolation ON %s USING (org_id = app.current_org()) WITH CHECK (org_id = app.current_org())',
    p_table);
  IF p_append_only THEN
    EXECUTE format('GRANT SELECT, INSERT ON %s TO app_api, app_gateway, app_jobs', p_table);
    EXECUTE format('CREATE TRIGGER append_only BEFORE UPDATE OR DELETE ON %s FOR EACH ROW EXECUTE FUNCTION app.forbid_mutation()', p_table);
    EXECUTE format('CREATE TRIGGER append_only_truncate BEFORE TRUNCATE ON %s FOR EACH STATEMENT EXECUTE FUNCTION app.forbid_mutation()', p_table);
  ELSE
    EXECUTE format('GRANT SELECT, INSERT, UPDATE, DELETE ON %s TO app_api, app_jobs', p_table);
    EXECUTE format('GRANT SELECT ON %s TO app_gateway', p_table);
  END IF;
END $$;

-- Adds updated_at maintenance to a mutable table.
CREATE FUNCTION app.register_mutable(p_table regclass) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format('CREATE TRIGGER touch_updated_at BEFORE UPDATE ON %s FOR EACH ROW EXECUTE FUNCTION app.touch_updated_at()', p_table);
END $$;

CREATE FUNCTION app.is_valid_timezone(p_tz text) RETURNS boolean
LANGUAGE sql STABLE AS $$
  SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = p_tz)
$$;

-- ---------------------------------------------------------------------------
-- Outbox (ADR-002). Not tenant-RLS: the relay dispatches for all organisations.
-- Producers can only insert; only the relay can read and mark.
-- ---------------------------------------------------------------------------
CREATE TABLE app.outbox (
  event_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id        uuid NOT NULL,
  event_type    text NOT NULL,
  aggregate_id  uuid NOT NULL,
  payload       jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at    timestamptz NOT NULL DEFAULT now(),
  dispatched_at timestamptz,
  attempts      integer NOT NULL DEFAULT 0 CHECK (attempts >= 0)
);
CREATE INDEX outbox_pending ON app.outbox (created_at) WHERE dispatched_at IS NULL;
COMMENT ON TABLE app.outbox IS 'Follow-up work written in the same transaction as the change (ADR-002). Consumers are idempotent by event_id.';
GRANT INSERT ON app.outbox TO app_api, app_gateway, app_jobs;
GRANT SELECT, UPDATE ON app.outbox TO app_relay;
INSERT INTO app.rls_exempt VALUES
  ('app.rls_exempt', 'Metadata about tables; holds no organisation data.'),
  ('app.outbox', 'Cross-organisation dispatch; producers insert-only, relay read/mark only (ADR-002).');

-- ---------------------------------------------------------------------------
-- Platform (vendor) — DOMAIN_MODEL §3.1, SM-ORG, SM-SUPPORT
-- ---------------------------------------------------------------------------
CREATE TABLE platform.vendor_user (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email       text NOT NULL UNIQUE,
  full_name   text NOT NULL,
  vendor_role text NOT NULL CHECK (vendor_role IN ('full_admin','support','read_only')),  -- VC-05
  active      boolean NOT NULL DEFAULT true,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid
);

CREATE TABLE platform.sector_template (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  code        text NOT NULL UNIQUE,      -- factory, supermarket, restaurant_bakery, office_services (G6)
  name        text NOT NULL,
  content     jsonb NOT NULL,            -- default settings, policy sets and lists; copied at organisation creation
  version     integer NOT NULL DEFAULT 1 CHECK (version > 0),
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid
);
COMMENT ON TABLE platform.sector_template IS 'Copied into an organisation at creation (VC-R1). Effect of later edits on existing organisations: OQ-V01-10.';

CREATE TABLE platform.plan (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  code            text NOT NULL UNIQUE,
  name            text NOT NULL,
  modules         text[] NOT NULL,       -- module codes (MD-02); content per plan: Q5
  employee_limit  integer CHECK (employee_limit IS NULL OR employee_limit > 0),
  mobile_app      boolean NOT NULL,
  created_at      timestamptz NOT NULL DEFAULT now(),
  updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE platform.organisation (
  id                 uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  legal_name         text NOT NULL,
  display_name       text NOT NULL,
  sector_template_id uuid REFERENCES platform.sector_template (id),
  state              text NOT NULL DEFAULT 'active' CHECK (state IN ('active','suspended')),  -- SM-ORG
  state_changed_at   timestamptz NOT NULL DEFAULT now(),
  state_reason       text,
  created_at         timestamptz NOT NULL DEFAULT now(),
  created_by         uuid REFERENCES platform.vendor_user (id),
  updated_at         timestamptz NOT NULL DEFAULT now(),
  updated_by         uuid
);
COMMENT ON TABLE platform.organisation IS 'Tenant root. Client roles see only their own row (RLS on id).';

CREATE TABLE platform.organisation_plan (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  plan_id    uuid NOT NULL REFERENCES platform.plan (id),
  valid      daterange NOT NULL CHECK (NOT isempty(valid)),
  set_by     uuid NOT NULL REFERENCES platform.vendor_user (id),
  set_at     timestamptz NOT NULL DEFAULT now(),
  reason     text,
  PRIMARY KEY (org_id, valid),
  EXCLUDE USING gist (org_id WITH =, valid WITH &&)
);
COMMENT ON TABLE platform.organisation_plan IS 'Dated plan history (VC-R2).';

CREATE TABLE platform.module_entitlement (
  org_id      uuid NOT NULL REFERENCES platform.organisation (id),
  module_code text NOT NULL CHECK (module_code ~ '^(V01|C[0-9]{2}|M[0-9]{2})$'),
  enabled     boolean NOT NULL,
  valid       daterange NOT NULL CHECK (NOT isempty(valid)),
  set_by      uuid NOT NULL REFERENCES platform.vendor_user (id),
  set_at      timestamptz NOT NULL DEFAULT now(),
  reason      text,
  PRIMARY KEY (org_id, module_code, valid),
  EXCLUDE USING gist (org_id WITH =, module_code WITH =, valid WITH &&)
);
COMMENT ON TABLE platform.module_entitlement IS 'Modules switched by the vendor only (MD-01), dated history (VC-R2). Common modules C01-C16 are always on.';

CREATE TABLE platform.employee_limit (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  max_active integer CHECK (max_active IS NULL OR max_active > 0),
  on_reach   text NOT NULL DEFAULT 'allow_and_alert' CHECK (on_reach IN ('block','allow_and_alert')),  -- VC-01
  valid      daterange NOT NULL CHECK (NOT isempty(valid)),
  set_by     uuid NOT NULL REFERENCES platform.vendor_user (id),
  set_at     timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, valid),
  EXCLUDE USING gist (org_id WITH =, valid WITH &&)
);

CREATE TABLE platform.subscription (
  org_id          uuid PRIMARY KEY REFERENCES platform.organisation (id),
  start_date      date NOT NULL,
  renewal_date    date NOT NULL CHECK (renewal_date >= start_date),
  billing_status  text NOT NULL,          -- values: OQ-V01-11
  auto_suspend_after_days integer CHECK (auto_suspend_after_days IS NULL OR auto_suspend_after_days >= 0),  -- VC-04 (NULL = manual only)
  renewal_reminder_days   integer NOT NULL DEFAULT 15 CHECK (renewal_reminder_days >= 0),                   -- VC-02
  updated_at      timestamptz NOT NULL DEFAULT now(),
  updated_by      uuid
);

CREATE TABLE platform.organisation_usage (
  org_id            uuid PRIMARY KEY REFERENCES platform.organisation (id),
  active_employees  integer NOT NULL DEFAULT 0,
  branches          integer NOT NULL DEFAULT 0,
  devices_online    integer NOT NULL DEFAULT 0,
  devices_total     integer NOT NULL DEFAULT 0,
  last_sign_in_at   timestamptz,
  periods_awaiting_lock integer NOT NULL DEFAULT 0,
  refreshed_at      timestamptz NOT NULL DEFAULT now()
);
COMMENT ON TABLE platform.organisation_usage IS 'Summary written by job J10 so the vendor console never reads client tables (VC-R3, VC-R8).';

CREATE TABLE platform.support_access_grant (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id        uuid NOT NULL REFERENCES platform.organisation (id),
  vendor_user_id uuid NOT NULL REFERENCES platform.vendor_user (id),
  reason        text NOT NULL,
  state         text NOT NULL DEFAULT 'requested'
                CHECK (state IN ('requested','active','rejected','expired','ended')),  -- SM-SUPPORT
  requested_at  timestamptz NOT NULL DEFAULT now(),
  decided_by    uuid,          -- iam.user_account id of the Organisation Admin (VC-03); NULL when VC-03 = off
  decided_at    timestamptz,
  window_start  timestamptz,
  window_end    timestamptz,
  ended_at      timestamptz,
  CHECK (window_end IS NULL OR window_end > window_start),
  CHECK (state <> 'active' OR (window_start IS NOT NULL AND window_end IS NOT NULL))
);
CREATE INDEX support_access_grant_org ON platform.support_access_grant (org_id, state);
COMMENT ON TABLE platform.support_access_grant IS 'Time-limited vendor access (VC-R6, VC-03). Write rights under a grant: OQ-V01-12.';

CREATE TABLE platform.announcement (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  title       text NOT NULL,
  body        text NOT NULL,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid NOT NULL REFERENCES platform.vendor_user (id)
);
CREATE TABLE platform.announcement_target (
  announcement_id uuid NOT NULL REFERENCES platform.announcement (id),
  org_id          uuid NOT NULL REFERENCES platform.organisation (id),
  PRIMARY KEY (announcement_id, org_id)
);

-- Platform access: vendor role sees everything in platform; client roles see only their organisation's rows.
GRANT USAGE ON SCHEMA platform TO app_vendor, app_api, app_gateway, app_jobs;
GRANT SELECT, INSERT, UPDATE ON ALL TABLES IN SCHEMA platform TO app_vendor;
GRANT SELECT ON platform.organisation, platform.organisation_plan, platform.module_entitlement,
                platform.employee_limit, platform.plan, platform.support_access_grant,
                platform.announcement, platform.announcement_target
  TO app_api, app_jobs, app_gateway;
GRANT UPDATE (state, decided_by, decided_at, window_start, window_end, ended_at) ON platform.support_access_grant TO app_api, app_jobs;
GRANT INSERT, UPDATE ON platform.organisation_usage TO app_jobs;
GRANT SELECT ON platform.organisation_usage TO app_jobs;

ALTER TABLE platform.organisation ENABLE ROW LEVEL SECURITY;
CREATE POLICY vendor_all ON platform.organisation TO app_vendor USING (true) WITH CHECK (true);
CREATE POLICY own_org    ON platform.organisation TO app_api, app_jobs, app_gateway USING (id = app.current_org());

DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['organisation_plan','module_entitlement','employee_limit','support_access_grant','announcement_target','organisation_usage','subscription'] LOOP
    EXECUTE format('ALTER TABLE platform.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY vendor_all ON platform.%I TO app_vendor USING (true) WITH CHECK (true)', t);
    EXECUTE format('CREATE POLICY own_org ON platform.%I TO app_api, app_jobs, app_gateway USING (org_id = app.current_org()) WITH CHECK (org_id = app.current_org())', t);
  END LOOP;
END $$;
-- Announcements: client roles see only those targeted at them.
ALTER TABLE platform.announcement ENABLE ROW LEVEL SECURITY;
CREATE POLICY vendor_all ON platform.announcement TO app_vendor USING (true) WITH CHECK (true);
CREATE POLICY targeted   ON platform.announcement TO app_api, app_jobs USING (
  EXISTS (SELECT 1 FROM platform.announcement_target t WHERE t.announcement_id = announcement.id AND t.org_id = app.current_org()));

INSERT INTO app.rls_exempt VALUES
  ('platform.vendor_user',     'Vendor-only table; no client role has any grant.'),
  ('platform.sector_template', 'Vendor-only table; no client role has any grant.'),
  ('platform.plan',            'Reference data: plan definitions, no organisation data; read-only to client roles.');

-- Resolves the organisation of a registered device by serial (gateway only; TSD §9.1). Returns NULL for unknown serials.
-- Defined in 0003 once timecap.device exists.

-- ---------------------------------------------------------------------------
-- Audit (BRD §16, TSD §15) — insert-only. org_id NULL = vendor-level action with no organisation.
-- ---------------------------------------------------------------------------
CREATE TABLE audit.audit_entry (
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  org_id           uuid REFERENCES platform.organisation (id),
  occurred_at      timestamptz NOT NULL DEFAULT now(),
  actor_kind       text NOT NULL CHECK (actor_kind IN ('user','vendor_user','system_job','device')),
  actor_id         uuid,
  support_grant_id uuid REFERENCES platform.support_access_grant (id),  -- set when acting under VC-R6
  action           text NOT NULL,      -- e.g. leave_request.approve, policy_version.publish
  target_type      text NOT NULL,
  target_id        uuid,
  reason           text,
  before_value     jsonb,
  after_value      jsonb,
  change_ref       text,
  correlation_id   uuid,
  PRIMARY KEY (id)
);
CREATE INDEX audit_entry_org_time   ON audit.audit_entry (org_id, occurred_at);
CREATE INDEX audit_entry_org_target ON audit.audit_entry (org_id, target_type, target_id);
COMMENT ON TABLE audit.audit_entry IS 'Insert-only audit log (BRD §16). Entries made under a support grant are visible to the organisation (VC-R6).';

ALTER TABLE audit.audit_entry ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit.audit_entry FORCE ROW LEVEL SECURITY;
CREATE POLICY own_org ON audit.audit_entry TO app_api, app_jobs, app_gateway
  USING (org_id = app.current_org()) WITH CHECK (org_id = app.current_org());
CREATE POLICY vendor_all ON audit.audit_entry TO app_vendor USING (true) WITH CHECK (true);
GRANT USAGE ON SCHEMA audit TO app_api, app_jobs, app_gateway, app_vendor;
GRANT SELECT, INSERT ON audit.audit_entry TO app_api, app_jobs, app_gateway, app_vendor;
CREATE TRIGGER append_only BEFORE UPDATE OR DELETE ON audit.audit_entry FOR EACH ROW EXECUTE FUNCTION app.forbid_mutation();
CREATE TRIGGER append_only_truncate BEFORE TRUNCATE ON audit.audit_entry FOR EACH STATEMENT EXECUTE FUNCTION app.forbid_mutation();

-- ---------------------------------------------------------------------------
-- Build check (TSD §4.5): tables in context schemas without forced RLS and not exempt.
-- ---------------------------------------------------------------------------
CREATE VIEW app.unprotected_tables AS
SELECT c.oid::regclass AS table_name
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind IN ('r','p')
  AND n.nspname IN ('app','platform','audit','iam','org','workforce','timecap','sched','policy',
                    'attendance','leave','perm','period','openbal','notify')
  AND NOT (c.relrowsecurity AND (c.relforcerowsecurity OR n.nspname = 'platform'))
  AND c.oid NOT IN (SELECT table_name FROM app.rls_exempt);
COMMENT ON VIEW app.unprotected_tables IS 'CI fails if this returns any row (NFR-01.4).';
