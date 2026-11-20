import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { Button } from "@/components/ui/button";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { LeaveType, Me, Page } from "@/types/api";
import { LeaveList } from "./leave-list";

const DECIDE = ["all", "scoped", "approve"] as const;

export default async function LeavePage() {
  const t = await getTranslations("leave");
  const [me, types] = await Promise.all([api<Me>("/v1/me"), api<Page<LeaveType>>("/v1/leave/types")]);
  const grants = me.ok ? me.data.grants : [];
  const decide = ["LVE-04", "LVE-05"].some((op) => can(grants, op, [...DECIDE]));
  return (
    <section className="grid gap-4">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("title")}</h1>
        {can(grants, "LVE-03") ? (
          <Button asChild>
            <Link href="/app/leave/new">{t("apply")}</Link>
          </Button>
        ) : null}
      </div>
      <LeaveList canDecide={decide} canSeeAll={can(grants, "LVE-08")} types={types.ok ? types.data.items : []} />
    </section>
  );
}
