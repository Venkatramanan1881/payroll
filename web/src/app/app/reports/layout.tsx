import { getTranslations } from "next-intl/server";
import { SubNav } from "@/components/sub-nav";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me } from "@/types/api";

/** Reports (C13), the audit trail (ADT-01) and the settings history (ADT-02); links only to what the grants allow. */
export default async function ReportsLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("rpt");
  const me = await api<Me>("/v1/me");
  const g = me.ok ? me.data.grants : [];
  const links = [
    { href: "/app/reports", text: t("sub.reports"), show: ["RPT-01", "RPT-02", "RPT-03", "RPT-04", "RPT-05", "RPT-06"].some((op) => can(g, op)) },
    { href: "/app/reports/audit", text: t("sub.audit"), show: can(g, "ADT-01") },
    { href: "/app/reports/settings", text: t("sub.settings"), show: can(g, "ADT-02") },
  ].filter((l) => l.show);
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <SubNav label={t("title")} links={links} />
      {children}
    </section>
  );
}
