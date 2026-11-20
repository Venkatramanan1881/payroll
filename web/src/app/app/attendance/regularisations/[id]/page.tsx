import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { wallDateTime, wallTime } from "@/lib/time";
import { api } from "@/server/api";
import type { Regularisation } from "@/types/api";
import { DecideHere } from "./decide-here";


export default async function RegularisationPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations("regularisation");
  const a = await getTranslations("attendance");
  const r = await api<Regularisation>(`/v1/attendance/regularisation-requests/${encodeURIComponent(id)}`);
  if (!r.ok) return <PageProblem problem={r.problem} />;
  const q = r.data;
  const TZ = q.schedule_tz ?? "Asia/Kolkata"; // the day's zone; India before any result exists
  const row = (label: string, value: React.ReactNode) => (
    <div className="grid grid-cols-[10rem_1fr] gap-2 py-0.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
  const pending = q.state === "pending_l1" || q.state === "pending_l2";
  return (
    <section className="grid max-w-3xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/attendance/regularisations" className="text-sm underline">{t("title")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t("detail")}: {q.employee_name}, {q.attendance_date}</h1>
        <Badge variant="secondary">{t(`rs.${q.state}`)}</Badge>
      </div>
      <Card>
        <CardContent className="pt-6">
          <dl className="text-sm">
            {row(t("employee"), `${q.employee_name} (${q.employee_code})`)}
            {row(t("date"), <Link className="underline" href={`/app/attendance/${q.employee_id}/${q.attendance_date}`}>{q.attendance_date} · {t("openDay")}</Link>)}
            {row(t("type"), a(`rt.${q.request_type}`))}
            {row(t("origin"), t(`or.${q.origin}`))}
            {row(t("proposedIn"), wallTime(q.proposed_in_at, TZ) || "—")}
            {row(t("proposedOut"), wallTime(q.proposed_out_at, TZ) || "—")}
            {row(t("reason"), q.reason)}
            {q.escalated ? row(t("escalated"), "✓") : null}
          </dl>
          <p className="mt-2 text-xs text-muted-foreground">{a("tzNote", { tz: TZ })}</p>
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle><h2>{t("steps")}</h2></CardTitle></CardHeader>
        <CardContent>
          <ol className="grid gap-1 text-sm">
            {q.approval_steps.map((s, i) => (
              <li key={i}>
                {wallDateTime(s.acted_at)} · {t("level", { level: s.level })} · {t(`act.${s.action}`)}
                {s.comment ? <span className="text-muted-foreground"> — {s.comment}</span> : null}
              </li>
            ))}
          </ol>
        </CardContent>
      </Card>
      {pending ? <DecideHere id={q.id} /> : null}
    </section>
  );
}
