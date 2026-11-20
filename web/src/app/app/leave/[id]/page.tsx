import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { wallDateTime } from "@/lib/time";
import { api } from "@/server/api";
import type { LeaveRequest } from "@/types/api";
import { LeaveActions } from "./leave-actions";

export default async function LeaveRequestPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations("leave");
  const r = await getTranslations("regularisation");
  const q = await api<LeaveRequest>(`/v1/leave/requests/${encodeURIComponent(id)}`);
  if (!q.ok) return <PageProblem problem={q.problem} />;
  const x = q.data;
  const row = (label: string, value: React.ReactNode) => (
    <div className="grid grid-cols-[10rem_1fr] gap-2 py-0.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
  return (
    <section className="grid max-w-3xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/leave" className="text-sm underline">{t("title")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t("detail")}: {x.employee_name}</h1>
        <Badge variant="secondary">{r(`rs.${x.state}`)}</Badge>
      </div>
      <Card>
        <CardContent className="pt-6">
          <dl className="text-sm">
            {row(t("employee"), `${x.employee_name} (${x.employee_code})`)}
            {row(t("type"), x.leave_type_name)}
            {row(t("from"), `${x.from_date} · ${t(`part.${x.from_part}`)}`)}
            {row(t("to"), `${x.to_date} · ${t(`part.${x.to_part}`)}`)}
            {row(t("reason"), x.reason ?? "—")}
            {row(t("submitted"), wallDateTime(x.submitted_at))}
            {x.cancelled_at ? row(r("rs.cancelled"), wallDateTime(x.cancelled_at)) : null}
          </dl>
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle><h2>{t("dayList")}</h2></CardTitle></CardHeader>
        <CardContent>
          <ul className="grid gap-1 text-sm">
            {x.days.map((d) => (
              <li key={d.leave_date}>{d.leave_date} · {d.fraction}{d.half ? ` · ${t(`part.${d.half}`)}` : ""}</li>
            ))}
          </ul>
        </CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle><h2>{t("steps")}</h2></CardTitle></CardHeader>
        <CardContent>
          <ol className="grid gap-1 text-sm">
            {x.steps.map((s, i) => (
              <li key={i}>
                {wallDateTime(s.acted_at)} · {r("level", { level: s.level })} · {r(`act.${s.action}`)}
                {s.comment ? <span className="text-muted-foreground"> — {s.comment}</span> : null}
              </li>
            ))}
          </ol>
        </CardContent>
      </Card>
      {/* The API says what this caller may do, by the same checks the commands make. */}
      <LeaveActions id={x.id} decide={x.can_decide === true} cancel={x.can_cancel === true} />
    </section>
  );
}
