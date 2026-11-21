"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Page, Regularisation } from "@/types/api";

/** The employee's own correction requests, with cancel while pending (SM-REQ R10). */
export function MyRegularisations({ employeeId, userId }: { employeeId: string; userId: string }) {
  const t = useTranslations("me");
  const r = useTranslations("regularisation");
  const a = useTranslations("attendance");
  const c = useTranslations("common");
  const { data, problem, loading, reload } = useBff<Page<Regularisation>>(`/attendance/regularisation-requests?employee_id=${employeeId}`); // own only, even for HR who are also employees
  const rows = data?.items ?? null;
  if (loading && !rows) return <Skeleton className="h-8" aria-busy="true" aria-label={c("loading")} />;
  if (problem) return <ProblemAlert problem={problem} />;
  if (!rows || rows.length === 0) return <p className="text-muted-foreground">{t("noRequests")}</p>;
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{r("date")}</TableHead>
          <TableHead>{r("type")}</TableHead>
          <TableHead>{r("reason")}</TableHead>
          <TableHead>{r("state")}</TableHead>
          <TableHead />
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((q) => (
          <TableRow key={q.id}>
            <TableCell>{q.attendance_date}</TableCell>
            <TableCell>{a(`rt.${q.request_type}`)}</TableCell>
            <TableCell className="max-w-xs truncate">{q.reason}</TableCell>
            <TableCell><Badge variant="secondary">{r(`rs.${q.state}`)}</Badge></TableCell>
            <TableCell>{/* Only the requester may cancel (SM-REQ R10), not the employee of a request filed for them. */}
              {(q.state === "pending_l1" || q.state === "pending_l2") && q.requested_by_user_id === userId ? <CancelOne id={q.id} onDone={reload} /> : null}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function CancelOne({ id, onDone }: { id: string; onDone: () => void }) {
  const t = useTranslations("me");
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  if (!open) return <Button size="sm" variant="outline" onClick={() => setOpen(true)}>{t("cancel")}…</Button>;
  return (
    <div className="grid gap-1">
      <Label htmlFor={`r-${id}`} className="text-xs">{t("cancelReason")}</Label>
      <Input id={`r-${id}`} value={reason} onChange={(e) => setReason(e.target.value)} maxLength={1000} />
      <Button
        size="sm"
        variant="destructive"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          setProblem(null);
          const res = await bff(`/attendance/regularisation-requests/${id}/cancel`, sendJson("POST", reason.trim() ? { reason: reason.trim() } : {}, { "idempotency-key": key }));
          setBusy(false);
          if (!res.ok) return setProblem(res.problem);
          onDone();
        }}
      >
        {t("confirmCancel")}
      </Button>
      <ProblemAlert problem={problem} />
    </div>
  );
}
