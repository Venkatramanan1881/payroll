import { getTranslations } from "next-intl/server";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me } from "@/types/api";
import { RequestList } from "./request-list";

export default async function RegularisationsPage() {
  const t = await getTranslations("regularisation");
  const me = await api<Me>("/v1/me");
  const grants = me.ok ? me.data.grants : [];
  const decide = ["ATT-03", "ATT-04", "ATT-05"].some((op) => can(grants, op, ["all", "scoped", "approve"]));
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <RequestList canDecide={decide} canSeeAll={can(grants, "ATT-01")} />
    </section>
  );
}
