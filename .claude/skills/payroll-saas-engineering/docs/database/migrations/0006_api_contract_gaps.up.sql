-- 0006 Schema additions found while writing the API contracts (Phase 4). Needs Gate 3 amendment approval.
-- Each item names the contract operation that needs it. No business rule is decided here; open items cite their ids.

-- 1. EM-03 employee contact-detail change requests (workforce.yaml workforceRequestMyContactChange).
--    New lifecycle SM-CONTACT (pending -> approved -> applied | rejected; applied directly when EM-03 needs no approval).
CREATE TABLE workforce.contact_change_request (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id   uuid NOT NULL,
  proposed      jsonb NOT NULL,                 -- only contact fields (EM-03); validated by the application
  previous      jsonb NOT NULL,
  state         text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','approved','rejected','applied')),
  requested_by  uuid NOT NULL,
  requested_at  timestamptz NOT NULL DEFAULT now(),
  decided_by    uuid,
  decided_at    timestamptz,
  decision_reason text,
  applied_at    timestamptz,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, decided_by) REFERENCES iam.user_account (org_id, id),
  CHECK (decided_by IS NULL OR decided_by <> requested_by),            -- BR-07
  CHECK (state <> 'applied' OR applied_at IS NOT NULL)
);
CREATE INDEX contact_change_request_pending ON workforce.contact_change_request (org_id, state) WHERE state = 'pending';
SELECT app.register_tenant_table('workforce.contact_change_request');

-- 2. AC-04 settings changes waiting for Organisation Admin approval (organisation.yaml SettingsChange).
--    New lifecycle SM-SETCHG (pending -> approved | rejected). Outcome after rejection: GAP-SM-16.
CREATE TABLE org.settings_change_request (
  org_id         uuid NOT NULL REFERENCES platform.organisation (id),
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  target         text NOT NULL CHECK (target IN ('organisation_settings','access_settings','employee_visibility','branch_ip_allow_list')),
  proposed       jsonb NOT NULL,
  effective_from date NOT NULL,
  reason         text,
  state          text NOT NULL DEFAULT 'pending' CHECK (state IN ('pending','approved','rejected')),
  submitted_by   uuid NOT NULL,
  submitted_at   timestamptz NOT NULL DEFAULT now(),
  decided_by     uuid,
  decided_at     timestamptz,
  resulting_version_id uuid,                     -- org.organisation_settings_version created on approval
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, submitted_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, decided_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, resulting_version_id) REFERENCES org.organisation_settings_version (org_id, id),
  CHECK (decided_by IS NULL OR decided_by <> submitted_by)             -- BR-07; one person with both roles: OQ-C03-11
);
SELECT app.register_tenant_table('org.settings_change_request');

-- 3. PL-06 period unlock requests (period.yaml UnlockRequest), SM-REQ states.
CREATE TABLE period.unlock_request (
  org_id         uuid NOT NULL,
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  period_id      uuid NOT NULL,
  reason         text NOT NULL,
  requested_by   uuid NOT NULL,
  requested_at   timestamptz NOT NULL DEFAULT now(),
  state          text NOT NULL DEFAULT 'pending_l1' CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),
  escalated      boolean NOT NULL DEFAULT false,
  decided_basis  text CHECK (decided_basis IN ('approver','auto_on_submit','auto_on_timeout')),
  current_approver_user_id uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK (state NOT IN ('approved','rejected') OR decided_basis IS NOT NULL)
);
SELECT app.register_tenant_table('period.unlock_request');

-- 4. VC-03 "support access needs Organisation Admin approval" (vendor.yaml). Who may change it: G-04.
ALTER TABLE platform.organisation ADD COLUMN support_access_needs_approval boolean NOT NULL DEFAULT true;

-- 5. In-app inbox "mark read" (notifications.yaml) and push device tokens for the mobile app.
ALTER TABLE notify.notification ADD COLUMN read_at timestamptz;
CREATE TABLE notify.push_device (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  user_id       uuid NOT NULL,
  platform      text NOT NULL CHECK (platform IN ('android','ios')),
  token         text NOT NULL,
  app_version   text,
  registered_at timestamptz NOT NULL DEFAULT now(),
  revoked_at    timestamptz,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, user_id) REFERENCES iam.user_account (org_id, id)
);
CREATE UNIQUE INDEX push_device_token_live ON notify.push_device (org_id, token) WHERE revoked_at IS NULL;
SELECT app.register_tenant_table('notify.push_device');

-- 6. CO-R4 employee's pay-or-comp-off choice (comp-off.yaml /v1/comp-off/choices). Effect of "pay" in V1: OQ-M02-06.
ALTER TABLE leave.comp_off_credit
  ADD COLUMN employee_choice text CHECK (employee_choice IN ('pay','comp_off')),
  ADD COLUMN chosen_at timestamptz,
  ADD CONSTRAINT comp_off_choice_time CHECK ((employee_choice IS NULL) = (chosen_at IS NULL));

-- 7. Consistency: shift change requests record how they were decided, like every other SM-REQ request;
--    contact changes join the approval request types.
ALTER TABLE sched.shift_change_request
  ADD COLUMN decided_basis text CHECK (decided_basis IN ('approver','auto_on_submit','auto_on_timeout'));
ALTER TABLE app.approval_step DROP CONSTRAINT approval_step_request_type_check;
ALTER TABLE app.approval_step ADD CONSTRAINT approval_step_request_type_check CHECK (request_type IN
  ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance','contact_change'));
ALTER TABLE iam.approver_rule DROP CONSTRAINT approver_rule_request_type_check;
ALTER TABLE iam.approver_rule ADD CONSTRAINT approver_rule_request_type_check CHECK (request_type IN
  ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance','contact_change'));
