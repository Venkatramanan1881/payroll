import { getTranslations } from "next-intl/server";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { LeaveType, Me, Page } from "@/types/api";
import { ApplyForm } from "./apply-form";

export default async function ApplyOnBehalfPage() {
  const t = await getTranslations("leave");
  const [types, me] = await Promise.all([api<Page<LeaveType>>("/v1/leave/types"), api<Me>("/v1/me")]);
  if (!me.ok || !can(me.data.grants, "LVE-03")) {
    const p = await getTranslations("problem");
    return <p className="text-muted-foreground">{p("FORBIDDEN")}</p>;
  }
  return (
    <section className="grid max-w-3xl gap-4">
      <h1 className="text-xl font-semibold">{t("applyTitle")}</h1>
      <p className="text-sm text-muted-foreground">{t("applyHelp")}</p>
      <ApplyForm types={types.ok ? types.data.items.filter((x) => !x.retired_at) : []} />
    </section>
  );
}
