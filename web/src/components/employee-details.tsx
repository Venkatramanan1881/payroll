import { getTranslations } from "next-intl/server";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { Lookups, Option } from "@/server/lookups";
import type { Employee } from "@/types/api";

/** Read-only employee record, shared by the admin detail page and self-service. Dates are shown as the API gives them. */
export async function EmployeeDetails({ e, lk }: { e: Employee; lk: Lookups }) {
  const t = await getTranslations("employees");
  const c = await getTranslations("common");
  const v = (x: string | null | undefined) => x || c("none");
  const name = (list: Option[], id: string | null | undefined) => list.find((o) => o.id === id)?.name ?? c("none");
  const row = (label: string, value: React.ReactNode) => (
    <div className="grid grid-cols-[10rem_1fr] gap-2 py-1">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
  const ec = e.emergency_contact;
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <Card>
        <CardHeader><CardTitle><h2>{t("personal")}</h2></CardTitle></CardHeader>
        <CardContent>
          <dl className="text-sm">
            {row(t("code"), <span className="font-mono">{e.employee_code}</span>)}
            {row(t("name"), e.full_name)}
            {row(t("dateOfBirth"), v(e.date_of_birth))}
            {row(t("mobile"), v(e.mobile))}
            {row(t("email"), v(e.email))}
            {row(t("address"), v(e.address))}
            {row(t("emergency"), ec ? `${ec.name}${ec.relation ? ` (${ec.relation})` : ""}, ${ec.phone}` : c("none"))}
          </dl>
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle><h2>{t("job")}</h2></CardTitle></CardHeader>
        <CardContent>
          <dl className="text-sm">
            {row(t("status"), <Badge variant="secondary">{t(`st.${e.employment_status}`)}</Badge>)}
            {row(t("joiningDate"), e.joining_date)}
            {e.probation_end_date ? row(t("probationEnd"), e.probation_end_date) : null}
            {e.exit_date ? row(t("exitDate"), e.exit_date) : null}
            {row(t("branch"), name(lk.branches, e.current.master_branch_id))}
            {row(t("department"), name(lk.departments, e.current.department_id))}
            {row(t("designation"), name(lk.designations, e.current.designation_id))}
            {row(t("category"), name(lk.categories, e.current.category_id))}
            {row(t("employeeType"), name(lk.employeeTypes, e.current.employee_type_id))}
          </dl>
        </CardContent>
      </Card>
    </div>
  );
}
