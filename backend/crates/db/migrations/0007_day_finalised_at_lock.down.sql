-- Restores the pre-D-17 state list. No row is changed back (provisional rows stay provisional).
ALTER TABLE attendance.attendance_day DROP CONSTRAINT attendance_day_processing_state_check;
ALTER TABLE attendance.attendance_day ADD CONSTRAINT attendance_day_processing_state_check
  CHECK (processing_state IN ('provisional','exception','absence_pending','finalised','locked'));
