"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import { Decide } from "@/components/decide";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import { can } from "@/lib/menu";
import type { EmployeeSummary, LeaveType, Me, ObBatch, ObEntry, Page, PermissionType } from "@/types/api";

const KINDS = ["leave", "comp_off", "permission_used", "late_counter", "early_counter"] as const;

/**
 * One opening-balance batch (OB-R1..R5): entries while a draft (add, change, remove, each with If-Match), submit,
 * confirm or reject; a confirmed entry changes by reversal with a reason (OBL-03). Every rule is checked by the API.
 */
export default function BatchPage() {
  const { id } = useParams<{ id: string }>();
  const t = useTranslations("ob");
  const batch = useBff<ObBatch>(`/opening-balances/batches/${id}`);
  const entries = useBff<Page<ObEntry>>(`/opening-balances/batches/${id}/entries`);
  const leaveTypes = useBff<Page<LeaveType>>("/leave/types");
  const permTypes = useBff<Page<PermissionType>>("/permissions/types");
  const me = useBff<Me>("/me");
  const grants = me.data?.grants ?? [];
  // batch_command: only an approve-type OBL-02 holder confirms or rejects a submitted batch.
  const approver = can(grants, "OBL-02", ["approve"]);
  const enter = can(grants, "OBL-01");
  const changeConfirmed = can(grants, "OBL-03");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const b = batch.data;
  const draft = b?.state === "draft";
  const reload = () => {
    batch.reload();
    entries.reload();
  };
  const typeName = (e: ObEntry) =>
    e.leave_type_id ? leaveTypes.data?.items.find((x) => x.id === e.leave_type_id)?.name : e.permission_type_id ? permTypes.data?.items.find((x) => x.id === e.permission_type_id)?.name : "";

  async function command(path: string, init: RequestInit) {
    setProblem(null);
    const r = await bff(path, init);
    if (!r.ok) {
      setProblem(r.problem);
      return false;
    }
    setKey(crypto.randomUUID());
    reload();
    return true;
  }

  return (
    <section className="grid max-w-5xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/opening-balances" className="text-sm underline">{t("title")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t("batch")}</h1>
        {b ? <Badge variant="secondary">{t(`bs.${b.state}`)}</Badge> : null}
      </div>
      <ProblemAlert problem={batch.problem ?? entries.problem ?? problem} />
      {!batch.data && !batch.problem ? <p className="text-sm text-muted-foreground" aria-busy="true">…</p> : null}
      {entries.data && entries.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noEntries")}</p> : null}
      {entries.data && entries.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("employee")}</TableHead>
              <TableHead>{t("kind")}</TableHead>
              <TableHead>{t("type")}</TableHead>
              <TableHead>{t("value")} / {t("asOf")}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {entries.data.items.map((e) => (
              <TableRow key={e.id}>
                <TableCell>{e.employee_name} <span className="font-mono text-xs text-muted-foreground">{e.employee_code}</span></TableCell>
                <TableCell>{t(`bk.${e.balance_kind}`)}</TableCell>
                <TableCell>{typeName(e)}</TableCell>
                <TableCell>
                  {(draft && enter) || (b?.state === "confirmed" && changeConfirmed) ? (
                    <form
                      className="flex flex-wrap items-center gap-2"
                      onSubmit={(ev) => {
                        ev.preventDefault();
                        const f = new FormData(ev.currentTarget);
                        const value = String(f.get("value"));
                        const as_of_date = String(f.get("as_of_date"));
                        if (draft) command(`/opening-balances/batches/${id}/entries/${e.id}`, sendJson("PATCH", { value, as_of_date }, { "if-match": `W/"${e.updated_at}"` }));
                        else command(`/opening-balances/entries/${e.id}/change`, sendJson("POST", { value, as_of_date, reason: String(f.get("reason") ?? "").trim() }, { "idempotency-key": key }));
                      }}
                    >
                      <Input aria-label={t("value")} name="value" defaultValue={e.value} key={`${e.id}-${e.updated_at}`} pattern="-?[0-9]{1,5}\.[0-9]{2}" required className="w-24" />
                      <Input aria-label={t("asOf")} name="as_of_date" type="date" defaultValue={e.as_of_date} required className="w-40" />
                      {!draft ? <Input aria-label={t("reason")} name="reason" placeholder={t("reason")} required maxLength={1000} className="w-40" /> : null}
                      <Button type="submit" size="sm" variant="outline" aria-label={`${draft ? t("edit") : t("change")}: ${e.employee_name ?? ""}`}>{draft ? t("edit") : t("change")}</Button>
                    </form>
                  ) : (
                    `${e.value} · ${e.as_of_date}`
                  )}
                </TableCell>
                <TableCell>
                  {draft && enter ? (
                    <Button
                      size="sm"
                      variant="ghost"
                      aria-label={`${t("remove")}: ${e.employee_name ?? ""}`}
                      onClick={() => window.confirm(t("confirmRemove")) && command(`/opening-balances/batches/${id}/entries/${e.id}`, { method: "DELETE", headers: { "if-match": `W/"${e.updated_at}"` } })}
                    >
                      {t("remove")}
                    </Button>
                  ) : null}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {b?.state === "confirmed" ? <p className="text-xs text-muted-foreground">{t("changeHelp")}</p> : null}

      {draft && b && enter ? <AddEntry batch={b} leaveTypes={leaveTypes.data?.items ?? []} permTypes={permTypes.data?.items ?? []} onAdd={(body) => command(`/opening-balances/batches/${id}/entries`, sendJson("POST", body, { "if-match": `W/"${b.updated_at}"`, "idempotency-key": key }))} /> : null}

      <div className="flex flex-wrap gap-3">
        {/* OB-01 decides which applies (HR confirms directly, or submits for the Organisation Admin); the API refuses the other. */}
        {draft && enter ? <Button onClick={() => command(`/opening-balances/batches/${id}/confirm`, sendJson("POST", {}, { "idempotency-key": key }))}>{t("confirm")}</Button> : null}
        {draft && enter ? <Button variant="outline" onClick={() => command(`/opening-balances/batches/${id}/submit`, sendJson("POST", {}, { "idempotency-key": key }))}>{t("submit")}</Button> : null}
      </div>
      {b?.state === "pending_approval" && approver ? <Decide base={`/opening-balances/batches/${id}`} yes="confirm" onDone={reload} /> : null}
    </section>
  );
}

function AddEntry({ batch, leaveTypes, permTypes, onAdd }: { batch: ObBatch; leaveTypes: LeaveType[]; permTypes: PermissionType[]; onAdd: (body: unknown) => Promise<boolean> }) {
  const t = useTranslations("ob");
  const [kind, setKind] = useState<(typeof KINDS)[number]>("leave");
  const [q, setQ] = useState("");
  const people = useBff<Page<EmployeeSummary>>(`/employees?limit=20${q.trim().length >= 2 ? `&q=${encodeURIComponent(q.trim())}` : ""}`);
  return (
    <form
      key={batch.updated_at}
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        await onAdd({
          entries: [{
            employee_id: String(f.get("employee_id")),
            balance_kind: kind,
            leave_type_id: kind === "leave" ? String(f.get("type_id")) : undefined,
            permission_type_id: kind === "permission_used" ? String(f.get("type_id")) : undefined,
            value: String(f.get("value")),
            as_of_date: String(f.get("as_of_date")),
          }],
        });
      }}
    >
      <h2 className="font-medium sm:col-span-3">{t("add")}</h2>
      <div className="grid gap-1">
        <Label htmlFor="ob-q">{t("employee")}</Label>
        <Input id="ob-q" type="search" value={q} onChange={(e) => setQ(e.target.value)} maxLength={100} />
        <NativeSelect aria-label={t("pick")} name="employee_id" required defaultValue="">
          <option value="">{t("pick")}</option>
          {(people.data?.items ?? []).map((p) => <option key={p.id} value={p.id}>{p.full_name} ({p.employee_code})</option>)}
        </NativeSelect>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="ob-kind">{t("kind")}</Label>
        <NativeSelect id="ob-kind" value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
          {KINDS.map((k) => <option key={k} value={k}>{t(`bk.${k}`)}</option>)}
        </NativeSelect>
      </div>
      {kind === "leave" || kind === "permission_used" ? (
        <div className="grid gap-1">
          <Label htmlFor="ob-type">{t("type")}</Label>
          <NativeSelect id="ob-type" name="type_id" required defaultValue="">
            <option value="" />
            {(kind === "leave" ? leaveTypes : permTypes).filter((x) => !x.retired_at).map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
          </NativeSelect>
        </div>
      ) : null}
      <div className="grid gap-1">
        <Label htmlFor="ob-value">{t("value")}</Label>
        <Input id="ob-value" name="value" required pattern="-?[0-9]{1,5}\.[0-9]{2}" aria-describedby="ob-value-help" />
        <p id="ob-value-help" className="text-xs text-muted-foreground">{t("addHelp")}</p>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="ob-asof">{t("asOf")}</Label>
        <Input id="ob-asof" name="as_of_date" type="date" required />
      </div>
      <Button type="submit" className="self-end justify-self-start">{t("add")}</Button>
    </form>
  );
}
