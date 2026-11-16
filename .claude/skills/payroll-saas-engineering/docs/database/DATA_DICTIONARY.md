# Data dictionary (V1)

Generated from the validated schema (migrations 0001–0006, PostgreSQL 16). Do not edit by hand: change the migration and regenerate.

100 tables and views. Constraints listed: CHECK, UNIQUE, EXCLUDE (full text). Primary and foreign keys are in the migration files.


## Schema `app`

### `app.approval_step`

Append-only: one row per approval action on any request (DOMAIN_MODEL §6, SM-REQ). BR-07 enforced on the row. Approver equal to requester at routing: OQ-C03-08.

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| acted_at | timestamp with time zone NOT NULL |  |
| level | smallint NOT NULL |  |
| requester_user_id | uuid NOT NULL |  |
| action | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| request_type | text NOT NULL |  |
| actor_user_id | uuid |  |
| on_behalf_of_user_id | uuid |  |
| comment | text |  |
| request_id | uuid NOT NULL |  |

Constraints:
- `approval_step_check`: `CHECK (((action <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (actor_user_id IS NOT NULL)))`
- `approval_step_check3`: `CHECK (((on_behalf_of_user_id IS NULL) OR ((actor_user_id IS NOT NULL) AND (actor_user_id <> on_behalf_of_user_id))))`
- `approval_step_request_type_check`: `CHECK ((request_type = ANY (ARRAY['leave'::text, 'permission'::text, 'on_duty'::text, 'regularisation'::text, 'comp_off'::text, 'shift_change'::text, 'punch_flag'::text, 'period_unlock'::text, 'settings_change'::text, 'opening_balance'::text, 'contact_change'::text])))`
- `approval_step_level_check`: `CHECK ((level = ANY (ARRAY[1, 2])))`
- `approval_step_action_check`: `CHECK ((action = ANY (ARRAY['submitted'::text, 'approved'::text, 'rejected'::text, 'escalated'::text, 'cancelled'::text, 'auto_approved'::text, 'auto_rejected'::text])))`
- `approval_step_check1`: `CHECK (((action <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (actor_user_id <> requester_user_id)))`
- `approval_step_check2`: `CHECK ((on_behalf_of_user_id IS DISTINCT FROM requester_user_id))`

### `app.outbox`

Follow-up work written in the same transaction as the change (ADR-002). Consumers are idempotent by event_id.

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| event_type | text NOT NULL |  |
| attempts | integer NOT NULL |  |
| payload | jsonb NOT NULL |  |
| event_id | uuid NOT NULL |  |
| dispatched_at | timestamp with time zone |  |
| aggregate_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `outbox_attempts_check`: `CHECK ((attempts >= 0))`

### `app.rls_exempt`

| Column | Type | Note |
|---|---|---|
| reason | text NOT NULL |  |
| table_name | regclass NOT NULL |  |

### `app.unprotected_tables` (view)

CI fails if this returns any row (NFR-01.4).

| Column | Type | Note |
|---|---|---|
| table_name | regclass |  |


## Schema `attendance`

### `attendance.absence_classification`

| Column | Type | Note |
|---|---|---|
| decided_by_result_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| classification | text NOT NULL |  |
| absence_date | date NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `absence_classification_classification_check`: `CHECK ((classification = ANY (ARRAY['informed_leave'::text, 'unpaid_leave'::text, 'uninformed_absence'::text])))`
- `absence_classification_org_id_decided_by_result_id_key`: `UNIQUE (org_id, decided_by_result_id)`

### `attendance.attendance_day`

| Column | Type | Note |
|---|---|---|
| updated_by | uuid |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| processing_state | text NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| current_result_id | uuid |  |
| attendance_date | date NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `attendance_day_processing_state_check`: `CHECK ((processing_state = ANY (ARRAY['provisional'::text, 'exception'::text, 'absence_pending'::text, 'finalised'::text, 'locked'::text])))`
- `attendance_day_org_id_employee_id_attendance_date_key`: `UNIQUE (org_id, employee_id, attendance_date)`
- `attendance_day_org_id_id_employee_id_attendance_date_key`: `UNIQUE (org_id, id, employee_id, attendance_date)`

### `attendance.bulk_waiver`

| Column | Type | Note |
|---|---|---|
| updated_by | uuid |  |
| reason | text NOT NULL |  |
| waiver_date | date NOT NULL |  |
| scope_kind | text NOT NULL |  |
| branch_id | uuid |  |
| shift_id | uuid |  |
| withdrawn_at | timestamp with time zone |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| rule_group | text NOT NULL |  |
| created_by | uuid |  |
| withdrawn_by | uuid |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `bulk_waiver_reason_check`: `CHECK ((btrim(reason) <> ''::text))`
- `bulk_waiver_check`: `CHECK ((((scope_kind = 'branch'::text) AND (branch_id IS NOT NULL) AND (shift_id IS NULL)) OR ((scope_kind = 'shift'::text) AND (shift_id IS NOT NULL) AND (branch_id IS NULL))))`
- `bulk_waiver_scope_kind_check`: `CHECK ((scope_kind = ANY (ARRAY['branch'::text, 'shift'::text])))`
- `bulk_waiver_rule_group_check`: `CHECK ((rule_group = ANY (ARRAY['late'::text, 'early'::text])))`

### `attendance.day_effect`

| Column | Type | Note |
|---|---|---|
| days | numeric(5,2) |  |
| minutes | integer |  |
| org_id | uuid NOT NULL |  |
| result_id | uuid NOT NULL |  |
| reason_code | text NOT NULL |  |
| rule_ref | text NOT NULL |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| effect_kind | text NOT NULL |  |

Constraints:
- `day_effect_check`: `CHECK (((days IS NOT NULL) OR (minutes IS NOT NULL)))`
- `day_effect_effect_kind_check`: `CHECK ((effect_kind = ANY (ARRAY['late_slab'::text, 'early_slab'::text, 'missed_punch'::text, 'hours_shortfall'::text, 'absence_lop'::text, 'uninformed_absence_cut'::text])))`
- `day_effect_days_check`: `CHECK ((days >= (0)::numeric))`
- `day_effect_minutes_check`: `CHECK ((minutes >= 0))`

### `attendance.day_result`

Append-only (BRX-03). Inserts for a date inside period.lock_coverage are refused unless for_adjustment (BR-06, DATA_MODEL §5.3). Overlay references and ExpectedSchedule snapshot are recorded in the explanation steps.

| Column | Type | Note |
|---|---|---|
| day_fraction_present | numeric(5,2) NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| sessions | jsonb NOT NULL |  |
| flags | text[] NOT NULL |  |
| early_mark | boolean NOT NULL |  |
| late_mark | boolean NOT NULL |  |
| overtime_identified_minutes | integer NOT NULL |  |
| early_arrival_minutes | integer NOT NULL |  |
| early_minutes | integer NOT NULL |  |
| late_minutes | integer NOT NULL |  |
| net_minutes | integer NOT NULL |  |
| break_minutes | integer NOT NULL |  |
| gross_minutes | integer NOT NULL |  |
| produced_at | timestamp with time zone NOT NULL |  |
| for_adjustment | boolean NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| org_id | uuid NOT NULL |  |
| attendance_day_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| attendance_date | date NOT NULL |  |
| supersedes_result_id | uuid |  |
| policy_version_id | uuid NOT NULL |  |
| shift_version_id | uuid |  |
| schedule_tz | text NOT NULL |  |
| costed_branch_id | uuid |  |
| input_fingerprint | bytea NOT NULL |  |
| engine_version | text NOT NULL |  |
| trigger_kind | text NOT NULL |  |
| trigger_ref | uuid |  |
| day_status | text NOT NULL |  |
| explanation | jsonb NOT NULL |  |

Constraints:
- `day_result_early_arrival_minutes_check`: `CHECK ((early_arrival_minutes >= 0))`
- `day_result_early_minutes_check`: `CHECK ((early_minutes >= 0))`
- `day_result_late_minutes_check`: `CHECK ((late_minutes >= 0))`
- `day_result_net_minutes_check`: `CHECK ((net_minutes >= 0))`
- `day_result_break_minutes_check`: `CHECK ((break_minutes >= 0))`
- `day_result_org_id_id_employee_id_attendance_date_key`: `UNIQUE (org_id, id, employee_id, attendance_date)`
- `day_result_org_id_attendance_day_id_id_key`: `UNIQUE (org_id, attendance_day_id, id)`
- `day_result_check`: `CHECK (((jsonb_typeof(explanation) = 'array'::text) AND (jsonb_typeof(sessions) = 'array'::text)))`
- `day_result_day_fraction_present_check`: `CHECK (((day_fraction_present >= (0)::numeric) AND (day_fraction_present <= (1)::numeric)))`
- `day_result_day_status_check`: `CHECK ((day_status = ANY (ARRAY['present'::text, 'half_day'::text, 'quarter_day'::text, 'absent'::text, 'leave'::text, 'half_day_leave'::text, 'holiday'::text, 'weekly_off'::text, 'on_duty'::text])))`
- `day_result_overtime_identified_minutes_check`: `CHECK ((overtime_identified_minutes >= 0))`
- `day_result_schedule_tz_check`: `CHECK (app.is_valid_timezone(schedule_tz))`
- `day_result_gross_minutes_check`: `CHECK ((gross_minutes >= 0))`

### `attendance.day_result_punch`

| Column | Type | Note |
|---|---|---|
| session_no | smallint |  |
| created_by | uuid |  |
| excluded | boolean NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| excluded_reason | text |  |
| org_id | uuid NOT NULL |  |
| result_id | uuid NOT NULL |  |
| raw_punch_id | uuid NOT NULL |  |
| assigned_direction | text |  |

Constraints:
- `day_result_punch_assigned_direction_check`: `CHECK ((assigned_direction = ANY (ARRAY['in'::text, 'out'::text])))`
- `day_result_punch_check1`: `CHECK ((excluded OR (assigned_direction IS NOT NULL)))`
- `day_result_punch_check`: `CHECK ((excluded = (excluded_reason IS NOT NULL)))`
- `day_result_punch_session_no_check`: `CHECK ((session_no > 0))`

### `attendance.period_counter`

| Column | Type | Note |
|---|---|---|
| counter_period | daterange NOT NULL |  |
| early_minutes | integer NOT NULL |  |
| org_id | uuid NOT NULL |  |
| early_marks | integer NOT NULL |  |
| late_minutes | integer NOT NULL |  |
| late_marks | integer NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| opening_early_minutes | integer NOT NULL |  |
| rebuilt_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| opening_late_minutes | integer NOT NULL |  |
| opening_early_marks | integer NOT NULL |  |
| opening_late_marks | integer NOT NULL |  |
| grace_uses_early | integer NOT NULL |  |
| grace_uses_late | integer NOT NULL |  |
| employee_id | uuid NOT NULL |  |

Constraints:
- `period_counter_counter_period_check`: `CHECK ((NOT isempty(counter_period)))`
- `period_counter_grace_uses_early_check`: `CHECK ((grace_uses_early >= 0))`
- `period_counter_opening_late_marks_check`: `CHECK ((opening_late_marks >= 0))`
- `period_counter_late_marks_check`: `CHECK ((late_marks >= 0))`
- `period_counter_early_marks_check`: `CHECK ((early_marks >= 0))`
- `period_counter_late_minutes_check`: `CHECK ((late_minutes >= 0))`
- `period_counter_early_minutes_check`: `CHECK ((early_minutes >= 0))`
- `period_counter_grace_uses_late_check`: `CHECK ((grace_uses_late >= 0))`
- `period_counter_opening_early_marks_check`: `CHECK ((opening_early_marks >= 0))`
- `period_counter_opening_late_minutes_check`: `CHECK ((opening_late_minutes >= 0))`
- `period_counter_opening_early_minutes_check`: `CHECK ((opening_early_minutes >= 0))`
- `period_counter_org_id_employee_id_counter_period_excl`: `EXCLUDE USING gist (org_id WITH =, employee_id WITH =, counter_period WITH &&)`

### `attendance.period_penalty`

| Column | Type | Note |
|---|---|---|
| explanation | jsonb NOT NULL |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| rule_ref | text NOT NULL |  |
| counter_period | daterange NOT NULL |  |
| period_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| run_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| recovered_as | text |  |
| minutes | integer |  |
| days | numeric(5,2) |  |

Constraints:
- `period_penalty_days_check`: `CHECK ((days >= (0)::numeric))`
- `period_penalty_rule_ref_check`: `CHECK ((rule_ref = ANY (ARRAY['LA-06'::text, 'LA-10'::text, 'EC-05'::text, 'EC-09'::text, 'LM-25'::text, 'LM-26'::text])))`
- `period_penalty_check`: `CHECK (((days IS NOT NULL) OR (minutes IS NOT NULL)))`
- `period_penalty_minutes_check`: `CHECK ((minutes >= 0))`

### `attendance.regularisation_request`

SM-REQ. Cancellation rules: GAP-SM-09. Pending at lock: OQ-C12-03. Window RG-04 and limits RG-03/RG-08 are application checks.

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| escalated | boolean NOT NULL |  |
| org_id | uuid NOT NULL |  |
| state | text NOT NULL |  |
| origin | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| decided_at | timestamp with time zone |  |
| decided_by_user_id | uuid |  |
| current_approver_user_id | uuid |  |
| requested_by_user_id | uuid NOT NULL |  |
| attachment_path | text |  |
| reason | text NOT NULL |  |
| target_raw_punch_id | uuid |  |
| proposed_out_at | timestamp with time zone |  |
| proposed_in_at | timestamp with time zone |  |
| decided_basis | text |  |
| request_type | text NOT NULL |  |
| attendance_date | date NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `regularisation_request_check3`: `CHECK (((origin = 'hr_direct'::text) OR (decided_by_user_id IS DISTINCT FROM requested_by_user_id)))`
- `regularisation_request_request_type_check`: `CHECK ((request_type = ANY (ARRAY['missed_punch'::text, 'late_arrival'::text, 'early_checkout'::text, 'wrong_punch'::text, 'whole_day'::text])))`
- `regularisation_request_origin_check`: `CHECK ((origin = ANY (ARRAY['employee'::text, 'hr_direct'::text])))`
- `regularisation_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`
- `regularisation_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `regularisation_request_reason_check`: `CHECK ((btrim(reason) <> ''::text))`
- `regularisation_request_check`: `CHECK (((proposed_out_at IS NULL) OR (proposed_in_at IS NULL) OR (proposed_out_at > proposed_in_at)))`
- `regularisation_request_check1`: `CHECK (((state <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (decided_basis IS NOT NULL)))`
- `regularisation_request_check2`: `CHECK (((decided_basis <> 'auto_on_submit'::text) OR (state = ANY (ARRAY['approved'::text, 'cancelled'::text]))))`


## Schema `audit`

### `audit.audit_entry`

Insert-only audit log (BRD §16). Entries made under a support grant are visible to the organisation (VC-R6).

| Column | Type | Note |
|---|---|---|
| actor_id | uuid |  |
| id | uuid NOT NULL |  |
| occurred_at | timestamp with time zone NOT NULL |  |
| support_grant_id | uuid |  |
| action | text NOT NULL |  |
| change_ref | text |  |
| org_id | uuid |  |
| reason | text |  |
| before_value | jsonb |  |
| after_value | jsonb |  |
| correlation_id | uuid |  |
| actor_kind | text NOT NULL |  |
| target_type | text NOT NULL |  |
| target_id | uuid |  |

Constraints:
- `audit_entry_actor_kind_check`: `CHECK ((actor_kind = ANY (ARRAY['user'::text, 'vendor_user'::text, 'system_job'::text, 'device'::text])))`


## Schema `iam`

### `iam.approver_rule`

| Column | Type | Note |
|---|---|---|
| level | smallint NOT NULL |  |
| org_id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| named_user_id | uuid |  |
| source | text NOT NULL |  |
| request_type | text NOT NULL |  |

Constraints:
- `approver_rule_level_check`: `CHECK ((level = ANY (ARRAY[1, 2])))`
- `approver_rule_source_check`: `CHECK ((source = ANY (ARRAY['reporting_manager'::text, 'named_user'::text, 'department_head'::text, 'hr'::text])))`
- `approver_rule_request_type_check`: `CHECK ((request_type = ANY (ARRAY['leave'::text, 'permission'::text, 'on_duty'::text, 'regularisation'::text, 'comp_off'::text, 'shift_change'::text, 'punch_flag'::text, 'period_unlock'::text, 'settings_change'::text, 'opening_balance'::text, 'contact_change'::text])))`
- `approver_rule_check`: `CHECK (((source = 'named_user'::text) = (named_user_id IS NOT NULL)))`

### `iam.delegation`

AC-03. Rules (who sets it, scope): OQ-C03-10. BR-07 is checked per request, not here.

| Column | Type | Note |
|---|---|---|
| delegate_user_id | uuid NOT NULL |  |
| valid | tstzrange NOT NULL |  |
| approver_user_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |

Constraints:
- `delegation_check`: `CHECK ((approver_user_id <> delegate_user_id))`
- `delegation_org_id_approver_user_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, approver_user_id WITH =, valid WITH &&)`
- `delegation_valid_check`: `CHECK ((NOT isempty(valid)))`

### `iam.otp_challenge`

| Column | Type | Note |
|---|---|---|
| user_id | uuid NOT NULL |  |
| consumed_at | timestamp with time zone |  |
| expires_at | timestamp with time zone NOT NULL |  |
| code_hash | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| attempts | integer NOT NULL |  |
| id | uuid NOT NULL |  |

Constraints:
- `otp_challenge_attempts_check`: `CHECK ((attempts >= 0))`

### `iam.role`

System roles (system_code set) and custom roles (AC-06). Editability of system roles: OQ-C03-01.

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| system_code | text |  |
| name | text NOT NULL |  |

Constraints:
- `role_org_id_name_key`: `UNIQUE (org_id, name)`
- `role_org_id_system_code_key`: `UNIQUE (org_id, system_code)`
- `role_system_code_check`: `CHECK ((system_code = ANY (ARRAY['org_admin'::text, 'hr_manager'::text, 'payroll_officer'::text, 'manager'::text, 'supervisor'::text, 'employee'::text, 'auditor'::text])))`

### `iam.role_grant`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| role_id | uuid NOT NULL |  |
| operation_code | text NOT NULL |  |
| grant_kind | text NOT NULL |  |
| created_by | uuid |  |
| setting_ref | text |  |

Constraints:
- `role_grant_grant_kind_check`: `CHECK ((grant_kind = ANY (ARRAY['all'::text, 'scoped'::text, 'self'::text, 'approve'::text])))`
- `role_grant_operation_code_check`: `CHECK ((operation_code ~ '^[A-Z]{3}-[0-9]{2}[a-z]?$'::text))`

### `iam.session`

| Column | Type | Note |
|---|---|---|
| device_label | text |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| refresh_token_hash | text NOT NULL |  |
| revoked_at | timestamp with time zone |  |
| org_id | uuid NOT NULL |  |
| expires_at | timestamp with time zone NOT NULL |  |
| user_id | uuid NOT NULL |  |

Constraints:
- `session_refresh_token_hash_key`: `UNIQUE (refresh_token_hash)`

### `iam.user_account`

| Column | Type | Note |
|---|---|---|
| email | text |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| active | boolean NOT NULL |  |
| language | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| last_sign_in_at | timestamp with time zone |  |
| mfa_secret_enc | bytea |  |
| password_hash | text |  |
| mobile | text |  |
| login_kind | text NOT NULL |  |
| employee_id | uuid |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `user_account_check`: `CHECK (((login_kind <> 'mobile_otp'::text) OR (mobile IS NOT NULL)))`
- `user_account_language_check`: `CHECK ((language = ANY (ARRAY['en'::text, 'ta'::text, 'hi'::text])))`
- `user_account_login_kind_check`: `CHECK ((login_kind = ANY (ARRAY['mobile_otp'::text, 'password'::text])))`
- `user_account_check1`: `CHECK (((login_kind <> 'password'::text) OR ((email IS NOT NULL) AND (password_hash IS NOT NULL))))`

### `iam.user_role`

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| user_id | uuid NOT NULL |  |
| role_id | uuid NOT NULL |  |
| created_by | uuid |  |
| org_id | uuid NOT NULL |  |

### `iam.user_scope`

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| department_kind | text |  |
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| department_id | uuid |  |
| branch_id | uuid |  |
| scope_kind | text NOT NULL |  |
| user_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `user_scope_scope_kind_check`: `CHECK ((scope_kind = ANY (ARRAY['all'::text, 'branch'::text, 'department'::text])))`
- `user_scope_check`: `CHECK ((((scope_kind = 'all'::text) AND (branch_id IS NULL) AND (department_id IS NULL)) OR ((scope_kind = 'branch'::text) AND (branch_id IS NOT NULL) AND (department_id IS NULL)) OR ((scope_kind = 'department'::text) AND (department_id IS NOT NULL) AND (branch_id IS NULL))))`


## Schema `leave`

### `leave.absconding_case`

| Column | Type | Note |
|---|---|---|
| closed_at | timestamp with time zone |  |
| close_reason | text |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| state | text NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| trigger_days | integer NOT NULL |  |
| opened_on | date NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| closed_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| pay_hold | boolean NOT NULL |  |

Constraints:
- `absconding_case_state_check`: `CHECK ((state = ANY (ARRAY['open'::text, 'closed_returned'::text, 'closed_offboarding'::text])))`
- `absconding_case_check2`: `CHECK (((state <> 'closed_returned'::text) OR (close_reason IS NOT NULL)))`
- `absconding_case_trigger_days_check`: `CHECK ((trigger_days > 0))`
- `absconding_case_check1`: `CHECK (((state = 'open'::text) OR (closed_by IS NOT NULL)))`
- `absconding_case_check`: `CHECK (((state = 'open'::text) = (closed_at IS NULL)))`

### `leave.comp_off_credit`

SM-COMPOFF. Correction of the worked day after credit: OQ-M02-04. Leave cancelled after expiry: GAP-SM-15.

| Column | Type | Note |
|---|---|---|
| decided_at | timestamp with time zone |  |
| chosen_at | timestamp with time zone |  |
| employee_choice | text |  |
| updated_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| escalated | boolean NOT NULL |  |
| id | uuid NOT NULL |  |
| decided_by | uuid |  |
| current_approver_user_id | uuid |  |
| expires_on | date |  |
| state | text NOT NULL |  |
| earned_basis | text NOT NULL |  |
| fraction | numeric(5,2) NOT NULL |  |
| worked_date | date NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `comp_off_credit_state_check`: `CHECK ((state = ANY (ARRAY['pending_approval'::text, 'credited'::text, 'partially_used'::text, 'used'::text, 'expired_lapsed'::text, 'expired_paid'::text, 'not_credited'::text])))`
- `comp_off_credit_earned_basis_check`: `CHECK ((earned_basis = ANY (ARRAY['policy'::text, 'employee_choice'::text, 'opening'::text])))`
- `comp_off_credit_check`: `CHECK (((state = ANY (ARRAY['pending_approval'::text, 'not_credited'::text])) OR (expires_on IS NOT NULL) OR (earned_basis = 'opening'::text)))`
- `comp_off_choice_time`: `CHECK (((employee_choice IS NULL) = (chosen_at IS NULL)))`
- `comp_off_credit_employee_choice_check`: `CHECK ((employee_choice = ANY (ARRAY['pay'::text, 'comp_off'::text])))`
- `comp_off_credit_fraction_check`: `CHECK ((fraction = ANY (ARRAY[0.5, 1.0])))`

### `leave.comp_off_ledger_entry`

Append-only comp-off ledger (DOMAIN_MODEL §3.9). Consumption order across credits: OQ-M02-09.

| Column | Type | Note |
|---|---|---|
| reason | text |  |
| created_by | uuid |  |
| org_id | uuid NOT NULL |  |
| credit_id | uuid NOT NULL |  |
| entry_date | date NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| entry_kind | text NOT NULL |  |
| quantity | numeric(5,2) NOT NULL |  |
| cause_kind | text NOT NULL |  |
| cause_id | uuid NOT NULL |  |
| reverses_entry_id | uuid |  |

Constraints:
- `comp_off_ledger_entry_entry_kind_check`: `CHECK ((entry_kind = ANY (ARRAY['credit'::text, 'debit'::text, 'reversal'::text, 'expiry'::text])))`
- `comp_off_ledger_entry_check`: `CHECK (((entry_kind = 'reversal'::text) = (reverses_entry_id IS NOT NULL)))`
- `comp_off_ledger_entry_org_id_cause_kind_cause_id_entry_kind_key`: `UNIQUE (org_id, cause_kind, cause_id, entry_kind, credit_id)`
- `comp_off_ledger_entry_check1`: `CHECK (((entry_kind <> ALL (ARRAY['debit'::text, 'expiry'::text])) OR (quantity < (0)::numeric)))`
- `comp_off_ledger_entry_check2`: `CHECK (((entry_kind <> 'credit'::text) OR (quantity > (0)::numeric)))`
- `comp_off_ledger_entry_quantity_check`: `CHECK ((quantity <> (0)::numeric))`

### `leave.leave_account`

Lock target for balance checks (LM-07). Balance = sum of leave_ledger_entry (LM-R4); never stored. Not append-only only because FOR UPDATE needs the UPDATE grant.

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| leave_type_id | uuid NOT NULL |  |

Constraints:
- `leave_account_org_id_employee_id_leave_type_id_key`: `UNIQUE (org_id, employee_id, leave_type_id)`

### `leave.leave_balance` (view)

Balance = sum of ledger entries (LM-R4). security_invoker so the caller's RLS applies.

| Column | Type | Note |
|---|---|---|
| balance | numeric(8,2) |  |
| org_id | uuid |  |
| account_id | uuid |  |
| employee_id | uuid |  |
| leave_type_id | uuid |  |

### `leave.leave_ledger_entry`

Append-only leave ledger (LM-R4, BR-01). Cancellation posts a reversal, never a delete. Pending-request reservation: OQ-C08-08. Debit order for penalties: DP-02.

| Column | Type | Note |
|---|---|---|
| cause_id | uuid NOT NULL |  |
| cause_kind | text NOT NULL |  |
| account_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| entry_date | date NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| quantity | numeric(6,2) NOT NULL |  |
| entry_kind | text NOT NULL |  |
| reverses_entry_id | uuid |  |

Constraints:
- `leave_ledger_entry_check1`: `CHECK (((entry_kind <> ALL (ARRAY['debit'::text, 'sandwich_debit'::text, 'penalty_debit'::text, 'absence_auto_debit'::text, 'lapse'::text, 'encashment'::text])) OR (quantity < (0)::numeric)))`
- `leave_ledger_entry_check`: `CHECK (((entry_kind = 'reversal'::text) = (reverses_entry_id IS NOT NULL)))`
- `leave_ledger_entry_org_id_cause_kind_cause_id_entry_kind_ac_key`: `UNIQUE (org_id, cause_kind, cause_id, entry_kind, account_id)`
- `leave_ledger_entry_check2`: `CHECK (((entry_kind <> ALL (ARRAY['accrual'::text, 'grant'::text])) OR (quantity > (0)::numeric)))`
- `leave_ledger_entry_check3`: `CHECK (((entry_kind <> 'adjustment'::text) OR (reason IS NOT NULL)))`
- `leave_ledger_entry_entry_kind_check`: `CHECK ((entry_kind = ANY (ARRAY['opening'::text, 'accrual'::text, 'grant'::text, 'debit'::text, 'reversal'::text, 'sandwich_debit'::text, 'penalty_debit'::text, 'absence_auto_debit'::text, 'carry_forward'::text, 'lapse'::text, 'encashment'::text, 'adjustment'::text])))`
- `leave_ledger_entry_quantity_check`: `CHECK ((quantity <> (0)::numeric))`

### `leave.leave_request`

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| leave_type_id | uuid NOT NULL |  |
| from_date | date NOT NULL |  |
| to_date | date NOT NULL |  |
| reason | text |  |
| document_path | text |  |
| state | text NOT NULL |  |
| decided_basis | text |  |
| current_approver_user_id | uuid |  |
| requested_by | uuid NOT NULL |  |
| cancelled_by | uuid |  |
| cancelled_at | timestamp with time zone |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| from_part | text NOT NULL |  |
| to_part | text NOT NULL |  |
| escalated | boolean NOT NULL |  |
| submitted_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `leave_request_from_part_check`: `CHECK ((from_part = ANY (ARRAY['full'::text, 'first_half'::text, 'second_half'::text])))`
- `leave_request_check`: `CHECK ((to_date >= from_date))`
- `leave_request_check4`: `CHECK (((state <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (decided_basis IS NOT NULL)))`
- `leave_request_check5`: `CHECK (((state = 'cancelled'::text) = (cancelled_at IS NOT NULL)))`
- `leave_request_check3`: `CHECK (((state <> ALL (ARRAY['pending_l1'::text, 'pending_l2'::text])) OR (decided_basis IS NULL)))`
- `leave_request_check2`: `CHECK (((document_path IS NULL) OR starts_with(document_path, ((org_id)::text || '/'::text))))`
- `leave_request_check1`: `CHECK (((from_date < to_date) OR (from_part = to_part)))`
- `leave_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `leave_request_to_part_check`: `CHECK ((to_part = ANY (ARRAY['full'::text, 'first_half'::text, 'second_half'::text])))`
- `leave_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`

### `leave.leave_request_day`

One row per requested date, written at submission (a submitted request is never edited: SM-REQ). Date within the request range is checked by the application.

| Column | Type | Note |
|---|---|---|
| leave_date | date NOT NULL |  |
| half | text |  |
| fraction | numeric(5,2) NOT NULL |  |
| created_by | uuid |  |
| leave_request_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `leave_request_day_fraction_check`: `CHECK ((fraction = ANY (ARRAY[0.5, 1.0])))`
- `leave_request_day_check`: `CHECK (((fraction = 0.5) = (half IS NOT NULL)))`
- `leave_request_day_half_check`: `CHECK ((half = ANY (ARRAY['first_half'::text, 'second_half'::text])))`

### `leave.leave_type`

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| code | text NOT NULL |  |
| name | text NOT NULL |  |
| retired_at | timestamp with time zone |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `leave_type_org_id_code_key`: `UNIQUE (org_id, code)`
- `leave_type_org_id_name_key`: `UNIQUE (org_id, name)`

### `leave.leave_type_version`

Append-only versions of a leave type (LM-01..15, G5). In force on a date = latest effective_from <= date. eligibility jsonb: category / employee type / after probation (LM-01); payroll group is V2 (OQ-C08-01).

| Column | Type | Note |
|---|---|---|
| half_day_allowed | boolean NOT NULL |  |
| year_end | text NOT NULL |  |
| proration | text NOT NULL |  |
| accrual_timing | text NOT NULL |  |
| credit_method | text NOT NULL |  |
| eligibility | jsonb NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| reason | text |  |
| document_after_days | numeric(5,2) |  |
| max_consecutive_days | numeric(5,2) |  |
| carry_forward_max | numeric(5,2) |  |
| max_balance | numeric(5,2) |  |
| earn_per_worked_days | integer |  |
| credit_days | numeric(5,2) |  |
| paid | boolean NOT NULL |  |
| effective_from | date NOT NULL |  |
| leave_type_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| combine_allowed | boolean NOT NULL |  |
| during_notice | text NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| timeout_action | text NOT NULL |  |
| timeout_days | integer |  |
| approval_levels | text NOT NULL |  |
| after_event_days | integer |  |
| min_notice_days | integer NOT NULL |  |
| negative_allowed_days | numeric(5,2) NOT NULL |  |

Constraints:
- `leave_type_version_carry_forward_max_check`: `CHECK (((carry_forward_max IS NULL) OR (carry_forward_max >= (0)::numeric)))`
- `leave_type_version_org_id_leave_type_id_effective_from_key`: `UNIQUE (org_id, leave_type_id, effective_from)`
- `leave_type_version_check1`: `CHECK (((credit_method = 'earned_by_days_worked'::text) = (earn_per_worked_days IS NOT NULL)))`
- `leave_type_version_check`: `CHECK (((year_end <> 'carry_forward'::text) OR (carry_forward_max IS NOT NULL)))`
- `leave_type_version_during_notice_check`: `CHECK ((during_notice = ANY (ARRAY['allow'::text, 'block'::text, 'extend_notice'::text])))`
- `leave_type_version_timeout_action_check`: `CHECK ((timeout_action = ANY (ARRAY['escalate'::text, 'auto_approve'::text, 'auto_reject'::text])))`
- `leave_type_version_timeout_days_check`: `CHECK (((timeout_days IS NULL) OR (timeout_days > 0)))`
- `leave_type_version_approval_levels_check`: `CHECK ((approval_levels = ANY (ARRAY['manager'::text, 'manager_then_hr'::text])))`
- `leave_type_version_document_after_days_check`: `CHECK (((document_after_days IS NULL) OR (document_after_days >= (0)::numeric)))`
- `leave_type_version_after_event_days_check`: `CHECK (((after_event_days IS NULL) OR (after_event_days >= 0)))`
- `leave_type_version_max_consecutive_days_check`: `CHECK (((max_consecutive_days IS NULL) OR (max_consecutive_days > (0)::numeric)))`
- `leave_type_version_min_notice_days_check`: `CHECK ((min_notice_days >= 0))`
- `leave_type_version_negative_allowed_days_check`: `CHECK ((negative_allowed_days >= (0)::numeric))`
- `leave_type_version_year_end_check`: `CHECK ((year_end = ANY (ARRAY['carry_forward'::text, 'lapse'::text, 'encash'::text])))`
- `leave_type_version_max_balance_check`: `CHECK (((max_balance IS NULL) OR (max_balance >= (0)::numeric)))`
- `leave_type_version_proration_check`: `CHECK ((proration = ANY (ARRAY['prorated'::text, 'full'::text, 'none'::text])))`
- `leave_type_version_accrual_timing_check`: `CHECK ((accrual_timing = ANY (ARRAY['start'::text, 'end'::text])))`
- `leave_type_version_earn_per_worked_days_check`: `CHECK (((earn_per_worked_days IS NULL) OR (earn_per_worked_days > 0)))`
- `leave_type_version_credit_days_check`: `CHECK (((credit_days IS NULL) OR (credit_days >= (0)::numeric)))`
- `leave_type_version_credit_method_check`: `CHECK ((credit_method = ANY (ARRAY['annual_grant'::text, 'monthly_accrual'::text, 'quarterly_accrual'::text, 'earned_by_days_worked'::text])))`


## Schema `notify`

### `notify.notification`

| Column | Type | Note |
|---|---|---|
| sent_at | timestamp with time zone |  |
| id | uuid NOT NULL |  |
| payload | jsonb NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| event_code | text NOT NULL |  |
| read_at | timestamp with time zone |  |
| updated_at | timestamp with time zone NOT NULL |  |
| attempts | integer NOT NULL |  |
| state | text NOT NULL |  |
| channel | text NOT NULL |  |
| language | text NOT NULL |  |
| dedupe_key | text NOT NULL |  |
| recipient_user_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `notification_event_code_check`: `CHECK ((event_code ~ '^NT-(0[1-9]|1[0-4])$'::text))`
- `notification_org_id_dedupe_key_key`: `UNIQUE (org_id, dedupe_key)`
- `notification_check`: `CHECK (((state = 'sent'::text) = (sent_at IS NOT NULL)))`
- `notification_attempts_check`: `CHECK ((attempts >= 0))`
- `notification_state_check`: `CHECK ((state = ANY (ARRAY['queued'::text, 'sent'::text, 'failed'::text])))`
- `notification_channel_check`: `CHECK ((channel = ANY (ARRAY['in_app'::text, 'email'::text, 'sms'::text, 'whatsapp'::text, 'push'::text])))`
- `notification_language_check`: `CHECK ((language = ANY (ARRAY['en'::text, 'ta'::text, 'hi'::text])))`

### `notify.notification_rule`

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| event_code | text NOT NULL |  |
| channel | text NOT NULL |  |
| recipient_kind | text NOT NULL |  |
| enabled | boolean NOT NULL |  |
| updated_by | uuid |  |

Constraints:
- `notification_rule_channel_check`: `CHECK ((channel = ANY (ARRAY['in_app'::text, 'email'::text, 'sms'::text, 'whatsapp'::text, 'push'::text])))`
- `notification_rule_event_code_check`: `CHECK ((event_code ~ '^NT-(0[1-9]|1[0-4])$'::text))`

### `notify.push_device`

| Column | Type | Note |
|---|---|---|
| app_version | text |  |
| token | text NOT NULL |  |
| id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| registered_at | timestamp with time zone NOT NULL |  |
| platform | text NOT NULL |  |
| user_id | uuid NOT NULL |  |
| revoked_at | timestamp with time zone |  |

Constraints:
- `push_device_platform_check`: `CHECK ((platform = ANY (ARRAY['android'::text, 'ios'::text])))`


## Schema `openbal`

### `openbal.opening_balance_batch`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| state | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| confirmed_at | timestamp with time zone |  |
| confirmed_by | uuid |  |
| submitted_by | uuid |  |
| upload_path | text |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `opening_balance_batch_state_check`: `CHECK ((state = ANY (ARRAY['draft'::text, 'pending_approval'::text, 'confirmed'::text])))`
- `opening_balance_batch_check`: `CHECK (((state = 'confirmed'::text) = ((confirmed_at IS NOT NULL) AND (confirmed_by IS NOT NULL))))`
- `opening_balance_batch_check1`: `CHECK (((submitted_by IS NULL) OR (confirmed_by IS DISTINCT FROM submitted_by)))`

### `openbal.opening_balance_entry`

| Column | Type | Note |
|---|---|---|
| batch_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| balance_kind | text NOT NULL |  |
| leave_type_id | uuid |  |
| permission_type_id | uuid |  |
| value | numeric(7,2) NOT NULL |  |
| as_of_date | date NOT NULL |  |
| superseded_by_entry_id | uuid |  |
| change_reason | text |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |

Constraints:
- `opening_balance_entry_check`: `CHECK (((balance_kind = 'leave'::text) = (leave_type_id IS NOT NULL)))`
- `opening_balance_entry_balance_kind_check`: `CHECK ((balance_kind = ANY (ARRAY['leave'::text, 'comp_off'::text, 'permission_used'::text, 'late_counter'::text, 'early_counter'::text])))`
- `opening_balance_entry_check2`: `CHECK (((superseded_by_entry_id IS NULL) OR (superseded_by_entry_id <> id)))`
- `opening_balance_entry_check1`: `CHECK (((balance_kind = 'permission_used'::text) = (permission_type_id IS NOT NULL)))`


## Schema `org`

### `org.branch`

| Column | Type | Note |
|---|---|---|
| code | text NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| closed_on | date |  |
| holiday_calendar_id | uuid |  |
| address | text |  |
| name | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |

Constraints:
- `branch_org_id_code_key`: `UNIQUE (org_id, code)`

### `org.branch_ip_allow`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| network | cidr NOT NULL |  |
| branch_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

### `org.branch_timezone`

| Column | Type | Note |
|---|---|---|
| tz | text NOT NULL |  |
| branch_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| valid | daterange NOT NULL |  |

Constraints:
- `branch_timezone_tz_check`: `CHECK (app.is_valid_timezone(tz))`
- `branch_timezone_org_id_branch_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, branch_id WITH =, valid WITH &&)`
- `branch_timezone_valid_check`: `CHECK ((NOT isempty(valid)))`

### `org.contractor`

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| retired_at | timestamp with time zone |  |
| contact | text |  |
| name | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `contractor_org_id_name_key`: `UNIQUE (org_id, name)`

### `org.geo_fence`

| Column | Type | Note |
|---|---|---|
| tested_at | timestamp with time zone |  |
| updated_by | uuid |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| enabled | boolean NOT NULL |  |
| radius_m | integer NOT NULL |  |
| latitude | numeric(9,6) NOT NULL |  |
| longitude | numeric(9,6) NOT NULL |  |
| name | text |  |
| branch_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| tested_by | uuid |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |

Constraints:
- `geo_fence_longitude_check`: `CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric)))`
- `geo_fence_latitude_check`: `CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric)))`
- `geo_fence_radius_m_check`: `CHECK ((radius_m > 0))`

### `org.list_entry`

| Column | Type | Note |
|---|---|---|
| updated_by | uuid |  |
| created_by | uuid |  |
| retired_at | timestamp with time zone |  |
| code | text |  |
| name | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| kind | text NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |

Constraints:
- `list_entry_kind_check`: `CHECK ((kind = ANY (ARRAY['department'::text, 'designation'::text, 'category'::text, 'employee_type'::text])))`
- `list_entry_org_id_id_kind_key`: `UNIQUE (org_id, id, kind)`
- `list_entry_org_id_kind_name_key`: `UNIQUE (org_id, kind, name)`

### `org.organisation_settings_version`

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| schema_version | integer NOT NULL |  |
| settings | jsonb NOT NULL |  |
| effective_from | date NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `organisation_settings_version_org_id_effective_from_key`: `UNIQUE (org_id, effective_from)`
- `organisation_settings_version_schema_version_check`: `CHECK ((schema_version > 0))`

### `org.settings_change_request`

| Column | Type | Note |
|---|---|---|
| state | text NOT NULL |  |
| submitted_at | timestamp with time zone NOT NULL |  |
| resulting_version_id | uuid |  |
| effective_from | date NOT NULL |  |
| decided_by | uuid |  |
| reason | text |  |
| proposed | jsonb NOT NULL |  |
| target | text NOT NULL |  |
| submitted_by | uuid NOT NULL |  |
| id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| decided_at | timestamp with time zone |  |

Constraints:
- `settings_change_request_check`: `CHECK (((decided_by IS NULL) OR (decided_by <> submitted_by)))`
- `settings_change_request_state_check`: `CHECK ((state = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text])))`
- `settings_change_request_target_check`: `CHECK ((target = ANY (ARRAY['organisation_settings'::text, 'access_settings'::text, 'employee_visibility'::text, 'branch_ip_allow_list'::text])))`


## Schema `period`

### `period.attendance_cycle`

Attendance period cycle (PL-01, PL-02). Owner is open (DP-01); locks are materialised per employee in lock_coverage, so the answer does not change the lock rule.

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| start_kind | text NOT NULL |  |
| frequency | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| valid | daterange NOT NULL |  |
| cutoff_day | smallint |  |
| custom_start_day | smallint |  |
| owner_id | uuid NOT NULL |  |
| owner_kind | text NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `attendance_cycle_cutoff_day_check`: `CHECK (((cutoff_day >= 1) AND (cutoff_day <= 31)))`
- `attendance_cycle_check`: `CHECK (((start_kind = 'custom'::text) = (custom_start_day IS NOT NULL)))`
- `attendance_cycle_org_id_owner_kind_owner_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, owner_kind WITH =, owner_id WITH =, valid WITH &&)`
- `attendance_cycle_start_kind_check`: `CHECK ((start_kind = ANY (ARRAY['calendar'::text, 'custom'::text])))`
- `attendance_cycle_custom_start_day_check`: `CHECK (((custom_start_day >= 1) AND (custom_start_day <= 31)))`
- `attendance_cycle_owner_kind_check`: `CHECK ((owner_kind = ANY (ARRAY['organisation'::text, 'branch'::text, 'policy_set'::text])))`
- `attendance_cycle_frequency_check`: `CHECK ((frequency = ANY (ARRAY['daily'::text, 'weekly'::text, 'bi_weekly'::text, 'monthly'::text])))`
- `attendance_cycle_valid_check`: `CHECK ((NOT isempty(valid)))`

### `period.attendance_period`

SM-PERIOD. Lock before period end: GAP-SM-12. Unlock effects on posted adjustments/exports: OQ-C12-07.

| Column | Type | Note |
|---|---|---|
| locked_by | uuid |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| state | text NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| period_end | date NOT NULL |  |
| period_start | date NOT NULL |  |
| cutoff_date | date |  |
| checklist_snapshot | jsonb |  |
| locked_at | timestamp with time zone |  |
| cycle_id | uuid NOT NULL |  |

Constraints:
- `attendance_period_org_id_cycle_id_daterange_excl`: `EXCLUDE USING gist (org_id WITH =, cycle_id WITH =, daterange(period_start, period_end, '[]'::text) WITH &&)`
- `attendance_period_check2`: `CHECK (((state <> 'locked'::text) OR (locked_at IS NOT NULL)))`
- `attendance_period_state_check`: `CHECK ((state = ANY (ARRAY['open'::text, 'past_cut_off'::text, 'awaiting_lock'::text, 'locked'::text, 'unlocked'::text])))`
- `attendance_period_check`: `CHECK ((period_end >= period_start))`
- `attendance_period_check1`: `CHECK (((cutoff_date IS NULL) OR ((cutoff_date >= period_start) AND (cutoff_date <= period_end))))`

### `period.lock_coverage`

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| dates | daterange NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| period_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `lock_coverage_dates_check`: `CHECK ((NOT isempty(dates)))`

### `period.payroll_input_export`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| produced_at | timestamp with time zone NOT NULL |  |
| contract_version | text NOT NULL |  |
| id | uuid NOT NULL |  |
| checksum | text NOT NULL |  |
| file_path | text NOT NULL |  |
| period_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| produced_by | uuid |  |

Constraints:
- `payroll_input_export_contract_version_check`: `CHECK ((contract_version = 'PayrollInput v1'::text))`

### `period.payroll_input_export_adjustment`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| export_id | uuid NOT NULL |  |
| adjustment_id | uuid NOT NULL |  |
| created_by | uuid |  |

### `period.period_adjustment`

SM-ADJ. Only state changes after insert (posted -> frozen -> exported). Repeat change of the same date: OQ-C12-06; source unlocked: OQ-C12-07.

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| source_period_id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| state | text NOT NULL |  |
| original_date | date NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| locked_result_id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| target_period_id | uuid NOT NULL |  |
| days | numeric(5,2) NOT NULL |  |
| difference | jsonb NOT NULL |  |
| new_result_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `period_adjustment_check1`: `CHECK ((new_result_id <> locked_result_id))`
- `period_adjustment_check`: `CHECK ((target_period_id <> source_period_id))`
- `period_adjustment_org_id_employee_id_original_date_new_resu_key`: `UNIQUE (org_id, employee_id, original_date, new_result_id)`
- `period_adjustment_state_check`: `CHECK ((state = ANY (ARRAY['posted'::text, 'frozen'::text, 'exported'::text])))`

### `period.period_state_history`

| Column | Type | Note |
|---|---|---|
| changed_by | uuid |  |
| org_id | uuid NOT NULL |  |
| period_id | uuid NOT NULL |  |
| from_state | text |  |
| to_state | text NOT NULL |  |
| reason | text |  |
| approved_by | uuid |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `period_state_history_to_state_check`: `CHECK ((to_state = ANY (ARRAY['open'::text, 'past_cut_off'::text, 'awaiting_lock'::text, 'locked'::text, 'unlocked'::text])))`
- `period_state_history_check1`: `CHECK (((approved_by IS NULL) OR (approved_by IS DISTINCT FROM changed_by)))`
- `period_state_history_from_state_check`: `CHECK ((from_state = ANY (ARRAY['open'::text, 'past_cut_off'::text, 'awaiting_lock'::text, 'locked'::text, 'unlocked'::text])))`
- `period_state_history_check`: `CHECK (((to_state <> 'unlocked'::text) OR ((reason IS NOT NULL) AND (approved_by IS NOT NULL))))`

### `period.period_totals`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| org_id | uuid NOT NULL |  |
| period_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| payable_days | numeric(5,2) NOT NULL |  |
| deduction_days | numeric(5,2) NOT NULL |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| frozen_at | timestamp with time zone NOT NULL |  |
| pay_hold | boolean NOT NULL |  |
| branch_days | jsonb NOT NULL |  |
| overtime_identified_minutes | integer NOT NULL |  |
| early_minutes | integer NOT NULL |  |
| late_minutes | integer NOT NULL |  |
| deduction_lines | jsonb NOT NULL |  |

Constraints:
- `period_totals_overtime_identified_minutes_check`: `CHECK ((overtime_identified_minutes >= 0))`
- `period_totals_payable_days_check`: `CHECK ((payable_days >= (0)::numeric))`
- `period_totals_deduction_days_check`: `CHECK ((deduction_days >= (0)::numeric))`
- `period_totals_late_minutes_check`: `CHECK ((late_minutes >= 0))`
- `period_totals_early_minutes_check`: `CHECK ((early_minutes >= 0))`

### `period.unlock_request`

| Column | Type | Note |
|---|---|---|
| state | text NOT NULL |  |
| reason | text NOT NULL |  |
| period_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| escalated | boolean NOT NULL |  |
| requested_by | uuid NOT NULL |  |
| decided_basis | text |  |
| current_approver_user_id | uuid |  |
| id | uuid NOT NULL |  |
| requested_at | timestamp with time zone NOT NULL |  |

Constraints:
- `unlock_request_check`: `CHECK (((state <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (decided_basis IS NOT NULL)))`
- `unlock_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `unlock_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`


## Schema `perm`

### `perm.on_duty_request`

OD-02..07. Never linked to permission usage (OD-12): no FK to perm.permission_usage_entry by design. Half-day part, OD-07 punch evidence: OQ-C09-12, OQ-C09-13.

| Column | Type | Note |
|---|---|---|
| state | text NOT NULL |  |
| place | text NOT NULL |  |
| purpose | text NOT NULL |  |
| to_time | time without time zone |  |
| from_time | time without time zone |  |
| to_date | date NOT NULL |  |
| from_date | date NOT NULL |  |
| duration_kind | text NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| cancelled_by | uuid |  |
| cancelled_at | timestamp with time zone |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| escalated | boolean NOT NULL |  |
| submitted_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| proof_path | text |  |
| requested_by | uuid NOT NULL |  |
| current_approver_user_id | uuid |  |
| decided_basis | text |  |

Constraints:
- `on_duty_request_check6`: `CHECK (((state = 'cancelled'::text) = (cancelled_at IS NOT NULL)))`
- `on_duty_request_check4`: `CHECK (((state <> ALL (ARRAY['pending_l1'::text, 'pending_l2'::text])) OR (decided_basis IS NULL)))`
- `on_duty_request_check3`: `CHECK (((proof_path IS NULL) OR starts_with(proof_path, ((org_id)::text || '/'::text))))`
- `on_duty_request_check2`: `CHECK (((duration_kind = 'hours'::text) = ((from_time IS NOT NULL) AND (to_time IS NOT NULL))))`
- `on_duty_request_check1`: `CHECK (((duration_kind = 'several_days'::text) OR (to_date = from_date)))`
- `on_duty_request_check`: `CHECK ((to_date >= from_date))`
- `on_duty_request_duration_kind_check`: `CHECK ((duration_kind = ANY (ARRAY['hours'::text, 'half_day'::text, 'full_day'::text, 'several_days'::text])))`
- `on_duty_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`
- `on_duty_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `on_duty_request_check5`: `CHECK (((state <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (decided_basis IS NOT NULL)))`

### `perm.permission_request`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| end_time | time without time zone NOT NULL |  |
| permission_date | date NOT NULL |  |
| permission_type_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| minutes | integer NOT NULL |  |
| reason | text NOT NULL |  |
| document_path | text |  |
| state | text NOT NULL |  |
| decided_basis | text |  |
| current_approver_user_id | uuid |  |
| requested_by | uuid |  |
| cancelled_by | uuid |  |
| cancelled_at | timestamp with time zone |  |
| start_time | time without time zone NOT NULL |  |
| origin | text NOT NULL |  |
| escalated | boolean NOT NULL |  |
| submitted_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |

Constraints:
- `permission_request_check3`: `CHECK (((state <> ALL (ARRAY['pending_l1'::text, 'pending_l2'::text])) OR (decided_basis IS NULL)))`
- `permission_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `permission_request_check`: `CHECK ((end_time <> start_time))`
- `permission_request_check1`: `CHECK (((origin <> 'applied'::text) OR (requested_by IS NOT NULL)))`
- `permission_request_check2`: `CHECK (((document_path IS NULL) OR starts_with(document_path, ((org_id)::text || '/'::text))))`
- `permission_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`
- `permission_request_check4`: `CHECK (((state <> ALL (ARRAY['approved'::text, 'rejected'::text])) OR (decided_basis IS NOT NULL)))`
- `permission_request_check5`: `CHECK (((state = 'cancelled'::text) = (cancelled_at IS NOT NULL)))`
- `permission_request_minutes_check`: `CHECK ((minutes > 0))`
- `permission_request_origin_check`: `CHECK ((origin = ANY (ARRAY['applied'::text, 'converted_late'::text, 'converted_early'::text])))`

### `perm.permission_type`

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| name | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| code | text NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| retired_at | timestamp with time zone |  |

Constraints:
- `permission_type_org_id_name_key`: `UNIQUE (org_id, name)`
- `permission_type_org_id_code_key`: `UNIQUE (org_id, code)`

### `perm.permission_type_version`

Append-only versions of a permission type (PM-02..30, G5). In force on a date = latest effective_from <= date. PM-28 reason list and PM-29 cancel rule are fixed behaviour, not columns.

| Column | Type | Note |
|---|---|---|
| reason | text |  |
| max_consecutive_days | integer |  |
| max_per_week | integer |  |
| effective_from | date NOT NULL |  |
| permission_type_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| limit_minutes | integer |  |
| carry_forward_max | integer |  |
| limit_count | integer |  |
| limit_basis | text NOT NULL |  |
| max_minutes | integer NOT NULL |  |
| min_minutes | integer NOT NULL |  |
| duration_unit | text NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| timeout_minutes | integer |  |
| timeout_action | text NOT NULL |  |
| over_limit | text NOT NULL |  |
| over_stay | text NOT NULL |  |
| same_day_half_leave_allowed | boolean NOT NULL |  |
| adjacent_off_allowed | boolean NOT NULL |  |
| mid_period_joiner | text NOT NULL |  |
| eligibility | jsonb NOT NULL |  |
| approval | text NOT NULL |  |
| min_notice_minutes | integer NOT NULL |  |
| blackout_enabled | boolean NOT NULL |  |
| blackout_dates | date[] NOT NULL |  |
| punch_requirement | jsonb NOT NULL |  |
| converted_uses_limit | boolean NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| after_event_days | integer |  |
| apply_window | text NOT NULL |  |
| counts_toward_overtime | boolean NOT NULL |  |
| counts_as_worked | boolean NOT NULL |  |
| paid | boolean NOT NULL |  |
| allowed_position | text NOT NULL |  |
| max_per_day | integer |  |
| limit_period | text NOT NULL |  |

Constraints:
- `permission_type_version_min_minutes_check`: `CHECK ((min_minutes > 0))`
- `permission_type_version_org_id_permission_type_id_effective_key`: `UNIQUE (org_id, permission_type_id, effective_from)`
- `permission_type_version_check3`: `CHECK (((apply_window <> 'after_within_days'::text) OR (after_event_days IS NOT NULL)))`
- `permission_type_version_check2`: `CHECK (((limit_basis = 'count'::text) OR (limit_minutes IS NOT NULL)))`
- `permission_type_version_check1`: `CHECK (((limit_basis = 'hours'::text) OR (limit_count IS NOT NULL)))`
- `permission_type_version_check`: `CHECK ((max_minutes >= min_minutes))`
- `permission_type_version_mid_period_joiner_check`: `CHECK ((mid_period_joiner = ANY (ARRAY['full'::text, 'prorated'::text])))`
- `permission_type_version_carry_forward_max_check`: `CHECK (((carry_forward_max IS NULL) OR (carry_forward_max > 0)))`
- `permission_type_version_max_consecutive_days_check`: `CHECK (((max_consecutive_days IS NULL) OR (max_consecutive_days > 0)))`
- `permission_type_version_over_stay_check`: `CHECK ((over_stay = ANY (ARRAY['late_early_minutes'::text, 'half_day_leave'::text, 'second_permission'::text, 'flag_only'::text])))`
- `permission_type_version_over_limit_check`: `CHECK ((over_limit = ANY (ARRAY['block'::text, 'hr_approval_unpaid'::text, 'convert_half_day_leave'::text, 'debit_leave_hourly'::text])))`
- `permission_type_version_timeout_action_check`: `CHECK ((timeout_action = ANY (ARRAY['escalate'::text, 'auto_approve'::text, 'auto_reject'::text])))`
- `permission_type_version_timeout_minutes_check`: `CHECK (((timeout_minutes IS NULL) OR (timeout_minutes > 0)))`
- `permission_type_version_approval_check`: `CHECK ((approval = ANY (ARRAY['auto_within_limit'::text, 'manager'::text, 'manager_then_hr'::text])))`
- `permission_type_version_min_notice_minutes_check`: `CHECK ((min_notice_minutes >= 0))`
- `permission_type_version_after_event_days_check`: `CHECK (((after_event_days IS NULL) OR (after_event_days >= 0)))`
- `permission_type_version_apply_window_check`: `CHECK ((apply_window = ANY (ARRAY['before_only'::text, 'before_or_on_day'::text, 'after_within_days'::text])))`
- `permission_type_version_allowed_position_check`: `CHECK ((allowed_position = ANY (ARRAY['start_of_shift'::text, 'end_of_shift'::text, 'middle_of_shift'::text, 'any'::text])))`
- `permission_type_version_max_per_week_check`: `CHECK (((max_per_week IS NULL) OR (max_per_week > 0)))`
- `permission_type_version_max_per_day_check`: `CHECK (((max_per_day IS NULL) OR (max_per_day > 0)))`
- `permission_type_version_limit_period_check`: `CHECK ((limit_period = ANY (ARRAY['week'::text, 'calendar_month'::text, 'payroll_period'::text, 'quarter'::text, 'year'::text])))`
- `permission_type_version_limit_minutes_check`: `CHECK (((limit_minutes IS NULL) OR (limit_minutes >= 0)))`
- `permission_type_version_limit_count_check`: `CHECK (((limit_count IS NULL) OR (limit_count >= 0)))`
- `permission_type_version_limit_basis_check`: `CHECK ((limit_basis = ANY (ARRAY['count'::text, 'hours'::text, 'both'::text])))`
- `permission_type_version_duration_unit_check`: `CHECK ((duration_unit = ANY (ARRAY['any_minutes'::text, 'block_30'::text, 'block_60'::text])))`

### `perm.permission_usage_entry`

Append-only PermissionAccount usage ledger (DOMAIN_MODEL §3.10). Usage = sum per employee, type and limit period. On-duty never posts here (OD-12). Reservation by pending requests: OQ-C09-04.

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| reverses_entry_id | uuid |  |
| limit_period | daterange NOT NULL |  |
| permission_type_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| entry_kind | text NOT NULL |  |
| origin | text NOT NULL |  |
| count_delta | integer NOT NULL |  |
| minutes_delta | integer NOT NULL |  |
| cause_kind | text NOT NULL |  |
| cause_id | uuid NOT NULL |  |

Constraints:
- `permission_usage_entry_check`: `CHECK (((entry_kind = 'reversal'::text) = (reverses_entry_id IS NOT NULL)))`
- `permission_usage_entry_org_id_cause_kind_cause_id_entry_kin_key`: `UNIQUE (org_id, cause_kind, cause_id, entry_kind)`
- `permission_usage_entry_check1`: `CHECK (((count_delta <> 0) OR (minutes_delta <> 0)))`
- `permission_usage_entry_origin_check`: `CHECK ((origin = ANY (ARRAY['request'::text, 'converted_late'::text, 'converted_early'::text, 'opening'::text])))`
- `permission_usage_entry_entry_kind_check`: `CHECK ((entry_kind = ANY (ARRAY['usage'::text, 'reversal'::text])))`
- `permission_usage_entry_limit_period_check`: `CHECK ((NOT isempty(limit_period)))`


## Schema `platform`

### `platform.announcement`

| Column | Type | Note |
|---|---|---|
| body | text NOT NULL |  |
| created_by | uuid NOT NULL |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| title | text NOT NULL |  |

### `platform.announcement_target`

| Column | Type | Note |
|---|---|---|
| announcement_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

### `platform.employee_limit`

| Column | Type | Note |
|---|---|---|
| on_reach | text NOT NULL |  |
| set_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| max_active | integer |  |
| valid | daterange NOT NULL |  |
| set_by | uuid NOT NULL |  |

Constraints:
- `employee_limit_on_reach_check`: `CHECK ((on_reach = ANY (ARRAY['block'::text, 'allow_and_alert'::text])))`
- `employee_limit_max_active_check`: `CHECK (((max_active IS NULL) OR (max_active > 0)))`
- `employee_limit_valid_check`: `CHECK ((NOT isempty(valid)))`
- `employee_limit_org_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, valid WITH &&)`

### `platform.module_entitlement`

Modules switched by the vendor only (MD-01), dated history (VC-R2). Common modules C01-C16 are always on.

| Column | Type | Note |
|---|---|---|
| valid | daterange NOT NULL |  |
| module_code | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| reason | text |  |
| set_at | timestamp with time zone NOT NULL |  |
| set_by | uuid NOT NULL |  |
| enabled | boolean NOT NULL |  |

Constraints:
- `module_entitlement_module_code_check`: `CHECK ((module_code ~ '^(V01|C[0-9]{2}|M[0-9]{2})$'::text))`
- `module_entitlement_valid_check`: `CHECK ((NOT isempty(valid)))`
- `module_entitlement_org_id_module_code_valid_excl`: `EXCLUDE USING gist (org_id WITH =, module_code WITH =, valid WITH &&)`

### `platform.organisation`

Tenant root. Client roles see only their own row (RLS on id).

| Column | Type | Note |
|---|---|---|
| updated_at | timestamp with time zone NOT NULL |  |
| state | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| state_reason | text |  |
| sector_template_id | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| display_name | text NOT NULL |  |
| legal_name | text NOT NULL |  |
| state_changed_at | timestamp with time zone NOT NULL |  |
| support_access_needs_approval | boolean NOT NULL |  |

Constraints:
- `organisation_state_check`: `CHECK ((state = ANY (ARRAY['active'::text, 'suspended'::text])))`

### `platform.organisation_plan`

Dated plan history (VC-R2).

| Column | Type | Note |
|---|---|---|
| set_at | timestamp with time zone NOT NULL |  |
| reason | text |  |
| set_by | uuid NOT NULL |  |
| valid | daterange NOT NULL |  |
| org_id | uuid NOT NULL |  |
| plan_id | uuid NOT NULL |  |

Constraints:
- `organisation_plan_valid_check`: `CHECK ((NOT isempty(valid)))`
- `organisation_plan_org_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, valid WITH &&)`

### `platform.organisation_usage`

Summary written by job J10 so the vendor console never reads client tables (VC-R3, VC-R8).

| Column | Type | Note |
|---|---|---|
| branches | integer NOT NULL |  |
| devices_total | integer NOT NULL |  |
| periods_awaiting_lock | integer NOT NULL |  |
| refreshed_at | timestamp with time zone NOT NULL |  |
| last_sign_in_at | timestamp with time zone |  |
| active_employees | integer NOT NULL |  |
| devices_online | integer NOT NULL |  |
| org_id | uuid NOT NULL |  |

### `platform.plan`

| Column | Type | Note |
|---|---|---|
| name | text NOT NULL |  |
| employee_limit | integer |  |
| mobile_app | boolean NOT NULL |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| code | text NOT NULL |  |
| modules | text[] NOT NULL |  |

Constraints:
- `plan_code_key`: `UNIQUE (code)`
- `plan_employee_limit_check`: `CHECK (((employee_limit IS NULL) OR (employee_limit > 0)))`

### `platform.sector_template`

Copied into an organisation at creation (VC-R1). Effect of later edits on existing organisations: OQ-V01-10.

| Column | Type | Note |
|---|---|---|
| name | text NOT NULL |  |
| code | text NOT NULL |  |
| id | uuid NOT NULL |  |
| version | integer NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| content | jsonb NOT NULL |  |
| created_by | uuid |  |
| updated_by | uuid |  |

Constraints:
- `sector_template_code_key`: `UNIQUE (code)`
- `sector_template_version_check`: `CHECK ((version > 0))`

### `platform.subscription`

| Column | Type | Note |
|---|---|---|
| start_date | date NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| renewal_reminder_days | integer NOT NULL |  |
| updated_by | uuid |  |
| auto_suspend_after_days | integer |  |
| billing_status | text NOT NULL |  |
| renewal_date | date NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `subscription_auto_suspend_after_days_check`: `CHECK (((auto_suspend_after_days IS NULL) OR (auto_suspend_after_days >= 0)))`
- `subscription_renewal_reminder_days_check`: `CHECK ((renewal_reminder_days >= 0))`
- `subscription_check`: `CHECK ((renewal_date >= start_date))`

### `platform.support_access_grant`

Time-limited vendor access (VC-R6, VC-03). Write rights under a grant: OQ-V01-12.

| Column | Type | Note |
|---|---|---|
| reason | text NOT NULL |  |
| id | uuid NOT NULL |  |
| requested_at | timestamp with time zone NOT NULL |  |
| state | text NOT NULL |  |
| decided_by | uuid |  |
| decided_at | timestamp with time zone |  |
| window_start | timestamp with time zone |  |
| ended_at | timestamp with time zone |  |
| org_id | uuid NOT NULL |  |
| vendor_user_id | uuid NOT NULL |  |
| window_end | timestamp with time zone |  |

Constraints:
- `support_access_grant_check`: `CHECK (((window_end IS NULL) OR (window_end > window_start)))`
- `support_access_grant_check1`: `CHECK (((state <> 'active'::text) OR ((window_start IS NOT NULL) AND (window_end IS NOT NULL))))`
- `support_access_grant_state_check`: `CHECK ((state = ANY (ARRAY['requested'::text, 'active'::text, 'rejected'::text, 'expired'::text, 'ended'::text])))`

### `platform.vendor_user`

| Column | Type | Note |
|---|---|---|
| email | text NOT NULL |  |
| full_name | text NOT NULL |  |
| vendor_role | text NOT NULL |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| active | boolean NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |

Constraints:
- `vendor_user_email_key`: `UNIQUE (email)`
- `vendor_user_vendor_role_check`: `CHECK ((vendor_role = ANY (ARRAY['full_admin'::text, 'support'::text, 'read_only'::text])))`


## Schema `policy`

### `policy.exemption`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| rule_group | policy.rule_group_code NOT NULL |  |
| scope_kind | text NOT NULL |  |
| list_entry_id | uuid |  |
| id | uuid NOT NULL |  |
| list_kind | text |  |
| employee_id | uuid |  |
| valid | daterange NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `exemption_scope_kind_check`: `CHECK ((scope_kind = ANY (ARRAY['category'::text, 'department'::text, 'employee_type'::text, 'employee'::text, 'date'::text])))`
- `exemption_check`: `CHECK ((((scope_kind = ANY (ARRAY['category'::text, 'department'::text, 'employee_type'::text])) AND (list_entry_id IS NOT NULL) AND (employee_id IS NULL)) OR ((scope_kind = 'employee'::text) AND (employee_id IS NOT NULL) AND (list_entry_id IS NULL)) OR ((scope_kind = 'date'::text) AND (employee_id IS NULL) AND (list_entry_id IS NULL))))`
- `exemption_valid_check`: `CHECK ((NOT isempty(valid)))`

### `policy.policy_assignment`

| Column | Type | Note |
|---|---|---|
| target_key | uuid |  |
| org_id | uuid NOT NULL |  |
| policy_set_id | uuid NOT NULL |  |
| level | text NOT NULL |  |
| branch_id | uuid |  |
| list_entry_id | uuid |  |
| shift_id | uuid |  |
| employee_id | uuid |  |
| valid | daterange NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| list_kind | text |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `policy_assignment_level_check`: `CHECK ((level = ANY (ARRAY['organisation'::text, 'branch'::text, 'department'::text, 'category'::text, 'shift'::text, 'employee'::text])))`
- `policy_assignment_org_id_level_target_key_valid_excl`: `EXCLUDE USING gist (org_id WITH =, level WITH =, target_key WITH =, valid WITH &&)`
- `policy_assignment_check`: `CHECK ((((level = 'organisation'::text) AND (branch_id IS NULL) AND (list_entry_id IS NULL) AND (shift_id IS NULL) AND (employee_id IS NULL)) OR ((level = 'branch'::text) AND (branch_id IS NOT NULL) AND (list_entry_id IS NULL) AND (shift_id IS NULL) AND (employee_id IS NULL)) OR ((level = ANY (ARRAY['department'::text, 'category'::text])) AND (list_entry_id IS NOT NULL) AND (branch_id IS NULL) AND (shift_id IS NULL) AND (employee_id IS NULL)) OR ((level = 'shift'::text) AND (shift_id IS NOT NULL) AND (branch_id IS NULL) AND (list_entry_id IS NULL) AND (employee_id IS NULL)) OR ((level = 'employee'::text) AND (employee_id IS NOT NULL) AND (branch_id IS NULL) AND (list_entry_id IS NULL) AND (shift_id IS NULL))))`
- `policy_assignment_valid_check`: `CHECK ((NOT isempty(valid)))`

### `policy.policy_rule_group`

| Column | Type | Note |
|---|---|---|
| settings | jsonb NOT NULL |  |
| enabled | boolean NOT NULL |  |
| updated_by | uuid |  |
| schema_version | integer NOT NULL |  |
| org_id | uuid NOT NULL |  |
| policy_version_id | uuid NOT NULL |  |
| rule_group | policy.rule_group_code NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |

Constraints:
- `policy_rule_group_schema_version_check`: `CHECK ((schema_version > 0))`

### `policy.policy_set`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| retired_at | timestamp with time zone |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| name | text NOT NULL |  |

Constraints:
- `policy_set_org_id_name_key`: `UNIQUE (org_id, name)`

### `policy.policy_version`

scheduled -> in_force -> superseded are date-derived (SM-POLICYVER V7/V8). Edit of a scheduled version and "in use": GAP-SM-16.

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| updated_at | timestamp with time zone NOT NULL |  |
| effective_from | date NOT NULL |  |
| policy_set_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| reason | text |  |
| approved_at | timestamp with time zone |  |
| state | text NOT NULL |  |
| id | uuid NOT NULL |  |
| approved_by | uuid |  |
| submitted_by | uuid |  |

Constraints:
- `policy_version_state_check`: `CHECK ((state = ANY (ARRAY['draft'::text, 'pending_approval'::text, 'scheduled'::text, 'in_force'::text, 'superseded'::text])))`


## Schema `sched`

### `sched.employee_schedule`

| Column | Type | Note |
|---|---|---|
| weekly_off_pattern_id | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| valid | daterange NOT NULL |  |
| reason | text |  |
| shift_id | uuid |  |
| source | text NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `employee_schedule_valid_check`: `CHECK ((NOT isempty(valid)))`
- `employee_schedule_check`: `CHECK (((source = 'fixed_shift'::text) = (shift_id IS NOT NULL)))`
- `employee_schedule_org_id_employee_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, employee_id WITH =, valid WITH &&)`
- `employee_schedule_source_check`: `CHECK ((source = ANY (ARRAY['fixed_shift'::text, 'roster'::text])))`

### `sched.holiday`

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| kind | text NOT NULL |  |
| name | text NOT NULL |  |
| holiday_date | date NOT NULL |  |
| calendar_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

Constraints:
- `holiday_kind_check`: `CHECK ((kind = ANY (ARRAY['national'::text, 'festival'::text, 'optional'::text])))`
- `holiday_org_id_calendar_id_holiday_date_key`: `UNIQUE (org_id, calendar_id, holiday_date)`

### `sched.holiday_calendar`

One per location (SH-10, HD-R1); a branch points at its calendar (org.branch.holiday_calendar_id).

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| retired_at | timestamp with time zone |  |
| org_id | uuid NOT NULL |  |
| name | text NOT NULL |  |

Constraints:
- `holiday_calendar_org_id_name_key`: `UNIQUE (org_id, name)`

### `sched.optional_holiday_choice`

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |
| holiday_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |

### `sched.roster_entry`

| Column | Type | Note |
|---|---|---|
| shift_version_id | uuid |  |
| source | text NOT NULL |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| is_weekly_off | boolean NOT NULL |  |
| created_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| roster_date | date NOT NULL |  |
| org_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |

Constraints:
- `roster_entry_org_id_employee_id_roster_date_key`: `UNIQUE (org_id, employee_id, roster_date)`
- `roster_entry_check`: `CHECK ((is_weekly_off = (shift_version_id IS NULL)))`
- `roster_entry_source_check`: `CHECK ((source = ANY (ARRAY['screen'::text, 'upload'::text, 'copy'::text, 'rotation'::text])))`

### `sched.shift`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| retired_at | timestamp with time zone |  |
| name | text NOT NULL |  |
| code | text NOT NULL |  |

Constraints:
- `shift_org_id_code_key`: `UNIQUE (org_id, code)`

### `sched.shift_change_request`

| Column | Type | Note |
|---|---|---|
| escalated | boolean NOT NULL |  |
| state | text NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| decided_at | timestamp with time zone |  |
| current_approver_id | uuid |  |
| reason | text |  |
| swap_with_employee_id | uuid |  |
| requested_shift_version_id | uuid |  |
| roster_date | date NOT NULL |  |
| kind | text NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| decided_basis | text |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| submitted_at | timestamp with time zone NOT NULL |  |

Constraints:
- `shift_change_request_kind_check`: `CHECK ((kind = ANY (ARRAY['change'::text, 'swap'::text])))`
- `shift_change_request_check1`: `CHECK (((swap_with_employee_id IS NULL) OR (swap_with_employee_id <> employee_id)))`
- `shift_change_request_check`: `CHECK (((kind = 'swap'::text) = (swap_with_employee_id IS NOT NULL)))`
- `shift_change_request_decided_basis_check`: `CHECK ((decided_basis = ANY (ARRAY['approver'::text, 'auto_on_submit'::text, 'auto_on_timeout'::text])))`
- `shift_change_request_state_check`: `CHECK ((state = ANY (ARRAY['pending_l1'::text, 'pending_l2'::text, 'approved'::text, 'rejected'::text, 'cancelled'::text])))`

### `sched.shift_version`

| Column | Type | Note |
|---|---|---|
| start_time | time without time zone |  |
| window_after_min | integer NOT NULL |  |
| is_night_shift | boolean NOT NULL |  |
| night_attendance_date | text NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| half_day_split | text NOT NULL |  |
| flex_latest_out | time without time zone |  |
| half_day_split_time | time without time zone |  |
| segments | jsonb |  |
| end_time | time without time zone |  |
| org_id | uuid NOT NULL |  |
| shift_id | uuid NOT NULL |  |
| effective_from | date NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| updated_by | uuid |  |
| id | uuid NOT NULL |  |
| state | text NOT NULL |  |
| shift_type | text NOT NULL |  |
| crosses_midnight | boolean |  |
| required_minutes | integer |  |
| required_per | text NOT NULL |  |
| window_before_min | integer NOT NULL |  |
| core_start | time without time zone |  |
| core_end | time without time zone |  |
| flex_earliest_in | time without time zone |  |

Constraints:
- `shift_version_half_day_split_check`: `CHECK ((half_day_split = ANY (ARRAY['middle_of_shift'::text, 'fixed_time'::text])))`
- `shift_version_check6`: `CHECK (((start_time IS NULL) = (end_time IS NULL)))`
- `shift_version_check5`: `CHECK (((segments IS NULL) OR (shift_type = 'split'::text)))`
- `shift_version_check4`: `CHECK (((shift_type <> 'night'::text) OR is_night_shift))`
- `shift_version_check3`: `CHECK (((shift_type <> 'flexible_core'::text) OR ((core_start IS NOT NULL) AND (core_end IS NOT NULL))))`
- `shift_version_check2`: `CHECK (((shift_type <> ALL (ARRAY['fixed'::text, 'night'::text])) OR ((start_time IS NOT NULL) AND (end_time IS NOT NULL))))`
- `shift_version_check1`: `CHECK (((shift_type <> 'open'::text) OR ((start_time IS NULL) AND (end_time IS NULL) AND (required_minutes IS NOT NULL))))`
- `shift_version_check`: `CHECK (((half_day_split <> 'fixed_time'::text) OR (half_day_split_time IS NOT NULL)))`
- `shift_version_night_attendance_date_check`: `CHECK ((night_attendance_date = ANY (ARRAY['shift_start_date'::text, 'shift_end_date'::text])))`
- `shift_version_window_after_min_check`: `CHECK ((window_after_min >= 0))`
- `shift_version_window_before_min_check`: `CHECK ((window_before_min >= 0))`
- `shift_version_required_per_check`: `CHECK ((required_per = ANY (ARRAY['day'::text, 'week'::text])))`
- `shift_version_required_minutes_check`: `CHECK ((required_minutes > 0))`
- `shift_version_shift_type_check`: `CHECK ((shift_type = ANY (ARRAY['fixed'::text, 'flexible'::text, 'flexible_core'::text, 'split'::text, 'night'::text, 'open'::text])))`
- `shift_version_state_check`: `CHECK ((state = ANY (ARRAY['draft'::text, 'pending_approval'::text, 'scheduled'::text, 'in_force'::text, 'superseded'::text])))`

### `sched.weekly_off_pattern`

| Column | Type | Note |
|---|---|---|
| updated_by | uuid |  |
| created_by | uuid |  |
| retired_at | timestamp with time zone |  |
| name | text NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| alt_weeks | smallint[] |  |
| alt_day | smallint |  |
| weekly_days | smallint[] |  |
| kind | text NOT NULL |  |

Constraints:
- `weekly_off_pattern_check1`: `CHECK ((((kind = 'fixed_days'::text) AND (weekly_days IS NOT NULL) AND (alt_day IS NULL)) OR ((kind = 'alternating'::text) AND (alt_day IS NOT NULL)) OR ((kind = 'from_roster'::text) AND (weekly_days IS NULL) AND (alt_day IS NULL))))`
- `weekly_off_pattern_org_id_name_key`: `UNIQUE (org_id, name)`
- `weekly_off_pattern_check`: `CHECK (((alt_day IS NULL) = (alt_weeks IS NULL)))`
- `weekly_off_pattern_alt_weeks_check`: `CHECK ((alt_weeks <@ '{1,2,3,4,5}'::smallint[]))`
- `weekly_off_pattern_alt_day_check`: `CHECK (((alt_day >= 1) AND (alt_day <= 7)))`
- `weekly_off_pattern_weekly_days_check`: `CHECK ((weekly_days <@ '{1,2,3,4,5,6,7}'::smallint[]))`
- `weekly_off_pattern_kind_check`: `CHECK ((kind = ANY (ARRAY['fixed_days'::text, 'alternating'::text, 'from_roster'::text])))`


## Schema `timecap`

### `timecap.device`

| Column | Type | Note |
|---|---|---|
| name | text |  |
| retired_at | timestamp with time zone |  |
| offline_alert_sent_at | timestamp with time zone |  |
| online_state | text NOT NULL |  |
| last_contact_at | timestamp with time zone |  |
| updated_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |
| org_id | uuid NOT NULL |  |
| created_by | uuid |  |
| serial | text NOT NULL |  |
| make | text NOT NULL |  |
| adapter | text NOT NULL |  |
| branch_id | uuid NOT NULL |  |

Constraints:
- `device_online_state_check`: `CHECK ((online_state = ANY (ARRAY['online'::text, 'offline'::text])))`
- `device_serial_key`: `UNIQUE (serial)`

### `timecap.enrolment_mapping`

A change ends the current range and inserts the next one (PC-R3). Earlier unmapped punches with the ID: OQ-C04-04.

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| device_id | uuid NOT NULL |  |
| enrolment_id | text NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| valid | daterange NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| created_by | uuid |  |

Constraints:
- `enrolment_mapping_valid_check`: `CHECK ((NOT isempty(valid)))`
- `enrolment_mapping_org_id_device_id_enrolment_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, device_id WITH =, enrolment_id WITH =, valid WITH &&)`

### `timecap.enrolment_status_history`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| from_status | text |  |
| to_status | text NOT NULL |  |
| created_by | uuid |  |
| reason | text |  |
| id | uuid NOT NULL |  |
| effective_at | timestamp with time zone NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| enrolment_mapping_id | uuid |  |
| employee_id | uuid NOT NULL |  |

Constraints:
- `enrolment_status_history_to_status_check`: `CHECK ((to_status = ANY (ARRAY['not_enrolled'::text, 'enrolled'::text, 're_enrolment_needed'::text])))`
- `enrolment_status_history_from_status_check`: `CHECK ((from_status = ANY (ARRAY['not_enrolled'::text, 'enrolled'::text, 're_enrolment_needed'::text])))`

### `timecap.punch_attempt_refused`

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| employee_id | uuid |  |
| source | text NOT NULL |  |
| device_id | uuid |  |
| rule_ref | text NOT NULL |  |
| message | text NOT NULL |  |
| latitude | numeric(9,6) |  |
| longitude | numeric(9,6) |  |
| accuracy_m | integer |  |
| client_ip | inet |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| attempted_at | timestamp with time zone NOT NULL |  |

Constraints:
- `punch_attempt_refused_longitude_check`: `CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric)))`
- `punch_attempt_refused_source_check`: `CHECK ((source = ANY (ARRAY['device'::text, 'app'::text, 'web'::text, 'kiosk'::text, 'import'::text, 'manual'::text])))`
- `punch_attempt_refused_accuracy_m_check`: `CHECK ((accuracy_m >= 0))`
- `punch_attempt_refused_latitude_check`: `CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric)))`
- `punch_attempt_refused_rule_ref_check`: `CHECK ((rule_ref ~ '^[A-Z]{2}-[0-9]{2}$'::text))`

### `timecap.punch_flag`

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| created_by | uuid |  |
| created_at | timestamp with time zone NOT NULL |  |
| evidence | jsonb NOT NULL |  |
| state | text NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| counts_before_decision | boolean NOT NULL |  |
| reason | text NOT NULL |  |
| punch_id | uuid NOT NULL |  |
| id | uuid NOT NULL |  |
| updated_by | uuid |  |

Constraints:
- `punch_flag_org_id_punch_id_reason_key`: `UNIQUE (org_id, punch_id, reason)`
- `punch_flag_reason_check`: `CHECK ((reason = ANY (ARRAY['outside_fence'::text, 'branch_not_allowed'::text, 'no_shift_or_off_day'::text, 'late_sync'::text, 'clock_mismatch'::text, 'location_unavailable'::text, 'low_accuracy'::text, 'ip_check_failed'::text])))`
- `punch_flag_state_check`: `CHECK ((state = ANY (ARRAY['raised'::text, 'accepted'::text, 'rejected'::text])))`

### `timecap.punch_flag_decision`

| Column | Type | Note |
|---|---|---|
| created_at | timestamp with time zone NOT NULL |  |
| org_id | uuid NOT NULL |  |
| flag_id | uuid NOT NULL |  |
| decision | text NOT NULL |  |
| decided_by | uuid NOT NULL |  |
| reason | text |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| decided_at | timestamp with time zone NOT NULL |  |

Constraints:
- `punch_flag_decision_check`: `CHECK (((decision <> 'rejected'::text) OR (reason IS NOT NULL)))`
- `punch_flag_decision_decision_check`: `CHECK ((decision = ANY (ARRAY['accepted'::text, 'rejected'::text])))`

### `timecap.raw_punch`

Insert-only evidence (BR-01). Rejected punches stay as evidence; the DayResult excludes them. Direction is assigned only in a DayResult (BRX-01).

| Column | Type | Note |
|---|---|---|
| fence_distance_m | integer |  |
| created_at | timestamp with time zone NOT NULL |  |
| received_at | timestamp with time zone NOT NULL |  |
| id | uuid NOT NULL |  |
| created_by | uuid |  |
| manual_approval_ref | uuid |  |
| manual_entered_by | uuid |  |
| manual_reason | text |  |
| direction_tag | text |  |
| device_uptime_ms | bigint |  |
| client_ip | inet |  |
| selfie_file_path | text |  |
| org_id | uuid NOT NULL |  |
| employee_id | uuid |  |
| source | text NOT NULL |  |
| device_id | uuid |  |
| enrolment_id | text |  |
| client_ref | text |  |
| event_at | timestamp with time zone NOT NULL |  |
| captured_tz | text NOT NULL |  |
| idempotency_key | text NOT NULL |  |
| raw_payload_ref | text |  |
| branch_id | uuid |  |
| latitude | numeric(9,6) |  |
| longitude | numeric(9,6) |  |
| accuracy_m | integer |  |
| inside_fence | boolean |  |

Constraints:
- `raw_punch_check3`: `CHECK (((latitude IS NULL) = (longitude IS NULL)))`
- `raw_punch_device_uptime_ms_check`: `CHECK ((device_uptime_ms >= 0))`
- `raw_punch_org_id_idempotency_key_key`: `UNIQUE (org_id, idempotency_key)`
- `raw_punch_check`: `CHECK (((source <> 'manual'::text) OR ((manual_reason IS NOT NULL) AND (manual_entered_by IS NOT NULL))))`
- `raw_punch_check1`: `CHECK (((source <> 'device'::text) OR (device_id IS NOT NULL)))`
- `raw_punch_source_check`: `CHECK ((source = ANY (ARRAY['device'::text, 'app'::text, 'web'::text, 'kiosk'::text, 'import'::text, 'manual'::text])))`
- `raw_punch_check2`: `CHECK (((employee_id IS NOT NULL) OR (source = 'device'::text)))`
- `raw_punch_captured_tz_check`: `CHECK (app.is_valid_timezone(captured_tz))`
- `raw_punch_latitude_check`: `CHECK (((latitude >= ('-90'::integer)::numeric) AND (latitude <= (90)::numeric)))`
- `raw_punch_longitude_check`: `CHECK (((longitude >= ('-180'::integer)::numeric) AND (longitude <= (180)::numeric)))`
- `raw_punch_accuracy_m_check`: `CHECK ((accuracy_m >= 0))`
- `raw_punch_direction_tag_check`: `CHECK ((direction_tag = ANY (ARRAY['in'::text, 'out'::text])))`
- `raw_punch_fence_distance_m_check`: `CHECK ((fence_distance_m >= 0))`


## Schema `workforce`

### `workforce.contact_change_request`

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| decided_at | timestamp with time zone |  |
| decided_by | uuid |  |
| requested_by | uuid NOT NULL |  |
| applied_at | timestamp with time zone |  |
| proposed | jsonb NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| previous | jsonb NOT NULL |  |
| decision_reason | text |  |
| requested_at | timestamp with time zone NOT NULL |  |
| state | text NOT NULL |  |

Constraints:
- `contact_change_request_check1`: `CHECK (((state <> 'applied'::text) OR (applied_at IS NOT NULL)))`
- `contact_change_request_check`: `CHECK (((decided_by IS NULL) OR (decided_by <> requested_by)))`
- `contact_change_request_state_check`: `CHECK ((state = ANY (ARRAY['pending'::text, 'approved'::text, 'rejected'::text, 'applied'::text])))`

### `workforce.custom_field_def`

| Column | Type | Note |
|---|---|---|
| data_type | text NOT NULL |  |
| org_id | uuid NOT NULL |  |
| name | text NOT NULL |  |
| choices | text[] |  |
| retired_at | timestamp with time zone |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| mandatory | boolean NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `custom_field_def_data_type_check`: `CHECK ((data_type = ANY (ARRAY['text'::text, 'number'::text, 'date'::text, 'boolean'::text, 'choice'::text])))`
- `custom_field_def_check`: `CHECK (((data_type <> 'choice'::text) OR (choices IS NOT NULL)))`
- `custom_field_def_org_id_name_key`: `UNIQUE (org_id, name)`

### `workforce.custom_field_value`

| Column | Type | Note |
|---|---|---|
| field_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| value | jsonb NOT NULL |  |
| updated_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| org_id | uuid NOT NULL |  |

### `workforce.employee`

Bank and identity numbers are not held in V1 (OQ-C02-05); when added they are field-encrypted (TSD §15).

| Column | Type | Note |
|---|---|---|
| joining_date | date NOT NULL |  |
| mobile | text |  |
| date_of_birth | date |  |
| full_name | text NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| updated_by | uuid |  |
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| rejoin_of_employee_id | uuid |  |
| employment_status | text NOT NULL |  |
| exit_date | date |  |
| probation_end_date | date |  |
| emergency_contact | jsonb |  |
| updated_at | timestamp with time zone NOT NULL |  |
| address | text |  |
| email | text |  |
| org_id | uuid NOT NULL |  |
| employee_code | text NOT NULL |  |

Constraints:
- `employee_check`: `CHECK (((exit_date IS NULL) OR (exit_date >= joining_date)))`
- `employee_check1`: `CHECK (((employment_status <> 'exited'::text) OR (exit_date IS NOT NULL)))`
- `employee_employment_status_check`: `CHECK ((employment_status = ANY (ARRAY['on_probation'::text, 'active'::text, 'on_notice'::text, 'exited'::text])))`

### `workforce.employee_allowed_branch`

Allowed branches (EM-R9, EM-06 = chosen branches). EM-06 = all branches is a setting, not rows.

| Column | Type | Note |
|---|---|---|
| org_id | uuid NOT NULL |  |
| created_by | uuid |  |
| branch_id | uuid NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| valid | daterange NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |

Constraints:
- `employee_allowed_branch_org_id_employee_id_branch_id_valid_excl`: `EXCLUDE USING gist (org_id WITH =, employee_id WITH =, branch_id WITH =, valid WITH &&)`
- `employee_allowed_branch_valid_check`: `CHECK ((NOT isempty(valid)))`

### `workforce.employee_assignment`

Dated history (EM-R6). A change ends the current range (upper bound) and inserts the next one; every change is audited. "Exactly one master branch on every employed date" is checked by the application (EM-R9).

| Column | Type | Note |
|---|---|---|
| id | uuid NOT NULL |  |
| reason | text |  |
| contractor_id | uuid |  |
| manager_employee_id | uuid |  |
| list_entry_id | uuid |  |
| branch_id | uuid |  |
| valid | daterange NOT NULL |  |
| kind | text NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| org_id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| list_kind | text |  |
| created_by | uuid |  |

Constraints:
- `employee_assignment_org_id_employee_id_kind_valid_excl`: `EXCLUDE USING gist (org_id WITH =, employee_id WITH =, kind WITH =, valid WITH &&)`
- `employee_assignment_check`: `CHECK ((((kind = 'master_branch'::text) AND (branch_id IS NOT NULL) AND (list_entry_id IS NULL) AND (manager_employee_id IS NULL) AND (contractor_id IS NULL)) OR ((kind = ANY (ARRAY['department'::text, 'designation'::text, 'category'::text, 'employee_type'::text])) AND (list_entry_id IS NOT NULL) AND (branch_id IS NULL) AND (manager_employee_id IS NULL) AND (contractor_id IS NULL)) OR ((kind = 'reporting_manager'::text) AND (manager_employee_id IS NOT NULL) AND (branch_id IS NULL) AND (list_entry_id IS NULL) AND (contractor_id IS NULL)) OR ((kind = 'contractor'::text) AND (contractor_id IS NOT NULL) AND (branch_id IS NULL) AND (list_entry_id IS NULL) AND (manager_employee_id IS NULL))))`
- `employee_assignment_valid_check`: `CHECK ((NOT isempty(valid)))`
- `employee_assignment_kind_check`: `CHECK ((kind = ANY (ARRAY['master_branch'::text, 'department'::text, 'designation'::text, 'category'::text, 'employee_type'::text, 'reporting_manager'::text, 'contractor'::text])))`
- `employee_assignment_check1`: `CHECK (((manager_employee_id IS NULL) OR (manager_employee_id <> employee_id)))`

### `workforce.employment_status_history`

| Column | Type | Note |
|---|---|---|
| created_by | uuid |  |
| id | uuid NOT NULL |  |
| created_at | timestamp with time zone NOT NULL |  |
| employee_id | uuid NOT NULL |  |
| from_status | text |  |
| to_status | text NOT NULL |  |
| effective_on | date NOT NULL |  |
| org_id | uuid NOT NULL |  |
| reason | text |  |

Constraints:
- `employment_status_history_to_status_check`: `CHECK ((to_status = ANY (ARRAY['on_probation'::text, 'active'::text, 'on_notice'::text, 'exited'::text])))`
