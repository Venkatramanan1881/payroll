"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { dateIn } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Holiday, HolidayCalendar, Page } from "@/types/api";

const KINDS = ["national", "festival", "optional"] as const;

/** One holiday calendar's year: add a holiday, change one (If-Match, with a reason), remove one; rename or retire. */
export default function CalendarPage() {
  const { id } = useParams<{ id: string }>();
  const t = useTranslations("sched");
  const [year, setYear] = useState(() => Number(dateIn("Asia/Kolkata").slice(0, 4)));
  const cals = useBff<Page<HolidayCalendar>>("/holiday-calendars");
  const cal = cals.data?.items.find((x) => x.id === id) ?? null;
  const list = useBff<Page<Holiday>>(`/holiday-calendars/${id}/holidays?from=${year}-01-01&to=${year}-12-31`);
  const [err, setErr] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const [editing, setEditing] = useState<string | null>(null);

  const call = async (path: string, init: RequestInit) => {
    setErr(null);
    const r = await bff(path, init);
    if (!r.ok) {
      setErr(r.problem);
      return false;
    }
    list.reload();
    cals.reload();
    return true;
  };

  return (
    <section className="grid max-w-4xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/holidays" className="text-sm underline">{t("holidays")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{cal?.name ?? "…"}</h1>
      </div>
      {cal ? (
        <form
          className="flex flex-wrap items-end gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            const name = String(new FormData(e.currentTarget).get("name") ?? "").trim();
            await call(`/holiday-calendars/${id}`, sendJson("PATCH", { name }, { "if-match": `W/"${cal.updated_at}"` }));
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="c-name">{t("name")}</Label>
            <Input id="c-name" name="name" defaultValue={cal.name} key={cal.updated_at} required maxLength={100} />
          </div>
          <Button type="submit" variant="outline">{t("rename")}</Button>
          {!cal.retired_at ? (
            <Button type="button" variant="ghost" onClick={() => window.confirm(t("confirmRetire")) && call(`/holiday-calendars/${id}`, sendJson("PATCH", { retired: true }, { "if-match": `W/"${cal.updated_at}"` }))}>{t("retire")}</Button>
          ) : null}
          <span className="text-xs text-muted-foreground">{cal.branch_ids.length ? `${t("branches")}: ${cal.branch_ids.length}` : t("noBranches")}</span>
        </form>
      ) : null}

      <div className="flex items-center gap-2">
        <Button variant="outline" size="sm" onClick={() => setYear(year - 1)} aria-label={`${t("year")} ${year - 1}`}>‹</Button>
        <span className="font-medium" aria-live="polite">{year}</span>
        <Button variant="outline" size="sm" onClick={() => setYear(year + 1)} aria-label={`${t("year")} ${year + 1}`}>›</Button>
      </div>
      <ProblemAlert problem={list.problem ?? err} />
      {list.data && list.data.items.length === 0 ? <p className="text-sm text-muted-foreground">{t("noHolidays")}</p> : null}
      {list.data && list.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("date")}</TableHead>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("kind")}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((h) =>
              editing === h.id ? (
                <TableRow key={h.id}>
                  <TableCell colSpan={4}>
                    <form
                      className="flex flex-wrap items-end gap-2"
                      onSubmit={async (e) => {
                        e.preventDefault();
                        const f = new FormData(e.currentTarget);
                        const ok = await call(`/holidays/${h.id}`, sendJson("PATCH", {
                          holiday_date: String(f.get("holiday_date")), name: String(f.get("name")).trim(), kind: String(f.get("kind")), reason: String(f.get("reason")).trim(),
                        }, { "if-match": `W/"${h.updated_at}"` }));
                        if (ok) setEditing(null);
                      }}
                    >
                      <Input aria-label={t("date")} name="holiday_date" type="date" defaultValue={h.holiday_date} required className="w-40" />
                      <Input aria-label={t("name")} name="name" defaultValue={h.name} required maxLength={100} className="w-48" />
                      <NativeSelect aria-label={t("kind")} name="kind" defaultValue={h.kind} className="w-36">
                        {KINDS.map((k) => <option key={k} value={k}>{t(`hk.${k}`)}</option>)}
                      </NativeSelect>
                      <Input aria-label={t("reason")} name="reason" placeholder={t("reason")} required maxLength={1000} className="w-48" />
                      <Button type="submit" size="sm">{t("change")}</Button>
                      <Button type="button" size="sm" variant="ghost" onClick={() => setEditing(null)}>✕</Button>
                    </form>
                  </TableCell>
                </TableRow>
              ) : (
                <TableRow key={h.id}>
                  <TableCell>{h.holiday_date}</TableCell>
                  <TableCell>{h.name}</TableCell>
                  <TableCell>{t(`hk.${h.kind}`)}</TableCell>
                  <TableCell className="flex gap-2">
                    <Button size="sm" variant="outline" onClick={() => setEditing(h.id)}>{t("change")}</Button>
                    <Button size="sm" variant="ghost" onClick={() => window.confirm(t("confirmRemove")) && call(`/holidays/${h.id}/remove`, sendJson("POST", {}, { "idempotency-key": crypto.randomUUID() }))}>{t("remove")}</Button>
                  </TableCell>
                </TableRow>
              ),
            )}
          </TableBody>
        </Table>
      ) : null}

      <form
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-4"
        onSubmit={async (e) => {
          e.preventDefault();
          const form = e.currentTarget;
          const f = new FormData(form);
          const ok = await call(`/holiday-calendars/${id}/holidays`, sendJson("POST", {
            holiday_date: String(f.get("holiday_date")), name: String(f.get("name")).trim(), kind: String(f.get("kind")), reason: String(f.get("reason")).trim(),
          }, { "idempotency-key": key }));
          if (ok) {
            setKey(crypto.randomUUID());
            form.reset();
          }
        }}
      >
        <h2 className="font-medium sm:col-span-4">{t("addHoliday")}</h2>
        <div className="grid gap-1">
          <Label htmlFor="h-date">{t("date")}</Label>
          <Input id="h-date" name="holiday_date" type="date" required />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="h-name">{t("name")}</Label>
          <Input id="h-name" name="name" required maxLength={100} />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="h-kind">{t("kind")}</Label>
          <NativeSelect id="h-kind" name="kind" defaultValue="national">
            {KINDS.map((k) => <option key={k} value={k}>{t(`hk.${k}`)}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="h-reason">{t("reason")}</Label>
          <Input id="h-reason" name="reason" required maxLength={1000} />
        </div>
        <Button type="submit" className="justify-self-start">{t("addHoliday")}</Button>
      </form>
    </section>
  );
}
