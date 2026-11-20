"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { Decide } from "@/components/decide";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { useBff } from "@/lib/use-bff";
import { wallDateTime } from "@/lib/time";
import type { Page, PunchFlag } from "@/types/api";

const STATES = ["raised", "accepted", "rejected"] as const;

/** Punch flags to review (TCP-06): accept with a comment or reject with a reason; the day is worked out again server-side. */
export default function FlagsPage() {
  const t = useTranslations("flags");
  const c = useTranslations("common");
  const [state, setState] = useState<(typeof STATES)[number]>("raised");
  const { data, problem, loading, reload } = useBff<Page<PunchFlag>>(`/punch-flags?state=${state}`);
  const rows = data?.items ?? null;

  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <div className="grid max-w-xs gap-1">
        <Label htmlFor="state">{t("state")}</Label>
        <NativeSelect id="state" value={state} onChange={(e) => setState(e.target.value as typeof state)}>
          {STATES.map((s) => <option key={s} value={s}>{t(`fs.${s}`)}</option>)}
        </NativeSelect>
      </div>
      <ProblemAlert problem={problem} />
      {loading && !rows ? <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} /> : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {rows?.map((f) => {
        // The API names the day that uses the punch (overnight punches belong to the shift's day); before that, the punch's own date.
        const date = f.attendance_date ?? new Intl.DateTimeFormat("en-CA", { timeZone: f.punch.captured_tz }).format(new Date(f.punch.event_at));
        return (
          <Card key={f.id}>
            <CardContent className="grid gap-3 pt-6 md:grid-cols-[1fr_20rem]">
              <div className="grid gap-1 text-sm">
                <div className="flex flex-wrap items-center gap-2">
                  <Link className="font-medium underline-offset-2 hover:underline" href={`/app/attendance/${f.employee_id}/${date}`}>{f.employee_name}</Link>
                  <span className="font-mono text-xs text-muted-foreground">{f.employee_code}</span>
                  <Badge variant="outline">{t(`fr.${f.reason}`)}</Badge>
                </div>
                <div>{t("punchAt")}: {wallDateTime(f.punch.event_at, f.punch.captured_tz)} · {f.punch.source}</div>
                <div className="font-mono text-xs text-muted-foreground">{t("evidence")}: {JSON.stringify(f.evidence)}</div>
                {f.decision ? <div>{t("decided", { decision: t(`fs.${f.decision.decision}`), at: wallDateTime(f.decision.decided_at, f.punch.captured_tz) })}{f.decision.reason ? ` — ${f.decision.reason}` : ""}</div> : null}
              </div>
              {f.state === "raised" ? <Decide base={`/punch-flags/${f.id}`} yes="accept" onDone={reload} /> : null}
            </CardContent>
          </Card>
        );
      })}
    </section>
  );
}
