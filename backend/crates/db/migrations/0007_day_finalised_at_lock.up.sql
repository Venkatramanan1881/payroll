-- 0007 Decision D-17 (2026-11-16): a day is finalised only at period lock, so "finalised" is the same as "locked".
-- Removes the 'finalised' processing state (SM-DAY). Existing rows (none expected before go-live) become 'provisional'.
UPDATE attendance.attendance_day SET processing_state = 'provisional' WHERE processing_state = 'finalised';
ALTER TABLE attendance.attendance_day DROP CONSTRAINT attendance_day_processing_state_check;
ALTER TABLE attendance.attendance_day ADD CONSTRAINT attendance_day_processing_state_check
  CHECK (processing_state IN ('provisional','exception','absence_pending','locked'));  -- SM-DAY after D-17
