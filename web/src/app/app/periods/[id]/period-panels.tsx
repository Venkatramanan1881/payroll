"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Decide } from "@/components/decide";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { AttendancePeriod, Checklist, Page, PayrollExport, PeriodAdjustment, PeriodTotals, UnlockRequest } from "@/types/api";

type May = { read: boolean; checklist: boolean; lock: boolean; requestUnlock: boolean; decideUnlock: boolean; exportIt: boolean; download: boolean };

/** The period's working sections. Each shows what the API returns; whether an action is allowed is the API's answer. */
export function PeriodPanels({ period, may }: { period: AttendancePeriod; may: May }) {
  const router = useRouter();
  const lockable = period.state === "awaiting_lock" || period.state === "unlocked";
  return (
    <div className="grid gap-4">
      {may.checklist && period.state !== "locked" ? <ChecklistCard id={period.id} /> : null}
      {may.lock && lockable ? <LockCard id={period.id} onDone={() => router.refresh()} /> : null}
      <Unlocks period={period} may={may} onDone={() => router.refresh()} />
      {/* Totals and adjustments need PER-08; a decide-only reader (the Organisation Admin) sees the period and its unlocks. */}
      {may.read ? <TotalsCard id={period.id} /> : null}
      {may.read ? <AdjustmentsCard id={period.id} /> : null}
      {may.exportIt || may.download ? <ExportsCard period={period} may={may} /> : null}
    </div>
  );
}

function ChecklistCard({ id }: { id: string }) {
  const t = useTranslations("per");
  const { data, problem } = useBff<Checklist>(`/periods/${id}/checklist`);
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("checklist")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        <ProblemAlert problem={problem} />
        {data ? (
          <p className="flex flex-wrap gap-2 text-sm">
            {Object.entries(data.counts).map(([k, n]) => <Badge key={k} variant={n ? "destructive" : "secondary"}>{t(`ck.${k}`)}: {n}</Badge>)}
          </p>
        ) : null}
        {data && data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("checklistEmpty")}</p> : null}
        {data && data.items.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("date")}</TableHead>
                <TableHead>{t("employee")}</TableHead>
                <TableHead>{t("checklist")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.items.map((x, i) => (
                <TableRow key={i}>
                  <TableCell>{x.date}</TableCell>
                  <TableCell>{x.employee_name ?? ""} <span className="font-mono text-xs text-muted-foreground">{x.employee_code}</span></TableCell>
                  <TableCell>{t(`ck.${x.kind}`)}{x.request_type ? ` · ${x.request_type}` : ""}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
      </CardContent>
    </Card>
  );
}

function LockCard({ id, onDone }: { id: string; onDone: () => void }) {
  const t = useTranslations("per");
  const [comment, setComment] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("lock")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-2">
        <p className="text-sm text-muted-foreground">{t("lockHelp")}</p>
        <Label htmlFor="lock-comment">{t("comment")}</Label>
        <Input id="lock-comment" value={comment} onChange={(e) => setComment(e.target.value)} maxLength={1000} />
        <Button
          className="justify-self-start"
          disabled={busy}
          onClick={async () => {
            if (!window.confirm(t("confirmLock"))) return;
            setBusy(true);
            setProblem(null);
            const r = await bff(`/periods/${id}/lock`, sendJson("POST", comment.trim() ? { comment: comment.trim() } : {}, { "idempotency-key": key }));
            setBusy(false);
            if (!r.ok) return setProblem(r.problem);
            onDone();
          }}
        >
          {t("lock")}
        </Button>
        <ProblemAlert problem={problem} />
      </CardContent>
    </Card>
  );
}

function Unlocks({ period, may, onDone }: { period: AttendancePeriod; may: May; onDone: () => void }) {
  const t = useTranslations("per");
  const r = useTranslations("regularisation");
  const list = useBff<Page<UnlockRequest>>(may.requestUnlock || may.decideUnlock ? `/periods/${period.id}/unlock-requests` : "");
  const [reason, setReason] = useState("");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  if (!may.requestUnlock && !may.decideUnlock) return null;
  const rows = list.data?.items ?? [];
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("unlocks")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        <ProblemAlert problem={list.problem ?? problem} />
        {list.data && rows.length === 0 ? <p className="text-sm text-muted-foreground">{t("noUnlocks")}</p> : null}
        {rows.map((u) => (
          <div key={u.id} className="grid gap-2 rounded-md border p-3 text-sm">
            <div>{wallDateTime(u.submitted_at)} · <Badge variant="secondary">{r(`rs.${u.state}`)}</Badge> — {u.reason}</div>
            {may.decideUnlock && (u.state === "pending_l1" || u.state === "pending_l2") ? (
              <Decide base={`/period/unlock-requests/${u.id}`} yes="approve" onDone={() => { list.reload(); onDone(); }} />
            ) : null}
          </div>
        ))}
        {may.requestUnlock && period.state === "locked" ? (
          <form
            className="flex flex-wrap items-end gap-2"
            onSubmit={async (e) => {
              e.preventDefault();
              const res = await bff(`/periods/${period.id}/unlock-requests`, sendJson("POST", { reason: reason.trim() }, { "idempotency-key": key }));
              if (!res.ok) return setProblem(res.problem);
              setProblem(null);
              setReason("");
              setKey(crypto.randomUUID());
              list.reload();
            }}
          >
            <div className="grid gap-1">
              <Label htmlFor="unlock-reason">{t("unlockReason")}</Label>
              <Input id="unlock-reason" value={reason} onChange={(e) => setReason(e.target.value)} required maxLength={1000} />
            </div>
            <Button type="submit" variant="outline">{t("requestUnlock")}</Button>
          </form>
        ) : null}
      </CardContent>
    </Card>
  );
}

function TotalsCard({ id }: { id: string }) {
  const t = useTranslations("per");
  const { data, problem } = useBff<Page<PeriodTotals>>(`/periods/${id}/totals`);
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("totals")}</h2></CardTitle></CardHeader>
      <CardContent>
        <ProblemAlert problem={problem} />
        {data && data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noTotals")}</p> : null}
        {data && data.items.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("employee")}</TableHead>
                <TableHead />
                <TableHead className="text-right">{t("payable")}</TableHead>
                <TableHead className="text-right">{t("deduction")}</TableHead>
                <TableHead className="text-right">{t("late")}</TableHead>
                <TableHead className="text-right">{t("early")}</TableHead>
                <TableHead>{t("hold")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.items.map((x) => (
                <TableRow key={x.employee_id}>
                  <TableCell>{x.employee_name} <span className="font-mono text-xs text-muted-foreground">{x.employee_code}</span></TableCell>
                  <TableCell><Badge variant="outline">{t(`basis.${x.basis}`)}</Badge></TableCell>
                  <TableCell className="text-right">{x.payable_days}</TableCell>
                  <TableCell className="text-right">{x.deduction_days}</TableCell>
                  <TableCell className="text-right">{x.late_minutes}</TableCell>
                  <TableCell className="text-right">{x.early_minutes}</TableCell>
                  <TableCell>{x.pay_hold ? "✓" : ""}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
      </CardContent>
    </Card>
  );
}

function AdjustmentsCard({ id }: { id: string }) {
  const t = useTranslations("per");
  const { data, problem } = useBff<Page<PeriodAdjustment>>(`/period/adjustments?target_period_id=${id}`);
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("adjustments")}</h2></CardTitle></CardHeader>
      <CardContent>
        <ProblemAlert problem={problem} />
        {data && data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noAdjustments")}</p> : null}
        {data && data.items.length > 0 ? (
          <Table>
            <TableBody>
              {data.items.map((a) => (
                <TableRow key={a.id}>
                  <TableCell>{a.employee_name} <span className="font-mono text-xs text-muted-foreground">{a.employee_code}</span></TableCell>
                  <TableCell>{a.original_date}</TableCell>
                  <TableCell className="text-right">{a.days}</TableCell>
                  <TableCell><Badge variant="secondary">{t(`as.${a.state}`)}</Badge></TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
      </CardContent>
    </Card>
  );
}

function ExportsCard({ period, may }: { period: AttendancePeriod; may: May }) {
  const t = useTranslations("per");
  const list = useBff<Page<PayrollExport>>(may.download ? `/periods/${period.id}/payroll-input-exports` : ""); // listing needs PER-07
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const [busy, setBusy] = useState(false);
  // A signed link is a short-lived capability: fetched on click and opened, never kept on the page. The tab is opened
  // inside the click (browsers block a popup opened after an await) and pointed at the link once it arrives.
  async function download(exportId: string) {
    const tab = window.open("", "_blank");
    if (tab) tab.opener = null;
    const r = await bff<{ url: string }>(`/period/payroll-input-exports/${exportId}/download-link`);
    if (!r.ok) {
      tab?.close();
      return setProblem(r.problem);
    }
    if (tab) tab.location.href = r.data.url;
    else window.location.assign(r.data.url);
  }
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("exports")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-3">
        <ProblemAlert problem={list.problem ?? problem} />
        {list.data && list.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noExports")}</p> : null}
        {list.data && list.data.items.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("producedAt")}</TableHead>
                <TableHead>{t("checksum")}</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {list.data.items.map((x) => (
                <TableRow key={x.id}>
                  <TableCell>{wallDateTime(x.produced_at)}</TableCell>
                  <TableCell className="max-w-48 truncate font-mono text-xs">{x.checksum}</TableCell>
                  <TableCell>{may.download ? <Button size="sm" variant="outline" onClick={() => download(x.id)}>{t("download")}</Button> : null}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
        {may.exportIt && period.state === "locked" ? (
          <Button
            className="justify-self-start"
            disabled={busy}
            onClick={async () => {
              setBusy(true);
              setProblem(null);
              const r = await bff(`/periods/${period.id}/payroll-input-exports`, sendJson("POST", {}, { "idempotency-key": key }));
              setBusy(false);
              if (!r.ok) return setProblem(r.problem);
              setKey(crypto.randomUUID());
              list.reload();
            }}
          >
            {t("produce")}
          </Button>
        ) : null}
        <p className="text-xs text-muted-foreground">{t("linkNote")}</p>
      </CardContent>
    </Card>
  );
}
