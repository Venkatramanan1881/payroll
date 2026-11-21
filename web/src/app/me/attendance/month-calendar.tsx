"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { dateIn } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { CalendarDay } from "@/types/api";

const shiftMonth = (m: string, by: number) => {
  const d = new Date(`${m}-01T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() + by);
  return d.toISOString().slice(0, 7);
};

/** The employee's month from the calendar endpoint; statuses are the API's (and hidden when AC-05 hides them). */
export function MonthCalendar({ employeeId }: { employeeId: string }) {
  const t = useTranslations("me");
  const a = useTranslations("attendance");
  const c = useTranslations("common");
  // shortcut: "this month" in India time; use the employee's branch zone when branches abroad exist.
  const [month, setMonth] = useState(() => dateIn("Asia/Kolkata").slice(0, 7));
  const { data, problem, loading } = useBff<{ days: CalendarDay[] }>(`/attendance/employees/${employeeId}/calendar?month=${month}`);
  const days = data?.days ?? [];
  // Monday-first grid: blank cells before the 1st.
  const lead = days.length ? (new Date(`${days[0].attendance_date}T00:00:00Z`).getUTCDay() + 6) % 7 : 0;
  const week = t.raw("week") as string[];

  return (
    <div className="grid gap-3">
      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => setMonth((m) => shiftMonth(m, -1))} aria-label={t("prev")}>‹</Button>
        <span className="min-w-24 text-center font-medium" aria-live="polite">{month}</span>
        <Button variant="outline" size="sm" onClick={() => setMonth((m) => shiftMonth(m, 1))} aria-label={t("next")}>›</Button>
      </div>
      <ProblemAlert problem={problem} />
      {loading && !data ? <Skeleton className="h-64" aria-busy="true" aria-label={c("loading")} /> : null}
      {days.length > 0 ? (
        <>
          <div className="grid grid-cols-7 gap-1 text-sm" aria-label={month}>
            {week.map((w) => <div key={w} className="p-1 text-center text-xs text-muted-foreground" aria-hidden="true">{w}</div>)}
            {Array.from({ length: lead }, (_, i) => <div key={`b${i}`} />)}
            {days.map((d) => (
              <Link
                key={d.attendance_date}
                href={`/me/attendance/${d.attendance_date}`}
                aria-label={[d.attendance_date, d.day_status ? a(`st.${d.day_status}`) : "", d.late_mark ? a("lateMark") : "", d.early_mark ? a("earlyMark") : "", d.has_exception ? a("ps.exception") : ""].filter(Boolean).join(", ")}
                className={`min-h-16 rounded-md border p-1 hover:bg-muted ${d.has_exception ? "border-destructive" : ""}`}
              >
                <div className="text-xs text-muted-foreground">{Number(d.attendance_date.slice(8))}</div>
                {d.day_status ? <div className="truncate text-xs">{a(`st.${d.day_status}`)}</div> : null}
                <div className="text-xs">
                  {d.late_mark ? "L " : ""}
                  {d.early_mark ? "E " : ""}
                  {d.has_exception ? "!" : ""}
                </div>
              </Link>
            ))}
          </div>
          <p className="text-xs text-muted-foreground">{t("legendLate")}</p>
        </>
      ) : null}
    </div>
  );
}
