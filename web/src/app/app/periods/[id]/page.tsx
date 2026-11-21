import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { can } from "@/lib/menu";
import { wallDateTime } from "@/lib/time";
import { api } from "@/server/api";
import type { AttendancePeriod, Me } from "@/types/api";
import { PeriodPanels } from "./period-panels";

/** One attendance period: what blocks the lock, the lock itself, unlocks, totals, adjustments and payroll exports. */
export default async function PeriodPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations("per");
  const [p, me] = await Promise.all([api<AttendancePeriod>(`/v1/periods/${encodeURIComponent(id)}`), api<Me>("/v1/me")]);
  if (!p.ok) return <PageProblem problem={p.problem} />;
  const g = me.ok ? me.data.grants : [];
  const x = p.data;
  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/periods" className="text-sm underline">{t("periods")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{x.period_start} – {x.period_end}</h1>
        <Badge variant={x.state === "locked" ? "default" : "secondary"}>{t(`st.${x.state}`)}</Badge>
      </div>
      {x.locked_at ? <p className="text-sm text-muted-foreground">{t("locked", { at: wallDateTime(x.locked_at) })}</p> : null}
      {/* Keyed on the state, so every card reads again after a lock, unlock or decision. */}
      <PeriodPanels
        key={`${x.state}-${x.locked_at ?? ""}`}
        period={x}
        may={{
          read: can(g, "PER-08"),
          checklist: can(g, "PER-02"),
          lock: can(g, "PER-03"), // the API also requires organisation-wide scope
          requestUnlock: can(g, "PER-04"),
          decideUnlock: can(g, "PER-05", ["all", "scoped", "approve"]),
          exportIt: can(g, "PER-06"),
          download: can(g, "PER-07"),
        }}
      />
      {x.state_history && x.state_history.length > 0 ? (
        <details className="rounded-lg border p-4">
          <summary className="cursor-pointer font-medium">{t("history")}</summary>
          <ol className="mt-2 grid gap-1 text-sm">
            {x.state_history.map((h, i) => (
              <li key={i}>
                {wallDateTime(h.created_at)} · {h.from_state ? `${t(`st.${h.from_state}`)} → ` : ""}{t(`st.${h.to_state}`)}
                {h.reason ? <span className="text-muted-foreground"> — {h.reason}</span> : null}
              </li>
            ))}
          </ol>
        </details>
      ) : null}
    </section>
  );
}
