"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { EmployeeSchedule, Page, Shift, WeeklyOffPattern } from "@/types/api";

/** An employee's dated schedules (SCH-03) and assigning a new one from a date (SCH-02). */
export function EmployeeScheduleCard({ employeeId, canAssign }: { employeeId: string; canAssign: boolean }) {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const list = useBff<Page<EmployeeSchedule>>(`/employee-schedules?employee_id=${employeeId}`);
  // Retired ones too, so older history rows still show their names.
  const shifts = useBff<Page<Shift>>("/shifts?include_retired=true");
  const patterns = useBff<Page<WeeklyOffPattern>>("/weekly-off-patterns?include_retired=true");
  const [source, setSource] = useState<"fixed_shift" | "roster">("fixed_shift");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const shiftName = (id: string | null) => shifts.data?.items.find((s) => s.id === id)?.code ?? (id ? "…" : t("none"));
  const patternName = (id: string | null) => patterns.data?.items.find((p) => p.id === id)?.name ?? (id ? "…" : t("none"));
  const rows = [...(list.data?.items ?? [])].sort((a, b) => b.valid_from.localeCompare(a.valid_from));

  return (
    <div className="grid gap-3">
      <ProblemAlert problem={list.problem} />
      {list.data && rows.length === 0 ? <p className="text-sm text-muted-foreground">{t("noSchedule")}</p> : null}
      {rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("from")}</TableHead>
              <TableHead>{t("to")}</TableHead>
              <TableHead>{t("source")}</TableHead>
              <TableHead>{t("shift")}</TableHead>
              <TableHead>{t("pattern")}</TableHead>
              <TableHead>{t("reason")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((s) => (
              <TableRow key={s.id}>
                <TableCell>{s.valid_from}</TableCell>
                <TableCell>{s.valid_to ?? ""}</TableCell>
                <TableCell>{t(`src.${s.source}`)}</TableCell>
                <TableCell>{s.source === "fixed_shift" ? shiftName(s.shift_id) : ""}</TableCell>
                <TableCell>{patternName(s.weekly_off_pattern_id)}</TableCell>
                <TableCell className="max-w-xs truncate">{s.reason}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {canAssign ? (
        <form
          className="grid gap-3 rounded-lg border p-4"
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const f = new FormData(form);
            const opt = (k: string) => String(f.get(k) ?? "") || undefined;
            setBusy(true);
            setProblem(null);
            const r = await bff("/employee-schedules", sendJson("POST", {
              employee_id: employeeId,
              source,
              shift_id: source === "fixed_shift" ? opt("shift_id") : undefined,
              weekly_off_pattern_id: opt("weekly_off_pattern_id"),
              valid_from: String(f.get("valid_from")),
              reason: String(f.get("reason") ?? "").trim(),
            }, { "idempotency-key": key }));
            setBusy(false);
            if (!r.ok) return setProblem(r.problem);
            setKey(crypto.randomUUID());
            form.reset();
            list.reload();
          }}
        >
          <h3 className="font-medium">{t("assign")}</h3>
          <div className="grid gap-3 sm:grid-cols-3">
            <div className="grid gap-1">
              <Label htmlFor="es-source">{t("source")}</Label>
              <NativeSelect id="es-source" value={source} onChange={(e) => setSource(e.target.value as typeof source)}>
                <option value="fixed_shift">{t("src.fixed_shift")}</option>
                <option value="roster">{t("src.roster")}</option>
              </NativeSelect>
            </div>
            {source === "fixed_shift" ? (
              <div className="grid gap-1">
                <Label htmlFor="es-shift">{t("shift")}</Label>
                <NativeSelect id="es-shift" name="shift_id" required defaultValue="">
                  <option value="" />
                  {(shifts.data?.items ?? []).filter((s) => !s.retired_at).map((s) => <option key={s.id} value={s.id}>{s.code} · {s.name}</option>)}
                </NativeSelect>
              </div>
            ) : null}
            <div className="grid gap-1">
              <Label htmlFor="es-pattern">{t("pattern")}</Label>
              <NativeSelect id="es-pattern" name="weekly_off_pattern_id" defaultValue="">
                <option value="">{t("none")}</option>
                {(patterns.data?.items ?? []).filter((p) => !p.retired_at).map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
              </NativeSelect>
            </div>
            <div className="grid gap-1">
              <Label htmlFor="es-from">{t("from")}</Label>
              <Input id="es-from" name="valid_from" type="date" required />
            </div>
            <div className="grid gap-1 sm:col-span-2">
              <Label htmlFor="es-reason">{t("reason")}</Label>
              <Input id="es-reason" name="reason" required maxLength={1000} />
            </div>
          </div>
          <ProblemAlert problem={problem} />
          <Button type="submit" disabled={busy} className="justify-self-start">{busy ? c("saving") : t("assign")}</Button>
        </form>
      ) : null}
    </div>
  );
}
