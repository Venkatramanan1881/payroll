// Shapes from docs/api/openapi (identity.yaml, workforce.yaml, organisation.yaml, vendor.yaml). Fields this slice reads.
import type { Grant, GrantKind, ModuleState } from "@/lib/menu";

export type Page<T> = { items: T[]; next_cursor: string | null };

export type Me = {
  id: string;
  login_kind: "mobile_otp" | "password";
  email: string | null;
  mobile: string | null;
  employee_id: string | null;
  language: string;
  roles: { id: string; name: string; system_code: string | null }[];
  grants: Grant[];
};
export type Modules = { items: ModuleState[]; mobile_app: boolean };

export type EmploymentStatus = "on_probation" | "active" | "on_notice" | "exited";
export type EmployeeSummary = {
  id: string;
  employee_code: string;
  full_name: string;
  employment_status: EmploymentStatus;
  master_branch_id: string;
  department_id: string | null;
  designation_id: string | null;
};
export type EmergencyContact = { name: string; relation?: string; phone: string };
export type Employee = {
  id: string;
  employee_code: string;
  full_name: string;
  date_of_birth: string | null;
  mobile: string | null;
  email: string | null;
  address: string | null;
  emergency_contact: EmergencyContact | null;
  joining_date: string;
  probation_end_date: string | null;
  exit_date: string | null;
  employment_status: EmploymentStatus;
  current: {
    master_branch_id?: string;
    department_id?: string | null;
    designation_id?: string | null;
    category_id?: string | null;
    employee_type_id?: string | null;
  };
  updated_at: string;
};
export type Branch = { id: string; name: string; code: string; tz: string | null; address?: string | null; time_zones?: BranchTimeZone[]; updated_at?: string };
export type ListEntry = { id: string; kind: string; name: string; code: string | null };
export type VendorOrganisation = { id: string; display_name: string; legal_name: string; state: "active" | "suspended" };

// attendance.yaml / punches.yaml
export type DayStatus = "present" | "half_day" | "quarter_day" | "absent" | "leave" | "half_day_leave" | "holiday" | "weekly_off" | "on_duty";
export type ProcessingState = "provisional" | "exception" | "absence_pending" | "locked";
export type DailyRow = {
  employee_id: string;
  employee_code: string;
  employee_name: string;
  branch_id: string | null;
  department_id: string | null;
  attendance_date: string;
  processing_state: ProcessingState;
  day_status: DayStatus | null;
  first_in_at: string | null;
  last_out_at: string | null;
  net_minutes: number;
  late_minutes: number;
  early_minutes: number;
  flags: string[];
};
export type Session = { in_at: string; out_at: string | null; minutes?: number };
export type PunchUsed = { raw_punch_id: string; assigned_direction: string | null; session_no: number | null; excluded: boolean; excluded_reason: string | null };
export type DayEffect = { effect_kind: string; days: string | null; minutes: number | null; reason_code: string | null; rule_ref: string | null };
export type ExplanationStep = { step: number; rule_ref: string; outcome: string; inputs?: Record<string, unknown> };
export type DayResult = {
  id: string;
  is_current: boolean;
  day_status?: DayStatus | null;
  day_fraction_present?: string | null;
  // Optional fields may be omitted for employees (AC-05), never nulled.
  gross_minutes?: number | null;
  break_minutes?: number | null;
  net_minutes?: number | null;
  late_minutes?: number | null;
  early_minutes?: number | null;
  overtime_identified_minutes?: number | null;
  late_mark?: boolean;
  early_mark?: boolean;
  flags?: string[] | null;
  sessions?: Session[] | null;
  /** Omitted for employees when AC-05 hides them. */
  punches_used?: PunchUsed[];
  day_effects?: DayEffect[];
  explanation?: ExplanationStep[] | null;
  schedule_tz: string | null;
  engine_version: string;
  trigger_kind: string;
  produced_at: string;
};
export type AttendanceDay = { employee_id: string; attendance_date: string; processing_state: ProcessingState; current_result: DayResult | null; history: DayResult[] };
export type Punch = {
  id: string;
  employee_id: string | null;
  source: string;
  event_at: string;
  captured_tz: string;
  is_manual: boolean;
  manual_reason: string | null;
  direction_tag: string | null;
};
export type RequestState = "pending_l1" | "pending_l2" | "approved" | "rejected" | "cancelled";
export type RegType = "missed_punch" | "late_arrival" | "early_checkout" | "wrong_punch" | "whole_day";
export type ApprovalStep = { level: number; action: string; comment: string | null; acted_at: string };
export type Regularisation = {
  id: string;
  employee_id: string;
  employee_code: string;
  employee_name: string;
  schedule_tz: string | null;
  attendance_date: string;
  request_type: RegType;
  origin: "employee" | "hr_direct";
  state: RequestState;
  escalated: boolean;
  proposed_in_at: string | null;
  proposed_out_at: string | null;
  reason: string;
  requested_by_user_id: string;
  submitted_at: string;
  decided_at: string | null;
  approval_steps: ApprovalStep[];
};
export type PunchFlag = {
  id: string;
  employee_id: string;
  employee_code: string;
  employee_name: string;
  attendance_date: string | null;
  reason: string;
  state: "raised" | "accepted" | "rejected";
  evidence: Record<string, unknown>;
  created_at: string;
  decision: { decision: string; reason: string | null; decided_at: string } | null;
  punch: Punch;
};

// leave.yaml
export type LeavePart = "full" | "first_half" | "second_half";
export type LeaveType = { id: string; code: string; name: string; retired_at: string | null; current_version: { half_day_allowed?: boolean } | null };
export type LeaveBalance = { leave_type_id: string; leave_type_name: string; balance: string; pending_days: string };
export type LeaveRequest = {
  id: string;
  employee_id: string;
  employee_code: string;
  employee_name: string;
  leave_type_id: string;
  leave_type_name: string;
  total_days: string;
  /** GET by id only: what the API would let this caller do now. */
  can_decide?: boolean;
  can_cancel?: boolean;
  from_date: string;
  to_date: string;
  from_part: LeavePart;
  to_part: LeavePart;
  reason: string | null;
  state: RequestState;
  escalated: boolean;
  submitted_at: string;
  cancelled_at: string | null;
  days: { leave_date: string; fraction: string; half: string | null }[];
  steps: ApprovalStep[];
};

export type CalendarDay = { attendance_date: string; processing_state?: ProcessingState; day_status?: DayStatus; late_mark?: boolean; early_mark?: boolean; has_exception?: boolean };
export type InboxItem = {
  id: string;
  event_code: string;
  title: string;
  body: string;
  link: { kind: string; id: string } | null;
  created_at: string;
  read_at: string | null;
  recipient_kind: "employee" | "approver";
};

// scheduling.yaml
export type VersionState = "draft" | "pending_approval" | "scheduled" | "in_force" | "superseded";
export type ShiftVersion = {
  id: string;
  shift_id: string;
  effective_from: string;
  state: VersionState;
  shift_type: string;
  start_time: string | null;
  end_time: string | null;
  updated_at: string;
  [field: string]: unknown;
};
export type Shift = { id: string; code: string; name: string; retired_at: string | null; current_version: ShiftVersion | null; versions: ShiftVersion[] };
export type WeeklyOffPattern = { id: string; name: string; kind: "fixed_days" | "alternating" | "from_roster"; weekly_days: number[] | null; alt_day: number | null; alt_weeks: number[] | null; retired_at: string | null };
export type EmployeeSchedule = { id: string; employee_id: string; source: "fixed_shift" | "roster"; shift_id: string | null; weekly_off_pattern_id: string | null; valid_from: string; valid_to: string | null; reason: string | null };
export type RosterEntry = { id: string; employee_id: string; roster_date: string; is_weekly_off: boolean; shift_id: string | null; shift_code: string | null };
export type HolidayCalendar = { id: string; name: string; retired_at: string | null; branch_ids: string[]; updated_at: string };
export type Holiday = { id: string; calendar_id: string; holiday_date: string; name: string; kind: "national" | "festival" | "optional"; updated_at: string };
export type ScheduleDay = { date: string; source: string; is_weekly_off: boolean; holiday_name: string | null; shift_code?: string; shift_name?: string; start_time?: string | null; end_time?: string | null };

// permissions.yaml / comp-off.yaml
type Decidable = { state: string; escalated: boolean; steps: ApprovalStep[]; can_decide?: boolean; can_cancel?: boolean; employee_id: string; employee_code: string; employee_name: string };
export type PermissionRequest = Decidable & {
  id: string;
  permission_type_id: string;
  permission_type_name: string;
  permission_date: string;
  start_time: string;
  end_time: string;
  minutes: number;
  reason: string;
  origin: "applied" | "converted_late" | "converted_early";
  submitted_at: string;
};
export type OnDutyRequest = Decidable & {
  id: string;
  duration_kind: "hours" | "half_day" | "full_day" | "several_days";
  from_date: string;
  to_date: string;
  from_time: string | null;
  to_time: string | null;
  purpose: string;
  place: string;
  submitted_at: string;
};
export type CompOffCredit = Decidable & {
  id: string;
  worked_date: string;
  fraction: string;
  earned_basis: "policy" | "employee_choice" | "opening";
  expires_on: string | null;
  remaining: string;
};
export type PermissionType = { id: string; code: string; name: string; retired_at: string | null };
export type PermissionUsage = { permission_type_id: string; limit_basis: string; period_from: string; period_to: string; used_count: number; used_minutes: number; remaining_count: number | null; remaining_minutes: number | null };
export type CompOffBalance = { balance: string; pending_approval_days: string; open_credits: CompOffCredit[] };

// period.yaml / opening-balances.yaml
export type PeriodState = "open" | "past_cut_off" | "awaiting_lock" | "locked" | "unlocked";
export type AttendancePeriod = {
  id: string;
  cycle_id: string;
  period_start: string;
  period_end: string;
  cutoff_date: string | null;
  state: PeriodState;
  locked_at: string | null;
  state_history?: { from_state: string | null; to_state: string; reason: string | null; created_at: string }[];
};
export type AttendanceCycle = { id: string; owner_kind: string; owner_id: string; frequency: string; start_kind: string; custom_start_day: number | null; cutoff_day: number | null; valid_from: string; valid_to: string | null; pending_approval?: boolean };
export type PeriodSettings = { after_cutoff_days: string; lock_mode: string; auto_lock_day: number | null; pending_at_lock: string; unlock: string; adjustment_past_periods: number; pending_approval?: boolean; updated_at?: string };
export type ChecklistItem = { kind: "pending_request" | "flagged_day" | "unresolved_missed_punch"; employee_id: string; employee_code: string; employee_name?: string; date: string; request_type: string | null; request_id: string | null };
export type Checklist = { counts: Record<string, number>; items: ChecklistItem[]; is_snapshot: boolean };
export type UnlockRequest = { id: string; reason: string; state: RequestState; submitted_at: string; approval_steps: ApprovalStep[] };
export type PeriodTotals = { employee_id: string; employee_code?: string; employee_name?: string; basis: "frozen" | "provisional"; payable_days: string; deduction_days: string; late_minutes: number; early_minutes: number; pay_hold: boolean };
export type PeriodAdjustment = { id: string; employee_id: string; employee_code?: string; employee_name?: string; original_date: string; days: string; state: "posted" | "frozen" | "exported" };
export type PayrollExport = { id: string; produced_at: string; checksum: string };
export type ObBatch = { id: string; state: "draft" | "pending_approval" | "confirmed"; entry_count: number; created_at: string; updated_at: string; approval_steps: ApprovalStep[] };
export type ObEntry = {
  id: string;
  employee_id: string;
  employee_code?: string;
  employee_name?: string;
  balance_kind: "leave" | "comp_off" | "permission_used" | "late_counter" | "early_counter";
  leave_type_id: string | null;
  permission_type_id: string | null;
  value: string;
  as_of_date: string;
  change_reason: string | null;
  superseded_by_entry_id: string | null;
  updated_at: string;
};

// Organisation structure and settings (organisation.yaml)
export type BranchTimeZone = { tz: string; valid_from: string; valid_to: string | null };
export type GeoFence = {
  id: string;
  branch_id: string;
  name: string | null;
  latitude: number;
  longitude: number;
  radius_m: number;
  enabled: boolean;
  tested_at: string | null;
  updated_at: string;
};
export type FenceTest = { inside: boolean; distance_m: number; radius_m?: number; tested_at: string };
export type IpAllowList = { branch_id: string; networks: string[] };
/** A list entry (department, designation, category, employee type) or a contractor: a name plus a code or a contact. */
export type NamedEntry = { id: string; name: string; code?: string | null; contact?: string | null; retired_at: string | null; updated_at: string };
export type OrgSettings = Record<string, unknown>;
export type OrgSettingsVersion = {
  id: string;
  effective_from: string;
  schema_version: number;
  settings: OrgSettings;
  reason: string | null;
  created_at: string;
};

// Users and access (identity.yaml)
export type UserScope = { scope_kind: "all" | "branch" | "department"; branch_id?: string | null; department_id?: string | null };
export type UserAccount = {
  id: string;
  login_kind: "mobile_otp" | "password";
  mobile: string | null;
  email: string | null;
  employee_id: string | null;
  active: boolean;
  last_sign_in_at: string | null;
  roles: { id: string; name: string; system_code: string | null }[];
  scopes: UserScope[];
  updated_at: string;
};
export type RoleGrant = { operation_code: string; grant_kind: GrantKind; setting_ref: string | null };
export type Role = { id: string; name: string; system_code: string | null; grants?: RoleGrant[]; updated_at: string };
export type ApproverSource = "reporting_manager" | "named_user" | "department_head" | "hr";
export type ApproverRule = { request_type: string; level: 1 | 2; source: ApproverSource; named_user_id: string | null };
export type Delegation = { id: string; approver_user_id: string; delegate_user_id: string; valid_from: string; valid_to: string };
export type EmployeeVisibility = Record<"own_punches" | "day_status" | "counters" | "permission_balance" | "deduction_reason", boolean>;
export type AccessSettings = {
  delegation_enabled: boolean;
  settings_change_mode: "hr_only" | "hr_with_org_admin_approval";
  custom_roles_enabled: boolean;
  admin_second_step: boolean;
  employee_visibility: EmployeeVisibility;
};
export type SupportGrant = {
  id: string;
  state: "requested" | "active" | "rejected" | "expired" | "ended";
  vendor_user_name?: string;
  reason: string;
  requested_at: string;
  window_start: string | null;
  window_end: string | null;
};
export type SupportAction = { occurred_at: string; vendor_user_name?: string; action: string; target: string | null };
