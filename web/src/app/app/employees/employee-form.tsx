"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { bff, sendJson } from "@/lib/bff";
import { fieldErrors, type Problem } from "@/lib/problem";
import type { Lookups } from "@/server/lookups";
import type { Employee } from "@/types/api";

type Props = { lookups: Lookups } & ({ mode: "create" } | { mode: "edit"; employee: Employee; etag: string });

// Field names match workforce.yaml so the API's field errors land on the right input.
const PERSONAL = ["employee_code", "full_name", "date_of_birth", "mobile", "email", "address"] as const;

/** Create (WRK-01) or edit (WRK-02) an employee. The form only collects input; every rule is checked by the API. */
export function EmployeeForm(props: Props) {
  const t = useTranslations("employees");
  const c = useTranslations("common");
  const router = useRouter();
  const [problem, setProblem] = useState<Problem | null>(null);
  const [busy, setBusy] = useState(false);
  // One key per form open: a double submit or a retry after a lost answer creates one employee (API_CONVENTIONS).
  const [key] = useState(() => crypto.randomUUID());
  const initial = props.mode === "edit" ? props.employee : null;
  const errs = fieldErrors(problem);

  function personal(f: FormData, edit: boolean): Record<string, unknown> {
    const out: Record<string, unknown> = {};
    for (const k of PERSONAL) {
      const v = String(f.get(k) ?? "").trim();
      if (!edit) {
        if (v) out[k] = v;
      } else if (v !== (initial?.[k] ?? "")) {
        out[k] = v === "" && k !== "full_name" && k !== "employee_code" ? null : v;
      }
    }
    const ec = { name: String(f.get("ec_name") ?? "").trim(), relation: String(f.get("ec_relation") ?? "").trim(), phone: String(f.get("ec_phone") ?? "").trim() };
    const any = ec.name || ec.relation || ec.phone;
    const value = any ? { name: ec.name, phone: ec.phone, ...(ec.relation ? { relation: ec.relation } : {}) } : null;
    if (!edit) {
      if (value) out.emergency_contact = value;
    } else if (JSON.stringify(value) !== JSON.stringify(initial?.emergency_contact ?? null)) {
      out.emergency_contact = value;
    }
    return out;
  }

  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const f = new FormData(e.currentTarget);
    setBusy(true);
    setProblem(null);
    let r;
    if (props.mode === "create") {
      const body = personal(f, false);
      for (const k of ["joining_date", "employment_status", "probation_end_date", "master_branch_id", "department_id", "designation_id", "category_id", "employee_type_id"]) {
        const v = String(f.get(k) ?? "");
        if (v) body[k] = v;
      }
      r = await bff<Employee>("/employees", sendJson("POST", body, { "idempotency-key": key }));
    } else {
      const body = personal(f, true);
      if (Object.keys(body).length === 0) return router.push(`/app/employees/${props.employee.id}`);
      r = await bff<Employee>(`/employees/${props.employee.id}`, sendJson("PATCH", body, { "if-match": props.etag }));
    }
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    router.push(`/app/employees/${r.data.id}`);
    router.refresh();
  }

  const field = (name: string, label: string, input: React.ReactNode, help?: string) => (
    <div className="grid gap-1">
      <Label htmlFor={name}>{label}</Label>
      {input}
      {help ? <p id={`${name}-help`} className="text-xs text-muted-foreground">{help}</p> : null}
      {errs[name] ? <p id={`${name}-err`} className="text-xs text-destructive">{errs[name]}</p> : null}
    </div>
  );
  const aria = (name: string, help = false) => ({
    id: name,
    name,
    "aria-invalid": Boolean(errs[name]) || undefined,
    "aria-describedby": [help ? `${name}-help` : "", errs[name] ? `${name}-err` : ""].filter(Boolean).join(" ") || undefined,
  });
  const pick = (name: string, label: string, list: { id: string; name: string }[], required = false) =>
    field(name, label, (
      <NativeSelect {...aria(name)} required={required} defaultValue="">
        <option value="">{required ? "" : c("none")}</option>
        {list.map((o) => <option key={o.id} value={o.id}>{o.name}</option>)}
      </NativeSelect>
    ));

  const lk = props.lookups;
  return (
    <form onSubmit={submit} className="grid max-w-3xl gap-6" noValidate={false}>
      <fieldset className="grid gap-4 sm:grid-cols-2">
        <legend className="mb-2 font-medium">{t("personal")}</legend>
        {field("full_name", t("name"), <Input {...aria("full_name")} required maxLength={200} defaultValue={initial?.full_name} />)}
        {field("employee_code", t("code"), <Input {...aria("employee_code", true)} maxLength={30} defaultValue={initial?.employee_code} />, props.mode === "create" ? t("codeHelp") : undefined)}
        {field("date_of_birth", t("dateOfBirth"), <Input {...aria("date_of_birth")} type="date" defaultValue={initial?.date_of_birth ?? ""} />)}
        {field("mobile", t("mobile"), <Input {...aria("mobile", true)} type="tel" pattern="\+[1-9][0-9]{6,14}" defaultValue={initial?.mobile ?? ""} />, t("mobileHelp"))}
        {field("email", t("email"), <Input {...aria("email")} type="email" maxLength={254} defaultValue={initial?.email ?? ""} />)}
        <div className="sm:col-span-2">
          {field("address", t("address"), <Textarea {...aria("address")} maxLength={500} defaultValue={initial?.address ?? ""} />)}
        </div>
      </fieldset>

      <fieldset className="grid gap-4 sm:grid-cols-3">
        <legend className="mb-2 font-medium">{t("emergency")}</legend>
        {field("ec_name", t("emergencyName"), <Input {...aria("ec_name")} maxLength={200} defaultValue={initial?.emergency_contact?.name ?? ""} />)}
        {field("ec_relation", t("emergencyRelation"), <Input {...aria("ec_relation")} maxLength={50} defaultValue={initial?.emergency_contact?.relation ?? ""} />)}
        {field("ec_phone", t("emergencyPhone"), <Input {...aria("ec_phone")} type="tel" maxLength={20} defaultValue={initial?.emergency_contact?.phone ?? ""} />)}
      </fieldset>

      {props.mode === "create" ? (
        <fieldset className="grid gap-4 sm:grid-cols-2">
          <legend className="mb-2 font-medium">{t("job")}</legend>
          {field("joining_date", t("joiningDate"), <Input {...aria("joining_date")} type="date" required />)}
          {field("employment_status", t("status"), (
            <NativeSelect {...aria("employment_status")} required defaultValue="active">
              <option value="active">{t("st.active")}</option>
              <option value="on_probation">{t("st.on_probation")}</option>
            </NativeSelect>
          ))}
          {field("probation_end_date", t("probationEnd"), <Input {...aria("probation_end_date")} type="date" />)}
          {pick("master_branch_id", t("branch"), lk.branches, true)}
          {pick("department_id", t("department"), lk.departments)}
          {pick("designation_id", t("designation"), lk.designations)}
          {pick("category_id", t("category"), lk.categories)}
          {pick("employee_type_id", t("employeeType"), lk.employeeTypes)}
        </fieldset>
      ) : (
        <p className="text-sm text-muted-foreground">{t("datedNote")}</p>
      )}

      <ProblemAlert problem={problem} />
      <div className="flex gap-3">
        <Button type="submit" disabled={busy}>{busy ? c("saving") : c("save")}</Button>
        <Button type="button" variant="outline" onClick={() => router.back()}>{c("cancel")}</Button>
        {problem?.status === 412 ? <Button type="button" variant="ghost" onClick={() => window.location.reload()}>{c("retry")}</Button> : null}
      </div>
    </form>
  );
}
