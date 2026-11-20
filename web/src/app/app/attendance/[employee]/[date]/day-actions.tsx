"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { wallTime, zonedToUtc } from "@/lib/time";
import type { Punch } from "@/types/api";

const TYPES = ["missed_punch", "late_arrival", "early_checkout", "wrong_punch", "whole_day"] as const;

/** The day after `date` (YYYY-MM-DD): out times of a shift that ends after midnight belong to it. */
const nextDay = (date: string) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + 1);
  return d.toISOString().slice(0, 10);
};

type Props = { employeeId: string; branchId: string | null; date: string; tz: string; punches: Punch[]; canCorrect: boolean; canManual: boolean };

/** HR correction (ATT-06) and manual attendance (TCP-04). The server records them and works the day out again. */
export function DayActions(p: Props) {
  if (!p.canCorrect && !p.canManual) return null;
  return (
    <div className="grid gap-4 md:grid-cols-2">
      {p.canCorrect ? <Correction {...p} /> : null}
      {p.canManual && p.branchId ? <Manual {...p} /> : null}
    </div>
  );
}

/** Shared submit with one Idempotency-Key per form instance; a new key after success. */
function useSubmit(done: string) {
  const router = useRouter();
  const [key, setKey] = useState(() => crypto.randomUUID());
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [ok, setOk] = useState(false);
  async function submit(form: HTMLFormElement, path: string, body: unknown) {
    setBusy(true);
    setProblem(null);
    setOk(false);
    const r = await bff(path, sendJson("POST", body, { "idempotency-key": key }));
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    setKey(crypto.randomUUID());
    form.reset(); // a second press must not record the same entry again under the new key
    setOk(true);
    router.refresh();
  }
  const status = (
    <>
      <ProblemAlert problem={problem} />
      {ok ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{done}</p> : null}
    </>
  );
  return { submit, busy, status };
}

function Correction({ employeeId, date, tz, punches }: Props) {
  const t = useTranslations("attendance");
  const c = useTranslations("common");
  const { submit, busy, status } = useSubmit(t("recorded"));
  const [type, setType] = useState<(typeof TYPES)[number]>("missed_punch");
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("correct")}</h2></CardTitle></CardHeader>
      <CardContent>
        <form
          className="grid gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            const at = (k: string, late?: string) => (f.get(k) ? zonedToUtc(late && f.get(late) ? nextDay(date) : date, String(f.get(k)), tz) : undefined);
            submit(e.currentTarget, "/attendance/hr-corrections", {
              employee_id: employeeId,
              attendance_date: date,
              request_type: type,
              proposed_in_at: at("in"),
              proposed_out_at: at("out", "outNext"),
              target_raw_punch_id: type === "wrong_punch" ? String(f.get("target") || "") || undefined : undefined,
              reason: String(f.get("reason") ?? "").trim(),
            });
          }}
        >
          <p className="text-sm text-muted-foreground">{t("correctHelp")}</p>
          <Label htmlFor="c-type">{t("requestType")}</Label>
          <NativeSelect id="c-type" value={type} onChange={(e) => setType(e.target.value as typeof type)}>
            {TYPES.map((x) => <option key={x} value={x}>{t(`rt.${x}`)}</option>)}
          </NativeSelect>
          {type === "wrong_punch" ? (
            <>
              <Label htmlFor="c-target">{t("targetPunch")}</Label>
              <NativeSelect id="c-target" name="target" required>
                <option value="" />
                {punches.map((x) => <option key={x.id} value={x.id}>{wallTime(x.event_at, tz)} ({x.source})</option>)}
              </NativeSelect>
            </>
          ) : null}
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1">
              <Label htmlFor="c-in">{t("inAt")}</Label>
              <Input id="c-in" name="in" type="time" />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="c-out">{t("outAt")}</Label>
              <Input id="c-out" name="out" type="time" />
              <label className="flex items-center gap-2 text-xs"><input type="checkbox" name="outNext" /> {t("nextDay")}</label>
            </div>
          </div>
          <Label htmlFor="c-reason">{t("reason")}</Label>
          <Input id="c-reason" name="reason" required maxLength={1000} />
          {status}
          <Button type="submit" disabled={busy} className="justify-self-start">{busy ? c("saving") : t("record")}</Button>
        </form>
      </CardContent>
    </Card>
  );
}

function Manual({ employeeId, branchId, date, tz }: Props) {
  const t = useTranslations("attendance");
  const c = useTranslations("common");
  const { submit, busy, status } = useSubmit(t("recorded"));
  return (
    <Card>
      <CardHeader><CardTitle><h2>{t("manualTitle")}</h2></CardTitle></CardHeader>
      <CardContent>
        <form
          className="grid gap-3"
          onSubmit={(e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            const entries = [["p1", ""], ["p2", "p2Next"]]
              .filter(([k]) => f.get(k))
              .map(([k, late]) => ({ employee_id: employeeId, event_at: zonedToUtc(late && f.get(late) ? nextDay(date) : date, String(f.get(k)), tz) }));
            submit(e.currentTarget, "/manual-attendance", { branch_id: branchId, reason: String(f.get("reason") ?? "").trim(), entries });
          }}
        >
          <p className="text-sm text-muted-foreground">{t("manualHelp")}</p>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1">
              <Label htmlFor="m-1">{t("punchIn")}</Label>
              <Input id="m-1" name="p1" type="time" required />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="m-2">{t("punchOut")}</Label>
              <Input id="m-2" name="p2" type="time" />
              <label className="flex items-center gap-2 text-xs"><input type="checkbox" name="p2Next" /> {t("nextDay")}</label>
            </div>
          </div>
          <Label htmlFor="m-reason">{t("reason")}</Label>
          <Input id="m-reason" name="reason" required maxLength={1000} />
          {status}
          <Button type="submit" disabled={busy} className="justify-self-start">{busy ? c("saving") : t("record")}</Button>
        </form>
      </CardContent>
    </Card>
  );
}
