"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useBff } from "@/lib/use-bff";
import { dateIn, hours, wallTime } from "@/lib/time";
import type { Option } from "@/server/lookups";
import type { DailyRow, Page } from "@/types/api";

const STATUSES = ["present", "half_day", "quarter_day", "absent", "leave", "half_day_leave", "holiday", "weekly_off", "on_duty"] as const;
const STATES = ["provisional", "exception", "absence_pending", "locked"] as const;
// shortcut: the default date and branchless times use India time; take the organisation's zone when branches abroad exist.
const FALLBACK_TZ = "Asia/Kolkata";

/** R1-shaped day board (attendance.yaml /v1/attendance/daily). Values are shown as the API worked them out. */
export function DailyBoard({ branches, departments }: { branches: Option[]; departments: Option[] }) {
  const t = useTranslations("attendance");
  const c = useTranslations("common");
  const [f, setF] = useState({ date: dateIn(FALLBACK_TZ, -1), branch_id: "", department_id: "", day_status: "", processing_state: "", sort: "employee_code" });
  const qs = new URLSearchParams();
  for (const [k, v] of Object.entries(f)) if (v) qs.set(k, v);
  const { data, problem, loading, reload } = useBff<Page<DailyRow>>(`/attendance/daily?${qs}`);
  const rows = data?.items ?? null;

  const set = (k: keyof typeof f) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => setF((o) => ({ ...o, [k]: e.target.value }));
  const tzOf = (id: string | null) => branches.find((b) => b.id === id)?.tz ?? FALLBACK_TZ;

  return (
    <div className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-3 lg:grid-cols-6">
        <div className="grid gap-1">
          <Label htmlFor="date">{t("date")}</Label>
          <Input id="date" type="date" value={f.date} onChange={set("date")} required />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="branch">{t("branch")}</Label>
          <NativeSelect id="branch" value={f.branch_id} onChange={set("branch_id")}>
            <option value="">{c("all")}</option>
            {branches.map((b) => <option key={b.id} value={b.id}>{b.name}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="department">{t("department")}</Label>
          <NativeSelect id="department" value={f.department_id} onChange={set("department_id")}>
            <option value="">{c("all")}</option>
            {departments.map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="day_status">{t("dayStatus")}</Label>
          <NativeSelect id="day_status" value={f.day_status} onChange={set("day_status")}>
            <option value="">{c("all")}</option>
            {STATUSES.map((s) => <option key={s} value={s}>{t(`st.${s}`)}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="processing_state">{t("processing")}</Label>
          <NativeSelect id="processing_state" value={f.processing_state} onChange={set("processing_state")}>
            <option value="">{c("all")}</option>
            {STATES.map((s) => <option key={s} value={s}>{t(`ps.${s}`)}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="sort">{t("sort")}</Label>
          <NativeSelect id="sort" value={f.sort} onChange={set("sort")}>
            <option value="employee_code">{t("code")}</option>
            <option value="day_status">{t("dayStatus")}</option>
          </NativeSelect>
        </div>
      </div>

      <ProblemAlert problem={problem} />
      {problem ? <Button variant="outline" className="justify-self-start" onClick={reload}>{c("retry")}</Button> : null}
      {loading ? (
        <div className="grid gap-2" aria-busy="true" aria-label={c("loading")}>
          <Skeleton className="h-8" />
          <Skeleton className="h-8" />
        </div>
      ) : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {rows && rows.length > 0 ? (
        <>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("code")}</TableHead>
                <TableHead>{t("name")}</TableHead>
                <TableHead>{t("dayStatus")}</TableHead>
                <TableHead>{t("firstIn")}</TableHead>
                <TableHead>{t("lastOut")}</TableHead>
                <TableHead className="text-right">{t("netHours")}</TableHead>
                <TableHead className="text-right">{t("late")}</TableHead>
                <TableHead className="text-right">{t("early")}</TableHead>
                <TableHead>{t("flags")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((r) => (
                <TableRow key={r.employee_id}>
                  <TableCell className="font-mono">{r.employee_code}</TableCell>
                  <TableCell>
                    <Link href={`/app/attendance/${r.employee_id}/${r.attendance_date}`} className="underline-offset-2 hover:underline">{r.employee_name}</Link>
                  </TableCell>
                  <TableCell>
                    <Badge variant={r.day_status === "absent" ? "destructive" : "secondary"}>{t(`st.${r.day_status ?? "none"}`)}</Badge>{" "}
                    {r.processing_state !== "provisional" ? <span className="text-xs text-muted-foreground">{t(`ps.${r.processing_state}`)}</span> : null}
                  </TableCell>
                  <TableCell>{wallTime(r.first_in_at, tzOf(r.branch_id))}</TableCell>
                  <TableCell>{wallTime(r.last_out_at, tzOf(r.branch_id))}</TableCell>
                  <TableCell className="text-right">{hours(r.net_minutes)}</TableCell>
                  <TableCell className="text-right">{r.late_minutes || ""}</TableCell>
                  <TableCell className="text-right">{r.early_minutes || ""}</TableCell>
                  <TableCell className="text-xs">{r.flags.join(", ")}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {rows.length >= 500 ? <p className="text-sm text-muted-foreground">{t("capNote")}</p> : null}
        </>
      ) : null}
    </div>
  );
}
