import { getTranslations } from "next-intl/server";
import { holds } from "@/lib/menu";
import { KINDS, type RequestKind } from "@/lib/requests";
import { api } from "@/server/api";
import type { Me, Modules } from "@/types/api";
import { RequestsBoard } from "./requests-board";

const MODULE: Record<RequestKind, string> = { permission: "C09", on_duty: "C09", comp_off: "M02" };

export default async function RequestsPage() {
  const t = await getTranslations("req");
  const [me, mods] = await Promise.all([api<Me>("/v1/me"), api<Modules>("/v1/me/modules")]);
  const grants = me.ok ? me.data.grants : [];
  const on = new Set((mods.ok ? mods.data.items : []).filter((m) => m.enabled).map((m) => m.module_code));
  // A tab for each type the user works with; which rows and actions they get is the API's answer.
  const kinds = (Object.keys(KINDS) as RequestKind[]).filter((k) => on.has(MODULE[k]) && holds(grants, KINDS[k].ops, ["all", "scoped", "approve"]));
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <RequestsBoard kinds={kinds} />
    </section>
  );
}
