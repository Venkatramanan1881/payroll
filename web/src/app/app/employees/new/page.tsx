import { getTranslations } from "next-intl/server";
import { lookups } from "@/server/lookups";
import { EmployeeForm } from "../employee-form";

export default async function NewEmployeePage() {
  const t = await getTranslations("employees");
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("new")}</h1>
      <EmployeeForm mode="create" lookups={await lookups()} />
    </section>
  );
}
