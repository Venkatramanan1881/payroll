"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Page, WeeklyOffPattern } from "@/types/api";

/** Weekly-off patterns (SCH-04 to change): fixed days, an alternating day on chosen weeks, or from the roster. */
export function Patterns({ canChange }: { canChange: boolean }) {
  const t = useTranslations("sched");
  const days = t.raw("weekday") as string[]; // ISO 1 = Monday
  const { data, problem, reload } = useBff<Page<WeeklyOffPattern>>("/weekly-off-patterns");
  const [kind, setKind] = useState<WeeklyOffPattern["kind"]>("fixed_days");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const describe = (p: WeeklyOffPattern) =>
    p.kind === "fixed_days"
      ? (p.weekly_days ?? []).map((d) => days[d - 1]).join(", ")
      : p.kind === "alternating"
        ? `${p.alt_day ? days[p.alt_day - 1] : ""} (${(p.alt_weeks ?? []).join(", ")})`
        : t("pk.from_roster");

  return (
    <div className="grid gap-3">
      <h2 className="mt-4 text-lg font-semibold">{t("patterns")}</h2>
      <ProblemAlert problem={problem ?? err} />
      {data ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("kind")}</TableHead>
              <TableHead>{t("days")}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.items.map((p) => (
              <TableRow key={p.id}>
                <TableCell>{p.name}</TableCell>
                <TableCell>{t(`pk.${p.kind}`)}</TableCell>
                <TableCell>{describe(p)}</TableCell>
                <TableCell>
                  {canChange && !p.retired_at ? (
                    <Button size="sm" variant="outline" disabled={busy} onClick={async () => {
                      if (!window.confirm(t("confirmRetire"))) return;
                      setBusy(true);
                      const r = await bff(`/weekly-off-patterns/${p.id}/retire`, sendJson("POST", {}, { "idempotency-key": crypto.randomUUID() }));
                      setBusy(false);
                      if (r.ok) reload();
                      else setErr(r.problem);
                    }}>{t("retire")}</Button>
                  ) : null}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {canChange ? (
        <form
          className="grid max-w-2xl gap-3 rounded-lg border p-4"
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const f = new FormData(form);
            const nums = (k: string) => f.getAll(k).map((x) => Number(x));
            const body = {
              name: String(f.get("name") ?? "").trim(),
              kind,
              ...(kind === "fixed_days" ? { weekly_days: nums("weekly_days") } : {}),
              ...(kind === "alternating" ? { alt_day: Number(f.get("alt_day")), alt_weeks: nums("alt_weeks") } : {}),
            };
            setBusy(true);
            const r = await bff("/weekly-off-patterns", sendJson("POST", body, { "idempotency-key": key }));
            setBusy(false);
            if (!r.ok) return setErr(r.problem);
            setKey(crypto.randomUUID());
            form.reset();
            reload();
          }}
        >
          <h3 className="font-medium">{t("newPattern")}</h3>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="grid gap-1">
              <Label htmlFor="p-name">{t("name")}</Label>
              <Input id="p-name" name="name" required maxLength={100} />
            </div>
            <div className="grid gap-1">
              <Label htmlFor="p-kind">{t("kind")}</Label>
              <NativeSelect id="p-kind" value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
                {(["fixed_days", "alternating", "from_roster"] as const).map((k) => <option key={k} value={k}>{t(`pk.${k}`)}</option>)}
              </NativeSelect>
            </div>
          </div>
          {kind === "fixed_days" ? (
            <fieldset className="flex flex-wrap gap-3 text-sm">
              <legend className="mb-1 text-sm">{t("days")}</legend>
              {days.map((d, i) => (
                <label key={d} className="flex items-center gap-1"><input type="checkbox" name="weekly_days" value={i + 1} /> {d}</label>
              ))}
            </fieldset>
          ) : null}
          {kind === "alternating" ? (
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="grid gap-1">
                <Label htmlFor="p-alt">{t("altDay")}</Label>
                <NativeSelect id="p-alt" name="alt_day" defaultValue="6">
                  {days.map((d, i) => <option key={d} value={i + 1}>{d}</option>)}
                </NativeSelect>
              </div>
              <fieldset className="flex flex-wrap items-end gap-3 text-sm">
                <legend className="mb-1 text-sm">{t("altWeeks")}</legend>
                {[1, 2, 3, 4, 5].map((w) => (
                  <label key={w} className="flex items-center gap-1"><input type="checkbox" name="alt_weeks" value={w} /> {w}</label>
                ))}
              </fieldset>
            </div>
          ) : null}
          <Button type="submit" className="justify-self-start" disabled={busy}>{t("addPattern")}</Button>
        </form>
      ) : null}
    </div>
  );
}
