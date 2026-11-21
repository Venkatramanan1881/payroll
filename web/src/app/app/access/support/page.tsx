"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { Decide } from "@/components/decide";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Me, Page, SupportAction, SupportGrant } from "@/types/api";

/**
 * Vendor support-access requests for this organisation: approve or reject (PLT-14, approve-type), end early (PLT-15)
 * and the vendor's actions under a grant (PLT-18). The window's default length is the server's (OQ-V01-13).
 */
export default function SupportPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const list = useBff<Page<SupportGrant>>("/support-access-grants");
  return (
    <div className="grid max-w-4xl gap-4">
      <ProblemAlert problem={list.problem} />
      {list.loading && !list.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {list.data && list.data.items.length === 0 ? <p className="text-muted-foreground">{t("noGrantsYet")}</p> : null}
      {list.data?.items.map((s) => (
        <div key={`${s.id}-${s.state}`} className="grid gap-3 rounded-lg border p-4">
          <div className="flex flex-wrap items-center gap-3 text-sm">
            <span className="font-medium">{s.vendor_user_name ?? t("vendor")}</span>
            <Badge variant={s.state === "active" ? "default" : "secondary"}>{t(`st.${s.state}`)}</Badge>
            <span className="text-muted-foreground">{t("requested")}: {wallDateTime(s.requested_at)}</span>
            {s.window_start && s.window_end ? <span className="text-muted-foreground">{t("window")}: {wallDateTime(s.window_start)} – {wallDateTime(s.window_end)}</span> : null}
          </div>
          <p>{s.reason}</p>
          {s.state === "requested" && can(g, "PLT-14", ["approve", "all"]) ? <Decide base={`/support-access-grants/${s.id}`} yes="approve" onDone={list.reload} label={s.reason} /> : null}
          {s.state === "active" && can(g, "PLT-15") ? <EndGrant id={s.id} label={s.reason} onDone={list.reload} /> : null}
          {can(g, "PLT-18") ? <Actions id={s.id} label={s.reason} /> : null}
        </div>
      ))}
    </div>
  );
}

function EndGrant({ id, label, onDone }: { id: string; label: string; onDone: () => void }) {
  const t = useTranslations("acc");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  return (
    <form
      className="flex flex-wrap items-end gap-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const reason = String(new FormData(e.currentTarget).get("reason") ?? "").trim();
        const r = await bff(`/support-access-grants/${id}/end`, sendJson("POST", { reason }, { "idempotency-key": key }));
        if (!r.ok) return setProblem(r.problem);
        onDone();
      }}
    >
      <div className="grid gap-1">
        <Label htmlFor={`end-${id}`}>{t("endReason")}</Label>
        <Input id={`end-${id}`} name="reason" required maxLength={1000} />
      </div>
      <Button type="submit" variant="destructive" aria-label={`${t("end")}: ${label}`}>{t("end")}</Button>
      <ProblemAlert problem={problem} />
    </form>
  );
}

function Actions({ id, label }: { id: string; label: string }) {
  const t = useTranslations("acc");
  const [open, setOpen] = useState(false);
  const list = useBff<Page<SupportAction>>(open ? `/support-access-grants/${id}/actions` : "");
  if (!open) return <Button variant="ghost" size="sm" className="justify-self-start" aria-label={`${t("showActions")}: ${label}`} onClick={() => setOpen(true)}>{t("showActions")}</Button>;
  return (
    <div className="grid gap-2">
      <h3 className="font-medium">{t("actions")}</h3>
      <ProblemAlert problem={list.problem} />
      {list.data && list.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noActions")}</p> : null}
      {list.data && list.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("when")}</TableHead>
              <TableHead>{t("vendor")}</TableHead>
              <TableHead>{t("actions")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((a, i) => (
              <TableRow key={i}>
                <TableCell>{wallDateTime(a.occurred_at)}</TableCell>
                <TableCell>{a.vendor_user_name ?? ""}</TableCell>
                <TableCell>{a.action}{a.target ? ` · ${a.target}` : ""}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </div>
  );
}
