"use client";
import { useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { dateIn } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { EmployeeSummary, Page, RosterEntry, Shift } from "@/types/api";

const addDays = (d: string, n: number) => {
  const x = new Date(`${d}T00:00:00Z`);
  x.setUTCDate(x.getUTCDate() + n);
  return x.toISOString().slice(0, 10);
};
const monday = (d: string) => addDays(d, -((new Date(`${d}T00:00:00Z`).getUTCDay() + 6) % 7));
const OFF = "off";

/**
 * A week of roster cells: employees in scope × 7 days. Only changed cells are sent, as one `roster-entries/set`.
 * What a cell means for attendance (and whether the employee is on a roster schedule) is the server's business.
 */
// shortcut: first 50 employees (searchable) and one week; add paging and copy-forward screens when rosters grow.
export function RosterGrid({ canEdit }: { canEdit: boolean }) {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const [week, setWeek] = useState(() => monday(dateIn("Asia/Kolkata")));
  const [q, setQ] = useState("");
  const [term, setTerm] = useState("");
  useEffect(() => {
    const id = setTimeout(() => setTerm(q.trim()), 300);
    return () => clearTimeout(id);
  }, [q]);
  const days = Array.from({ length: 7 }, (_, i) => addDays(week, i));
  const people = useBff<Page<EmployeeSummary>>(`/employees?limit=50&sort=full_name${term.length >= 2 ? `&q=${encodeURIComponent(term)}` : ""}`);
  const roster = useBff<Page<RosterEntry>>(`/roster-entries?from=${days[0]}&to=${days[6]}`);
  const shifts = useBff<Page<Shift>>("/shifts");
  const [edits, setEdits] = useState<Record<string, string>>({}); // "employee|date" -> shift id | "off" | ""
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());

  const byCell = new Map((roster.data?.items ?? []).map((r) => [`${r.employee_id}|${r.roster_date}`, r]));
  const saved = (emp: string, date: string) => {
    const e = byCell.get(`${emp}|${date}`);
    return !e ? "" : e.is_weekly_off ? OFF : (e.shift_id ?? "");
  };
  const value = (emp: string, date: string) => edits[`${emp}|${date}`] ?? saved(emp, date);
  const changed = Object.entries(edits).filter(([k, v]) => {
    const [emp, date] = k.split("|");
    return v !== saved(emp, date);
  });
  const pending = changed.length > 0;

  async function save(reason: string) {
    setBusy(true);
    setProblem(null);
    const entries = changed.map(([k, v]) => {
      const [employee_id, roster_date] = k.split("|");
      return v === OFF ? { employee_id, roster_date, is_weekly_off: true } : { employee_id, roster_date, is_weekly_off: false, shift_id: v };
    });
    const r = await bff("/roster-entries/set", sendJson("POST", { reason: reason || undefined, entries }, { "idempotency-key": key }));
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    setKey(crypto.randomUUID());
    setEdits({});
    roster.reload();
  }

  const shiftOptions = (shifts.data?.items ?? []).filter((s) => !s.retired_at);
  return (
    <div className="grid gap-3">
      <div className="flex flex-wrap items-end gap-3">
        {/* Unsaved changes belong to this week and these rows: they must be saved or discarded before either changes. */}
        <Button variant="outline" size="sm" aria-label={t("prevWeek")} disabled={pending} onClick={() => setWeek(addDays(week, -7))}>‹</Button>
        <span className="font-medium" aria-live="polite">{t("week")} {week}</span>
        <Button variant="outline" size="sm" aria-label={t("nextWeek")} disabled={pending} onClick={() => setWeek(addDays(week, 7))}>›</Button>
        <div className="grid gap-1">
          <Label htmlFor="r-q" className="text-xs">{t("searchEmployees")}</Label>
          <Input id="r-q" type="search" value={q} onChange={(e) => setQ(e.target.value)} maxLength={100} disabled={pending} />
        </div>
        {pending ? <Button variant="ghost" size="sm" onClick={() => setEdits({})}>{t("discard")}</Button> : null}
      </div>
      <p className="text-xs text-muted-foreground">{pending ? t("pendingWeek") : t("rosterNote")}</p>
      <ProblemAlert problem={people.problem ?? roster.problem ?? shifts.problem ?? problem} />
      {(!people.data || !roster.data) && !people.problem && !roster.problem ? <Skeleton className="h-40" aria-busy="true" aria-label={c("loading")} /> : null}
      {people.data && roster.data ? (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr>
                <th className="p-1 text-left">{t("employee")}</th>
                {days.map((d) => <th key={d} className="p-1 text-left font-normal">{d.slice(5)}</th>)}
              </tr>
            </thead>
            <tbody>
              {people.data.items.map((p) => (
                <tr key={p.id} className="border-t">
                  <th scope="row" className="p-1 text-left font-normal">{p.full_name} <span className="font-mono text-xs text-muted-foreground">{p.employee_code}</span></th>
                  {days.map((d) => {
                    const entry = byCell.get(`${p.id}|${d}`);
                    // A saved shift no longer offered (retired) still shows as itself, never as "unset".
                    const extra = entry?.shift_id && !shiftOptions.some((s) => s.id === entry.shift_id) ? entry : null;
                    return (
                      <td key={d} className="p-1">
                        <NativeSelect
                          aria-label={`${p.full_name} ${d}`}
                          value={value(p.id, d)}
                          disabled={!canEdit || busy}
                          onChange={(e) => setEdits((o) => ({ ...o, [`${p.id}|${d}`]: e.target.value }))}
                          className="h-8 min-w-24"
                        >
                          {/* The API has no "clear": a saved cell can only change to a shift or a weekly off. */}
                          {!entry ? <option value="">{t("unset")}</option> : null}
                          <option value={OFF}>{t("off")}</option>
                          {extra ? <option value={extra.shift_id!}>{extra.shift_code ?? "?"}</option> : null}
                          {shiftOptions.map((s) => <option key={s.id} value={s.id}>{s.code}</option>)}
                        </NativeSelect>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      {canEdit && changed.length > 0 ? (
        <form
          className="flex flex-wrap items-end gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            save(String(new FormData(e.currentTarget).get("reason") ?? "").trim());
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="r-reason">{t("rosterReason")}</Label>
            <Input id="r-reason" name="reason" maxLength={1000} />
          </div>
          <Button type="submit" disabled={busy}>{t("save")} ({t("changes", { count: changed.length })})</Button>
        </form>
      ) : null}
    </div>
  );
}
