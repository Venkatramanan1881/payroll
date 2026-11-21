import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { LeaveBalances } from "@/components/leave-balances";
import { PageProblem } from "@/components/page-problem";
import { Button } from "@/components/ui/button";
import { api } from "@/server/api";
import type { Me } from "@/types/api";
import { MyLeave } from "./my-leave";

export default async function MyLeavePage() {
  const t = await getTranslations();
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  const id = me.data.employee_id;
  if (!id) return <p className="text-muted-foreground">{t("me.noEmployeeHere")}</p>;
  return (
    <>
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("me.nav.leave")}</h1>
        <Button asChild>
          <Link href="/me/leave/new">{t("me.applyLeave")}</Link>
        </Button>
      </div>
      <h2 className="text-lg font-semibold">{t("leave.balances")}</h2>
      <LeaveBalances employeeId={id} />
      <h2 className="mt-4 text-lg font-semibold">{t("me.myLeave")}</h2>
      <MyLeave employeeId={id} />
    </>
  );
}
