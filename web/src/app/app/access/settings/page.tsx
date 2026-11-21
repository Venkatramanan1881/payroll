"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { AccessSettings, EmployeeVisibility, Me } from "@/types/api";

const SWITCHES = ["delegation_enabled", "custom_roles_enabled", "admin_second_step"] as const;
const VISIBILITY: (keyof EmployeeVisibility)[] = ["own_punches", "day_status", "counters", "permission_balance", "deduction_reason"];

/**
 * Access settings AC-03/04/06/08 (IAM-06) and employee self-visibility AC-05 (IAM-07), read with ORG-07. Each change
 * is a dated settings version from today or later, with a reason and If-Match.
 */
export default function AccessSettingsPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const s = useBff<AccessSettings>("/access-settings");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState(false);
  const [key, setKey] = useState(() => crypto.randomUUID());
  if (!s.data) return s.problem ? <ProblemAlert problem={s.problem} /> : <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} />;
  const v = s.data;

  async function send(path: string, f: FormData, body: Record<string, unknown>) {
    setSaved(false);
    const r = await bff(path, sendJson("POST", { ...body, effective_from: String(f.get("effective_from")), reason: String(f.get("reason") ?? "").trim() }, {
      "idempotency-key": key,
      "if-match": s.etag ?? "",
    }));
    if (!r.ok) return setProblem(r.problem);
    setProblem(null);
    setSaved(true);
    setKey(crypto.randomUUID());
    s.reload();
  }
  const dateAndReason = (prefix: string) => (
    <>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-from`}>{t("effectiveFrom")}</Label>
        <Input id={`${prefix}-from`} name="effective_from" type="date" required />
      </div>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-reason`}>{t("reason")}</Label>
        <Input id={`${prefix}-reason`} name="reason" required maxLength={1000} />
      </div>
    </>
  );
  const onOff = (on: boolean) => (on ? t("on") : t("off"));
  return (
    <div className="grid max-w-3xl gap-6">
      <p className="text-sm text-muted-foreground">{t("inForceToday")}</p>
      <ProblemAlert problem={problem} />
      {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
      <form
        key={JSON.stringify([v.delegation_enabled, v.settings_change_mode, v.custom_roles_enabled, v.admin_second_step])}
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2"
        aria-label={t("accessSettings")}
        onSubmit={(e) => {
          e.preventDefault();
          const f = new FormData(e.currentTarget);
          const body: Record<string, unknown> = { settings_change_mode: String(f.get("settings_change_mode")) };
          for (const k of SWITCHES) body[k] = f.get(k) === "on";
          send("/access-settings/change", f, body);
        }}
      >
        <h2 className="text-lg font-semibold sm:col-span-2">{t("accessSettings")}</h2>
        {SWITCHES.map((k) => (
          <div key={k} className="grid gap-1">
            <Label htmlFor={`as-${k}`}>{t(`s.${k}`)}</Label>
            <NativeSelect id={`as-${k}`} name={k} defaultValue={v[k] ? "on" : "off"} disabled={!can(g, "IAM-06")}>
              <option value="on">{onOff(true)}</option>
              <option value="off">{onOff(false)}</option>
            </NativeSelect>
          </div>
        ))}
        <div className="grid gap-1">
          <Label htmlFor="as-mode">{t("s.settings_change_mode")}</Label>
          <NativeSelect id="as-mode" name="settings_change_mode" defaultValue={v.settings_change_mode} disabled={!can(g, "IAM-06")}>
            {(["hr_only", "hr_with_org_admin_approval"] as const).map((m) => <option key={m} value={m}>{t(`mode.${m}`)}</option>)}
          </NativeSelect>
        </div>
        {can(g, "IAM-06") ? (
          <>
            {dateAndReason("as")}
            <Button type="submit" className="justify-self-start" disabled={!s.etag}>{t("saveSettings")}</Button>
          </>
        ) : null}
      </form>
      <form
        key={JSON.stringify(v.employee_visibility)}
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2"
        aria-label={t("visibility")}
        onSubmit={(e) => {
          e.preventDefault();
          const f = new FormData(e.currentTarget);
          send("/access-settings/employee-visibility/change", f, { employee_visibility: Object.fromEntries(VISIBILITY.map((k) => [k, f.get(k) === "on"])) });
        }}
      >
        <h2 className="text-lg font-semibold sm:col-span-2">{t("visibility")}</h2>
        <fieldset className="grid gap-1 sm:col-span-2" disabled={!can(g, "IAM-07")}>
          <legend className="sr-only">{t("visibility")}</legend>
          {VISIBILITY.map((k) => (
            <label key={k} className="flex items-center gap-2 text-sm">
              <input type="checkbox" name={k} defaultChecked={v.employee_visibility[k]} />
              {t(`vis.${k}`)}
            </label>
          ))}
        </fieldset>
        {can(g, "IAM-07") ? (
          <>
            {dateAndReason("ev")}
            <Button type="submit" className="justify-self-start" disabled={!s.etag}>{t("saveVisibility")}</Button>
          </>
        ) : null}
      </form>
    </div>
  );
}
