-- 0003 Time Capture, Scheduling, Policy. DOMAIN_MODEL §3.5-§3.7, STATE_MACHINES SM-DEVICE, SM-ENROL, SM-FLAG, SM-REQ, SM-POLICYVER.
-- Conventions: docs/database/DATA_MODEL.md §1. Org-owned tables: PK (org_id, id), composite FKs, register_tenant_table().

CREATE SCHEMA timecap;
CREATE SCHEMA sched;
CREATE SCHEMA policy;
GRANT USAGE ON SCHEMA timecap, sched, policy TO app_api, app_jobs, app_gateway;

-- G5 (DATA_MODEL §5.1): a version whose state is in_force or superseded is never edited or deleted.
-- The only change allowed is the state move in_force -> superseded with every other column unchanged.
-- scheduled stays editable until GAP-SM-16 is decided. Shared by every versioned table with a "state" column.
CREATE FUNCTION app.guard_frozen_version() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  -- generated columns are not yet computed in NEW inside a BEFORE trigger, so they are left out of the comparison
  skip text[] := ARRAY(SELECT attname::text FROM pg_attribute WHERE attrelid = TG_RELID AND attgenerated <> '')
                 || ARRAY['state','updated_at','updated_by'];
BEGIN
  IF OLD.state IN ('in_force','superseded') THEN
    IF TG_OP = 'UPDATE' AND OLD.state = 'in_force' AND NEW.state = 'superseded'
       AND (to_jsonb(NEW) - skip) = (to_jsonb(OLD) - skip) THEN
      RETURN NEW;
    END IF;
    RAISE EXCEPTION '%.%: version % is %, it cannot be changed (G5)', TG_TABLE_SCHEMA, TG_TABLE_NAME, OLD.id, OLD.state
      USING ERRCODE = 'insufficient_privilege';
  END IF;
  RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;

-- ===========================================================================
-- Time Capture (C04) — DOMAIN_MODEL §3.5
-- ===========================================================================
CREATE TABLE timecap.device (
  org_id          uuid NOT NULL REFERENCES platform.organisation (id),
  id              uuid NOT NULL DEFAULT gen_random_uuid(),
  serial          text NOT NULL UNIQUE,     -- globally unique: the gateway resolves the organisation from it (TSD §9.1)
  make            text NOT NULL,
  adapter         text NOT NULL,            -- vendor adapter code (TSD §9.1)
  name            text,
  branch_id       uuid NOT NULL,            -- PC-R2; branch move / replacement: GAP-SM-05
  last_contact_at timestamptz,
  online_state    text NOT NULL DEFAULT 'offline' CHECK (online_state IN ('online','offline')),  -- SM-DEVICE; initial state: GAP-SM-05
  offline_alert_sent_at timestamptz,        -- D4: one alert per offline spell (PC-10, PC-R10)
  retired_at      timestamptz,              -- deregistration rules: GAP-SM-05
  created_at      timestamptz NOT NULL DEFAULT now(),
  created_by      uuid,
  updated_at      timestamptz NOT NULL DEFAULT now(),
  updated_by      uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id)
);
CREATE INDEX device_branch ON timecap.device (org_id, branch_id);
SELECT app.register_tenant_table('timecap.device');
SELECT app.register_mutable('timecap.device');
GRANT UPDATE (last_contact_at, online_state, offline_alert_sent_at) ON timecap.device TO app_gateway;  -- SM-DEVICE D2

-- Device push -> organisation (TSD §9.1). Runs before the organisation is known, so it bypasses RLS and returns only org_id.
-- NULL for unknown or retired serials: the push is refused and logged (DOMAIN_MODEL §3.5).
CREATE FUNCTION app.resolve_device_org(p_serial text) RETURNS uuid
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT d.org_id FROM timecap.device d WHERE d.serial = p_serial AND d.retired_at IS NULL
$$;
GRANT USAGE ON SCHEMA timecap TO app_resolver;
GRANT SELECT ON timecap.device TO app_resolver;
ALTER FUNCTION app.resolve_device_org(text) OWNER TO app_resolver;
REVOKE ALL ON FUNCTION app.resolve_device_org(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION app.resolve_device_org(text) TO app_gateway;

-- Enrolment ID -> employee on a device, dated (PC-R3, PC-R8).
CREATE TABLE timecap.enrolment_mapping (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  device_id     uuid NOT NULL,
  enrolment_id  text NOT NULL,
  employee_id   uuid NOT NULL,
  valid         daterange NOT NULL CHECK (NOT isempty(valid)),
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, device_id) REFERENCES timecap.device (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  -- DP-07 (OQ-C02-03): uniqueness scope of an enrolment ID is open. Per device is the narrowest safe rule;
  -- a per-branch or per-organisation answer adds a stronger constraint by migration.
  EXCLUDE USING gist (org_id WITH =, device_id WITH =, enrolment_id WITH =, valid WITH &&)
);
CREATE INDEX enrolment_mapping_employee ON timecap.enrolment_mapping (org_id, employee_id);
COMMENT ON TABLE timecap.enrolment_mapping IS 'A change ends the current range and inserts the next one (PC-R3). Earlier unmapped punches with the ID: OQ-C04-04.';
SELECT app.register_tenant_table('timecap.enrolment_mapping');

CREATE TABLE timecap.enrolment_status_history (
  org_id         uuid NOT NULL,
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id    uuid NOT NULL,
  enrolment_mapping_id uuid,               -- status per employee or per device mapping: GAP-SM-04
  from_status    text CHECK (from_status IN ('not_enrolled','enrolled','re_enrolment_needed')),
  to_status      text NOT NULL CHECK (to_status IN ('not_enrolled','enrolled','re_enrolment_needed')),  -- SM-ENROL, PC-R8
  effective_at   timestamptz NOT NULL DEFAULT now(),
  reason         text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, enrolment_mapping_id) REFERENCES timecap.enrolment_mapping (org_id, id)
);
CREATE INDEX enrolment_status_history_emp ON timecap.enrolment_status_history (org_id, employee_id, effective_at);
SELECT app.register_tenant_table('timecap.enrolment_status_history', true);

-- Evidence of presence. Insert-only (BR-01, PC-R6). Carries no inferred direction (BRX-01).
CREATE TABLE timecap.raw_punch (
  org_id            uuid NOT NULL,
  id                uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id       uuid,                   -- NULL = device punch with no enrolment mapping yet; later resolution: OQ-C04-04 (row is never updated)
  source            text NOT NULL CHECK (source IN ('device','app','web','kiosk','import','manual')),  -- PC-01
  device_id         uuid,
  enrolment_id      text,
  client_ref        text,                   -- app/web/kiosk client identifier
  event_at          timestamptz NOT NULL,   -- original event time (BR-11)
  captured_tz       text NOT NULL CHECK (app.is_valid_timezone(captured_tz)),  -- BR-11
  received_at       timestamptz NOT NULL DEFAULT now(),  -- late sync PC-07, clock check PC-08
  idempotency_key   text NOT NULL,          -- TSD §5.5, §9.3; DATA_MODEL §5.4
  raw_payload_ref   text,                   -- file path, starts with org_id (DATA_MODEL §4)
  branch_id         uuid,                   -- where known (device branch, fence matched, IP)
  latitude          numeric(9,6) CHECK (latitude BETWEEN -90 AND 90),
  longitude         numeric(9,6) CHECK (longitude BETWEEN -180 AND 180),
  accuracy_m        integer CHECK (accuracy_m >= 0),        -- PC-14
  fence_distance_m  integer CHECK (fence_distance_m >= 0),  -- server computed (PC-R13, TSD §9.2)
  inside_fence      boolean,                -- server computed; client claims ignored (TSD §9.2)
  selfie_file_path  text,                   -- PC-R11
  client_ip         inet,                   -- PC-05
  device_uptime_ms  bigint CHECK (device_uptime_ms >= 0),   -- offline punches (TSD §9.3)
  direction_tag     text CHECK (direction_tag IN ('in','out')),  -- only when the source provides it (PR-01); never inferred (BRX-01)
  manual_reason     text,                   -- PC-R9, RG-10
  manual_entered_by uuid,
  manual_approval_ref uuid,                 -- regularisation request or HR correction (RG-10, DP-08); no FK: target type varies
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, idempotency_key),         -- a retry returns the existing punch
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, device_id) REFERENCES timecap.device (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  FOREIGN KEY (org_id, manual_entered_by) REFERENCES iam.user_account (org_id, id),
  CHECK (source <> 'manual' OR (manual_reason IS NOT NULL AND manual_entered_by IS NOT NULL)),  -- BR-01, PC-R9
  CHECK (source <> 'device' OR device_id IS NOT NULL),
  CHECK (employee_id IS NOT NULL OR source = 'device'),       -- only device punches arrive unresolved
  CHECK ((latitude IS NULL) = (longitude IS NULL))
);
CREATE INDEX raw_punch_employee_time ON timecap.raw_punch (org_id, employee_id, event_at);                      -- day processing (§9.6 step 2)
CREATE INDEX raw_punch_unresolved ON timecap.raw_punch (org_id, device_id, enrolment_id) WHERE employee_id IS NULL;  -- waiting for a mapping
COMMENT ON TABLE timecap.raw_punch IS 'Insert-only evidence (BR-01). Rejected punches stay as evidence; the DayResult excludes them. Direction is assigned only in a DayResult (BRX-01).';
SELECT app.register_tenant_table('timecap.raw_punch', true);

-- One flag per punch per reason (SM-FLAG). Several flags on one punch: GAP-SM-06.
CREATE TABLE timecap.punch_flag (
  org_id     uuid NOT NULL,
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  punch_id   uuid NOT NULL,
  reason     text NOT NULL CHECK (reason IN (
               'outside_fence',          -- PC-04
               'branch_not_allowed',     -- EM-07
               'no_shift_or_off_day',    -- PC-06: no shift, weekly off or holiday
               'late_sync',              -- PC-07
               'clock_mismatch',         -- PC-08, TSD §9.3
               'location_unavailable',   -- PC-13
               'low_accuracy',           -- PC-14
               'ip_check_failed')),      -- PC-05 / PC-15 (flag vs block: OQ-C04-11)
  state      text NOT NULL DEFAULT 'raised' CHECK (state IN ('raised','accepted','rejected')),  -- SM-FLAG
  counts_before_decision boolean NOT NULL,  -- DP-05: captured when raised (E18: outside fence does not count)
  evidence   jsonb NOT NULL DEFAULT '{}'::jsonb,  -- values that triggered the flag (distance, accuracy, delay ...)
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, punch_id, reason),
  FOREIGN KEY (org_id, punch_id) REFERENCES timecap.raw_punch (org_id, id)
);
CREATE INDEX punch_flag_open ON timecap.punch_flag (org_id, state) WHERE state = 'raised';  -- R5
SELECT app.register_tenant_table('timecap.punch_flag');
SELECT app.register_mutable('timecap.punch_flag');
GRANT INSERT ON timecap.punch_flag TO app_gateway;  -- F1: raised at ingestion

CREATE TABLE timecap.punch_flag_decision (
  org_id     uuid NOT NULL,
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  flag_id    uuid NOT NULL,
  decision   text NOT NULL CHECK (decision IN ('accepted','rejected')),  -- SM-FLAG F2/F3
  decided_by uuid NOT NULL,                 -- BR-07 (not the punch's employee) is checked by the application
  reason     text,
  decided_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, flag_id) REFERENCES timecap.punch_flag (org_id, id),
  FOREIGN KEY (org_id, decided_by) REFERENCES iam.user_account (org_id, id),
  CHECK (decision <> 'rejected' OR reason IS NOT NULL)  -- F3 needs a reason
);
-- Not unique per flag: whether a decision can be reversed is GAP-SM-06.
CREATE INDEX punch_flag_decision_flag ON timecap.punch_flag_decision (org_id, flag_id);
SELECT app.register_tenant_table('timecap.punch_flag_decision', true);

-- Blocked punch attempts: nothing was accepted, so not a RawPunch (E18, PC-13, DOMAIN_MODEL §3.5).
CREATE TABLE timecap.punch_attempt_refused (
  org_id       uuid NOT NULL,
  id           uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id  uuid,                        -- NULL for an unresolved device attempt (device block: OQ-C04-06)
  source       text NOT NULL CHECK (source IN ('device','app','web','kiosk','import','manual')),  -- PC-01
  device_id    uuid,
  attempted_at timestamptz NOT NULL DEFAULT now(),
  rule_ref     text NOT NULL CHECK (rule_ref ~ '^[A-Z]{2}-[0-9]{2}$'),  -- the setting that blocked it, e.g. PC-13, PC-14
  message      text NOT NULL,               -- reason shown to the employee (E18)
  latitude     numeric(9,6) CHECK (latitude BETWEEN -90 AND 90),
  longitude    numeric(9,6) CHECK (longitude BETWEEN -180 AND 180),
  accuracy_m   integer CHECK (accuracy_m >= 0),
  client_ip    inet,
  created_at   timestamptz NOT NULL DEFAULT now(),
  created_by   uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, device_id) REFERENCES timecap.device (org_id, id)
);
CREATE INDEX punch_attempt_refused_emp ON timecap.punch_attempt_refused (org_id, employee_id, attempted_at);
SELECT app.register_tenant_table('timecap.punch_attempt_refused', true);

-- ===========================================================================
-- Scheduling (C05) — DOMAIN_MODEL §3.6
-- ===========================================================================
CREATE TABLE sched.holiday_calendar (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  retired_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name)
);
COMMENT ON TABLE sched.holiday_calendar IS 'One per location (SH-10, HD-R1); a branch points at its calendar (org.branch.holiday_calendar_id).';
SELECT app.register_tenant_table('sched.holiday_calendar');
SELECT app.register_mutable('sched.holiday_calendar');

ALTER TABLE org.branch
  ADD CONSTRAINT branch_holiday_calendar_fk FOREIGN KEY (org_id, holiday_calendar_id) REFERENCES sched.holiday_calendar (org_id, id);

CREATE TABLE sched.holiday (
  org_id       uuid NOT NULL,
  id           uuid NOT NULL DEFAULT gen_random_uuid(),
  calendar_id  uuid NOT NULL,
  holiday_date date NOT NULL,
  name         text NOT NULL,
  kind         text NOT NULL CHECK (kind IN ('national','festival','optional')),  -- HD-R1
  created_at   timestamptz NOT NULL DEFAULT now(),
  created_by   uuid,
  updated_at   timestamptz NOT NULL DEFAULT now(),
  updated_by   uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, calendar_id, holiday_date),
  FOREIGN KEY (org_id, calendar_id) REFERENCES sched.holiday_calendar (org_id, id)
);
SELECT app.register_tenant_table('sched.holiday');
SELECT app.register_mutable('sched.holiday');

-- Employee's optional-holiday picks (HD-R3). The yearly limit HD-01 is checked by the application.
CREATE TABLE sched.optional_holiday_choice (
  org_id      uuid NOT NULL,
  employee_id uuid NOT NULL,
  holiday_id  uuid NOT NULL,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, employee_id, holiday_id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, holiday_id) REFERENCES sched.holiday (org_id, id)
);
SELECT app.register_tenant_table('sched.optional_holiday_choice');

CREATE TABLE sched.shift (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  code       text NOT NULL,
  name       text NOT NULL,
  retired_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, code)
);
SELECT app.register_tenant_table('sched.shift');
SELECT app.register_mutable('sched.shift');

-- Shift definition, versioned (G5). No SM exists for shift versions; the SM-POLICYVER states are reused
-- (PERMISSION_MATRIX SCH-01 "publish Shift version" under AC-04).
CREATE TABLE sched.shift_version (
  org_id              uuid NOT NULL,
  id                  uuid NOT NULL DEFAULT gen_random_uuid(),
  shift_id            uuid NOT NULL,
  effective_from      date NOT NULL,
  state               text NOT NULL DEFAULT 'draft'
                      CHECK (state IN ('draft','pending_approval','scheduled','in_force','superseded')),
  -- SH-01 lists "roster", but that is a schedule source (EM-R3: fixed shift or roster), held in sched.employee_schedule,
  -- not a shift shape. 'night' follows SH-R1 / DOMAIN_MODEL §3.6.
  shift_type          text NOT NULL DEFAULT 'fixed'
                      CHECK (shift_type IN ('fixed','flexible','flexible_core','split','night','open')),  -- SH-R1, SH-01
  start_time          time,                 -- SH-02
  end_time            time,
  crosses_midnight    boolean GENERATED ALWAYS AS (end_time < start_time) STORED,
  required_minutes    integer CHECK (required_minutes > 0),                   -- SH-03
  required_per        text NOT NULL DEFAULT 'day' CHECK (required_per IN ('day','week')),  -- SH-03
  core_start          time,                 -- SH-04
  core_end            time,
  flex_earliest_in    time,                 -- SH-05
  flex_latest_out     time,
  window_before_min   integer NOT NULL DEFAULT 180 CHECK (window_before_min >= 0),  -- SH-07; edges: OQ-C05-03, overlap: OQ-C05-04
  window_after_min    integer NOT NULL DEFAULT 360 CHECK (window_after_min >= 0),
  is_night_shift      boolean NOT NULL DEFAULT false,  -- what marks a night shift: OQ-C05-02
  night_attendance_date text NOT NULL DEFAULT 'shift_start_date'
                      CHECK (night_attendance_date IN ('shift_start_date','shift_end_date')),  -- SH-06
  half_day_split      text NOT NULL DEFAULT 'middle_of_shift' CHECK (half_day_split IN ('middle_of_shift','fixed_time')),  -- SH-11
  half_day_split_time time,
  segments            jsonb,                -- split shifts only; shape is OQ-C05-01, so kept neutral until answered
  reason              text,                 -- G9
  created_at          timestamptz NOT NULL DEFAULT now(),
  created_by          uuid,
  updated_at          timestamptz NOT NULL DEFAULT now(),
  updated_by          uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, shift_id) REFERENCES sched.shift (org_id, id),
  CHECK (half_day_split <> 'fixed_time' OR half_day_split_time IS NOT NULL),   -- SH-11
  CHECK (shift_type <> 'open' OR (start_time IS NULL AND end_time IS NULL AND required_minutes IS NOT NULL)),  -- SH-01 open: hours only
  CHECK (shift_type NOT IN ('fixed','night') OR (start_time IS NOT NULL AND end_time IS NOT NULL)),
  CHECK (shift_type <> 'flexible_core' OR (core_start IS NOT NULL AND core_end IS NOT NULL)),  -- SH-04
  CHECK (shift_type <> 'night' OR is_night_shift),
  CHECK (segments IS NULL OR shift_type = 'split'),
  CHECK ((start_time IS NULL) = (end_time IS NULL))
);
CREATE UNIQUE INDEX shift_version_effective ON sched.shift_version (org_id, shift_id, effective_from) WHERE state <> 'draft';
CREATE TRIGGER guard_frozen_version BEFORE UPDATE OR DELETE ON sched.shift_version
  FOR EACH ROW EXECUTE FUNCTION app.guard_frozen_version();
SELECT app.register_tenant_table('sched.shift_version');
SELECT app.register_mutable('sched.shift_version');

-- SH-09 / HD-R2. Alternating holds one weekday and its weeks of the month (e.g. Saturday, {2,4}), optionally on top of weekly days.
CREATE TABLE sched.weekly_off_pattern (
  org_id      uuid NOT NULL REFERENCES platform.organisation (id),
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  name        text NOT NULL,
  kind        text NOT NULL CHECK (kind IN ('fixed_days','alternating','from_roster')),  -- SH-09
  weekly_days smallint[] CHECK (weekly_days <@ '{1,2,3,4,5,6,7}'::smallint[]),           -- ISO weekday, off every week
  alt_day     smallint CHECK (alt_day BETWEEN 1 AND 7),
  alt_weeks   smallint[] CHECK (alt_weeks <@ '{1,2,3,4,5}'::smallint[]),                  -- week of month
  retired_at  timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  updated_at  timestamptz NOT NULL DEFAULT now(),
  updated_by  uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name),
  CHECK ((alt_day IS NULL) = (alt_weeks IS NULL)),
  CHECK (
    (kind = 'fixed_days'  AND weekly_days IS NOT NULL AND alt_day IS NULL) OR
    (kind = 'alternating' AND alt_day IS NOT NULL) OR
    (kind = 'from_roster' AND weekly_days IS NULL AND alt_day IS NULL))
);
SELECT app.register_tenant_table('sched.weekly_off_pattern');
SELECT app.register_mutable('sched.weekly_off_pattern');

-- Schedule source per employee, dated (EM-R3: fixed shift or roster). No row = no schedule (SH-08 auto-detect).
-- Shift also set by a shift-level policy assignment (G3): OQ-C05-07.
CREATE TABLE sched.employee_schedule (
  org_id      uuid NOT NULL,
  id          uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id uuid NOT NULL,
  source      text NOT NULL CHECK (source IN ('fixed_shift','roster')),  -- EM-R3
  shift_id    uuid,                         -- version resolved by date (G5)
  weekly_off_pattern_id uuid,               -- SH-09
  valid       daterange NOT NULL CHECK (NOT isempty(valid)),
  reason      text,
  created_at  timestamptz NOT NULL DEFAULT now(),
  created_by  uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, shift_id) REFERENCES sched.shift (org_id, id),
  FOREIGN KEY (org_id, weekly_off_pattern_id) REFERENCES sched.weekly_off_pattern (org_id, id),
  CHECK ((source = 'fixed_shift') = (shift_id IS NOT NULL)),
  EXCLUDE USING gist (org_id WITH =, employee_id WITH =, valid WITH &&)
);
SELECT app.register_tenant_table('sched.employee_schedule');

CREATE TABLE sched.roster_entry (
  org_id           uuid NOT NULL,
  id               uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id      uuid NOT NULL,
  roster_date      date NOT NULL,
  shift_version_id uuid,
  is_weekly_off    boolean NOT NULL DEFAULT false,
  source           text NOT NULL CHECK (source IN ('screen','upload','copy','rotation')),  -- SH-R2, SH-R3
  created_at       timestamptz NOT NULL DEFAULT now(),
  created_by       uuid,
  updated_at       timestamptz NOT NULL DEFAULT now(),
  updated_by       uuid,
  PRIMARY KEY (org_id, id),
  -- One entry per employee per date. Two shifts on one date is OQ-C05-06; a yes answer relaxes this by migration.
  UNIQUE (org_id, employee_id, roster_date),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, shift_version_id) REFERENCES sched.shift_version (org_id, id),
  CHECK (is_weekly_off = (shift_version_id IS NULL))
);
SELECT app.register_tenant_table('sched.roster_entry');
SELECT app.register_mutable('sched.roster_entry');

-- SH-12 change / swap request (SM-REQ). Consent and second approver for swaps: OQ-C05-12; cancellation: GAP-SM-09.
CREATE TABLE sched.shift_change_request (
  org_id              uuid NOT NULL,
  id                  uuid NOT NULL DEFAULT gen_random_uuid(),
  employee_id         uuid NOT NULL,
  kind                text NOT NULL CHECK (kind IN ('change','swap')),  -- SH-12
  roster_date         date NOT NULL,
  requested_shift_version_id uuid,
  swap_with_employee_id uuid,
  reason              text,
  state               text NOT NULL DEFAULT 'pending_l1'
                      CHECK (state IN ('pending_l1','pending_l2','approved','rejected','cancelled')),  -- SM-REQ (no draft: GAP-SM-08)
  escalated           boolean NOT NULL DEFAULT false,  -- SM-REQ R7 flag, not a state
  current_approver_id uuid,
  submitted_at        timestamptz NOT NULL DEFAULT now(),
  decided_at          timestamptz,
  created_at          timestamptz NOT NULL DEFAULT now(),
  created_by          uuid,
  updated_at          timestamptz NOT NULL DEFAULT now(),
  updated_by          uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, requested_shift_version_id) REFERENCES sched.shift_version (org_id, id),
  FOREIGN KEY (org_id, swap_with_employee_id) REFERENCES workforce.employee (org_id, id),
  FOREIGN KEY (org_id, current_approver_id) REFERENCES iam.user_account (org_id, id),
  CHECK ((kind = 'swap') = (swap_with_employee_id IS NOT NULL)),
  CHECK (swap_with_employee_id IS NULL OR swap_with_employee_id <> employee_id)
);
CREATE INDEX shift_change_request_pending ON sched.shift_change_request (org_id, state, current_approver_id)
  WHERE state IN ('pending_l1','pending_l2');
CREATE INDEX shift_change_request_emp ON sched.shift_change_request (org_id, employee_id, roster_date);
SELECT app.register_tenant_table('sched.shift_change_request');
SELECT app.register_mutable('sched.shift_change_request');

-- ===========================================================================
-- Policy — DOMAIN_MODEL §3.7, SM-POLICYVER
-- ===========================================================================
-- Rule groups held in policy sets: the list in DOMAIN_MODEL §3.7 (attendance, leave, permission, comp-off, punch collection;
-- BUSINESS_RULES §5). Whether HD, SH, OT also sit in policy sets is not stated ("…" in DOMAIN_MODEL §3.7): added by migration if so.
CREATE DOMAIN policy.rule_group_code AS text
  CHECK (VALUE IN ('PR','BK','LA','EC','EA','MP','DS','RD','RG','LM','LV','PM','OD','HW','PC'));

CREATE TABLE policy.policy_set (
  org_id     uuid NOT NULL REFERENCES platform.organisation (id),
  id         uuid NOT NULL DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  retired_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  created_by uuid,
  updated_at timestamptz NOT NULL DEFAULT now(),
  updated_by uuid,
  PRIMARY KEY (org_id, id),
  UNIQUE (org_id, name)
);
SELECT app.register_tenant_table('policy.policy_set');
SELECT app.register_mutable('policy.policy_set');

CREATE TABLE policy.policy_version (
  org_id         uuid NOT NULL,
  id             uuid NOT NULL DEFAULT gen_random_uuid(),
  policy_set_id  uuid NOT NULL,
  effective_from date NOT NULL,
  state          text NOT NULL DEFAULT 'draft'
                 CHECK (state IN ('draft','pending_approval','scheduled','in_force','superseded')),  -- SM-POLICYVER
  reason         text,                      -- G9 (required on save: application)
  submitted_by   uuid,
  approved_by    uuid,                      -- AC-04; approver = submitter: OQ-C03-11
  approved_at    timestamptz,
  created_at     timestamptz NOT NULL DEFAULT now(),
  created_by     uuid,
  updated_at     timestamptz NOT NULL DEFAULT now(),
  updated_by     uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, policy_set_id) REFERENCES policy.policy_set (org_id, id),
  FOREIGN KEY (org_id, submitted_by) REFERENCES iam.user_account (org_id, id),
  FOREIGN KEY (org_id, approved_by) REFERENCES iam.user_account (org_id, id)
);
-- Effective periods of one set's versions never overlap (each runs to the next effective_from).
CREATE UNIQUE INDEX policy_version_effective ON policy.policy_version (org_id, policy_set_id, effective_from) WHERE state <> 'draft';
CREATE TRIGGER guard_frozen_version BEFORE UPDATE OR DELETE ON policy.policy_version
  FOR EACH ROW EXECUTE FUNCTION app.guard_frozen_version();
COMMENT ON TABLE policy.policy_version IS 'scheduled -> in_force -> superseded are date-derived (SM-POLICYVER V7/V8). Edit of a scheduled version and "in use": GAP-SM-16.';
SELECT app.register_tenant_table('policy.policy_version');
SELECT app.register_mutable('policy.policy_version');

-- One settings document per rule group per version (TSD §5.4), with its master switch (G2).
CREATE TABLE policy.policy_rule_group (
  org_id            uuid NOT NULL,
  policy_version_id uuid NOT NULL,
  rule_group        policy.rule_group_code NOT NULL,
  enabled           boolean NOT NULL,       -- G2
  settings          jsonb NOT NULL,         -- validated against the core settings schema before saving (TSD §5.4)
  schema_version    integer NOT NULL CHECK (schema_version > 0),
  created_at        timestamptz NOT NULL DEFAULT now(),
  created_by        uuid,
  updated_at        timestamptz NOT NULL DEFAULT now(),
  updated_by        uuid,
  PRIMARY KEY (org_id, policy_version_id, rule_group),
  FOREIGN KEY (org_id, policy_version_id) REFERENCES policy.policy_version (org_id, id)
);
SELECT app.register_tenant_table('policy.policy_rule_group');
SELECT app.register_mutable('policy.policy_rule_group');

-- G5: rule groups of an in_force or superseded version cannot be added, changed or removed.
CREATE FUNCTION policy.guard_rule_group() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
  IF (TG_OP <> 'INSERT' AND EXISTS (SELECT 1 FROM policy.policy_version v
        WHERE v.org_id = OLD.org_id AND v.id = OLD.policy_version_id AND v.state IN ('in_force','superseded')))
  OR (TG_OP <> 'DELETE' AND EXISTS (SELECT 1 FROM policy.policy_version v
        WHERE v.org_id = NEW.org_id AND v.id = NEW.policy_version_id AND v.state IN ('in_force','superseded'))) THEN
    RAISE EXCEPTION 'policy.policy_rule_group: version is in force or superseded, it cannot be changed (G5)'
      USING ERRCODE = 'insufficient_privilege';
  END IF;
  RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END $$;
CREATE TRIGGER guard_frozen_version BEFORE INSERT OR UPDATE OR DELETE ON policy.policy_rule_group
  FOR EACH ROW EXECUTE FUNCTION policy.guard_rule_group();

-- G3: dated assignment of a set at one level. Ranking between levels is resolver logic (DP-03).
-- "shift" as a policy level vs the employee's own shift: OQ-C05-07.
CREATE TABLE policy.policy_assignment (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  policy_set_id uuid NOT NULL,
  level         text NOT NULL CHECK (level IN ('organisation','branch','department','category','shift','employee')),  -- G3
  branch_id     uuid,
  list_entry_id uuid,
  list_kind     text GENERATED ALWAYS AS (CASE WHEN level IN ('department','category') THEN level END) STORED,
  shift_id      uuid,
  employee_id   uuid,
  target_key    uuid GENERATED ALWAYS AS (coalesce(branch_id, list_entry_id, shift_id, employee_id, org_id)) STORED,
  valid         daterange NOT NULL CHECK (NOT isempty(valid)),
  reason        text,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, policy_set_id) REFERENCES policy.policy_set (org_id, id),
  FOREIGN KEY (org_id, branch_id) REFERENCES org.branch (org_id, id),
  FOREIGN KEY (org_id, list_entry_id, list_kind) REFERENCES org.list_entry (org_id, id, kind),
  FOREIGN KEY (org_id, shift_id) REFERENCES sched.shift (org_id, id),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  CHECK (
    (level = 'organisation' AND branch_id IS NULL AND list_entry_id IS NULL AND shift_id IS NULL AND employee_id IS NULL) OR
    (level = 'branch'       AND branch_id IS NOT NULL AND list_entry_id IS NULL AND shift_id IS NULL AND employee_id IS NULL) OR
    (level IN ('department','category')
                            AND list_entry_id IS NOT NULL AND branch_id IS NULL AND shift_id IS NULL AND employee_id IS NULL) OR
    (level = 'shift'        AND shift_id IS NOT NULL AND branch_id IS NULL AND list_entry_id IS NULL AND employee_id IS NULL) OR
    (level = 'employee'     AND employee_id IS NOT NULL AND branch_id IS NULL AND list_entry_id IS NULL AND shift_id IS NULL)
  ),
  -- Same level and target never overlap in time (DOMAIN_MODEL §3.7); organisation level keys on org_id.
  EXCLUDE USING gist (org_id WITH =, level WITH =, target_key WITH =, valid WITH &&)
);
CREATE INDEX policy_assignment_set ON policy.policy_assignment (org_id, policy_set_id);
SELECT app.register_tenant_table('policy.policy_assignment');

-- G7 / LA-22. Whether an exemption belongs to a policy set or is organisation-wide is not stated: kept organisation-wide.
CREATE TABLE policy.exemption (
  org_id        uuid NOT NULL,
  id            uuid NOT NULL DEFAULT gen_random_uuid(),
  rule_group    policy.rule_group_code NOT NULL,
  scope_kind    text NOT NULL CHECK (scope_kind IN ('category','department','employee_type','employee','date')),  -- G7, LA-22
  list_entry_id uuid,
  list_kind     text GENERATED ALWAYS AS (CASE WHEN scope_kind IN ('category','department','employee_type') THEN scope_kind END) STORED,
  employee_id   uuid,
  valid         daterange NOT NULL CHECK (NOT isempty(valid)),  -- for scope 'date' these are the exempt dates
  reason        text,
  created_at    timestamptz NOT NULL DEFAULT now(),
  created_by    uuid,
  updated_at    timestamptz NOT NULL DEFAULT now(),
  updated_by    uuid,
  PRIMARY KEY (org_id, id),
  FOREIGN KEY (org_id, list_entry_id, list_kind) REFERENCES org.list_entry (org_id, id, kind),
  FOREIGN KEY (org_id, employee_id) REFERENCES workforce.employee (org_id, id),
  CHECK (
    (scope_kind IN ('category','department','employee_type') AND list_entry_id IS NOT NULL AND employee_id IS NULL) OR
    (scope_kind = 'employee' AND employee_id IS NOT NULL AND list_entry_id IS NULL) OR
    (scope_kind = 'date'     AND employee_id IS NULL AND list_entry_id IS NULL))
);
CREATE INDEX exemption_rule_group ON policy.exemption (org_id, rule_group);
SELECT app.register_tenant_table('policy.exemption');
SELECT app.register_mutable('policy.exemption');
