"use client";
import { useTranslations } from "next-intl";
import { ProblemAlert } from "@/components/problem-alert";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useBff } from "@/lib/use-bff";
import type { ScheduleDay } from "@/types/api";

const hm = (t: string | null | undefined) => (t ? t.slice(0, 5) : "");

/** The employee's next 14 days as the server resolves them (shift, weekly off, holiday). */
export default function MySchedulePage() {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const { data, problem, loading } = useBff<{ items: ScheduleDay[] }>("/me/roster");
  return (
    <>
      <h1 className="text-xl font-semibold">{t("mySchedule")}</h1>
      <p className="text-sm text-muted-foreground">{t("next14")}</p>
      <ProblemAlert problem={problem} />
      {loading && !data ? <Skeleton className="h-40" aria-busy="true" aria-label={c("loading")} /> : null}
      {data ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("date")}</TableHead>
              <TableHead>{t("shift")}</TableHead>
              <TableHead>{t("times")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.items.map((d) => (
              <TableRow key={d.date}>
                <TableCell>{d.date}</TableCell>
                <TableCell>
                  {d.holiday_name ? t("holiday", { name: d.holiday_name }) : d.is_weekly_off ? t("off") : (d.shift_name ?? t("none"))}
                </TableCell>
                <TableCell>{!d.is_weekly_off && !d.holiday_name && d.start_time ? `${hm(d.start_time)}–${hm(d.end_time)}` : ""}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </>
  );
}
