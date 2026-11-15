-- 0005 Leave & Absence, Permission & On-duty, shared approval steps, Notifications. DOMAIN_MODEL §3.9, §3.10, §3.13, §6.
-- Conventions: docs/database/DATA_MODEL.md §1. Org-owned tables: PK (org_id, id), composite FKs, register_tenant_table().
-- Ledger causes (cause_kind, cause_id) are deliberately NOT foreign keys: a cause can be a leave request, a day result,
-- a period penalty, an opening-balance entry, a job run, etc. Idempotency comes from the UNIQUE cause key (DATA_MODEL §5.2).
-- Versioned types (leave_type_version, permission_type_version) are append-only rows with effective_from; the version in
-- force on a date is the latest effective_from <= date (G5). No draft/scheduled state: a correction is a new row (GAP-SM-16).

CREATE SCHEMA leave;
CREATE SCHEMA perm;
CREATE SCHEMA notify;
GRANT USAGE ON SCHEMA leave, perm, notify TO app_api, app_jobs, app_gateway;

-- ===========================================================================
-- Leave & Absence (C08) — DOMAIN_MODEL §3.9
-- ===========================================================================
CREATE TABLE leave.leave_type (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  code       text NOT NULL,
  name       text NOT NULL,
  retired_at timestamptz,                  -- retired, never deleted once used (DATA_MODEL §1)
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, code),
  UNIQUE (org_id, name)
);
SELECT app.register_tenant_table('leave.leave_type');
SELECT app.register_mutable('leave.leave_type');

CREATE TABLE leave.leave_type_version (
  org_id                 uuid NOT NULL,
  id                     uuid NOT NULL DEFAULT gen_random_uuid(),
  leave_type_id          uuid NOT NULL,
  effective_from         date NOT NULL,
  paid                   boolean NOT NULL,                                          -- LM-01
  eligibility            jsonb NOT NULL DEFAULT '{}'::jsonb,                        -- LM-01; payroll-group criterion is V2 (OQ-C08-01)
  credit_method          text NOT NULL DEFAULT 'monthly_accrual'
                         CHECK (credit_method IN ('annual_grant','monthly_accrual','quarterly_accrual','earned_by_days_worked')),  -- LM-02
  credit_days            numeric(5,2) CHECK (credit_days IS NULL OR credit_days >= 0),        -- days per grant / accrual / earning step (LM-02)
  earn_per_worked_days   integer CHECK (earn_per_worked_days IS NULL OR earn_per_worked_days > 0),  -- LM-02 "1 day per N days worked"
  accrual_timing         text NOT NULL DEFAULT 'end' CHECK (accrual_timing IN ('start','end')),               -- LM-03
  proration              text NOT NULL DEFAULT 'prorated' CHECK (proration IN ('prorated','full','none')),   -- LM-04
  max_balance            numeric(5,2) CHECK (max_balance IS NULL OR max_balance >= 0),        -- LM-05; NULL = no cap
  year_end               text NOT NULL DEFAULT 'lapse' CHECK (year_end IN ('carry_forward','lapse','encash')),  -- LM-06; encash pays: V2 (BRX-02)
  carry_forward_max      numeric(5,2) CHECK (carry_forward_max IS NULL OR carry_forward_max >= 0),  -- LM-06 N
  negative_allowed_days  numeric(5,2) NOT NULL DEFAULT 0 CHECK (negative_allowed_days >= 0),  -- LM-07; 0 = no
  half_day_allowed       boolean NOT NULL DEFAULT true,                                       -- LM-08
  min_notice_days        integer NOT NULL DEFAULT 0 CHECK (min_notice_days >= 0),             -- LM-09
  max_consecutive_days   numeric(5,2) CHECK (max_consecutive_days IS NULL OR max_consecutive_days > 0),  -- LM-10; NULL = no cap
  after_event_days       integer DEFAULT 3 CHECK (after_event_days IS NULL OR after_event_days >= 0),    -- LM-11; NULL = not allowed
  document_after_days    numeric(5,2) CHECK (document_after_days IS NULL OR document_after_days >= 0),  -- LM-12; NULL = off
  approval_levels        text NOT NULL DEFAULT 'manager' CHECK (approval_levels IN ('manager','manager_then_hr')),  -- LM-13
  timeout_days           integer DEFAULT 2 CHECK (timeout_days IS NULL OR timeout_days > 0),  -- LM-13; day counting: G-F
  timeout_action         text NOT NULL DEFAULT 'escalate' CHECK (timeout_action IN ('escalate','auto_approve','auto_reject')),  -- LM-13
  combine_allowed        boolean NOT NULL DEFAULT true,                                       -- LM-14
  during_notice          text NOT NULL DEFAULT 'allow' CHECK (during_notice IN ('allow','block','extend_notice')),  -- LM-15
  reason                 text,
  created_at             timestamptz NOT NULL DEFAULT now(),
  created_by             uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, leave_type_id, effective_from),
  FOREIGN KEY (org_id, leave_type_id) REFERENCES leave.leave_type (org_id, id),
  CHECK (year_end <> 'carry_forward' OR carry_forward_max IS NOT NULL),                     -- LM-06 "up to N"
  CHECK ((credit_method = 'earned_by_days_worked') = (earn_per_worked_days IS NOT NULL))    -- LM-02
);
COMMENT ON TABLE leave.leave_type_version IS 'Append-only versions of a leave type (LM-01..15, G5). In force on a date = latest effective_from <= date. eligibility jsonb: category / employee type / after probation (LM-01); payroll group is V2 (OQ-C08-01).';
SELECT app.register_tenant_table('leave.leave_type_version', true);

-- One row per employee per leave type. Holds no balance: it is the row locked (SELECT ... FOR UPDATE) while a balance is checked and posted.
CREATE TABLE leave.leave_account (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id   uuid NOT NULL,
  leave_type_id uuid NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, employee_id, leave_type_id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, leave_type_id) REFERENCES leave.leave_type (org_id, id)
);
COMMENT ON TABLE leave.leave_account IS 'Lock target for balance checks (LM-07). Balance = sum of leave_ledger_entry (LM-R4); never stored. Not append-only only because FOR UPDATE needs the UPDATE grant.';
SELECT app.register_tenant_table('leave.leave_account');

CREATE TABLE leave.leave_ledger_entry (
  org_id            uuid NOT NULL,
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  account_id        uuid NOT NULL,
  entry_date        date NOT NULL,
  entry_kind        text NOT NULL CHECK (entry_kind IN ('opening','accrual','grant','debit','reversal','sandwich_debit','penalty_debit',
                                                        'absence_auto_debit','carry_forward','lapse','encashment','adjustment')),  -- §3.9; encashment V2
  quantity          numeric(6,2) NOT NULL CHECK (quantity <> 0),   -- signed days; rounding of accruals: not fixed (LM-02)
  cause_kind        text NOT NULL,          -- e.g. leave_request, period_penalty, day_result, opening_balance_entry, accrual_run; no FK (polymorphic)
  cause_id          uuid NOT NULL,
  reverses_entry_id uuid,
  reason            text,
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, cause_kind, cause_id, entry_kind, account_id),   -- idempotent posting (DATA_MODEL §5.2, AT21)
  FOREIGN KEY (org_id, account_id) REFERENCES leave.leave_account (org_id, id),
  FOREIGN KEY (org_id, reverses_entry_id) REFERENCES leave.leave_ledger_entry (org_id, id),
  CHECK ((entry_kind = 'reversal') = (reverses_entry_id IS NOT NULL)),                       -- AC-C08-06.2
  CHECK (entry_kind NOT IN ('debit','sandwich_debit','penalty_debit','absence_auto_debit','lapse','encashment') OR quantity < 0),
  CHECK (entry_kind NOT IN ('accrual','grant') OR quantity > 0),
  CHECK (entry_kind <> 'adjustment' OR reason IS NOT NULL)                                    -- §3.9 "adjustment with a reason", OB-02
);
CREATE UNIQUE INDEX leave_ledger_entry_reversed_once ON leave.leave_ledger_entry (org_id, reverses_entry_id) WHERE reverses_entry_id IS NOT NULL;
CREATE INDEX leave_ledger_entry_account ON leave.leave_ledger_entry (org_id, account_id, entry_date);
COMMENT ON TABLE leave.leave_ledger_entry IS 'Append-only leave ledger (LM-R4, BR-01). Cancellation posts a reversal, never a delete. Pending-request reservation: OQ-C08-08. Debit order for penalties: DP-02.';
SELECT app.register_tenant_table('leave.leave_ledger_entry', true);

CREATE VIEW leave.leave_balance WITH (security_invoker = true) AS
SELECT a.org_id, a.id AS account_id, a.employee_id, a.leave_type_id, coalesce(sum(e.quantity), 0)::numeric(8,2) AS balance
FROM leave.leave_account a
LEFT JOIN leave.leave_ledger_entry e ON e.org_id = a.org_id AND e.account_id = a.id
GROUP BY a.org_id, a.id, a.employee_id, a.leave_type_id;
COMMENT ON VIEW leave.leave_balance IS 'Balance = sum of ledger entries (LM-R4). security_invoker so the caller''s RLS applies.';
GRANT SELECT ON leave.leave_balance TO app_api, app_jobs;

-- SM-REQ request. No draft state (GAP-SM-08).
CREATE TABLE leave.leave_request (
  org_id                   uuid NOT NULL,
  id                       uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id              uuid NOT NULL,
  leave_type_id            uuid NOT NULL,
  from_date                date NOT NULL,
  to_date                  date NOT NULL,
  from_part                text NOT NULL DEFAULT 'full' CHECK (from_part IN ('full','first_half','second_half')),  -- LM-08
  to_part                  text NOT NULL DEFAULT 'full' CHECK (to_part IN ('full','first_half','second_half')),    -- LM-08
  reason                   text,
  document_path            text,            -- LM-12; file stored by path only (TB-02)
  state                    text NOT NULL CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),  -- SM-REQ
  escalated                boolean NOT NULL DEFAULT false,                                     -- SM-REQ R7 flag
  decided_basis            text CHECK (decided_basis IN ('approver','auto_on_submit','auto_on_timeout')),  -- SM-REQ Approved/Rejected
  current_approver_user_id uuid,
  requested_by             uuid NOT NULL,   -- user who submitted (employee, or HR on behalf)
  submitted_at             timestamptz NOT NULL DEFAULT now(),
  cancelled_by             uuid,
  cancelled_at             timestamptz,
  created_at               timestamptz NOT NULL DEFAULT now(),
  created_by               uuid,
  updated_at               timestamptz NOT NULL DEFAULT now(),
  updated_by               uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, leave_type_id) REFERENCES leave.leave_type (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, cancelled_by) REFERENCES iam.user_account (org_id, id),
  CHECK (to_date >= from_date),
  CHECK (from_date < to_date OR from_part = to_part),
  CHECK (document_path IS NULL OR starts_with(document_path, org_id::text || '/')),           -- DATA_MODEL §4.4
  CHECK (state NOT IN ('pending_l1','pending_l2') OR decided_basis IS NULL),
  CHECK (state NOT IN ('approved','rejected') OR decided_basis IS NOT NULL),
  CHECK ((state = 'cancelled') = (cancelled_at IS NOT NULL))                                  -- who may cancel: OQ-C08-10
);
CREATE INDEX leave_request_pending ON leave.leave_request (org_id, state, current_approver_user_id) WHERE state IN ('pending_l1','pending_l2');  -- DB-R5, J3
CREATE INDEX leave_request_employee ON leave.leave_request (org_id, employee_id, from_date);
SELECT app.register_tenant_table('leave.leave_request');
SELECT app.register_mutable('leave.leave_request');

CREATE TABLE leave.leave_request_day (
  org_id           uuid NOT NULL,
  leave_request_id uuid NOT NULL,
  leave_date       date NOT NULL,
  fraction         numeric(5,2) NOT NULL CHECK (fraction IN (0.5, 1.0)),
  half             text CHECK (half IN ('first_half','second_half')),
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  PRIMARY KEY (org_id, leave_request_id, leave_date),
  FOREIGN KEY (org_id, leave_request_id) REFERENCES leave.leave_request (org_id, id),
  CHECK ((fraction = 0.5) = (half IS NOT NULL))
);
COMMENT ON TABLE leave.leave_request_day IS 'One row per requested date, written at submission (a submitted request is never edited: SM-REQ). Date within the request range is checked by the application.';
SELECT app.register_tenant_table('leave.leave_request_day', true);

CREATE TABLE leave.absconding_case (
  org_id       uuid NOT NULL,
  id           uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id  uuid NOT NULL,
  opened_on    date NOT NULL,
  trigger_days integer NOT NULL CHECK (trigger_days > 0),     -- LM-30 value used; working vs calendar days: G-L, OQ-C08-17
  pay_hold     boolean NOT NULL,                              -- LM-31; V1 meaning: OQ-C08-13
  state        text NOT NULL DEFAULT 'open' CHECK (state IN ('open','closed_returned','closed_offboarding')),  -- SM-ABS; offboarding in V1: GAP-SM-14
  closed_by    uuid,
  closed_at    timestamptz,
  close_reason text,
  created_at   timestamptz NOT NULL DEFAULT now(),
  created_by   uuid,
  updated_at   timestamptz NOT NULL DEFAULT now(),
  updated_by   uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, closed_by) REFERENCES iam.user_account (org_id, id),
  CHECK ((state = 'open') = (closed_at IS NULL)),
  CHECK (state = 'open' OR closed_by IS NOT NULL),            -- LM-32 HR decides
  CHECK (state <> 'closed_returned' OR close_reason IS NOT NULL)  -- SM-ABS K2 reason
);
CREATE UNIQUE INDEX absconding_case_one_open ON leave.absconding_case (org_id, employee_id) WHERE state = 'open';  -- SM-ABS idempotency
SELECT app.register_tenant_table('leave.absconding_case');
SELECT app.register_mutable('leave.absconding_case');

-- One row per earned worked day; it is also the CompOffClaim (approval steps use request_type 'comp_off', request_id = this id).
CREATE TABLE leave.comp_off_credit (
  org_id                   uuid NOT NULL,
  id                       uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id              uuid NOT NULL,
  worked_date              date NOT NULL,
  fraction                 numeric(5,2) NOT NULL CHECK (fraction IN (0.5, 1.0)),   -- HW-04
  earned_basis             text NOT NULL CHECK (earned_basis IN ('policy','employee_choice','opening')),  -- HW-01 / CO-R4; no choice made: OQ-M02-07; opening: OB-R2
  state                    text NOT NULL CHECK (state IN ('pending_approval','credited','partially_used','used',
                                                          'expired_lapsed','expired_paid','not_credited')),  -- SM-COMPOFF; expired_paid is V2 (HW-06, OQ-M02-01)
  expires_on               date,            -- HW-05; counted from worked or credit date: OQ-M02-10; opening comp-off: OQ-C15-02
  current_approver_user_id uuid,            -- approver: OQ-M02-05
  escalated                boolean NOT NULL DEFAULT false,
  decided_by               uuid,
  decided_at               timestamptz,
  created_at               timestamptz NOT NULL DEFAULT now(),
  created_by               uuid,
  updated_at               timestamptz NOT NULL DEFAULT now(),
  updated_by               uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, decided_by) REFERENCES iam.user_account (org_id, id),
  CHECK (state IN ('pending_approval','not_credited') OR expires_on IS NOT NULL OR earned_basis = 'opening')
);
CREATE UNIQUE INDEX comp_off_credit_one_per_day ON leave.comp_off_credit (org_id, employee_id, worked_date) WHERE state <> 'not_credited';  -- SM-COMPOFF "one credit per earned worked day"
CREATE INDEX comp_off_credit_pending ON leave.comp_off_credit (org_id, state, current_approver_user_id) WHERE state = 'pending_approval';
CREATE INDEX comp_off_credit_expiry ON leave.comp_off_credit (org_id, expires_on) WHERE state IN ('credited','partially_used');  -- J6
COMMENT ON TABLE leave.comp_off_credit IS 'SM-COMPOFF. Correction of the worked day after credit: OQ-M02-04. Leave cancelled after expiry: GAP-SM-15.';
SELECT app.register_tenant_table('leave.comp_off_credit');
SELECT app.register_mutable('leave.comp_off_credit');

CREATE TABLE leave.comp_off_ledger_entry (
  org_id            uuid NOT NULL,
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  credit_id         uuid NOT NULL,
  entry_date        date NOT NULL,
  entry_kind        text NOT NULL CHECK (entry_kind IN ('credit','debit','reversal','expiry')),  -- SM-COMPOFF C2/C3, C6/C7, C8, C9
  quantity          numeric(5,2) NOT NULL CHECK (quantity <> 0),   -- signed days
  cause_kind        text NOT NULL,          -- e.g. comp_off_credit, leave_request, expiry_run, opening_balance_entry; no FK (polymorphic)
  cause_id          uuid NOT NULL,
  reverses_entry_id uuid,
  reason            text,
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, cause_kind, cause_id, entry_kind, credit_id),    -- idempotent posting (DATA_MODEL §5.2); J6: one expiry per credit
  FOREIGN KEY (org_id, credit_id) REFERENCES leave.comp_off_credit (org_id, id),
  FOREIGN KEY (org_id, reverses_entry_id) REFERENCES leave.comp_off_ledger_entry (org_id, id),
  CHECK ((entry_kind = 'reversal') = (reverses_entry_id IS NOT NULL)),
  CHECK (entry_kind NOT IN ('debit','expiry') OR quantity < 0),
  CHECK (entry_kind <> 'credit' OR quantity > 0)
);
CREATE INDEX comp_off_ledger_entry_credit ON leave.comp_off_ledger_entry (org_id, credit_id, entry_date);
COMMENT ON TABLE leave.comp_off_ledger_entry IS 'Append-only comp-off ledger (DOMAIN_MODEL §3.9). Consumption order across credits: OQ-M02-09.';
SELECT app.register_tenant_table('leave.comp_off_ledger_entry', true);

-- ===========================================================================
-- Permission & On-duty (C09) — DOMAIN_MODEL §3.10
-- ===========================================================================
CREATE TABLE perm.permission_type (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  code       text NOT NULL,
  name       text NOT NULL,                -- PM-02 (default "Personal")
  retired_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, code),
  UNIQUE (org_id, name)
);
SELECT app.register_tenant_table('perm.permission_type');
SELECT app.register_mutable('perm.permission_type');

CREATE TABLE perm.permission_type_version (
  org_id                    uuid NOT NULL,
  id                        uuid NOT NULL DEFAULT gen_random_uuid(),
  permission_type_id        uuid NOT NULL,
  effective_from            date NOT NULL,
  duration_unit             text NOT NULL DEFAULT 'block_30' CHECK (duration_unit IN ('any_minutes','block_30','block_60')),  -- PM-03
  min_minutes               integer NOT NULL DEFAULT 30 CHECK (min_minutes > 0),                    -- PM-04
  max_minutes               integer NOT NULL DEFAULT 120,                                           -- PM-04
  limit_basis               text NOT NULL DEFAULT 'count' CHECK (limit_basis IN ('count','hours','both')),  -- PM-05
  limit_count               integer DEFAULT 2 CHECK (limit_count IS NULL OR limit_count >= 0),      -- PM-06
  limit_minutes             integer DEFAULT 240 CHECK (limit_minutes IS NULL OR limit_minutes >= 0),  -- PM-07 (hours stored as minutes)
  limit_period              text NOT NULL DEFAULT 'calendar_month'
                            CHECK (limit_period IN ('week','calendar_month','payroll_period','quarter','year')),  -- PM-08; payroll_period in V1: OQ-C09-01
  max_per_day               integer DEFAULT 1 CHECK (max_per_day IS NULL OR max_per_day > 0),       -- PM-09; NULL = no cap
  max_per_week              integer CHECK (max_per_week IS NULL OR max_per_week > 0),               -- PM-09; NULL = no cap
  allowed_position          text NOT NULL DEFAULT 'any' CHECK (allowed_position IN ('start_of_shift','end_of_shift','middle_of_shift','any')),  -- PM-10
  paid                      boolean NOT NULL DEFAULT true,                                          -- PM-11
  counts_as_worked          boolean NOT NULL DEFAULT true,                                          -- PM-12
  counts_toward_overtime    boolean NOT NULL DEFAULT false,                                         -- PM-13
  apply_window              text NOT NULL DEFAULT 'after_within_days'
                            CHECK (apply_window IN ('before_only','before_or_on_day','after_within_days')),  -- PM-14
  after_event_days          integer DEFAULT 2 CHECK (after_event_days IS NULL OR after_event_days >= 0),     -- PM-14 N
  min_notice_minutes        integer NOT NULL DEFAULT 0 CHECK (min_notice_minutes >= 0),             -- PM-15 (hours stored as minutes)
  approval                  text NOT NULL DEFAULT 'manager' CHECK (approval IN ('auto_within_limit','manager','manager_then_hr')),  -- PM-16
  timeout_minutes           integer DEFAULT 1440 CHECK (timeout_minutes IS NULL OR timeout_minutes > 0),  -- PM-17
  timeout_action            text NOT NULL DEFAULT 'escalate' CHECK (timeout_action IN ('escalate','auto_approve','auto_reject')),  -- PM-17
  over_limit                text NOT NULL DEFAULT 'block'
                            CHECK (over_limit IN ('block','hr_approval_unpaid','convert_half_day_leave','debit_leave_hourly')),  -- PM-18
  over_stay                 text NOT NULL DEFAULT 'late_early_minutes'
                            CHECK (over_stay IN ('late_early_minutes','half_day_leave','second_permission','flag_only')),  -- PM-19
  same_day_half_leave_allowed boolean NOT NULL DEFAULT false,                                       -- PM-20
  adjacent_off_allowed      boolean NOT NULL DEFAULT true,                                          -- PM-21
  max_consecutive_days      integer CHECK (max_consecutive_days IS NULL OR max_consecutive_days > 0),  -- PM-22; NULL = no cap
  carry_forward_max         integer CHECK (carry_forward_max IS NULL OR carry_forward_max > 0),     -- PM-23; NULL = no carry forward
  mid_period_joiner         text NOT NULL DEFAULT 'full' CHECK (mid_period_joiner IN ('full','prorated')),  -- PM-24
  eligibility               jsonb NOT NULL DEFAULT '{}'::jsonb,   -- PM-25: employee type, category, probation status, notice period
  blackout_enabled          boolean NOT NULL DEFAULT false,                                         -- PM-26
  blackout_dates            date[] NOT NULL DEFAULT '{}',                                           -- PM-26
  punch_requirement         jsonb NOT NULL DEFAULT '{"middle_of_shift": true, "start_of_shift": false, "end_of_shift": false}'::jsonb,  -- PM-27: required per position
  converted_uses_limit      boolean NOT NULL DEFAULT true,                                          -- PM-30
  reason                    text,
  created_at                timestamptz NOT NULL DEFAULT now(),
  created_by                uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, permission_type_id, effective_from),
  FOREIGN KEY (org_id, permission_type_id) REFERENCES perm.permission_type (org_id, id),
  CHECK (max_minutes >= min_minutes),
  CHECK (limit_basis = 'hours' OR limit_count IS NOT NULL),
  CHECK (limit_basis = 'count' OR limit_minutes IS NOT NULL),
  CHECK (apply_window <> 'after_within_days' OR after_event_days IS NOT NULL)
);
COMMENT ON TABLE perm.permission_type_version IS 'Append-only versions of a permission type (PM-02..30, G5). In force on a date = latest effective_from <= date. PM-28 reason list and PM-29 cancel rule are fixed behaviour, not columns.';
SELECT app.register_tenant_table('perm.permission_type_version', true);

CREATE TABLE perm.permission_request (
  org_id                   uuid NOT NULL,
  id                       uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id              uuid NOT NULL,
  permission_type_id       uuid NOT NULL,
  permission_date          date NOT NULL,
  start_time               time NOT NULL,
  end_time                 time NOT NULL,
  minutes                  integer NOT NULL CHECK (minutes > 0),
  origin                   text NOT NULL DEFAULT 'applied' CHECK (origin IN ('applied','converted_late','converted_early')),  -- LA-15, EC-13; approval for converted: GAP-SM-11
  reason                   text NOT NULL,   -- PM-28
  document_path            text,            -- PM-28 attachment optional (TB-02)
  state                    text NOT NULL CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),  -- SM-REQ
  escalated                boolean NOT NULL DEFAULT false,
  decided_basis            text CHECK (decided_basis IN ('approver','auto_on_submit','auto_on_timeout')),
  current_approver_user_id uuid,
  requested_by             uuid,            -- NULL = created by the system (converted late/early)
  submitted_at             timestamptz NOT NULL DEFAULT now(),
  cancelled_by             uuid,
  cancelled_at             timestamptz,
  created_at               timestamptz NOT NULL DEFAULT now(),
  created_by               uuid,
  updated_at               timestamptz NOT NULL DEFAULT now(),
  updated_by               uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, permission_type_id) REFERENCES perm.permission_type (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, cancelled_by) REFERENCES iam.user_account (org_id, id),
  CHECK (end_time <> start_time),           -- overnight shifts: end may be earlier than start
  CHECK (origin <> 'applied' OR requested_by IS NOT NULL),
  CHECK (document_path IS NULL OR starts_with(document_path, org_id::text || '/')),
  CHECK (state NOT IN ('pending_l1','pending_l2') OR decided_basis IS NULL),
  CHECK (state NOT IN ('approved','rejected') OR decided_basis IS NOT NULL),
  CHECK ((state = 'cancelled') = (cancelled_at IS NOT NULL))           -- PM-29
);
CREATE INDEX permission_request_pending ON perm.permission_request (org_id, state, current_approver_user_id) WHERE state IN ('pending_l1','pending_l2');
CREATE INDEX permission_request_employee ON perm.permission_request (org_id, employee_id, permission_date);
SELECT app.register_tenant_table('perm.permission_request');
SELECT app.register_mutable('perm.permission_request');

CREATE TABLE perm.permission_usage_entry (
  org_id             uuid NOT NULL,
  id                 uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id        uuid NOT NULL,
  permission_type_id uuid NOT NULL,
  limit_period       daterange NOT NULL CHECK (NOT isempty(limit_period)),   -- PM-08 period the usage counts against
  entry_kind         text NOT NULL DEFAULT 'usage' CHECK (entry_kind IN ('usage','reversal')),
  origin             text NOT NULL CHECK (origin IN ('request','converted_late','converted_early','opening')),  -- PM-30, OB-R2
  count_delta        integer NOT NULL,
  minutes_delta      integer NOT NULL,
  cause_kind         text NOT NULL,         -- e.g. permission_request, opening_balance_entry; no FK (polymorphic)
  cause_id           uuid NOT NULL,
  reverses_entry_id  uuid,
  reason             text,
  created_at         timestamptz NOT NULL DEFAULT now(),
  created_by         uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, cause_kind, cause_id, entry_kind),                 -- idempotent posting (DATA_MODEL §5.2)
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, permission_type_id) REFERENCES perm.permission_type (org_id, id),
  FOREIGN KEY (org_id, reverses_entry_id) REFERENCES perm.permission_usage_entry (org_id, id),
  CHECK ((entry_kind = 'reversal') = (reverses_entry_id IS NOT NULL)),
  CHECK (count_delta <> 0 OR minutes_delta <> 0)
);
CREATE INDEX permission_usage_entry_lookup ON perm.permission_usage_entry (org_id, employee_id, permission_type_id, limit_period);
COMMENT ON TABLE perm.permission_usage_entry IS 'Append-only PermissionAccount usage ledger (DOMAIN_MODEL §3.10). Usage = sum per employee, type and limit period. On-duty never posts here (OD-12). Reservation by pending requests: OQ-C09-04.';
SELECT app.register_tenant_table('perm.permission_usage_entry', true);

CREATE TABLE perm.on_duty_request (
  org_id                   uuid NOT NULL,
  id                       uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id              uuid NOT NULL,
  duration_kind            text NOT NULL CHECK (duration_kind IN ('hours','half_day','full_day','several_days')),  -- OD-02
  from_date                date NOT NULL,
  to_date                  date NOT NULL,
  from_time                time,            -- set for duration_kind = hours
  to_time                  time,
  purpose                  text NOT NULL,   -- OD-06
  place                    text NOT NULL,   -- OD-06
  proof_path               text,            -- OD-06 (optional or mandatory per setting), TB-02
  state                    text NOT NULL CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),  -- SM-REQ
  escalated                boolean NOT NULL DEFAULT false,
  decided_basis            text CHECK (decided_basis IN ('approver','auto_on_submit','auto_on_timeout')),
  current_approver_user_id uuid,
  requested_by             uuid NOT NULL,
  submitted_at             timestamptz NOT NULL DEFAULT now(),
  cancelled_by             uuid,
  cancelled_at             timestamptz,
  created_at               timestamptz NOT NULL DEFAULT now(),
  created_by               uuid,
  updated_at               timestamptz NOT NULL DEFAULT now(),
  updated_by               uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, requested_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, cancelled_by) REFERENCES iam.user_account (org_id, id),
  CHECK (to_date >= from_date),
  CHECK (duration_kind = 'several_days' OR to_date = from_date),
  CHECK ((duration_kind = 'hours') = (from_time IS NOT NULL AND to_time IS NOT NULL)),
  CHECK (proof_path IS NULL OR starts_with(proof_path, org_id::text || '/')),
  CHECK (state NOT IN ('pending_l1','pending_l2') OR decided_basis IS NULL),
  CHECK (state NOT IN ('approved','rejected') OR decided_basis IS NOT NULL),
  CHECK ((state = 'cancelled') = (cancelled_at IS NOT NULL))           -- who may cancel: OQ-C09-10
);
CREATE INDEX on_duty_request_pending ON perm.on_duty_request (org_id, state, current_approver_user_id) WHERE state IN ('pending_l1','pending_l2');
CREATE INDEX on_duty_request_employee ON perm.on_duty_request (org_id, employee_id, from_date);
COMMENT ON TABLE perm.on_duty_request IS 'OD-02..07. Never linked to permission usage (OD-12): no FK to perm.permission_usage_entry by design. Half-day part, OD-07 punch evidence: OQ-C09-12, OQ-C09-13.';
SELECT app.register_tenant_table('perm.on_duty_request');
SELECT app.register_mutable('perm.on_duty_request');

-- ===========================================================================
-- Shared approval steps — DOMAIN_MODEL §6, SM-REQ
-- ===========================================================================
CREATE TABLE app.approval_step (
  org_id               uuid NOT NULL,
  id                   uuid NOT NULL DEFAULT gen_random_uuid(),
  request_type         text NOT NULL CHECK (request_type IN ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance')),  -- = iam.approver_rule
  request_id           uuid NOT NULL,       -- no FK: polymorphic over request tables
  level                smallint NOT NULL CHECK (level IN (1,2)),
  action               text NOT NULL CHECK (action IN ('submitted','approved','rejected','escalated','cancelled','auto_approved','auto_rejected')),
  requester_user_id    uuid NOT NULL,       -- copied from the request so BR-07 is checked on this row
  actor_user_id        uuid,                -- NULL = system job (J3)
  on_behalf_of_user_id uuid,                -- delegation (AC-03)
  comment              text,
  acted_at             timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, requester_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, actor_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, on_behalf_of_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK (action NOT IN ('approved','rejected') OR actor_user_id IS NOT NULL),             -- manual decisions name a person
  CHECK (action NOT IN ('approved','rejected') OR actor_user_id <> requester_user_id),   -- BR-07
  CHECK (on_behalf_of_user_id IS DISTINCT FROM requester_user_id),                       -- BR-07 through delegation
  CHECK (on_behalf_of_user_id IS NULL OR (actor_user_id IS NOT NULL AND actor_user_id <> on_behalf_of_user_id))
);
CREATE INDEX approval_step_request ON app.approval_step (org_id, request_type, request_id, acted_at);
COMMENT ON TABLE app.approval_step IS 'Append-only: one row per approval action on any request (DOMAIN_MODEL §6, SM-REQ). BR-07 enforced on the row. Approver equal to requester at routing: OQ-C03-08.';
SELECT app.register_tenant_table('app.approval_step', true);

-- ===========================================================================
-- Notifications (C14) — DOMAIN_MODEL §3.13
-- ===========================================================================
CREATE TABLE notify.notification_rule (
  org_id         uuid NOT NULL REFERENCES platform.organisation (id),
  event_code     text NOT NULL CHECK (event_code ~ '^NT-(0[1-9]|1[0-4])$'),   -- §9.14; NT-12, NT-14 are V2 events
  channel        text NOT NULL CHECK (channel IN ('in_app','email','sms','whatsapp','push')),  -- push from TSD §14: OQ-C14-02
  recipient_kind text NOT NULL,             -- e.g. employee, manager, hr, payroll; list not fixed by BRD (OQ-C14-01)
  enabled        boolean NOT NULL,          -- defaults: OQ-C14-01
  updated_at     timestamptz NOT NULL DEFAULT now(),
  updated_by     uuid,
  PRIMARY KEY (org_id, event_code, channel, recipient_kind)
);
SELECT app.register_tenant_table('notify.notification_rule');
SELECT app.register_mutable('notify.notification_rule');

CREATE TABLE notify.notification (
  org_id            uuid NOT NULL,
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  event_code        text NOT NULL CHECK (event_code ~ '^NT-(0[1-9]|1[0-4])$'),
  recipient_user_id uuid NOT NULL,
  channel           text NOT NULL CHECK (channel IN ('in_app','email','sms','whatsapp','push')),
  language          text NOT NULL CHECK (language IN ('en','ta','hi')),   -- UM-R4, N7
  payload           jsonb NOT NULL DEFAULT '{}'::jsonb,
  state             text NOT NULL DEFAULT 'queued' CHECK (state IN ('queued','sent','failed')),
  attempts          integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  dedupe_key        text NOT NULL,
  created_at        timestamptz NOT NULL DEFAULT now(),
  sent_at           timestamptz,
  updated_at        timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, dedupe_key),               -- idempotent enqueue (TB-03)
  FOREIGN KEY (org_id, recipient_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK ((state = 'sent') = (sent_at IS NOT NULL))
);
CREATE INDEX notification_recipient ON notify.notification (org_id, recipient_user_id, created_at);
SELECT app.register_tenant_table('notify.notification');
SELECT app.register_mutable('notify.notification');
