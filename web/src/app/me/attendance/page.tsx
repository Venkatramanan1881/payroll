import { getTranslations } from "next-intl/server";
import { PageProblem } from "@/components/page-problem";
import { api } from "@/server/api";
import type { Me } from "@/types/api";
import { MonthCalendar } from "./month-calendar";
import { MyRegularisations } from "./my-regularisations";

export default async function MyAttendancePage() {
  const t = await getTranslations("me");
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  if (!me.data.employee_id) return <p className="text-muted-foreground">{t("noEmployeeHere")}</p>;
  return (
    <>
      <h1 className="text-xl font-semibold">{t("nav.attendance")}</h1>
      <MonthCalendar employeeId={me.data.employee_id} />
      <h2 className="mt-4 text-lg font-semibold">{t("myRequests")}</h2>
      <MyRegularisations employeeId={me.data.employee_id} userId={me.data.id} />
    </>
  );
}
