import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { notFound } from "next/navigation";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { can } from "@/lib/menu";
import { hours, wallDateTime, wallTime } from "@/lib/time";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { AttendanceDay, DayResult, Employee, Me, Page, Punch } from "@/types/api";
import { DayActions } from "./day-actions";

const DATE = /^\d{4}-\d{2}-\d{2}$/;
const shift = (d: string, days: number) => {
  const x = new Date(`${d}T00:00:00Z`);
  x.setUTCDate(x.getUTCDate() + days);
  return x.toISOString().slice(0, 10);
};

/** One employee's day as the server worked it out (BR-08 explanation, BRX-03 history). Nothing is computed here. */
export default async function DayPage({ params }: { params: Promise<{ employee: string; date: string }> }) {
  const { employee, date } = await params;
  if (!DATE.test(date)) notFound();
  const t = await getTranslations("attendance");
  const id = encodeURIComponent(employee);
  const [emp, day, me, punches, lk] = await Promise.all([
    api<Employee>(`/v1/employees/${id}`),
    api<AttendanceDay>(`/v1/attendance/employees/${id}/days/${date}`),
    api<Me>("/v1/me"),
    api<Page<Punch>>(`/v1/punches?employee_id=${id}&from=${shift(date, -1)}&to=${shift(date, 1)}`),
    lookups(),
  ]);
  if (!emp.ok) return <PageProblem problem={emp.problem} />;
  if (!day.ok && day.problem.status !== 404) return <PageProblem problem={day.problem} />;
  const d = day.ok ? day.data : null;
  const r = d?.current_result ?? null;
  // The day's own schedule zone; before any result, the employee's branch zone.
  const tz = r?.schedule_tz ?? lk.branches.find((b) => b.id === emp.data.current.master_branch_id)?.tz ?? "Asia/Kolkata";
  const byId = new Map((punches.ok ? punches.data.items : []).map((p) => [p.id, p]));
  const grants = me.ok ? me.data.grants : [];

  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/attendance" className="text-sm underline">{t("title")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t("dayTitle", { name: emp.data.full_name, date })}</h1>
        {d ? <Badge variant="outline">{t(`ps.${d.processing_state}`)}</Badge> : null}
      </div>
      <p className="text-sm text-muted-foreground">{t("tzNote", { tz })}</p>

      {!r ? (
        <p className="text-muted-foreground">{t("noDay")}</p>
      ) : (
        <>
          <ResultCard r={r} tz={tz} title={t("result")} />
          <Card>
            <CardHeader><CardTitle><h2>{t("punches")}</h2></CardTitle></CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("time")}</TableHead>
                    <TableHead>{t("source")}</TableHead>
                    <TableHead>{t("direction")}</TableHead>
                    <TableHead />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {r.punches_used.map((u) => {
                    const p = byId.get(u.raw_punch_id);
                    return (
                      <TableRow key={u.raw_punch_id} className={u.excluded ? "text-muted-foreground" : undefined}>
                        <TableCell>{p ? wallTime(p.event_at, tz) : u.raw_punch_id.slice(0, 8)}</TableCell>
                        <TableCell>{p?.source}</TableCell>
                        <TableCell>{u.excluded ? t("excluded", { reason: u.excluded_reason ?? "" }) : `${u.assigned_direction ?? ""} ${u.session_no ? `#${u.session_no}` : ""}`}</TableCell>
                        <TableCell className="text-xs">{p?.is_manual ? t("manual", { reason: p.manual_reason ?? "" }) : null}</TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
          {r.day_effects.length > 0 ? (
            <Card>
              <CardHeader><CardTitle><h2>{t("effects")}</h2></CardTitle></CardHeader>
              <CardContent>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>{t("effect")}</TableHead>
                      <TableHead>{t("days")}</TableHead>
                      <TableHead>{t("minutes")}</TableHead>
                      <TableHead>{t("reasonCode")}</TableHead>
                      <TableHead>{t("rule")}</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {r.day_effects.map((e, i) => (
                      <TableRow key={i}>
                        <TableCell>{e.effect_kind}</TableCell>
                        <TableCell>{e.days}</TableCell>
                        <TableCell>{e.minutes}</TableCell>
                        <TableCell>{e.reason_code}</TableCell>
                        <TableCell className="font-mono text-xs">{e.rule_ref}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </CardContent>
            </Card>
          ) : null}
          <Explanation r={r} title={t("explanation")} labels={{ step: t("step"), rule: t("rule"), outcome: t("outcome"), inputs: t("inputs") }} />
          {d && d.history.length > 0 ? (
            <details className="rounded-lg border p-4">
              <summary className="cursor-pointer font-medium">{t("history", { count: d.history.length })}</summary>
              <div className="mt-4 grid gap-4">
                {d.history.map((h) => <ResultCard key={h.id} r={h} tz={h.schedule_tz ?? tz} title={t("produced", { at: wallDateTime(h.produced_at, tz), trigger: h.trigger_kind, engine: h.engine_version })} />)}
              </div>
            </details>
          ) : null}
        </>
      )}

      <DayActions
        employeeId={emp.data.id}
        branchId={emp.data.current.master_branch_id ?? null}
        date={date}
        tz={tz}
        punches={r ? r.punches_used.map((u) => byId.get(u.raw_punch_id)).filter((p): p is Punch => Boolean(p)) : []}
        canCorrect={can(grants, "ATT-06")}
        canManual={can(grants, "TCP-04")}
      />
    </section>
  );
}

async function ResultCard({ r, tz, title }: { r: DayResult; tz: string; title: string }) {
  const t = await getTranslations("attendance");
  const row = (label: string, value: React.ReactNode) => (
    <div className="grid grid-cols-[10rem_1fr] gap-2 py-0.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
  return (
    <Card>
      <CardHeader><CardTitle><h2>{title}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-4 md:grid-cols-2">
        <dl className="text-sm">
          {row(t("dayStatus"), <Badge variant={r.day_status === "absent" ? "destructive" : "secondary"}>{t(`st.${r.day_status ?? "none"}`)}</Badge>)}
          {row(t("fraction"), r.day_fraction_present)}
          {row(t("gross"), hours(r.gross_minutes))}
          {row(t("breaks"), hours(r.break_minutes))}
          {row(t("netHours"), hours(r.net_minutes))}
          {row(t("late"), r.late_minutes ?? 0)}
          {row(t("early"), r.early_minutes ?? 0)}
          {row(t("overtime"), r.overtime_identified_minutes ?? 0)}
          {row(t("marks"), [r.late_mark ? t("lateMark") : "", r.early_mark ? t("earlyMark") : ""].filter(Boolean).join(", "))}
          {row(t("flags"), (r.flags ?? []).join(", "))}
        </dl>
        <div className="text-sm">
          <h3 className="mb-1 font-medium">{t("sessions")}</h3>
          <ul className="grid gap-1">
            {(r.sessions ?? []).map((s, i) => (
              <li key={i}>
                {t("in")} {wallTime(s.in_at, tz)} · {t("out")} {wallTime(s.out_at, tz) || "—"}
                {s.minutes != null ? ` · ${hours(s.minutes)} h` : ""}
              </li>
            ))}
          </ul>
        </div>
      </CardContent>
    </Card>
  );
}

function Explanation({ r, title, labels }: { r: DayResult; title: string; labels: { step: string; rule: string; outcome: string; inputs: string } }) {
  const steps = [...(r.explanation ?? [])].sort((a, b) => a.step - b.step);
  if (steps.length === 0) return null;
  return (
    <Card>
      <CardHeader><CardTitle><h2>{title}</h2></CardTitle></CardHeader>
      <CardContent>
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{labels.step}</TableHead>
              <TableHead>{labels.rule}</TableHead>
              <TableHead>{labels.outcome}</TableHead>
              <TableHead>{labels.inputs}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {steps.map((s) => (
              <TableRow key={s.step}>
                <TableCell>{s.step}</TableCell>
                <TableCell className="font-mono text-xs">{s.rule_ref}</TableCell>
                <TableCell>{s.outcome}</TableCell>
                <TableCell className="max-w-md truncate font-mono text-xs" title={s.inputs ? JSON.stringify(s.inputs) : ""}>
                  {s.inputs ? JSON.stringify(s.inputs) : ""}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
