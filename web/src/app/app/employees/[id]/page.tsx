import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { EmployeeDetails } from "@/components/employee-details";
import { LeaveBalances } from "@/components/leave-balances";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PageProblem } from "@/components/page-problem";
import { Button } from "@/components/ui/button";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Employee, Me } from "@/types/api";

export default async function EmployeePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations();
  const [emp, me, lk] = await Promise.all([api<Employee>(`/v1/employees/${encodeURIComponent(id)}`), api<Me>("/v1/me"), lookups()]);
  if (!emp.ok) return <PageProblem problem={emp.problem} />;
  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/employees" className="text-sm underline">{t("common.back")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{emp.data.full_name}</h1>
        {me.ok && can(me.data.grants, "WRK-02") ? (
          <Button asChild variant="outline">
            <Link href={`/app/employees/${emp.data.id}/edit`}>{t("common.edit")}</Link>
          </Button>
        ) : null}
      </div>
      <EmployeeDetails e={emp.data} lk={lk} />
      {me.ok && can(me.data.grants, "LVE-08") ? (
        <Card>
          <CardHeader><CardTitle><h2>{t("leave.balances")}</h2></CardTitle></CardHeader>
          <CardContent><LeaveBalances employeeId={emp.data.id} /></CardContent>
        </Card>
      ) : null}
    </section>
  );
}
