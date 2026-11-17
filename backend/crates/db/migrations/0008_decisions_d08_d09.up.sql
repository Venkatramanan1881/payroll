-- 0008 Decision D-08 (2026-11-16): missed-punch cut-off per shift.
-- An unresolved missed punch gets its MP-03 outcome at this time of day on the calendar day after the shift's END date,
-- in the zone of the branch where the punch happened (D-04). Default 02:30, editable per shift version.
-- Example: shift ends Mon 18:00 -> cut-off Tue 02:30; night shift ends Tue 06:00 -> cut-off Wed 02:30.
ALTER TABLE sched.shift_version
  ADD COLUMN missed_punch_cutoff time NOT NULL DEFAULT '02:30';
COMMENT ON COLUMN sched.shift_version.missed_punch_cutoff IS
  'D-08: MP-03 applies at this time on the day after the shift end date (branch zone). Proposed BRD setting MP-06.';

-- Decision D-09 (2026-11-16): V1 never identifies overtime (M01 is V3). NULL = not identified, so it is never
-- confused with "0 minutes of overtime". Existing rows keep their values.
ALTER TABLE attendance.day_result  ALTER COLUMN overtime_identified_minutes DROP NOT NULL, ALTER COLUMN overtime_identified_minutes DROP DEFAULT;
ALTER TABLE period.period_totals   ALTER COLUMN overtime_identified_minutes DROP NOT NULL, ALTER COLUMN overtime_identified_minutes DROP DEFAULT;
COMMENT ON COLUMN attendance.day_result.overtime_identified_minutes IS 'D-09: NULL when the Overtime module (M01, V3) is off.';
COMMENT ON COLUMN period.period_totals.overtime_identified_minutes IS 'D-09: NULL when the Overtime module (M01, V3) is off.';
