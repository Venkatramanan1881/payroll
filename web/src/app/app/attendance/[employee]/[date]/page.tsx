import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { notFound } from "next/navigation";
import { PageProblem } from "@/components/page-problem";
import { DayView } from "@/components/day-view";
import { Badge } from "@/components/ui/badge";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { AttendanceDay, Employee, Me, Page, Punch } from "@/types/api";
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

      <DayView day={d} punches={byId} tz={tz} />


      <DayActions
        employeeId={emp.data.id}
        branchId={emp.data.current.master_branch_id ?? null}
        date={date}
        tz={tz}
        punches={r ? (r.punches_used ?? []).map((u) => byId.get(u.raw_punch_id)).filter((p): p is Punch => Boolean(p)) : []}
        canCorrect={can(grants, "ATT-06")}
        canManual={can(grants, "TCP-04")}
      />
    </section>
  );
}
