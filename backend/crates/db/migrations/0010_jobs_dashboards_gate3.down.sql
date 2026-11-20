ALTER TABLE iam.user_account DROP COLUMN mfa_last_step;
DROP INDEX audit.audit_entry_change_ref;
REVOKE DELETE ON app.outbox FROM app_relay;
DROP INDEX app.outbox_dispatched_by_org;
DROP INDEX app.outbox_pending_by_org;
DROP TABLE iam.dashboard_default_filter;
DROP TABLE iam.dashboard_widget_choice;
DROP TABLE app.job;
