import { getTranslations } from "next-intl/server";
import { redirect } from "next/navigation";
import { buildMenu } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me, Modules } from "@/types/api";

export default async function AppHome() {
  const t = await getTranslations("nav");
  const [me, mods] = await Promise.all([api<Me>("/v1/me"), api<Modules>("/v1/me/modules")]);
  const first = me.ok && mods.ok ? buildMenu(me.data.grants, mods.data.items)[0] : undefined;
  if (first) redirect(first.href);
  return <p className="text-muted-foreground">{t("nothing")}</p>;
}
