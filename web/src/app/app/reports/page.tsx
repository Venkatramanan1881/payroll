"use client";
import { useTranslations } from "next-intl";
import { useEffect, useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import { can, type Grant } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { AttendancePeriod, Branch, Job, ListEntry, Me, Page } from "@/types/api";

// Each report's run path and the grant it needs (reports.yaml).
const REPORTS: { code: string; path: string; op: string }[] = [
  ...["R1", "R2", "R3", "R4", "R5"].map((r) => ({ code: r, path: `/reports/attendance/${r}/runs`, op: "RPT-01" })),
  ...["R6", "R7", "R8", "R9"].map((r) => ({ code: r, path: `/reports/leave/${r}/runs`, op: "RPT-02" })),
  { code: "R10", path: "/reports/people/runs", op: "RPT-03" },
  { code: "R13", path: "/reports/payroll-input-summary/runs", op: "RPT-04" },
  ...["R25", "R26"].map((r) => ({ code: r, path: `/reports/control/${r}/runs`, op: "RPT-05" })),
  { code: "R27", path: "/reports/audit-trail/runs", op: "RPT-06" },
];
const mine = (g: Grant[]) => REPORTS.filter((r) => can(g, r.op));

/** Run a report (a job) and download its spreadsheet. Filters apply after the caller's scope (AC-C13-01.2). */
export default function ReportsPage() {
  const t = useTranslations("rpt");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const reports = mine(g);
  const [code, setCode] = useState("");
  const [range, setRange] = useState<"dates" | "period">("dates");
  const [job, setJob] = useState<Job | null>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [busy, setBusy] = useState(false);
  // One Idempotency-Key per distinct run: a retry of the same run reuses it; any other report or filter gets a new one.
  const [last, setLast] = useState<{ run: string; key: string } | null>(null);
  const branches = useBff<Page<Branch>>("/branches?limit=200&sort=name");
  const departments = useBff<Page<ListEntry>>("/lists/departments?limit=200");
  const periods = useBff<Page<AttendancePeriod>>("/periods"); // the API decides who reads periods (PER-08, PER-05)
  const report = reports.find((r) => r.code === code) ?? reports[0];
  const byPeriod = report?.code === "R13" || range === "period"; // R13 always reads a period

  // Follow the job until it finishes; reports usually finish before the run answers.
  useEffect(() => {
    if (!job || job.state === "succeeded" || job.state === "failed") return;
    const timer = setTimeout(async () => {
      const r = await bff<Job>(`/jobs/${job.id}`);
      if (r.ok) return setJob({ ...r.data, download_url: null }); // links are asked for on each download, never kept
      setProblem(r.problem);
      if (r.problem.status === 0 || r.problem.status >= 500) setJob({ ...job }); // try again after a passing failure
    }, 2000);
    return () => clearTimeout(timer);
  }, [job]);

  // A link is short-lived: ask for a fresh one per click, opening the tab inside the click so it is not blocked.
  async function download() {
    if (!job) return;
    const tab = window.open("", "_blank");
    if (tab) tab.opener = null;
    const r = await bff<Job>(`/jobs/${job.id}`);
    if (!r.ok || !r.data.download_url) {
      tab?.close();
      return setProblem(r.ok ? { status: 0, code: "NO_LINK" } : r.problem); // finished, but no file store gave a link
    }
    if (tab) tab.location.href = r.data.download_url;
    else window.location.assign(r.data.download_url);
  }

  if (me.data && !report) return <p className="text-muted-foreground">{t("noReports")}</p>;
  return (
    <form
      className="grid max-w-3xl gap-3 rounded-lg border p-4 sm:grid-cols-2"
      aria-label={t("sub.reports")}
      onSubmit={async (e) => {
        e.preventDefault();
        if (!report) return;
        const f = new FormData(e.currentTarget);
        const text = (k: string) => String(f.get(k) ?? "");
        const one = (k: string) => (text(k) ? [text(k)] : undefined);
        const body = {
          ...(byPeriod ? { period_id: text("period_id") } : { from: text("from"), to: text("to") }),
          branch_ids: one("branch"),
          department_ids: one("department"),
          format: text("format"),
        };
        const run = JSON.stringify([report.path, body]);
        const key = last?.run === run ? last.key : crypto.randomUUID();
        setLast({ run, key });
        setBusy(true);
        setJob(null);
        const r = await bff<Job>(report.path, sendJson("POST", body, { "idempotency-key": key }));
        setBusy(false);
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setLast(null); // done: the next run, even an identical one, is a new run
        setJob({ ...r.data, download_url: null });
      }}
    >
      <div className="grid gap-1 sm:col-span-2">
        <Label htmlFor="rp-report">{t("report")}</Label>
        <NativeSelect id="rp-report" value={report?.code ?? ""} onChange={(e) => setCode(e.target.value)}>
          {reports.map((r) => <option key={r.code} value={r.code}>{t(`r.${r.code}`)}</option>)}
        </NativeSelect>
      </div>
      {report?.code === "R13" ? (
        <p className="text-sm text-muted-foreground sm:col-span-2">{t("periodOnly")}</p>
      ) : (
        <div className="grid gap-1">
          <Label htmlFor="rp-range">{t("range")}</Label>
          <NativeSelect id="rp-range" value={range} onChange={(e) => setRange(e.target.value as typeof range)}>
            <option value="dates">{t("byDates")}</option>
            <option value="period">{t("byPeriod")}</option>
          </NativeSelect>
        </div>
      )}
      {byPeriod ? (
        <div className="grid gap-1">
          <Label htmlFor="rp-period">{t("period")}</Label>
          <NativeSelect id="rp-period" name="period_id" required defaultValue="" aria-describedby={periods.problem ? "rp-period-problem" : undefined}>
            <option value="" />
            {[...(periods.data?.items ?? [])].sort((a, b) => b.period_start.localeCompare(a.period_start)).map((p) => (
              <option key={p.id} value={p.id}>{p.period_start} – {p.period_end}</option>
            ))}
          </NativeSelect>
          {periods.problem ? <div id="rp-period-problem"><ProblemAlert problem={periods.problem} /></div> : null}
        </div>
      ) : (
        <>
          <div className="grid gap-1">
            <Label htmlFor="rp-from">{t("from")}</Label>
            <Input id="rp-from" name="from" type="date" required />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="rp-to">{t("to")}</Label>
            <Input id="rp-to" name="to" type="date" required />
          </div>
        </>
      )}
      <div className="grid gap-1">
        <Label htmlFor="rp-branch">{t("branch")}</Label>
        <NativeSelect id="rp-branch" name="branch" defaultValue="">
          <option value="">{t("any")}</option>
          {(branches.data?.items ?? []).map((b) => <option key={b.id} value={b.id}>{b.name}</option>)}
        </NativeSelect>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="rp-dept">{t("department")}</Label>
        <NativeSelect id="rp-dept" name="department" defaultValue="">
          <option value="">{t("any")}</option>
          {(departments.data?.items ?? []).map((d) => <option key={d.id} value={d.id}>{d.name}</option>)}
        </NativeSelect>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="rp-format">{t("format")}</Label>
        <NativeSelect id="rp-format" name="format" defaultValue="xlsx">
          <option value="xlsx">Excel (.xlsx)</option>
          <option value="csv">CSV</option>
        </NativeSelect>
      </div>
      <Button type="submit" className="self-end justify-self-start" disabled={busy || !report}>{busy ? t("running") : t("run")}</Button>
      <div className="grid gap-2 sm:col-span-2">
        <ProblemAlert problem={problem} />
        {/* Always mounted, so screen readers announce the change of state. */}
        <div role="status" className="flex flex-wrap items-center gap-3 text-sm">
          {job ? (
            <>
              <span>{t(`r.${job.kind.replace("report:", "")}`)}</span>
              <Badge variant={job.state === "succeeded" ? "default" : "secondary"}>{t(`st.${job.state}`)}</Badge>
            </>
          ) : null}
        </div>
        {job?.state === "succeeded" ? <Button type="button" size="sm" variant="outline" className="justify-self-start" onClick={download}>{t("download")}</Button> : null}
      </div>
    </form>
  );
}
