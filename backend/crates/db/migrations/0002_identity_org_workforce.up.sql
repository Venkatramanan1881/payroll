-- 0002 Identity & Access, Organisation Structure, Workforce. DOMAIN_MODEL §3.2-§3.4.
-- Conventions: docs/database/DATA_MODEL.md §1. Org-owned tables: PK (org_id, id), composite FKs, register_tenant_table().

CREATE SCHEMA org;
CREATE SCHEMA iam;
CREATE SCHEMA workforce;
GRANT USAGE ON SCHEMA org, iam, workforce TO app_api, app_jobs, app_gateway;

-- ===========================================================================
-- Organisation Structure (C01)
-- ===========================================================================
CREATE TABLE org.branch (
  org_id      uuid NOT NULL REFERENCES platform.organisation (id),
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  code        text NOT NULL,
  name        text NOT NULL,
  address     text,
  holiday_calendar_id uuid,               -- FK added in 0003 (sched.holiday_calendar); OR-R1, SH-10
  closed_on   date,                       -- branch closure behaviour: OQ-C01-02
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, code)
);
SELECT app.register_tenant_table('org.branch');
SELECT app.register_mutable('org.branch');

-- Dated time zone per branch (BR-11). Past punches keep the zone they were captured in; change rules: OQ-C01-03.
CREATE TABLE org.branch_timezone (
  org_id     uuid NOT NULL,
  branch_id  uuid NOT NULL,
  tz         text NOT NULL CHECK (app.is_valid_timezone(tz)),
  valid      daterange NOT NULL CHECK (NOT isempty(valid)),
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  PRIMARY KEY (org_id, branch_id, valid),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  EXCLUDE USING gist (org_id WITH =, branch_id WITH =, valid WITH &&)
);
SELECT app.register_tenant_table('org.branch_timezone', true);

CREATE TABLE org.geo_fence (
  org_id      uuid NOT NULL,
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  branch_id   uuid NOT NULL,
  name        text,
  latitude    numeric(9,6) NOT NULL CHECK (latitude BETWEEN -90 AND 90),
  longitude   numeric(9,6) NOT NULL CHECK (longitude BETWEEN -180 AND 180),
  radius_m    integer NOT NULL DEFAULT 200 CHECK (radius_m > 0),        -- PC-03
  enabled     boolean NOT NULL DEFAULT false,                         -- switched on after a test (PC-R14)
  tested_at   timestamptz,
  tested_by   uuid,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id)
);
CREATE INDEX geo_fence_branch ON org.geo_fence (org_id, branch_id);
SELECT app.register_tenant_table('org.geo_fence');
SELECT app.register_mutable('org.geo_fence');

CREATE TABLE org.branch_ip_allow (
  org_id     uuid NOT NULL,
  branch_id  uuid NOT NULL,
  network    cidr NOT NULL,                                           -- PC-05
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  PRIMARY KEY (org_id, branch_id, network),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id)
);
SELECT app.register_tenant_table('org.branch_ip_allow');

-- Departments, designations, categories, employee types: organisation-extensible lists (OR-R2, OR-R3, OR-R5).
CREATE TABLE org.list_entry (
  org_id     uuid NOT NULL,
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  kind       text NOT NULL CHECK (kind IN ('department','designation','category','employee_type')),
  name       text NOT NULL,
  code       text,
  retired_at timestamptz,                -- retired, never deleted once used (OQ-C01-04)
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, id, kind),             -- target of kind-checked composite FKs
  UNIQUE (org_id, kind, name),
  FOREIGN KEY (org_id) REFERENCES platform.organisation (id)
);
SELECT app.register_tenant_table('org.list_entry');
SELECT app.register_mutable('org.list_entry');

CREATE TABLE org.contractor (
  org_id     uuid NOT NULL,
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  contact    text,
  retired_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name),
  FOREIGN KEY (org_id) REFERENCES platform.organisation (id)
);
SELECT app.register_tenant_table('org.contractor');
SELECT app.register_mutable('org.contractor');

-- Organisation-wide settings (OR-01..04, EM-01..09, AC-01..08 ...), versioned and append-only (G5, G9, DP-10).
CREATE TABLE org.organisation_settings_version (
  org_id         uuid NOT NULL REFERENCES platform.organisation (id),
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  effective_from date NOT NULL,
  settings       jsonb NOT NULL,          -- validated against the core settings schema (TSD §5.4)
  schema_version integer NOT NULL CHECK (schema_version > 0),
  reason         text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, effective_from)
);
SELECT app.register_tenant_table('org.organisation_settings_version', true);

-- ===========================================================================
-- Workforce (C02)
-- ===========================================================================
CREATE TABLE workforce.employee (
  org_id            uuid NOT NULL REFERENCES platform.organisation (id),
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_code     text NOT NULL,
  full_name         text NOT NULL,
  date_of_birth     date,
  mobile            text,
  email             text,
  address           text,
  emergency_contact jsonb,                -- EM-R1
  joining_date      date NOT NULL,
  probation_end_date date,                -- EM-04
  exit_date         date,
  employment_status text NOT NULL CHECK (employment_status IN ('on_probation','active','on_notice','exited')),  -- SM-EMP
  rejoin_of_employee_id uuid,             -- EM-05; record reuse semantics: OQ-C02-14
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  updated_at        timestamptz NOT NULL DEFAULT now(),
  updated_by        uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, rejoin_of_employee_id) REFERENCES workforce.employee (org_id, id),
  CHECK (exit_date IS NULL OR exit_date >= joining_date),
  CHECK (employment_status <> 'exited' OR exit_date IS NOT NULL)
);
-- Code unique among non-exited employees, so "reuse old code" (EM-05) works under either answer to OQ-C02-14.
CREATE UNIQUE INDEX employee_code_live ON workforce.employee (org_id, employee_code) WHERE employment_status <> 'exited';
CREATE INDEX employee_mobile ON workforce.employee (org_id, mobile);
SELECT app.register_tenant_table('workforce.employee');
SELECT app.register_mutable('workforce.employee');
COMMENT ON TABLE workforce.employee IS 'Bank and identity numbers are not held in V1 (OQ-C02-05); when added they are field-encrypted (TSD §15).';

CREATE TABLE workforce.employment_status_history (
  org_id      uuid NOT NULL,
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id uuid NOT NULL,
  from_status text,
  to_status   text NOT NULL CHECK (to_status IN ('on_probation','active','on_notice','exited')),
  effective_on date NOT NULL,
  reason      text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id)
);
CREATE INDEX employment_status_history_emp ON workforce.employment_status_history (org_id, employee_id, effective_on);
SELECT app.register_tenant_table('workforce.employment_status_history', true);

-- Dated job assignments (EM-R2, EM-R6, EM-09). One row per kind per date range; ranges of a kind never overlap.
-- Exactly one typed target column is set, matching the kind. List entries are kind-checked by composite FK.
CREATE TABLE workforce.employee_assignment (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id   uuid NOT NULL,
  kind          text NOT NULL CHECK (kind IN ('master_branch','department','designation','category','employee_type','reporting_manager','contractor')),
  valid         daterange NOT NULL CHECK (NOT isempty(valid)),
  branch_id     uuid,
  list_entry_id uuid,
  list_kind     text GENERATED ALWAYS AS (CASE WHEN kind IN ('department','designation','category','employee_type') THEN kind END) STORED,
  manager_employee_id uuid,
  contractor_id uuid,
  reason        text,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  FOREIGN KEY (org_id, list_entry_id, list_kind) REFERENCES org.list_entry (org_id, id, kind),
  FOREIGN KEY (org_id, manager_employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, contractor_id) REFERENCES org.contractor (org_id, id),
  CHECK (
    (kind = 'master_branch'     AND branch_id IS NOT NULL AND list_entry_id IS NULL AND manager_employee_id IS NULL AND contractor_id IS NULL) OR
    (kind IN ('department','designation','category','employee_type')
                                AND list_entry_id IS NOT NULL AND branch_id IS NULL AND manager_employee_id IS NULL AND contractor_id IS NULL) OR
    (kind = 'reporting_manager' AND manager_employee_id IS NOT NULL AND branch_id IS NULL AND list_entry_id IS NULL AND contractor_id IS NULL) OR
    (kind = 'contractor'        AND contractor_id IS NOT NULL AND branch_id IS NULL AND list_entry_id IS NULL AND manager_employee_id IS NULL)
  ),
  CHECK (manager_employee_id IS NULL OR manager_employee_id <> employee_id),   -- BR-07 depends on it
  EXCLUDE USING gist (org_id WITH =, employee_id WITH =, kind WITH =, valid WITH &&)
);
COMMENT ON TABLE workforce.employee_assignment IS 'Dated history (EM-R6). A change ends the current range (upper bound) and inserts the next one; every change is audited. "Exactly one master branch on every employed date" is checked by the application (EM-R9).';
CREATE INDEX employee_assignment_lookup ON workforce.employee_assignment (org_id, employee_id, kind);
CREATE INDEX employee_assignment_manager ON workforce.employee_assignment (org_id, manager_employee_id) WHERE kind = 'reporting_manager';
SELECT app.register_tenant_table('workforce.employee_assignment');

CREATE TABLE workforce.employee_allowed_branch (
  org_id      uuid NOT NULL,
  employee_id uuid NOT NULL,
  branch_id   uuid NOT NULL,
  valid       daterange NOT NULL CHECK (NOT isempty(valid)),
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, employee_id, branch_id, valid),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  EXCLUDE USING gist (org_id WITH =, employee_id WITH =, branch_id WITH =, valid WITH &&)
);
COMMENT ON TABLE workforce.employee_allowed_branch IS 'Allowed branches (EM-R9, EM-06 = chosen branches). EM-06 = all branches is a setting, not rows.';
SELECT app.register_tenant_table('workforce.employee_allowed_branch');

CREATE TABLE workforce.custom_field_def (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  data_type  text NOT NULL CHECK (data_type IN ('text','number','date','boolean','choice')),
  choices    text[],
  mandatory  boolean NOT NULL DEFAULT false,      -- EM-02
  retired_at timestamptz,                         -- removal with data: OQ-C02-13
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name),
  CHECK (data_type <> 'choice' OR choices IS NOT NULL)
);
SELECT app.register_tenant_table('workforce.custom_field_def');

CREATE TABLE workforce.custom_field_value (
  org_id      uuid NOT NULL,
  employee_id uuid NOT NULL,
  field_id    uuid NOT NULL,
  value       jsonb NOT NULL,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid,
  PRIMARY KEY (org_id, employee_id, field_id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, field_id) REFERENCES workforce.custom_field_def (org_id, id)
);
SELECT app.register_tenant_table('workforce.custom_field_value');
SELECT app.register_mutable('workforce.custom_field_value');

-- ===========================================================================
-- Identity & Access (C03)
-- ===========================================================================
CREATE TABLE iam.user_account (
  org_id        uuid NOT NULL REFERENCES platform.organisation (id),
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id   uuid,
  login_kind    text NOT NULL CHECK (login_kind IN ('mobile_otp','password')),   -- UM-R3
  mobile        text,
  email         text,
  password_hash text,                     -- argon2 (TSD §2); never the password
  mfa_secret_enc bytea,                   -- second step for admin roles (AC-08), encrypted
  language      text NOT NULL DEFAULT 'en' CHECK (language IN ('en','ta','hi')),  -- UM-R4, N7
  active        boolean NOT NULL DEFAULT true,
  last_sign_in_at timestamptz,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  updated_at    timestamptz NOT NULL DEFAULT now(),
  updated_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  CHECK (login_kind <> 'mobile_otp' OR mobile IS NOT NULL),
  CHECK (login_kind <> 'password' OR (email IS NOT NULL AND password_hash IS NOT NULL))
);
CREATE UNIQUE INDEX user_account_mobile ON iam.user_account (org_id, mobile) WHERE mobile IS NOT NULL;
CREATE UNIQUE INDEX user_account_email  ON iam.user_account (org_id, lower(email)) WHERE email IS NOT NULL;
CREATE UNIQUE INDEX user_account_employee ON iam.user_account (org_id, employee_id) WHERE employee_id IS NOT NULL;
SELECT app.register_tenant_table('iam.user_account');
SELECT app.register_mutable('iam.user_account');

-- Sign-in happens before the organisation is known, so RLS would hide every account.
-- This narrow function returns only (org_id, user_id) candidates; the caller still verifies the code/password.
-- Same mobile in two organisations returns two rows: handling is OQ-C03-05.
CREATE FUNCTION iam.resolve_login(p_mobile text, p_email text)
RETURNS TABLE (org_id uuid, user_id uuid)
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog, iam AS $$
  SELECT u.org_id, u.id
  FROM iam.user_account u
  JOIN platform.organisation o ON o.id = u.org_id
  WHERE u.active
    AND ((p_mobile IS NOT NULL AND u.mobile = p_mobile) OR (p_email IS NOT NULL AND lower(u.email) = lower(p_email)))
$$;
-- Owned by a non-login role allowed to bypass RLS, with SELECT on exactly these two tables (tables force RLS even for their owner).
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'app_resolver') THEN
    BEGIN
      CREATE ROLE app_resolver NOLOGIN BYPASSRLS;
    EXCEPTION WHEN duplicate_object OR unique_violation THEN NULL;  -- roles are cluster-wide; another database may create it concurrently
    END;
  END IF;
END $$;
GRANT USAGE ON SCHEMA iam, platform TO app_resolver;
GRANT SELECT ON iam.user_account, platform.organisation TO app_resolver;
ALTER FUNCTION iam.resolve_login(text, text) OWNER TO app_resolver;
REVOKE ALL ON FUNCTION iam.resolve_login(text, text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION iam.resolve_login(text, text) TO app_api;

CREATE TABLE iam.otp_challenge (
  org_id      uuid NOT NULL,
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  user_id     uuid NOT NULL,
  code_hash   text NOT NULL,
  expires_at  timestamptz NOT NULL,
  attempts    integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),   -- limits: OQ-C03-05
  consumed_at timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id)
);
SELECT app.register_tenant_table('iam.otp_challenge');

CREATE TABLE iam.session (
  org_id             uuid NOT NULL,
  id                 uuid NOT NULL DEFAULT gen_random_uuid(),
  user_id            uuid NOT NULL,
  refresh_token_hash text NOT NULL,
  device_label       text,
  created_at         timestamptz NOT NULL DEFAULT now(),
  expires_at         timestamptz NOT NULL,
  revoked_at         timestamptz,
  PRIMARY KEY (org_id, id),
  UNIQUE (refresh_token_hash),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id)
);
SELECT app.register_tenant_table('iam.session');

CREATE TABLE iam.role (
  org_id      uuid NOT NULL REFERENCES platform.organisation (id),
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  name        text NOT NULL,
  system_code text CHECK (system_code IN ('org_admin','hr_manager','payroll_officer','manager','supervisor','employee','auditor')),  -- BRD §5
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name),
  UNIQUE (org_id, system_code)
);
COMMENT ON TABLE iam.role IS 'System roles (system_code set) and custom roles (AC-06). Editability of system roles: OQ-C03-01.';
SELECT app.register_tenant_table('iam.role');
SELECT app.register_mutable('iam.role');

-- Operation codes are the row IDs of docs/domain/PERMISSION_MATRIX.md (e.g. LVE-03).
CREATE TABLE iam.role_grant (
  org_id         uuid NOT NULL,
  role_id        uuid NOT NULL,
  operation_code text NOT NULL CHECK (operation_code ~ '^[A-Z]{3}-[0-9]{2}[a-z]?$'),
  grant_kind     text NOT NULL CHECK (grant_kind IN ('all','scoped','self','approve')),
  setting_ref    text,                    -- e.g. PC-11, AC-04: grant applies only when that setting allows it
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  PRIMARY KEY (org_id, role_id, operation_code),
  FOREIGN KEY (org_id, role_id) REFERENCES iam.role (org_id, id)
);
SELECT app.register_tenant_table('iam.role_grant');

CREATE TABLE iam.user_role (
  org_id     uuid NOT NULL,
  user_id    uuid NOT NULL,
  role_id    uuid NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  PRIMARY KEY (org_id, user_id, role_id),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, role_id) REFERENCES iam.role (org_id, id)
);
SELECT app.register_tenant_table('iam.user_role');

-- AC-07 access scope. No rows = no data scope beyond self. scope_kind 'all' = all branches.
CREATE TABLE iam.user_scope (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL,
  scope_kind    text NOT NULL CHECK (scope_kind IN ('all','branch','department')),
  branch_id     uuid,
  department_id uuid,
  department_kind text GENERATED ALWAYS AS (CASE WHEN scope_kind = 'department' THEN 'department' END) STORED,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  FOREIGN KEY (org_id, department_id, department_kind) REFERENCES org.list_entry (org_id, id, kind),
  CHECK ((scope_kind = 'all' AND branch_id IS NULL AND department_id IS NULL) OR
         (scope_kind = 'branch' AND branch_id IS NOT NULL AND department_id IS NULL) OR
         (scope_kind = 'department' AND department_id IS NOT NULL AND branch_id IS NULL))
);
CREATE INDEX user_scope_user ON iam.user_scope (org_id, user_id);
SELECT app.register_tenant_table('iam.user_scope');

CREATE TABLE iam.delegation (
  org_id           uuid NOT NULL,
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  approver_user_id uuid NOT NULL,
  delegate_user_id uuid NOT NULL,
  valid            tstzrange NOT NULL CHECK (NOT isempty(valid)),
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, delegate_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK (approver_user_id <> delegate_user_id),
  EXCLUDE USING gist (org_id WITH =, approver_user_id WITH =, valid WITH &&)
);
COMMENT ON TABLE iam.delegation IS 'AC-03. Rules (who sets it, scope): OQ-C03-10. BR-07 is checked per request, not here.';
SELECT app.register_tenant_table('iam.delegation');

-- AC-02: approver source per request type and level. Timeouts and escalation come from the policy (LM-13, PM-17, RG-07, OD-05).
CREATE TABLE iam.approver_rule (
  org_id         uuid NOT NULL REFERENCES platform.organisation (id),
  request_type   text NOT NULL CHECK (request_type IN ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance')),
  level          smallint NOT NULL CHECK (level IN (1,2)),
  source         text NOT NULL CHECK (source IN ('reporting_manager','named_user','department_head','hr')),
  named_user_id  uuid,
  updated_at     timestamptz NOT NULL DEFAULT now(),
  updated_by     uuid,
  PRIMARY KEY (org_id, request_type, level),
  FOREIGN KEY (org_id, named_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK ((source = 'named_user') = (named_user_id IS NOT NULL))
);
SELECT app.register_tenant_table('iam.approver_rule');
SELECT app.register_mutable('iam.approver_rule');

-- Support-grant decisions reference an Organisation Admin user (VC-03).
ALTER TABLE platform.support_access_grant
  ADD CONSTRAINT support_access_grant_decider FOREIGN KEY (org_id, decided_by) REFERENCES iam.user_account (org_id, id);
