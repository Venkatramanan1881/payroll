-- Reverses 0003.
ALTER TABLE org.branch DROP CONSTRAINT IF EXISTS branch_holiday_calendar_fk;
DROP FUNCTION IF EXISTS app.resolve_device_org(text);
DROP SCHEMA IF EXISTS policy CASCADE;
DROP SCHEMA IF EXISTS sched CASCADE;
DROP SCHEMA IF EXISTS timecap CASCADE;   -- also removes app_resolver's grants on it
DROP FUNCTION IF EXISTS app.guard_frozen_version();
