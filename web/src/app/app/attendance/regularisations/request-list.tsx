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
import { wallDateTime } from "@/lib/time";
import type { Page, Regularisation } from "@/types/api";

const STATES = ["pending_l1", "pending_l2", "approved", "rejected", "cancelled"] as const;

/** "Waiting for me" (pending-approval) and "All" (ATT-01) lists of regularisation requests. */
export function RequestList({ canDecide, canSeeAll }: { canDecide: boolean; canSeeAll: boolean }) {
  const t = useTranslations("regularisation");
  const a = useTranslations("attendance");
  const c = useTranslations("common");
  const [tab, setTab] = useState<"mine" | "all">(canDecide ? "mine" : "all");
  const [f, setF] = useState({ state: "", from: "", to: "" });
  const qs = new URLSearchParams();
  if (tab === "all") for (const [k, v] of Object.entries(f)) if (v) qs.set(k, v);
  const { data, problem, loading } = useBff<Page<Regularisation>>(`/attendance/regularisation-requests${tab === "mine" ? "/pending-approval" : ""}?${qs}`);
  const rows = data?.items ?? null;

  const set = (k: keyof typeof f) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => setF((o) => ({ ...o, [k]: e.target.value }));

  return (
    <div className="grid gap-4">
      <div role="tablist" className="flex gap-2">
        {canDecide ? <Button role="tab" aria-selected={tab === "mine"} variant={tab === "mine" ? "default" : "outline"} onClick={() => setTab("mine")}>{t("mine")}</Button> : null}
        {canSeeAll ? <Button role="tab" aria-selected={tab === "all"} variant={tab === "all" ? "default" : "outline"} onClick={() => setTab("all")}>{t("all")}</Button> : null}
      </div>
      {tab === "all" ? (
        <div className="grid gap-3 sm:grid-cols-3">
          <div className="grid gap-1">
            <Label htmlFor="state">{t("state")}</Label>
            <NativeSelect id="state" value={f.state} onChange={set("state")}>
              <option value="">{c("all")}</option>
              {STATES.map((s) => <option key={s} value={s}>{t(`rs.${s}`)}</option>)}
            </NativeSelect>
          </div>
          <div className="grid gap-1">
            <Label htmlFor="from">{t("from")}</Label>
            <Input id="from" type="date" value={f.from} onChange={set("from")} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="to">{t("to")}</Label>
            <Input id="to" type="date" value={f.to} onChange={set("to")} />
          </div>
        </div>
      ) : null}

      <ProblemAlert problem={problem} />
      {loading ? <Skeleton className="h-8" aria-busy="true" aria-label={c("loading")} /> : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{tab === "mine" ? t("emptyMine") : t("empty")}</p> : null}
      {rows && rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("date")}</TableHead>
              <TableHead>{t("employee")}</TableHead>
              <TableHead>{t("type")}</TableHead>
              <TableHead>{t("state")}</TableHead>
              <TableHead>{t("submitted")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((r) => (
              <TableRow key={r.id}>
                <TableCell>
                  <Link href={`/app/attendance/regularisations/${r.id}`} className="underline-offset-2 hover:underline">{r.attendance_date}</Link>
                </TableCell>
                <TableCell>{r.employee_name} <span className="font-mono text-xs text-muted-foreground">{r.employee_code}</span></TableCell>
                <TableCell>{a(`rt.${r.request_type}`)}</TableCell>
                <TableCell>
                  <Badge variant="secondary">{t(`rs.${r.state}`)}</Badge>
                  {r.escalated ? <Badge variant="outline" className="ml-1">{t("escalated")}</Badge> : null}
                </TableCell>
                <TableCell>{wallDateTime(r.submitted_at)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </div>
  );
}
