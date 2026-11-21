"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { RequestTable } from "@/components/request-table";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { KINDS, type RequestKind } from "@/lib/requests";
import { useBff } from "@/lib/use-bff";
import type { CompOffCredit, OnDutyRequest, Page, PermissionRequest } from "@/types/api";

/** Per type: "Waiting for me" (awaiting_me, the API's decide check) and "All" (what the caller may read). */
export function RequestsBoard({ kinds }: { kinds: RequestKind[] }) {
  const t = useTranslations("req");
  const c = useTranslations("common");
  const [kind, setKind] = useState<RequestKind | undefined>(kinds[0]);
  const [tab, setTab] = useState<"mine" | "all">("mine");
  const { data, problem, loading } = useBff<Page<PermissionRequest | OnDutyRequest | CompOffCredit>>(
    kind ? `${KINDS[kind].list}${tab === "mine" ? "?awaiting_me=true" : ""}` : "",
  );
  if (!kind) return <p className="text-muted-foreground">{t("empty")}</p>;
  const rows = data?.items ?? null;
  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap gap-2" role="group" aria-label={t("type")}>
        {kinds.map((k) => (
          <Button key={k} aria-pressed={kind === k} variant={kind === k ? "default" : "outline"} onClick={() => setKind(k)}>{t(`kinds.${k}`)}</Button>
        ))}
      </div>
      <div className="flex gap-2" role="group" aria-label={t("state")}>
        {(["mine", "all"] as const).map((x) => (
          <Button key={x} size="sm" aria-pressed={tab === x} variant={tab === x ? "secondary" : "ghost"} onClick={() => setTab(x)}>{t(x)}</Button>
        ))}
      </div>
      <ProblemAlert problem={problem} />
      {loading && !rows ? <Skeleton className="h-8" aria-busy="true" aria-label={c("loading")} /> : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{tab === "mine" ? t("emptyMine") : t("empty")}</p> : null}
      {rows && rows.length > 0 ? <RequestTable kind={kind} rows={rows} base="/app/requests" /> : null}
    </div>
  );
}
