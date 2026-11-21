"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { HolidayCalendar, Page } from "@/types/api";

/** Holiday calendars (SCH-05). A branch points at its calendar from the branch settings. */
export default function HolidaysPage() {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const { data, problem, loading, reload } = useBff<Page<HolidayCalendar>>("/holiday-calendars");
  const [err, setErr] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  return (
    <section className="grid max-w-3xl gap-4">
      <h1 className="text-xl font-semibold">{t("holidays")}</h1>
      <ProblemAlert problem={problem ?? err} />
      {loading && !data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      <ul className="grid gap-2">
        {data?.items.map((cal) => (
          <li key={cal.id} className="flex items-center gap-3 rounded-lg border p-3">
            <Link className="flex-1 font-medium underline-offset-2 hover:underline" href={`/app/holidays/${cal.id}`}>{cal.name}</Link>
            {cal.retired_at ? <Badge variant="outline">{t("retired")}</Badge> : null}
            <span className="text-xs text-muted-foreground">{t("branches")}: {cal.branch_ids.length}</span>
          </li>
        ))}
      </ul>
      <form
        className="flex flex-wrap items-end gap-3"
        onSubmit={async (e) => {
          e.preventDefault();
          const form = e.currentTarget;
          const r = await bff("/holiday-calendars", sendJson("POST", { name: String(new FormData(form).get("name") ?? "").trim() }, { "idempotency-key": key }));
          if (!r.ok) return setErr(r.problem);
          setErr(null);
          setKey(crypto.randomUUID());
          form.reset();
          reload();
        }}
      >
        <div className="grid gap-1">
          <Label htmlFor="cal-name">{t("newCalendar")}</Label>
          <Input id="cal-name" name="name" required maxLength={100} />
        </div>
        <Button type="submit">{t("addCalendar")}</Button>
      </form>
    </section>
  );
}
