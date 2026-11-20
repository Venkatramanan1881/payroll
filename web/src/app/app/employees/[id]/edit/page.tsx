import { getTranslations } from "next-intl/server";
import { PageProblem } from "@/components/page-problem";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Employee } from "@/types/api";
import { EmployeeForm } from "../../employee-form";

export default async function EditEmployeePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations("employees");
  const [emp, lk] = await Promise.all([api<Employee>(`/v1/employees/${encodeURIComponent(id)}`), lookups()]);
  if (!emp.ok) return <PageProblem problem={emp.problem} />;
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("edit")}: {emp.data.full_name}</h1>
      <EmployeeForm mode="edit" employee={emp.data} etag={emp.etag ?? ""} lookups={lk} />
    </section>
  );
}
