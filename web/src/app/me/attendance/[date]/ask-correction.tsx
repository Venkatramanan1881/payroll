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
const nextDay = (date: string) => {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + 1);
  return d.toISOString().slice(0, 10);
};

/** ATT-02: the employee asks for a correction. Window, limits and approval routing are the API's (RG-01..09). */
export function AskCorrection({ date, tz, punches }: { date: string; tz: string; punches: Punch[] }) {
  const t = useTranslations("attendance");
  const m = useTranslations("me");
  const c = useTranslations("common");
  const router = useRouter();
  const [type, setType] = useState<(typeof TYPES)[number]>("missed_punch");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [sent, setSent] = useState(false);
  const [key, setKey] = useState(() => crypto.randomUUID());

  return (
    <Card>
      <CardHeader><CardTitle><h2>{m("ask")}</h2></CardTitle></CardHeader>
      <CardContent>
        <form
          className="grid gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const f = new FormData(form);
            const at = (k: string, late?: string) => (f.get(k) ? zonedToUtc(late && f.get(late) ? nextDay(date) : date, String(f.get(k)), tz) : undefined);
            setBusy(true);
            setProblem(null);
            setSent(false);
            const r = await bff("/attendance/regularisation-requests", sendJson("POST", {
              attendance_date: date,
              request_type: type,
              proposed_in_at: at("in"),
              proposed_out_at: at("out", "outNext"),
              target_raw_punch_id: type === "wrong_punch" ? String(f.get("target") || "") || undefined : undefined,
              reason: String(f.get("reason") ?? "").trim(),
            }, { "idempotency-key": key }));
            setBusy(false);
            if (!r.ok) return setProblem(r.problem);
            form.reset(); // a second press must not send the same request again
            setKey(crypto.randomUUID());
            setSent(true);
            router.refresh();
          }}
        >
          <p className="text-sm text-muted-foreground">{m("askHelp")}</p>
          <Label htmlFor="a-type">{t("requestType")}</Label>
          <NativeSelect id="a-type" value={type} onChange={(e) => setType(e.target.value as typeof type)}>
            {/* A wrong punch is named from the punches the employee can see; with none visible it cannot be asked for here. */}
            {TYPES.filter((x) => x !== "wrong_punch" || punches.length > 0).map((x) => <option key={x} value={x}>{t(`rt.${x}`)}</option>)}
          </NativeSelect>
          {type === "wrong_punch" ? (
            <>
              <Label htmlFor="a-target">{t("targetPunch")}</Label>
              <NativeSelect id="a-target" name="target" required>
                <option value="" />
                {punches.map((x) => <option key={x.id} value={x.id}>{wallTime(x.event_at, tz)} ({x.source})</option>)}
              </NativeSelect>
            </>
          ) : null}
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1">
              <Label htmlFor="a-in">{t("inAt")}</Label>
              <Input id="a-in" name="in" type="time" />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="a-out">{t("outAt")}</Label>
              <Input id="a-out" name="out" type="time" />
              <label className="flex items-center gap-2 text-xs"><input type="checkbox" name="outNext" /> {t("nextDay")}</label>
            </div>
          </div>
          <Label htmlFor="a-reason">{t("reason")}</Label>
          <Input id="a-reason" name="reason" required maxLength={1000} />
          <ProblemAlert problem={problem} />
          {sent ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{m("sent")}</p> : null}
          <Button type="submit" disabled={busy} className="justify-self-start">{busy ? c("saving") : m("ask")}</Button>
        </form>
      </CardContent>
    </Card>
  );
}
