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
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Option } from "@/server/lookups";
import type { AttendanceCycle, AttendancePeriod, Page, PeriodSettings } from "@/types/api";

/** Periods (PER-08), and for organisation-wide PER-01 holders the cycles and period settings (PL-01..PL-07). */
export function PeriodsView({ canSetUp, branches, orgId }: { canSetUp: boolean; branches: Option[]; orgId: string }) {
  const t = useTranslations("per");
  const c = useTranslations("common");
  const periods = useBff<Page<AttendancePeriod>>("/periods");
  const rows = [...(periods.data?.items ?? [])].sort((a, b) => b.period_start.localeCompare(a.period_start));
  return (
    <div className="grid gap-6">
      <ProblemAlert problem={periods.problem} />
      {periods.loading && !periods.data ? <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} /> : null}
      {periods.data ? (
        <Table aria-label={t("periods")}>
          <TableHeader>
            <TableRow>
              <TableHead>{t("from")}</TableHead>
              <TableHead>{t("to")}</TableHead>
              <TableHead>{t("cutoff")}</TableHead>
              <TableHead>{t("state")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((p) => (
              <TableRow key={p.id}>
                <TableCell><Link className="underline-offset-2 hover:underline" href={`/app/periods/${p.id}`}>{p.period_start}</Link></TableCell>
                <TableCell>{p.period_end}</TableCell>
                <TableCell>{p.cutoff_date ?? ""}</TableCell>
                <TableCell><Badge variant={p.state === "locked" ? "default" : "secondary"}>{t(`st.${p.state}`)}</Badge></TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {canSetUp ? <Cycles branches={branches} orgId={orgId} onChange={periods.reload} /> : null}
      {canSetUp ? <Settings /> : null}
    </div>
  );
}

function Cycles({ branches, orgId, onChange }: { branches: Option[]; orgId: string; onChange: () => void }) {
  const t = useTranslations("per");
  const list = useBff<Page<AttendanceCycle>>("/period/cycles");
  const [owner, setOwner] = useState<"organisation" | "branch">("organisation");
  const [start, setStart] = useState<"calendar" | "custom">("calendar");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const ownerName = (cy: AttendanceCycle) => (cy.owner_kind === "branch" ? (branches.find((b) => b.id === cy.owner_id)?.name ?? "") : t(`ok.${cy.owner_kind}`));
  return (
    <div className="grid gap-3">
      <h2 className="text-lg font-semibold">{t("cycles")}</h2>
      <ProblemAlert problem={list.problem ?? problem} />
      {list.data ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("owner")}</TableHead>
              <TableHead>{t("frequency")}</TableHead>
              <TableHead>{t("start")}</TableHead>
              <TableHead>{t("validFrom")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((cy) => (
              <TableRow key={cy.id}>
                <TableCell>{ownerName(cy)}</TableCell>
                <TableCell>{t(`fq.${cy.frequency}`)}</TableCell>
                <TableCell>{t(`sk.${cy.start_kind}`)}{cy.custom_start_day ? ` (${cy.custom_start_day})` : ""}</TableCell>
                <TableCell>
                  {cy.valid_from}{cy.valid_to ? ` – ${cy.valid_to}` : ""}
                  {cy.pending_approval ? <Badge variant="outline" className="ml-2">{t("pendingApproval")}</Badge> : null}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      <form
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-4"
        onSubmit={async (e) => {
          e.preventDefault();
          const form = e.currentTarget;
          const f = new FormData(form);
          const num = (k: string) => (f.get(k) ? Number(f.get(k)) : undefined);
          const org = owner === "organisation";
          const r = await bff("/period/cycles", sendJson("POST", {
            owner_kind: owner,
            owner_id: org ? orgId || undefined : String(f.get("owner_id")), // the API checks it is the caller's own organisation
            frequency: String(f.get("frequency")),
            start_kind: start,
            custom_start_day: start === "custom" ? num("custom_start_day") : undefined,
            cutoff_day: num("cutoff_day"),
            valid_from: String(f.get("valid_from")),
            reason: String(f.get("reason") ?? "").trim() || undefined,
          }, { "idempotency-key": key }));
          if (!r.ok) return setProblem(r.problem);
          setProblem(null);
          setKey(crypto.randomUUID());
          form.reset();
          list.reload();
          onChange();
        }}
      >
        <h3 className="font-medium sm:col-span-4">{t("newCycle")}</h3>
        <div className="grid gap-1">
          <Label htmlFor="cy-owner">{t("owner")}</Label>
          <NativeSelect id="cy-owner" value={owner} onChange={(e) => setOwner(e.target.value as typeof owner)}>
            <option value="organisation">{t("ok.organisation")}</option>
            <option value="branch">{t("ok.branch")}</option>
          </NativeSelect>
        </div>
        {owner === "branch" ? (
          <div className="grid gap-1">
            <Label htmlFor="cy-branch">{t("branch")}</Label>
            <NativeSelect id="cy-branch" name="owner_id" required defaultValue="">
              <option value="" />
              {branches.map((b) => <option key={b.id} value={b.id}>{b.name}</option>)}
            </NativeSelect>
          </div>
        ) : null}
        <div className="grid gap-1">
          <Label htmlFor="cy-freq">{t("frequency")}</Label>
          <NativeSelect id="cy-freq" name="frequency" defaultValue="monthly">
            {["monthly", "bi_weekly", "weekly", "daily"].map((x) => <option key={x} value={x}>{t(`fq.${x}`)}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="cy-start">{t("start")}</Label>
          <NativeSelect id="cy-start" value={start} onChange={(e) => setStart(e.target.value as typeof start)}>
            <option value="calendar">{t("sk.calendar")}</option>
            <option value="custom">{t("sk.custom")}</option>
          </NativeSelect>
        </div>
        {start === "custom" ? (
          <div className="grid gap-1">
            <Label htmlFor="cy-day">{t("customDay")}</Label>
            <Input id="cy-day" name="custom_start_day" type="number" min={1} max={31} required />
          </div>
        ) : null}
        <div className="grid gap-1">
          <Label htmlFor="cy-cut">{t("cutoffDay")}</Label>
          <Input id="cy-cut" name="cutoff_day" type="number" min={1} max={31} />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="cy-from">{t("validFrom")}</Label>
          <Input id="cy-from" name="valid_from" type="date" required />
        </div>
        <div className="grid gap-1 sm:col-span-2">
          <Label htmlFor="cy-reason">{t("reason")}</Label>
          <Input id="cy-reason" name="reason" maxLength={1000} />
        </div>
        <Button type="submit" className="self-end justify-self-start">{t("addCycle")}</Button>
      </form>
    </div>
  );
}

const SETTING_OPTIONS: Record<string, string[]> = {
  after_cutoff_days: ["assumed_present", "held_until_actuals"],
  lock_mode: ["manual", "automatic"],
  pending_at_lock: ["block", "allow_as_absent", "allow_as_present"],
  unlock: ["never", "with_org_admin_approval"],
};

function Settings() {
  const t = useTranslations("per");
  const s = useBff<PeriodSettings>("/period/settings");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState(false);
  if (!s.data) return <ProblemAlert problem={s.problem} />;
  const v = s.data;
  return (
    <form
      key={s.etag ?? ""}
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const body: Record<string, unknown> = { reason: String(f.get("reason") ?? "").trim() };
        for (const k of Object.keys(SETTING_OPTIONS)) body[k] = String(f.get(k));
        body.auto_lock_day = f.get("auto_lock_day") ? Number(f.get("auto_lock_day")) : null;
        body.adjustment_past_periods = Number(f.get("adjustment_past_periods"));
        setSaved(false);
        const r = await bff("/period/settings", sendJson("PATCH", body, { "if-match": s.etag ?? "" }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setSaved(true);
        s.reload();
      }}
    >
      <h2 className="text-lg font-semibold sm:col-span-3">{t("settings")}{v.pending_approval ? <Badge variant="outline" className="ml-2">{t("pendingApproval")}</Badge> : null}</h2>
      {Object.entries(SETTING_OPTIONS).map(([k, opts]) => (
        <div key={k} className="grid gap-1">
          <Label htmlFor={`ps-${k}`}>{t(`s.${k}`)}</Label>
          <NativeSelect id={`ps-${k}`} name={k} defaultValue={String(v[k as keyof PeriodSettings] ?? opts[0])}>
            {opts.map((o) => <option key={o} value={o}>{t(`sv.${o}`)}</option>)}
          </NativeSelect>
        </div>
      ))}
      <div className="grid gap-1">
        <Label htmlFor="ps-auto">{t("s.auto_lock_day")}</Label>
        <Input id="ps-auto" name="auto_lock_day" type="number" min={1} max={31} defaultValue={v.auto_lock_day ?? ""} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="ps-adj">{t("s.adjustment_past_periods")}</Label>
        <Input id="ps-adj" name="adjustment_past_periods" type="number" min={0} required defaultValue={v.adjustment_past_periods} />
      </div>
      <div className="grid gap-1 sm:col-span-2">
        <Label htmlFor="ps-reason">{t("reason")}</Label>
        <Input id="ps-reason" name="reason" required maxLength={1000} />
      </div>
      {/* Without a settings version there is no ETag to change against: the organisation's settings come first. */}
      <Button type="submit" className="self-end justify-self-start" disabled={!s.etag}>{t("saveSettings")}</Button>
      <div className="sm:col-span-3">
        <ProblemAlert problem={problem} />
        {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
      </div>
    </form>
  );
}
