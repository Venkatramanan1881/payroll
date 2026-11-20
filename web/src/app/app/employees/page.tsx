import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Me } from "@/types/api";
import { EmployeeList } from "./employee-list";

export default async function EmployeesPage() {
  const t = await getTranslations("employees");
  const [me, lk] = await Promise.all([api<Me>("/v1/me"), lookups()]);
  return (
    <section className="grid gap-4">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("title")}</h1>
        {me.ok && can(me.data.grants, "WRK-01") ? (
          <Button asChild>
            <Link href="/app/employees/new">{t("new")}</Link>
          </Button>
        ) : null}
      </div>
      <EmployeeList branches={lk.branches} departments={lk.departments} />
    </section>
  );
}
