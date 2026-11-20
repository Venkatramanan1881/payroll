-- 0010 Report-run jobs, dashboard choices and filters, and three pending Gate 3 items.
-- Approved 2026-11-20 by the project owner (Gate 3 amendment, D-38).

-- 1. Jobs (API_CONVENTIONS §7): report runs and other long work a client polls (GET /v1/jobs/{id}).
CREATE TABLE app.job (
  org_id       uuid NOT NULL REFERENCES platform.organisation (id),
  id           uuid NOT NULL DEFAULT gen_random_uuid(),
  kind         text NOT NULL,                       -- e.g. report:R1
  requested_by uuid NOT NULL,
  params       jsonb NOT NULL DEFAULT '{}'::jsonb,
  state        text NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','running','succeeded','failed')),
  result_path  text,                                -- storage path of the result file (TB-02)
  error        text,
  created_at   timestamptz NOT NULL DEFAULT now(),
  finished_at  timestamptz,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  CHECK (result_path IS NULL OR starts_with(result_path, org_id::text || '/')),
  CHECK ((state IN ('succeeded','failed')) = (finished_at IS NOT NULL))
);
CREATE INDEX job_requester ON app.job (org_id, requested_by, created_at);
SELECT app.register_tenant_table('app.job');

-- 2. DB-01 dashboard widgets chosen per role by the Organisation Admin (slice 18b).
CREATE TABLE iam.dashboard_widget_choice (
  org_id     uuid NOT NULL,
  role_id    uuid NOT NULL,
  widgets    text[] NOT NULL,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, role_id),
  FOREIGN KEY (org_id, role_id) REFERENCES iam.role (org_id, id)
);
SELECT app.register_tenant_table('iam.dashboard_widget_choice');

-- 3. DB-02 default dashboard filters per user (slice 18b).
CREATE TABLE iam.dashboard_default_filter (
  org_id     uuid NOT NULL,
  user_id    uuid NOT NULL,
  branch_id  uuid,
  period     text CHECK (period IN ('current','previous')),
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, user_id),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id)
);
SELECT app.register_tenant_table('iam.dashboard_default_filter');

-- 4. Outbox (slice 17 review): the relay reads one organisation's pending events of the types it handles, and
--    removes events once they are old (dispatched after 30 days, never consumed after 90).
CREATE INDEX outbox_pending_by_org ON app.outbox (org_id, event_type, created_at) WHERE dispatched_at IS NULL;
GRANT DELETE ON app.outbox TO app_relay;

-- 5. Vendor command replays look up the client's key on the audit entry (slice 11 shortcut).
CREATE INDEX audit_entry_change_ref ON audit.audit_entry (org_id, actor_id, action, change_ref) WHERE change_ref IS NOT NULL;

-- 6. Authenticator codes are single-use: the last accepted 30-second step per user (slice 11 shortcut).
ALTER TABLE iam.user_account ADD COLUMN mfa_last_step bigint;
