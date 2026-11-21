import { getTranslations } from "next-intl/server";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me } from "@/types/api";
import { RosterGrid } from "./roster-grid";

export default async function RosterPage() {
  const t = await getTranslations("sched");
  const me = await api<Me>("/v1/me");
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("roster")}</h1>
      <RosterGrid canEdit={me.ok && can(me.data.grants, "SCH-02")} />
    </section>
  );
}
