import { getTranslations } from "next-intl/server";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { hours, wallDateTime, wallTime } from "@/lib/time";
import type { AttendanceDay, DayResult, Punch } from "@/types/api";

/**
 * One day as the server worked it out: result, punches, effects, explanation (BR-08) and superseded results (BRX-03).
 * Shared by HR's and the employee's pages. Parts the API omits for employees (AC-05) are simply not shown.
 */
export async function DayView({ day, punches, tz }: { day: AttendanceDay | null; punches: Map<string, Punch>; tz: string }) {
  const t = await getTranslations("attendance");
  const d = day;
  const r = d?.current_result ?? null;
  const byId = punches;
  return (
    <>
    {!r ? (
      <p className="text-muted-foreground">{t("noDay")}</p>
    ) : (
      <>
        <ResultCard r={r} tz={tz} title={t("result")} />
        {r.punches_used ? (
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
                {(r.punches_used ?? []).map((u) => {
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
        ) : null}
        {(r.day_effects ?? []).length > 0 ? (
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
                  {(r.day_effects ?? []).map((e, i) => (
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
    </>
  );
}

async function ResultCard({ r, tz, title }: { r: DayResult; tz: string; title: string }) {
  const t = await getTranslations("attendance");
  // A row only for a field the API sent: for employees, AC-05 may omit it, and an omitted field is not "none" or 0.
  const row = (label: string, value: React.ReactNode, field: keyof DayResult) => !(field in r) ? null : (
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
          {row(t("dayStatus"), <Badge variant={r.day_status === "absent" ? "destructive" : "secondary"}>{t(`st.${r.day_status ?? "none"}`)}</Badge>, "day_status")}
          {row(t("fraction"), r.day_fraction_present, "day_fraction_present")}
          {row(t("gross"), hours(r.gross_minutes), "gross_minutes")}
          {row(t("breaks"), hours(r.break_minutes), "break_minutes")}
          {row(t("netHours"), hours(r.net_minutes), "net_minutes")}
          {row(t("late"), r.late_minutes ?? 0, "late_minutes")}
          {row(t("early"), r.early_minutes ?? 0, "early_minutes")}
          {row(t("overtime"), r.overtime_identified_minutes ?? 0, "overtime_identified_minutes")}
          {row(t("marks"), [r.late_mark ? t("lateMark") : "", r.early_mark ? t("earlyMark") : ""].filter(Boolean).join(", "), "late_mark")}
          {row(t("flags"), (r.flags ?? []).join(", "), "flags")}
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
