-- Restores NOT NULL DEFAULT 0 (NULLs become 0 first) and removes the cut-off column.
UPDATE period.period_totals SET overtime_identified_minutes = 0 WHERE overtime_identified_minutes IS NULL;
ALTER TABLE period.period_totals  ALTER COLUMN overtime_identified_minutes SET DEFAULT 0, ALTER COLUMN overtime_identified_minutes SET NOT NULL;
-- day_result is append-only: its NULLs cannot be updated, so NOT NULL is restored only if none exist.
DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM attendance.day_result WHERE overtime_identified_minutes IS NULL) THEN
    ALTER TABLE attendance.day_result ALTER COLUMN overtime_identified_minutes SET DEFAULT 0, ALTER COLUMN overtime_identified_minutes SET NOT NULL;
  ELSE
    RAISE EXCEPTION 'day_result has NULL overtime rows; down migration not possible without data loss (append-only table)';
  END IF;
END $$;
ALTER TABLE sched.shift_version DROP COLUMN missed_punch_cutoff;
