// Shapes from docs/api/openapi (identity.yaml, workforce.yaml, organisation.yaml, vendor.yaml). Fields this slice reads.
import type { Grant, ModuleState } from "@/lib/menu";

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
export type Branch = { id: string; name: string; code: string; tz: string };
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
