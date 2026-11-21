"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { RequestTable } from "@/components/request-table";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { CompOffBalance, CompOffCredit, OnDutyRequest, Page, PermissionRequest, PermissionType, PermissionUsage } from "@/types/api";

/** A form that posts once per key; cleared and re-keyed after success so a second press cannot repeat it. */
function useOnce(done: () => void) {
  const [key, setKey] = useState(() => crypto.randomUUID());
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [ok, setOk] = useState(false);
  async function send(form: HTMLFormElement, path: string, body: unknown) {
    setBusy(true);
    setProblem(null);
    setOk(false);
    const r = await bff(path, sendJson("POST", body, { "idempotency-key": key }));
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    form.reset();
    setKey(crypto.randomUUID());
    setOk(true);
    done();
  }
  return { send, busy, problem, ok };
}

/** PRM-02 apply, PRM-13 usage, and the employee's own permissions (the API checks limits, windows and overlaps). */
export function MyPermissions({ employeeId }: { employeeId: string }) {
  const t = useTranslations("req");
  const types = useBff<Page<PermissionType>>("/permissions/types");
  const usage = useBff<{ items: PermissionUsage[] }>(`/permissions/employees/${employeeId}/usage`);
  const mine = useBff<Page<PermissionRequest>>(`/permissions/requests?employee_id=${employeeId}`); // own only, even for HR who are also employees
  const f = useOnce(() => { mine.reload(); usage.reload(); });
  const name = (id: string) => types.data?.items.find((x) => x.id === id)?.name ?? "";
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("kinds.permission")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-4">
        {usage.data && usage.data.items.length > 0 ? (
          <Table aria-label={t("usage")}>
            <TableHeader>
              <TableRow>
                <TableHead>{t("type")}</TableHead>
                <TableHead>{t("dates")}</TableHead>
                <TableHead>{t("used")}</TableHead>
                <TableHead>{t("left")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {usage.data.items.map((u) => (
                <TableRow key={u.permission_type_id}>
                  <TableCell>{name(u.permission_type_id)}</TableCell>
                  <TableCell>{u.period_from} – {u.period_to}</TableCell>
                  <TableCell>{t("times", { count: u.used_count })} · {t("minutes", { count: u.used_minutes })}</TableCell>
                  <TableCell>{u.remaining_count != null ? t("times", { count: u.remaining_count }) : ""}{u.remaining_minutes != null ? ` ${t("minutes", { count: u.remaining_minutes })}` : ""}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
        <ProblemAlert problem={usage.problem ?? types.problem} />
        <form
          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-4"
          onSubmit={(e) => {
            e.preventDefault();
            const d = new FormData(e.currentTarget);
            f.send(e.currentTarget, "/permissions/requests", {
              permission_type_id: String(d.get("permission_type_id")),
              permission_date: String(d.get("permission_date")),
              start_time: String(d.get("start_time")),
              end_time: String(d.get("end_time")),
              reason: String(d.get("reason") ?? "").trim(),
            });
          }}
        >
          <h3 className="font-medium sm:col-span-4">{t("applyPermission")}</h3>
          <div className="grid gap-1">
            <Label htmlFor="pm-type">{t("type")}</Label>
            <NativeSelect id="pm-type" name="permission_type_id" required defaultValue="">
              <option value="" />
              {(types.data?.items ?? []).filter((x) => !x.retired_at).map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
            </NativeSelect>
          </div>
          <div className="grid gap-1">
            <Label htmlFor="pm-date">{t("date")}</Label>
            <Input id="pm-date" name="permission_date" type="date" required />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="pm-from">{t("fromTime")}</Label>
            <Input id="pm-from" name="start_time" type="time" required />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="pm-to">{t("toTime")}</Label>
            <Input id="pm-to" name="end_time" type="time" required />
          </div>
          <div className="grid gap-1 sm:col-span-3">
            <Label htmlFor="pm-reason">{t("reason")}</Label>
            <Input id="pm-reason" name="reason" required maxLength={1000} />
          </div>
          <Button type="submit" disabled={f.busy} className="self-end">{t("submit")}</Button>
          <div className="sm:col-span-4">
            <ProblemAlert problem={f.problem} />
            {f.ok ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("sent")}</p> : null}
          </div>
        </form>
        <h3 className="font-medium">{t("myPermissions")}</h3>
        <ProblemAlert problem={mine.problem} />
        {mine.data && mine.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("empty")}</p> : null}
        {mine.data && mine.data.items.length > 0 ? <RequestTable kind="permission" rows={mine.data.items} base="/me/requests" showEmployee={false} /> : null}
      </CardContent>
    </Card>
  );
}

const DURATIONS = ["hours", "half_day", "full_day", "several_days"] as const;

/** PRM-08 apply and the employee's own on-duty requests (kinds allowed, windows and clashes are the API's: OD-01..05). */
export function MyOnDuty({ employeeId }: { employeeId: string }) {
  const t = useTranslations("req");
  const mine = useBff<Page<OnDutyRequest>>(`/on-duty/requests?employee_id=${employeeId}`);
  const f = useOnce(() => mine.reload());
  const [kind, setKind] = useState<(typeof DURATIONS)[number]>("full_day");
  const withTimes = kind === "hours"; // the API takes times exactly for "hours" (OD-02)
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("kinds.on_duty")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-4">
        <form
          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-4"
          onSubmit={(e) => {
            e.preventDefault();
            const d = new FormData(e.currentTarget);
            const opt = (k: string) => String(d.get(k) ?? "") || undefined;
            f.send(e.currentTarget, "/on-duty/requests", {
              duration_kind: kind,
              from_date: String(d.get("from_date")),
              to_date: kind === "several_days" ? String(d.get("to_date")) : String(d.get("from_date")),
              from_time: withTimes ? opt("from_time") : undefined,
              to_time: withTimes ? opt("to_time") : undefined,
              purpose: String(d.get("purpose") ?? "").trim(),
              place: String(d.get("place") ?? "").trim(),
            });
          }}
        >
          <h3 className="font-medium sm:col-span-4">{t("applyOnDuty")}</h3>
          <div className="grid gap-1">
            <Label htmlFor="od-kind">{t("kind")}</Label>
            <NativeSelect id="od-kind" value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
              {DURATIONS.map((x) => <option key={x} value={x}>{t(`dk.${x}`)}</option>)}
            </NativeSelect>
          </div>
          <div className="grid gap-1">
            <Label htmlFor="od-from">{t("from")}</Label>
            <Input id="od-from" name="from_date" type="date" required />
          </div>
          {kind === "several_days" ? (
            <div className="grid gap-1">
              <Label htmlFor="od-to">{t("to")}</Label>
              <Input id="od-to" name="to_date" type="date" required />
            </div>
          ) : null}
          {withTimes ? (
            <>
              <div className="grid gap-1">
                <Label htmlFor="od-ft">{t("fromTime")}</Label>
                <Input id="od-ft" name="from_time" type="time" required />
              </div>
              <div className="grid gap-1">
                <Label htmlFor="od-tt">{t("toTime")}</Label>
                <Input id="od-tt" name="to_time" type="time" required />
              </div>
            </>
          ) : null}
          <div className="grid gap-1 sm:col-span-2">
            <Label htmlFor="od-purpose">{t("purpose")}</Label>
            <Input id="od-purpose" name="purpose" required maxLength={1000} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="od-place">{t("place")}</Label>
            <Input id="od-place" name="place" required maxLength={200} />
          </div>
          <Button type="submit" disabled={f.busy} className="self-end">{t("submit")}</Button>
          <div className="sm:col-span-4">
            <ProblemAlert problem={f.problem} />
            {f.ok ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("sent")}</p> : null}
          </div>
        </form>
        <h3 className="font-medium">{t("myOnDuty")}</h3>
        <ProblemAlert problem={mine.problem} />
        {mine.data && mine.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("empty")}</p> : null}
        {mine.data && mine.data.items.length > 0 ? <RequestTable kind="on_duty" rows={mine.data.items} base="/me/requests" showEmployee={false} /> : null}
      </CardContent>
    </Card>
  );
}

/**
 * The employee's comp-off balance and credits.
 * shortcut: no "pay or comp-off" choice form while the API refuses every choice (D-28, HW-01); add it when that opens.
 */
export function MyCompOff({ employeeId }: { employeeId: string }) {
  const t = useTranslations("req");
  const balance = useBff<CompOffBalance>(`/comp-off/employees/${employeeId}/balance`);
  const credits = useBff<Page<CompOffCredit>>(`/comp-off/credits?employee_id=${employeeId}`);
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("compOff")}</h2></CardTitle></CardHeader>
      <CardContent className="grid gap-4">
        <ProblemAlert problem={balance.problem ?? credits.problem} />
        {balance.data ? (
          <p className="text-sm">{t("balance")}: <strong>{balance.data.balance}</strong> · {t("pendingDays")}: {balance.data.pending_approval_days}</p>
        ) : null}
        {credits.data && credits.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("empty")}</p> : null}
        {credits.data && credits.data.items.length > 0 ? <RequestTable kind="comp_off" rows={credits.data.items} base="/me/requests" showEmployee={false} /> : null}
      </CardContent>
    </Card>
  );
}
