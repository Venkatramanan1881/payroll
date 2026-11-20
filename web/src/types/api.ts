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
export type Branch = { id: string; name: string; code: string };
export type ListEntry = { id: string; kind: string; name: string; code: string | null };
export type VendorOrganisation = { id: string; display_name: string; legal_name: string; state: "active" | "suspended" };
