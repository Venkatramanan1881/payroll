import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { notFound } from "next/navigation";
import { DayView } from "@/components/day-view";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { api } from "@/server/api";
import type { AttendanceDay, Me, Page, Punch } from "@/types/api";
import { AskCorrection } from "./ask-correction";

const DATE = /^\d{4}-\d{2}-\d{2}$/;
const shift = (d: string, days: number) => {
  const x = new Date(`${d}T00:00:00Z`);
  x.setUTCDate(x.getUTCDate() + days);
  return x.toISOString().slice(0, 10);
};

/** The employee's own day: what the API lets them see (AC-05), and a way to ask for a correction (ATT-02). */
export default async function MyDayPage({ params }: { params: Promise<{ date: string }> }) {
  const { date } = await params;
  if (!DATE.test(date)) notFound();
  const t = await getTranslations();
  const me = await api<Me>("/v1/me");
  if (!me.ok) return <PageProblem problem={me.problem} />;
  const id = me.data.employee_id;
  if (!id) return <p className="text-muted-foreground">{t("me.noEmployeeHere")}</p>;
  const [day, punches] = await Promise.all([
    api<AttendanceDay>(`/v1/attendance/employees/${id}/days/${date}`),
    api<Page<Punch>>(`/v1/punches?employee_id=${id}&from=${shift(date, -1)}&to=${shift(date, 1)}`), // refused when AC-05 hides punches
  ]);
  if (!day.ok && day.problem.status !== 404) return <PageProblem problem={day.problem} />;
  const d = day.ok ? day.data : null;
  // shortcut: India time before any result exists; employees cannot read branch settings to learn their zone.
  const tz = d?.current_result?.schedule_tz ?? "Asia/Kolkata";
  const byId = new Map((punches.ok ? punches.data.items : []).map((p) => [p.id, p]));
  return (
    <>
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/me/attendance" className="text-sm underline">{t("me.nav.attendance")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{date}</h1>
        {d ? <Badge variant="outline">{t(`attendance.ps.${d.processing_state}`)}</Badge> : null}
      </div>
      <p className="text-sm text-muted-foreground">{t("attendance.tzNote", { tz })}</p>
      <DayView day={d} punches={byId} tz={tz} />
      <AskCorrection date={date} tz={tz} punches={[...byId.values()]} />
    </>
  );
}
