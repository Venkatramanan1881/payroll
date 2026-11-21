import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { RequestActions } from "@/components/request-actions";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { KINDS, type RequestKind } from "@/lib/requests";
import { wallDateTime } from "@/lib/time";
import { api } from "@/server/api";
import type { CompOffCredit, OnDutyRequest, PermissionRequest } from "@/types/api";

/** One permission, on-duty request or comp-off credit; actions shown are the ones the API says the caller may take. */
export async function RequestDetail({ kind, id, back }: { kind: RequestKind; id: string; back: string }) {
  const t = await getTranslations("req");
  const r = await getTranslations("regularisation");
  const res = await api<PermissionRequest & OnDutyRequest & CompOffCredit>(`/v1${KINDS[kind].one(encodeURIComponent(id))}`);
  if (!res.ok) return <PageProblem problem={res.problem} />;
  const x = res.data;
  const row = (label: string, value: React.ReactNode) => (
    <div className="grid grid-cols-[10rem_1fr] gap-2 py-0.5">
      <dt className="text-muted-foreground">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
  const fields =
    kind === "permission"
      ? [row(t("type"), x.permission_type_name), row(t("date"), x.permission_date), row(t("time"), `${x.start_time}–${x.end_time} (${t("minutes", { count: x.minutes })})`), row(t("origin"), t(`po.${x.origin}`)), row(t("reason"), x.reason)]
      : kind === "on_duty"
        ? [row(t("kind"), t(`dk.${x.duration_kind}`)), row(t("dates"), `${x.from_date}${x.to_date !== x.from_date ? ` – ${x.to_date}` : ""}`), x.from_time ? row(t("time"), `${x.from_time}–${x.to_time ?? ""}`) : null, row(t("place"), x.place), row(t("purpose"), x.purpose)]
        : [row(t("worked"), x.worked_date), row(t("fraction"), x.fraction), row(t("basis"), t(`cb.${x.earned_basis}`)), row(t("expires"), x.expires_on ?? "—"), row(t("remaining"), x.remaining)];
  return (
    <section className="grid max-w-3xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href={back} className="text-sm underline">{t("title")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t(`one.${kind}`)}: {x.employee_name}</h1>
        <Badge variant="secondary">{kind === "comp_off" ? t(`cs.${x.state}`) : r(`rs.${x.state}`)}</Badge>
      </div>
      <Card>
        <CardContent className="pt-6">
          <dl className="text-sm">
            {row(t("employee"), `${x.employee_name} (${x.employee_code})`)}
            {fields}
          </dl>
        </CardContent>
      </Card>
      {x.steps.length > 0 ? (
        <Card>
          <CardHeader><CardTitle><h2>{t("steps")}</h2></CardTitle></CardHeader>
          <CardContent>
            <ol className="grid gap-1 text-sm">
              {x.steps.map((s, i) => (
                <li key={i}>
                  {wallDateTime(s.acted_at)} · {r("level", { level: s.level })} · {r(`act.${s.action}`)}
                  {s.comment ? <span className="text-muted-foreground"> — {s.comment}</span> : null}
                </li>
              ))}
            </ol>
          </CardContent>
        </Card>
      ) : null}
      <RequestActions path={KINDS[kind].one(x.id)} decide={x.can_decide === true} cancel={x.can_cancel === true} />
    </section>
  );
}
