"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { LeaveRequest, LeaveType, Page } from "@/types/api";

const STATES = ["pending_l1", "pending_l2", "approved", "rejected", "cancelled"] as const;

/** "Waiting for me" (awaiting_me) and "All" leave requests; the API decides what each caller may see. */
export function LeaveList({ canDecide, canSeeAll, types }: { canDecide: boolean; canSeeAll: boolean; types: LeaveType[] }) {
  const t = useTranslations("leave");
  const r = useTranslations("regularisation");
  const c = useTranslations("common");
  const [tab, setTab] = useState<"mine" | "all">(canDecide ? "mine" : "all");
  const [f, setF] = useState({ state: "", leave_type_id: "" });
  const qs = new URLSearchParams();
  if (tab === "mine") qs.set("awaiting_me", "true");
  else for (const [k, v] of Object.entries(f)) if (v) qs.set(k, v);
  const { data, problem, loading } = useBff<Page<LeaveRequest>>(`/leave/requests?${qs}`);
  const rows = data?.items ?? null;
  const set = (k: keyof typeof f) => (e: React.ChangeEvent<HTMLSelectElement>) => setF((o) => ({ ...o, [k]: e.target.value }));

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
              {STATES.map((s) => <option key={s} value={s}>{r(`rs.${s}`)}</option>)}
            </NativeSelect>
          </div>
          <div className="grid gap-1">
            <Label htmlFor="type">{t("type")}</Label>
            <NativeSelect id="type" value={f.leave_type_id} onChange={set("leave_type_id")}>
              <option value="">{c("all")}</option>
              {types.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
            </NativeSelect>
          </div>
        </div>
      ) : null}

      <ProblemAlert problem={problem} />
      {loading && !rows ? <Skeleton className="h-8" aria-busy="true" aria-label={c("loading")} /> : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{tab === "mine" ? t("emptyMine") : t("empty")}</p> : null}
      {rows && rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("employee")}</TableHead>
              <TableHead>{t("type")}</TableHead>
              <TableHead>{t("dates")}</TableHead>
              <TableHead className="text-right">{t("days")}</TableHead>
              <TableHead>{t("state")}</TableHead>
              <TableHead>{t("submitted")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((q) => (
              <TableRow key={q.id}>
                <TableCell>
                  <Link href={`/app/leave/${q.id}`} className="underline-offset-2 hover:underline">{q.employee_name}</Link>{" "}
                  <span className="font-mono text-xs text-muted-foreground">{q.employee_code}</span>
                </TableCell>
                <TableCell>{q.leave_type_name}</TableCell>
                <TableCell>
                  {q.from_date}
                  {q.from_part !== "full" ? ` (${t(`part.${q.from_part}`)})` : ""}
                  {q.to_date !== q.from_date ? ` – ${q.to_date}${q.to_part !== "full" ? ` (${t(`part.${q.to_part}`)})` : ""}` : ""}
                </TableCell>
                <TableCell className="text-right">{q.total_days}</TableCell>
                <TableCell>
                  <Badge variant="secondary">{r(`rs.${q.state}`)}</Badge>
                  {q.escalated ? <Badge variant="outline" className="ml-1">{r("escalated")}</Badge> : null}
                </TableCell>
                <TableCell>{wallDateTime(q.submitted_at)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </div>
  );
}
