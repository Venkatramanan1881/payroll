import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { EmployeeDetails } from "@/components/employee-details";
import { PageProblem } from "@/components/page-problem";
import { SignOutButton } from "@/components/sign-out-button";
import { landing } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Employee, Me } from "@/types/api";

// Employee self-service: their own record (WRK-08 Self). More self-service screens arrive with their slices.
export default async function MePage() {
  const t = await getTranslations();
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  const emp = me.data.employee_id ? await api<Employee>(`/v1/employees/${me.data.employee_id}`) : null;
  return (
    <div className="min-h-screen">
      <header className="border-b">
        <div className="mx-auto flex max-w-4xl items-center gap-4 px-4 py-3">
          <span className="flex-1 font-semibold">{t("common.appName")}</span>
          {landing(me.data.grants) === "/app" ? <Link href="/app" className="text-sm underline">{t("nav.home")}</Link> : null}
          <SignOutButton to="/sign-in" />
        </div>
      </header>
      <main className="mx-auto grid max-w-4xl gap-4 px-4 py-6">
        <h1 className="text-xl font-semibold">{t("me.title")}</h1>
        {!emp ? (
          <p className="text-muted-foreground">{t("me.noEmployee")}</p>
        ) : emp.ok ? (
          <EmployeeDetails e={emp.data} lk={await lookups()} />
        ) : (
          <PageProblem problem={emp.problem} />
        )}
      </main>
    </div>
  );
}
