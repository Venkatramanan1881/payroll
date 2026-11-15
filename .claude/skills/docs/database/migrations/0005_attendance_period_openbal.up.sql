-- 0004 Attendance, Period, Opening Balances. DOMAIN_MODEL §3.8, §3.11, §3.12, §7; STATE_MACHINES SM-DAY, SM-REQ, SM-PERIOD, SM-ADJ, SM-OB.
-- Conventions: docs/database/DATA_MODEL.md §1. Org-owned tables: PK (org_id, id), composite FKs, register_tenant_table().
-- References tables from 0003 (timecap, sched, policy) and 0005 (leave, perm) by their DATA_MODEL §3 names.

CREATE SCHEMA attendance;
CREATE SCHEMA period;
CREATE SCHEMA openbal;
GRANT USAGE ON SCHEMA attendance, period, openbal TO app_api, app_jobs, app_gateway;

-- ===========================================================================
-- Period (C12) — created first: the day-result lock trigger reads period.lock_coverage.
-- ===========================================================================

-- DP-01: who owns the cycle is open, so the owner is (kind, id). No FK on owner_id because it is polymorphic;
-- the application checks it. owner_kind = 'organisation' => owner_id = org_id. V2 adds 'payroll_group'.
CREATE TABLE period.attendance_cycle (
  org_id           uuid NOT NULL REFERENCES platform.organisation (id),
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  owner_kind       text NOT NULL CHECK (owner_kind IN ('organisation','branch','policy_set')),   -- DP-01
  owner_id         uuid NOT NULL,
  frequency        text NOT NULL DEFAULT 'monthly' CHECK (frequency IN ('daily','weekly','bi_weekly','monthly')),  -- PL-01
  start_kind       text NOT NULL DEFAULT 'calendar' CHECK (start_kind IN ('calendar','custom')),  -- PL-01 calendar month / custom cycle (e.g. 26th to 25th)
  custom_start_day smallint CHECK (custom_start_day BETWEEN 1 AND 31),                            -- PL-01
  cutoff_day       smallint CHECK (cutoff_day BETWEEN 1 AND 31),                                  -- PL-02; default when unset: OQ-C12-09
  valid            daterange NOT NULL CHECK (NOT isempty(valid)),                                 -- PL-01 change and transition period: OQ-C12-09
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  updated_at       timestamptz NOT NULL DEFAULT now(),
  updated_by       uuid,
  PRIMARY KEY (org_id, id),
  CHECK ((start_kind = 'custom') = (custom_start_day IS NOT NULL)),
  EXCLUDE USING gist (org_id WITH =, owner_kind WITH =, owner_id WITH =, valid WITH &&)
);
COMMENT ON TABLE period.attendance_cycle IS 'Attendance period cycle (PL-01, PL-02). Owner is open (DP-01); locks are materialised per employee in lock_coverage, so the answer does not change the lock rule.';
SELECT app.register_tenant_table('period.attendance_cycle');
SELECT app.register_mutable('period.attendance_cycle');

CREATE TABLE period.attendance_period (
  org_id             uuid NOT NULL,
  id                 uuid NOT NULL DEFAULT gen_random_uuid(),
  cycle_id           uuid NOT NULL,
  period_start       date NOT NULL,
  period_end         date NOT NULL,
  cutoff_date        date,                                                                 -- PL-02
  state              text NOT NULL DEFAULT 'open'
                     CHECK (state IN ('open','past_cut_off','awaiting_lock','locked','unlocked')),  -- SM-PERIOD
  checklist_snapshot jsonb,                                                                -- PL-R2, stored at lock (P5)
  locked_at          timestamptz,
  locked_by          uuid,                                                                 -- NULL = automatic lock (PL-04)
  created_at         timestamptz NOT NULL DEFAULT now(),
  created_by         uuid,
  updated_at         timestamptz NOT NULL DEFAULT now(),
  updated_by         uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, cycle_id) REFERENCES period.attendance_cycle (org_id, id),
  FOREIGN KEY (org_id, locked_by) REFERENCES iam.user_account (org_id, id),
  CHECK (period_end >= period_start),
  CHECK (cutoff_date IS NULL OR cutoff_date BETWEEN period_start AND period_end),        -- PL-02 "within the period"
  CHECK (state <> 'locked' OR locked_at IS NOT NULL),
  EXCLUDE USING gist (org_id WITH =, cycle_id WITH =, daterange(period_start, period_end, '[]') WITH &&)
);
CREATE INDEX attendance_period_state ON period.attendance_period (org_id, state);
COMMENT ON TABLE period.attendance_period IS 'SM-PERIOD. Lock before period end: GAP-SM-12. Unlock effects on posted adjustments/exports: OQ-C12-07.';
SELECT app.register_tenant_table('period.attendance_period');
SELECT app.register_mutable('period.attendance_period');

CREATE TABLE period.period_state_history (
  org_id              uuid NOT NULL,
  id                  uuid NOT NULL DEFAULT gen_random_uuid(),
  period_id           uuid NOT NULL,
  from_state          text CHECK (from_state IN ('open','past_cut_off','awaiting_lock','locked','unlocked')),
  to_state            text NOT NULL CHECK (to_state IN ('open','past_cut_off','awaiting_lock','locked','unlocked')),
  reason              text,
  changed_by          uuid,                       -- NULL = system (J7, automatic lock)
  approved_by         uuid,                       -- Organisation Admin for unlock (PL-06)
  created_at          timestamptz NOT NULL DEFAULT now(),
  created_by          uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, changed_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, approved_by) REFERENCES iam.user_account (org_id, id),
  CHECK (to_state <> 'unlocked' OR (reason IS NOT NULL AND approved_by IS NOT NULL)),    -- PL-06
  CHECK (approved_by IS NULL OR approved_by IS DISTINCT FROM changed_by)                  -- BR-07; requester of an unlock is not stated (SM-PERIOD P7)
);
CREATE INDEX period_state_history_period ON period.period_state_history (org_id, period_id, created_at);
SELECT app.register_tenant_table('period.period_state_history', true);

-- Materialised lock per employee (DATA_MODEL §5.3, BR-06). Written at lock (P5/P8), deleted on unlock (P7), so mutable.
CREATE TABLE period.lock_coverage (
  org_id      uuid NOT NULL,
  period_id   uuid NOT NULL,
  employee_id uuid NOT NULL,
  dates       daterange NOT NULL CHECK (NOT isempty(dates)),
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, period_id, employee_id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id)
);
CREATE INDEX lock_coverage_lookup ON period.lock_coverage USING gist (org_id, employee_id, dates);
SELECT app.register_tenant_table('period.lock_coverage');

CREATE FUNCTION period.is_locked(p_org uuid, p_employee uuid, p_date date) RETURNS boolean
LANGUAGE sql STABLE AS $$
  SELECT EXISTS (SELECT 1 FROM period.lock_coverage
                 WHERE org_id = p_org AND employee_id = p_employee AND dates @> p_date)
$$;

-- ===========================================================================
-- Attendance (C06)
-- ===========================================================================
CREATE TABLE attendance.attendance_day (
  org_id            uuid NOT NULL,
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id       uuid NOT NULL,
  attendance_date   date NOT NULL,
  processing_state  text NOT NULL DEFAULT 'provisional'
                    CHECK (processing_state IN ('provisional','exception','absence_pending','finalised','locked')),  -- SM-DAY; GAP-SM-07, DP-06
  current_result_id uuid,                        -- the only update besides processing_state (DATA_MODEL §5.2, BRX-03)
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  updated_at        timestamptz NOT NULL DEFAULT now(),
  updated_by        uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, employee_id, attendance_date),
  UNIQUE (org_id, id, employee_id, attendance_date),   -- target of day_result's consistency FK
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id)
);
CREATE INDEX attendance_day_date ON attendance.attendance_day (org_id, attendance_date);   -- R1, R2, DB-R3
SELECT app.register_tenant_table('attendance.attendance_day');
SELECT app.register_mutable('attendance.attendance_day');

-- Append-only result history (BRX-03, BR-08). Outputs contain no money (BRX-02).
CREATE TABLE attendance.day_result (
  org_id               uuid NOT NULL,
  id                   uuid NOT NULL DEFAULT gen_random_uuid(),
  attendance_day_id    uuid NOT NULL,
  employee_id          uuid NOT NULL,              -- denormalised, checked by FK to attendance_day
  attendance_date      date NOT NULL,              -- denormalised for the lock trigger, checked by FK
  supersedes_result_id uuid,
  for_adjustment       boolean NOT NULL DEFAULT false,   -- recalculated for a locked date only to measure a PeriodAdjustment (SM-DAY A10); never current
  -- inputs
  policy_version_id    uuid NOT NULL,              -- BR-05; several versions on one day: explanation lists them
  shift_version_id     uuid,                       -- NULL when no shift applies (weekly off, holiday, hours-only)
  schedule_tz          text NOT NULL CHECK (app.is_valid_timezone(schedule_tz)),   -- DP-04: zone actually used
  costed_branch_id     uuid,                       -- EM-08 branch the day is costed to
  input_fingerprint    bytea NOT NULL,             -- BR-02, AT5: identical fingerprint => re-run skipped
  engine_version       text NOT NULL,
  trigger_kind         text NOT NULL,              -- e.g. first_input, j1_finalise, recalculation, cut_off; vocabulary not fixed by BRD/SM-DAY
  trigger_ref          uuid,                       -- the input/request/job that caused it
  produced_at          timestamptz NOT NULL DEFAULT now(),
  -- outputs
  gross_minutes        integer NOT NULL DEFAULT 0 CHECK (gross_minutes >= 0),
  break_minutes        integer NOT NULL DEFAULT 0 CHECK (break_minutes >= 0),
  net_minutes          integer NOT NULL DEFAULT 0 CHECK (net_minutes >= 0),
  late_minutes         integer NOT NULL DEFAULT 0 CHECK (late_minutes >= 0),
  early_minutes        integer NOT NULL DEFAULT 0 CHECK (early_minutes >= 0),
  early_arrival_minutes integer NOT NULL DEFAULT 0 CHECK (early_arrival_minutes >= 0),          -- EA
  overtime_identified_minutes integer NOT NULL DEFAULT 0 CHECK (overtime_identified_minutes >= 0),  -- computed only (DP-11)
  late_mark            boolean NOT NULL DEFAULT false,
  early_mark           boolean NOT NULL DEFAULT false,                                          -- never for a missing check-out (BR-09)
  -- DS-01..07, §13 E2/E6/E12/E16. 'quarter_day' only when DS-06 on. Comp-off taken = 'leave' (CO-R2).
  -- Uncertain: optional holiday (SH) and absconding days have no own status in the BRD; they use holiday/absent.
  day_status           text NOT NULL CHECK (day_status IN ('present','half_day','quarter_day','absent','leave','half_day_leave','holiday','weekly_off','on_duty')),
  day_fraction_present numeric(5,2) NOT NULL CHECK (day_fraction_present BETWEEN 0 AND 1),  -- not forced to 0.25 steps: DS-07 "pay in proportion"
  flags                text[] NOT NULL DEFAULT '{}',
  sessions             jsonb NOT NULL DEFAULT '[]'::jsonb,   -- ordered in/out pairs with assigned directions
  explanation          jsonb NOT NULL,                       -- ordered steps: rule, version, punches, approvals (BR-08)
  created_at           timestamptz NOT NULL DEFAULT now(),
  created_by           uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, attendance_day_id, id),                     -- target of attendance_day.current_result FK
  UNIQUE (org_id, id, employee_id, attendance_date),          -- target of absence_classification FK
  FOREIGN KEY (org_id, attendance_day_id, employee_id, attendance_date)
    REFERENCES attendance.attendance_day (org_id, id, employee_id, attendance_date),
  FOREIGN KEY (org_id, attendance_day_id, supersedes_result_id) REFERENCES attendance.day_result (org_id, attendance_day_id, id),
  FOREIGN KEY (org_id, policy_version_id) REFERENCES policy.policy_version (org_id, id),
  FOREIGN KEY (org_id, shift_version_id) REFERENCES sched.shift_version (org_id, id),
  FOREIGN KEY (org_id, costed_branch_id) REFERENCES org.branch (org_id, id),
  CHECK (jsonb_typeof(explanation) = 'array' AND jsonb_typeof(sessions) = 'array')
);
CREATE INDEX day_result_day ON attendance.day_result (org_id, attendance_day_id, produced_at);
CREATE INDEX day_result_emp_date ON attendance.day_result (org_id, employee_id, attendance_date);
COMMENT ON TABLE attendance.day_result IS 'Append-only (BRX-03). Inserts for a date inside period.lock_coverage are refused unless for_adjustment (BR-06, DATA_MODEL §5.3). Overlay references and ExpectedSchedule snapshot are recorded in the explanation steps.';
SELECT app.register_tenant_table('attendance.day_result', true);

ALTER TABLE attendance.attendance_day
  ADD CONSTRAINT attendance_day_current_result
  FOREIGN KEY (org_id, id, current_result_id) REFERENCES attendance.day_result (org_id, attendance_day_id, id)
  DEFERRABLE INITIALLY DEFERRED;

-- BR-06: no result for a locked date except one measured for an adjustment.
CREATE FUNCTION attendance.reject_locked_result() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF NOT NEW.for_adjustment AND period.is_locked(NEW.org_id, NEW.employee_id, NEW.attendance_date) THEN
    RAISE EXCEPTION 'attendance date % is in a locked period; post a period_adjustment instead (BR-06)', NEW.attendance_date
      USING ERRCODE = 'check_violation';
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER reject_locked_result BEFORE INSERT ON attendance.day_result
  FOR EACH ROW EXECUTE FUNCTION attendance.reject_locked_result();

-- BR-06: the current result of a locked date cannot move; an adjustment-only result is never current.
CREATE FUNCTION attendance.guard_current_result() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF NEW.current_result_id IS NOT NULL THEN
    IF period.is_locked(NEW.org_id, NEW.employee_id, NEW.attendance_date) THEN
      RAISE EXCEPTION 'attendance date % is in a locked period; current result cannot change (BR-06)', NEW.attendance_date
        USING ERRCODE = 'check_violation';
    END IF;
    IF EXISTS (SELECT 1 FROM attendance.day_result r
               WHERE r.org_id = NEW.org_id AND r.id = NEW.current_result_id AND r.for_adjustment) THEN
      RAISE EXCEPTION 'an adjustment-only day result cannot be current' USING ERRCODE = 'check_violation';
    END IF;
  END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER guard_current_result_ins BEFORE INSERT ON attendance.attendance_day
  FOR EACH ROW EXECUTE FUNCTION attendance.guard_current_result();
CREATE TRIGGER guard_current_result_upd BEFORE UPDATE OF current_result_id, employee_id, attendance_date ON attendance.attendance_day
  FOR EACH ROW WHEN (NEW.current_result_id IS DISTINCT FROM OLD.current_result_id
                     OR NEW.employee_id <> OLD.employee_id OR NEW.attendance_date <> OLD.attendance_date)
  EXECUTE FUNCTION attendance.guard_current_result();

-- Day effects (fractions of deduction days or minutes, with reason). Append-only with their result.
-- "One shortfall produces at most one pay effect; the larger applies" (BR-04) is decided by the engine, not the schema.
CREATE TABLE attendance.day_effect (
  org_id      uuid NOT NULL,
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  result_id   uuid NOT NULL,
  effect_kind text NOT NULL CHECK (effect_kind IN (
                'late_slab',            -- LA-05
                'early_slab',           -- EC-04
                'missed_punch',         -- MP-01, MP-02, MP-03
                'hours_shortfall',      -- DS-02..07
                'absence_lop',          -- DS-10, LM-28
                'uninformed_absence_cut')),  -- LM-21, LM-22
  days        numeric(5,2) CHECK (days >= 0),
  minutes     integer CHECK (minutes >= 0),          -- "deduct actual minutes" (LA-05, EC-04, DS-05)
  reason_code text NOT NULL,
  rule_ref    text NOT NULL,                         -- setting ID, e.g. LA-05
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, result_id) REFERENCES attendance.day_result (org_id, id),
  CHECK (days IS NOT NULL OR minutes IS NOT NULL)
);
CREATE INDEX day_effect_result ON attendance.day_effect (org_id, result_id);
SELECT app.register_tenant_table('attendance.day_effect', true);

-- Punches used by a result, with assigned direction and pairing (BR-08, BR-01: punches are referenced, never edited).
CREATE TABLE attendance.day_result_punch (
  org_id             uuid NOT NULL,
  result_id          uuid NOT NULL,
  raw_punch_id       uuid NOT NULL,
  assigned_direction text CHECK (assigned_direction IN ('in','out')),
  session_no         smallint CHECK (session_no > 0),
  excluded           boolean NOT NULL DEFAULT false,
  excluded_reason    text,                          -- e.g. duplicate, flag not accepted (DP-05)
  created_at         timestamptz NOT NULL DEFAULT now(),
  created_by         uuid,
  PRIMARY KEY (org_id, result_id, raw_punch_id),
  FOREIGN KEY (org_id, result_id) REFERENCES attendance.day_result (org_id, id),
  FOREIGN KEY (org_id, raw_punch_id) REFERENCES timecap.raw_punch (org_id, id),
  CHECK (excluded = (excluded_reason IS NOT NULL)),
  CHECK (excluded OR assigned_direction IS NOT NULL)
);
CREATE INDEX day_result_punch_punch ON attendance.day_result_punch (org_id, raw_punch_id);
SELECT app.register_tenant_table('attendance.day_result_punch', true);

-- RegularisationRequest (RG-01..11) and HR direct correction (RG-10, DP-08). SM-REQ; no draft (GAP-SM-08).
-- Approved requests produce a manual raw_punch or are read by the engine as an override; punches are never edited (BR-01).
CREATE TABLE attendance.regularisation_request (
  org_id                   uuid NOT NULL,
  id                       uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id              uuid NOT NULL,
  attendance_date          date NOT NULL,
  request_type             text NOT NULL CHECK (request_type IN ('missed_punch','late_arrival','early_checkout','wrong_punch','whole_day')),  -- RG-02
  origin                   text NOT NULL DEFAULT 'employee' CHECK (origin IN ('employee','hr_direct')),  -- DP-08: approval path for hr_direct not fixed here
  state                    text NOT NULL DEFAULT 'pending_l1'
                           CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),  -- SM-REQ
  escalated                boolean NOT NULL DEFAULT false,                                                    -- SM-REQ R7
  decided_basis            text CHECK (decided_basis IN ('manual','auto_on_submit','auto_on_timeout')),     -- SM-REQ R2/R5/R8/R9
  proposed_in_at           timestamptz,
  proposed_out_at          timestamptz,
  target_raw_punch_id      uuid,                                  -- punch disputed by a wrong_punch request
  reason                   text NOT NULL CHECK (btrim(reason) <> ''),   -- RG-09
  attachment_path          text,                                  -- RG-09; path starts with org_id (TB-02)
  requested_by_user_id     uuid NOT NULL,
  current_approver_user_id uuid,
  decided_by_user_id       uuid,
  decided_at               timestamptz,
  created_at               timestamptz NOT NULL DEFAULT now(),
  created_by               uuid,
  updated_at               timestamptz NOT NULL DEFAULT now(),
  updated_by               uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, target_raw_punch_id) REFERENCES timecap.raw_punch (org_id, id),
  FOREIGN KEY (org_id, requested_by_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, current_approver_user_id) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, decided_by_user_id) REFERENCES iam.user_account (org_id, id),
  CHECK (proposed_out_at IS NULL OR proposed_in_at IS NULL OR proposed_out_at > proposed_in_at),
  CHECK (state NOT IN ('approved','rejected') OR decided_basis IS NOT NULL),
  CHECK (decided_basis <> 'auto_on_submit' OR state IN ('approved','cancelled')),
  CHECK (origin = 'hr_direct' OR decided_by_user_id IS DISTINCT FROM requested_by_user_id)   -- BR-07 (delegates: ApprovalService)
);
CREATE INDEX regularisation_pending ON attendance.regularisation_request (org_id, current_approver_user_id, state)
  WHERE state IN ('pending_l1','pending_l2');                                                        -- DB-R5, J3
CREATE INDEX regularisation_emp_date ON attendance.regularisation_request (org_id, employee_id, attendance_date);
COMMENT ON TABLE attendance.regularisation_request IS 'SM-REQ. Cancellation rules: GAP-SM-09. Pending at lock: OQ-C12-03. Window RG-04 and limits RG-03/RG-08 are application checks.';
SELECT app.register_tenant_table('attendance.regularisation_request');
SELECT app.register_mutable('attendance.regularisation_request');

-- LA-17, EC-16: bulk waiver by HR for a date and a location or shift. Create/withdraw triggers recalculation (E14).
CREATE TABLE attendance.bulk_waiver (
  org_id         uuid NOT NULL,
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  waiver_date    date NOT NULL,
  scope_kind     text NOT NULL CHECK (scope_kind IN ('branch','shift')),
  branch_id      uuid,
  shift_id       uuid,
  rule_group     text NOT NULL CHECK (rule_group IN ('late','early')),   -- LA-17, EC-16
  reason         text NOT NULL CHECK (btrim(reason) <> ''),
  withdrawn_at   timestamptz,
  withdrawn_by   uuid,
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  updated_at     timestamptz NOT NULL DEFAULT now(),
  updated_by     uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  FOREIGN KEY (org_id, shift_id) REFERENCES sched.shift (org_id, id),
  FOREIGN KEY (org_id, withdrawn_by) REFERENCES iam.user_account (org_id, id),
  CHECK ((scope_kind = 'branch' AND branch_id IS NOT NULL AND shift_id IS NULL) OR
         (scope_kind = 'shift'  AND shift_id IS NOT NULL AND branch_id IS NULL))
);
CREATE INDEX bulk_waiver_date ON attendance.bulk_waiver (org_id, waiver_date);
SELECT app.register_tenant_table('attendance.bulk_waiver');
SELECT app.register_mutable('attendance.bulk_waiver');

-- PeriodCounterSet: derived, rebuilt from current day results plus opening counters (OB-R2). Mutable.
-- Counter period follows LA-18 / EC-17; combined late+early counter (EC-10) is application logic.
CREATE TABLE attendance.period_counter (
  org_id                uuid NOT NULL,
  id                    uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id           uuid NOT NULL,
  counter_period        daterange NOT NULL CHECK (NOT isempty(counter_period)),
  late_marks            integer NOT NULL DEFAULT 0 CHECK (late_marks >= 0),
  early_marks           integer NOT NULL DEFAULT 0 CHECK (early_marks >= 0),
  late_minutes          integer NOT NULL DEFAULT 0 CHECK (late_minutes >= 0),
  early_minutes         integer NOT NULL DEFAULT 0 CHECK (early_minutes >= 0),
  grace_uses_late       integer NOT NULL DEFAULT 0 CHECK (grace_uses_late >= 0),    -- LA-04
  grace_uses_early      integer NOT NULL DEFAULT 0 CHECK (grace_uses_early >= 0),   -- EC-03
  opening_late_marks    integer NOT NULL DEFAULT 0 CHECK (opening_late_marks >= 0), -- OB-R2
  opening_early_marks   integer NOT NULL DEFAULT 0 CHECK (opening_early_marks >= 0),
  opening_late_minutes  integer NOT NULL DEFAULT 0 CHECK (opening_late_minutes >= 0),
  opening_early_minutes integer NOT NULL DEFAULT 0 CHECK (opening_early_minutes >= 0),
  rebuilt_at            timestamptz NOT NULL DEFAULT now(),
  created_at            timestamptz NOT NULL DEFAULT now(),
  created_by            uuid,
  updated_at            timestamptz NOT NULL DEFAULT now(),
  updated_by            uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  EXCLUDE USING gist (org_id WITH =, employee_id WITH =, counter_period WITH &&)
);
SELECT app.register_tenant_table('attendance.period_counter');
SELECT app.register_mutable('attendance.period_counter');

-- Period-end penalties (BR-10), append-only. A rebuild (SM-PERIOD P4) inserts a new run; the latest run_id per (employee, period) is current.
CREATE TABLE attendance.period_penalty (
  org_id         uuid NOT NULL,
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  run_id         uuid NOT NULL,
  employee_id    uuid NOT NULL,
  period_id      uuid NOT NULL,
  counter_period daterange NOT NULL,
  rule_ref       text NOT NULL CHECK (rule_ref IN ('LA-06','LA-10','EC-05','EC-09','LM-25','LM-26')),  -- LM-25/26 excess absence also applied at period end (§7)
  days           numeric(5,2) CHECK (days >= 0),
  minutes        integer CHECK (minutes >= 0),          -- LA-11 "deduct the actual excess minutes"
  recovered_as   text,                                  -- LA-12 / EC-11 recovery order outcome, e.g. leave type then LOP (E5)
  explanation    jsonb NOT NULL,                        -- BR-08
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  CHECK (days IS NOT NULL OR minutes IS NOT NULL)
);
CREATE INDEX period_penalty_emp ON attendance.period_penalty (org_id, period_id, employee_id, created_at);
SELECT app.register_tenant_table('attendance.period_penalty', true);

-- LM-R6 / LM-27: one classification per day result; the current one belongs to the day's current result. Append-only.
CREATE TABLE attendance.absence_classification (
  org_id               uuid NOT NULL,
  id                   uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id          uuid NOT NULL,
  absence_date         date NOT NULL,
  classification       text NOT NULL CHECK (classification IN ('informed_leave','unpaid_leave','uninformed_absence')),  -- LM-R6
  decided_by_result_id uuid NOT NULL,
  created_at           timestamptz NOT NULL DEFAULT now(),
  created_by           uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, decided_by_result_id),
  FOREIGN KEY (org_id, decided_by_result_id, employee_id, absence_date)
    REFERENCES attendance.day_result (org_id, id, employee_id, attendance_date)
);
CREATE INDEX absence_classification_emp ON attendance.absence_classification (org_id, employee_id, absence_date);
SELECT app.register_tenant_table('attendance.absence_classification', true);

-- ===========================================================================
-- Period: totals, adjustments, exports
-- ===========================================================================

-- PeriodTotals frozen at lock (PL-R3). Append-only; a re-lock after unlock (P8) inserts a new row, latest frozen_at is current.
CREATE TABLE period.period_totals (
  org_id                      uuid NOT NULL,
  id                          uuid NOT NULL DEFAULT gen_random_uuid(),
  period_id                   uuid NOT NULL,
  employee_id                 uuid NOT NULL,
  payable_days                numeric(5,2) NOT NULL CHECK (payable_days >= 0),
  deduction_days              numeric(5,2) NOT NULL CHECK (deduction_days >= 0),
  deduction_lines             jsonb NOT NULL DEFAULT '[]'::jsonb,   -- by reason line with explanation refs (§7, BR-08)
  late_minutes                integer NOT NULL DEFAULT 0 CHECK (late_minutes >= 0),    -- after grace and waivers (PD-07)
  early_minutes               integer NOT NULL DEFAULT 0 CHECK (early_minutes >= 0),
  overtime_identified_minutes integer NOT NULL DEFAULT 0 CHECK (overtime_identified_minutes >= 0),
  branch_days                 jsonb NOT NULL DEFAULT '{}'::jsonb,   -- EM-08 day counts per costed branch
  pay_hold                    boolean NOT NULL DEFAULT false,       -- LM-31 absconding pay-hold flag (§7)
  frozen_at                   timestamptz NOT NULL DEFAULT now(),
  created_at                  timestamptz NOT NULL DEFAULT now(),
  created_by                  uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id)
);
CREATE INDEX period_totals_lookup ON period.period_totals (org_id, period_id, employee_id, frozen_at);
SELECT app.register_tenant_table('period.period_totals', true);

-- PeriodAdjustment (PL-R4, E15, AT8). Difference only, no money (BRX-02). SM-ADJ.
CREATE TABLE period.period_adjustment (
  org_id           uuid NOT NULL,
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id      uuid NOT NULL,
  original_date    date NOT NULL,
  source_period_id uuid NOT NULL,                 -- locked period holding original_date; PL-07 limit (OQ-C12-04)
  locked_result_id uuid NOT NULL,
  new_result_id    uuid NOT NULL,                 -- a day_result with for_adjustment = true
  difference       jsonb NOT NULL,
  days             numeric(5,2) NOT NULL,         -- signed difference in payable days
  target_period_id uuid NOT NULL,
  state            text NOT NULL DEFAULT 'posted' CHECK (state IN ('posted','frozen','exported')),  -- SM-ADJ
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  updated_at       timestamptz NOT NULL DEFAULT now(),
  updated_by       uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, employee_id, original_date, new_result_id),         -- SM-ADJ idempotency
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, source_period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, target_period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, locked_result_id, employee_id, original_date) REFERENCES attendance.day_result (org_id, id, employee_id, attendance_date),
  FOREIGN KEY (org_id, new_result_id, employee_id, original_date)    REFERENCES attendance.day_result (org_id, id, employee_id, attendance_date),
  CHECK (target_period_id <> source_period_id),
  CHECK (new_result_id <> locked_result_id)
);
CREATE INDEX period_adjustment_target ON period.period_adjustment (org_id, target_period_id, employee_id);
COMMENT ON TABLE period.period_adjustment IS 'SM-ADJ. Only state changes after insert (posted -> frozen -> exported). Repeat change of the same date: OQ-C12-06; source unlocked: OQ-C12-07.';
SELECT app.register_tenant_table('period.period_adjustment');
SELECT app.register_mutable('period.period_adjustment');

-- Immutable snapshot for MD-06 / Payroll V2 (DOMAIN_MODEL §7). Re-production rules: GAP-SM-12.
CREATE TABLE period.payroll_input_export (
  org_id           uuid NOT NULL,
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  period_id        uuid NOT NULL,
  contract_version text NOT NULL DEFAULT 'PayrollInput v1' CHECK (contract_version IN ('PayrollInput v1')),
  file_path        text NOT NULL,                 -- path starts with org_id (TB-02)
  checksum         text NOT NULL,
  produced_by      uuid,                          -- NULL = system
  produced_at      timestamptz NOT NULL DEFAULT now(),
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, period_id) REFERENCES period.attendance_period (org_id, id),
  FOREIGN KEY (org_id, produced_by) REFERENCES iam.user_account (org_id, id)
);
CREATE INDEX payroll_input_export_period ON period.payroll_input_export (org_id, period_id);
SELECT app.register_tenant_table('period.payroll_input_export', true);

CREATE TABLE period.payroll_input_export_adjustment (
  org_id        uuid NOT NULL,
  export_id     uuid NOT NULL,
  adjustment_id uuid NOT NULL,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, export_id, adjustment_id),
  FOREIGN KEY (org_id, export_id) REFERENCES period.payroll_input_export (org_id, id),
  FOREIGN KEY (org_id, adjustment_id) REFERENCES period.period_adjustment (org_id, id)
);
CREATE INDEX payroll_input_export_adjustment_adj ON period.payroll_input_export_adjustment (org_id, adjustment_id);
SELECT app.register_tenant_table('period.payroll_input_export_adjustment', true);

-- ===========================================================================
-- Opening Balances (C15)
-- ===========================================================================
CREATE TABLE openbal.opening_balance_batch (
  org_id       uuid NOT NULL REFERENCES platform.organisation (id),
  id           uuid NOT NULL DEFAULT gen_random_uuid(),
  state        text NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','pending_approval','confirmed')),  -- SM-OB; reject outcome GAP-SM-13
  upload_path  text,                              -- OB-R3 spreadsheet upload; error report handling OQ-C15-09
  submitted_by uuid,                              -- B4, OB-01 = with OA approval
  confirmed_by uuid,
  confirmed_at timestamptz,
  created_at   timestamptz NOT NULL DEFAULT now(),
  created_by   uuid,
  updated_at   timestamptz NOT NULL DEFAULT now(),
  updated_by   uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, submitted_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, confirmed_by) REFERENCES iam.user_account (org_id, id),
  CHECK ((state = 'confirmed') = (confirmed_at IS NOT NULL AND confirmed_by IS NOT NULL)),  -- OB-R5
  CHECK (submitted_by IS NULL OR confirmed_by IS DISTINCT FROM submitted_by)               -- BR-07 (B5)
);
SELECT app.register_tenant_table('openbal.opening_balance_batch');
SELECT app.register_mutable('openbal.opening_balance_batch');

CREATE TABLE openbal.opening_balance_entry (
  org_id                 uuid NOT NULL,
  id                     uuid NOT NULL DEFAULT gen_random_uuid(),
  batch_id               uuid NOT NULL,
  employee_id            uuid NOT NULL,
  balance_kind           text NOT NULL CHECK (balance_kind IN ('leave','comp_off','permission_used','late_counter','early_counter')),  -- OB-R2 (advances/loans: V2)
  leave_type_id          uuid,
  permission_type_id     uuid,
  value                  numeric(7,2) NOT NULL,   -- days for leave/comp-off, count for permissions; counters: marks vs minutes not settled (OQ-C15-03)
  as_of_date             date NOT NULL,           -- OB-R4
  superseded_by_entry_id uuid,                    -- OB-02: change = new entry + pointer, nothing overwritten
  change_reason          text,                    -- OB-02, OB-R5
  created_at             timestamptz NOT NULL DEFAULT now(),
  created_by             uuid,
  updated_at             timestamptz NOT NULL DEFAULT now(),
  updated_by             uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, batch_id) REFERENCES openbal.opening_balance_batch (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, leave_type_id) REFERENCES leave.leave_type (org_id, id),
  FOREIGN KEY (org_id, permission_type_id) REFERENCES perm.permission_type (org_id, id),
  FOREIGN KEY (org_id, superseded_by_entry_id) REFERENCES openbal.opening_balance_entry (org_id, id)
    DEFERRABLE INITIALLY DEFERRED,   -- set the pointer first, then insert the replacement (live-entry unique index)
  CHECK ((balance_kind = 'leave') = (leave_type_id IS NOT NULL)),
  CHECK ((balance_kind = 'permission_used') = (permission_type_id IS NOT NULL)),
  CHECK (superseded_by_entry_id IS NULL OR superseded_by_entry_id <> id)
);
CREATE UNIQUE INDEX opening_balance_entry_live ON openbal.opening_balance_entry
  (org_id, batch_id, employee_id, balance_kind, coalesce(leave_type_id, permission_type_id, '00000000-0000-0000-0000-000000000000'::uuid))
  WHERE superseded_by_entry_id IS NULL;
CREATE INDEX opening_balance_entry_emp ON openbal.opening_balance_entry (org_id, employee_id);
SELECT app.register_tenant_table('openbal.opening_balance_entry');
SELECT app.register_mutable('openbal.opening_balance_entry');

-- OB-R5 / OB-02: once the batch is confirmed, entries are not edited or deleted; only the supersede pointer is set once,
-- and entries added afterwards need a reason. Whether OB-02 = allowed is a setting checked by the application.
CREATE FUNCTION openbal.guard_entry() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE v_state text;
BEGIN
  SELECT state INTO v_state FROM openbal.opening_balance_batch
   WHERE org_id = (CASE WHEN TG_OP = 'DELETE' THEN OLD.org_id ELSE NEW.org_id END)
     AND id     = (CASE WHEN TG_OP = 'DELETE' THEN OLD.batch_id ELSE NEW.batch_id END);
  IF v_state = 'confirmed' THEN
    IF TG_OP = 'DELETE' THEN
      RAISE EXCEPTION 'opening balance batch is confirmed: entries cannot be deleted (OB-R5)' USING ERRCODE = 'check_violation';
    ELSIF TG_OP = 'INSERT' AND coalesce(btrim(NEW.change_reason), '') = '' THEN
      RAISE EXCEPTION 'change after confirmation needs a reason (OB-02)' USING ERRCODE = 'check_violation';
    ELSIF TG_OP = 'UPDATE' AND (OLD.superseded_by_entry_id IS NOT NULL
          OR (to_jsonb(NEW) - 'superseded_by_entry_id' - 'updated_at' - 'updated_by')
             <> (to_jsonb(OLD) - 'superseded_by_entry_id' - 'updated_at' - 'updated_by')) THEN
      RAISE EXCEPTION 'opening balance batch is confirmed: only the supersede pointer may be set (OB-R5)' USING ERRCODE = 'check_violation';
    END IF;
  END IF;
  RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;
CREATE TRIGGER guard_entry BEFORE INSERT OR UPDATE OR DELETE ON openbal.opening_balance_entry
  FOR EACH ROW EXECUTE FUNCTION openbal.guard_entry();
