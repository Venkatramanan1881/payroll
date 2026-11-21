import { getTranslations } from "next-intl/server";
import { PageProblem } from "@/components/page-problem";
import { api } from "@/server/api";
import type { Me } from "@/types/api";
import { MyCompOff, MyOnDuty, MyPermissions } from "./my-requests";

export default async function MyRequestsPage() {
  const t = await getTranslations("me");
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  const id = me.data.employee_id;
  if (!id) return <p className="text-muted-foreground">{t("noEmployeeHere")}</p>;
  return (
    <>
      <h1 className="text-xl font-semibold">{t("nav.requests")}</h1>
      <MyPermissions employeeId={id} />
      <MyOnDuty employeeId={id} />
      <MyCompOff employeeId={id} />
    </>
  );
}
