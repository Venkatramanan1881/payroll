"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { useBff } from "@/lib/use-bff";
import type { AuditEntry, Page } from "@/types/api";
import { AuditTable } from "../audit-table";

const KINDS = ["user", "vendor_user", "system_job", "device"] as const;

/** The audit trail (ADT-01): filters are sent to the API, which applies them after the caller's scope. */
export default function AuditPage() {
  const t = useTranslations("rpt");
  const c = useTranslations("common");
  const [query, setQuery] = useState("");
  const list = useBff<Page<AuditEntry>>(`/audit/entries${query}`);
  return (
    <div className="grid gap-4">
      <form
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
        aria-label={t("sub.audit")}
        onSubmit={(e) => {
          e.preventDefault();
          const f = new FormData(e.currentTarget);
          const q = new URLSearchParams();
          for (const k of ["from", "to", "action", "target_type", "actor_kind"]) {
            const v = String(f.get(k) ?? "").trim();
            if (v) q.set(k, v);
          }
          if (f.get("support_access_only")) q.set("support_access_only", "true");
          setQuery(q.size ? `?${q}` : "");
        }}
      >
        <div className="grid gap-1">
          <Label htmlFor="au-from">{t("from")}</Label>
          <Input id="au-from" name="from" type="date" />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="au-to">{t("to")}</Label>
          <Input id="au-to" name="to" type="date" />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="au-kind">{t("actorKind")}</Label>
          <NativeSelect id="au-kind" name="actor_kind" defaultValue="">
            <option value="">{t("any")}</option>
            {KINDS.map((k) => <option key={k} value={k}>{t(`ak.${k}`)}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="au-action">{t("action")}</Label>
          <Input id="au-action" name="action" maxLength={64} placeholder="leave.approved" />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="au-target">{t("targetType")}</Label>
          <Input id="au-target" name="target_type" maxLength={64} placeholder="employee" />
        </div>
        <label className="flex items-center gap-2 self-end text-sm">
          <input type="checkbox" name="support_access_only" />
          {t("supportOnly")}
        </label>
        <Button type="submit" className="justify-self-start">{t("apply")}</Button>
      </form>
      <ProblemAlert problem={list.problem} />
      {list.loading && !list.data ? <Skeleton className="h-32" aria-busy="true" aria-label={c("loading")} /> : null}
      {list.data ? (
        <>
          <p className="text-sm text-muted-foreground">{t("newest")}</p>
          <AuditTable items={list.data.items} />
        </>
      ) : null}
    </div>
  );
}
