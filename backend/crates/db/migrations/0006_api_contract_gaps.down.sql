ALTER TABLE iam.approver_rule DROP CONSTRAINT approver_rule_request_type_check;
ALTER TABLE iam.approver_rule ADD CONSTRAINT approver_rule_request_type_check CHECK (request_type IN
  ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance'));
ALTER TABLE app.approval_step DROP CONSTRAINT approval_step_request_type_check;
ALTER TABLE app.approval_step ADD CONSTRAINT approval_step_request_type_check CHECK (request_type IN
  ('leave','permission','on_duty','regularisation','comp_off','shift_change','punch_flag','period_unlock','settings_change','opening_balance'));
ALTER TABLE sched.shift_change_request DROP COLUMN decided_basis;
ALTER TABLE leave.comp_off_credit DROP CONSTRAINT comp_off_choice_time, DROP COLUMN chosen_at, DROP COLUMN employee_choice;
DROP TABLE notify.push_device;
ALTER TABLE notify.notification DROP COLUMN read_at;
ALTER TABLE platform.organisation DROP COLUMN support_access_needs_approval;
DROP TABLE period.unlock_request;
DROP TABLE org.settings_change_request;
DROP TABLE workforce.contact_change_request;
