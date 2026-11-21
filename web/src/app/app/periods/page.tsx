import { getTranslations } from "next-intl/server";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Me } from "@/types/api";
import { PeriodsView } from "./periods-view";

export default async function PeriodsPage() {
  const t = await getTranslations("per");
  const [me, lk, org] = await Promise.all([api<Me>("/v1/me"), lookups(), api<{ id: string }>("/v1/organisation")]);
  const grants = me.ok ? me.data.grants : [];
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <PeriodsView canSetUp={can(grants, "PER-01", ["all"])} branches={lk.branches} orgId={org.ok ? org.data.id : ""} />
    </section>
  );
}
