import { getTranslations } from "next-intl/server";
import { ApplyForm } from "@/components/leave-apply-form";
import { PageProblem } from "@/components/page-problem";
import { api } from "@/server/api";
import type { LeaveType, Me, Page } from "@/types/api";

/** LVE-02: the employee applies for themselves; the API lists the types they are eligible for (LM-01). */
export default async function ApplyLeavePage() {
  const t = await getTranslations("me");
  const [me, types] = await Promise.all([api<Me>("/v1/me"), api<Page<LeaveType>>("/v1/leave/types")]);
  if (!me.ok) return <PageProblem problem={me.problem} />;
  if (!me.data.employee_id) return <p className="text-muted-foreground">{t("noEmployeeHere")}</p>;
  return (
    <>
      <h1 className="text-xl font-semibold">{t("applyTitle")}</h1>
      <ApplyForm base="/me/leave" self={me.data.employee_id} types={types.ok ? types.data.items.filter((x) => !x.retired_at) : []} />
    </>
  );
}
