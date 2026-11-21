import { getTranslations } from "next-intl/server";
import { EmployeeDetails } from "@/components/employee-details";
import { PageProblem } from "@/components/page-problem";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Employee, Me } from "@/types/api";

// Employee self-service: their own record (WRK-08 Self).
export default async function MePage() {
  const t = await getTranslations("me");
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  const emp = me.data.employee_id ? await api<Employee>(`/v1/employees/${me.data.employee_id}`) : null;
  return (
    <>
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      {!emp ? (
        <p className="text-muted-foreground">{t("noEmployee")}</p>
      ) : emp.ok ? (
        <EmployeeDetails e={emp.data} lk={await lookups()} />
      ) : (
        <PageProblem problem={emp.problem} />
      )}
    </>
  );
}
