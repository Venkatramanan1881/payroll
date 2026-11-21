"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { useBff } from "@/lib/use-bff";
import type { AuditEntry, Page } from "@/types/api";
import { AuditTable } from "../audit-table";

/** Settings and policy changes with who, when, old, new and reason (ADT-02, G9). */
export default function SettingsHistoryPage() {
  const t = useTranslations("rpt");
  const c = useTranslations("common");
  const [query, setQuery] = useState("");
  const list = useBff<Page<AuditEntry>>(`/audit/settings-changes${query}`);
  return (
    <div className="grid gap-4">
      <form
        className="grid gap-3 rounded-lg border p-4 sm:grid-cols-4"
        aria-label={t("sub.settings")}
        onSubmit={(e) => {
          e.preventDefault();
          const f = new FormData(e.currentTarget);
          const q = new URLSearchParams();
          for (const k of ["from", "to", "setting_ref"]) {
            const v = String(f.get(k) ?? "").trim();
            if (v) q.set(k, v);
          }
          setQuery(q.size ? `?${q}` : "");
        }}
      >
        <div className="grid gap-1">
          <Label htmlFor="sh-from">{t("from")}</Label>
          <Input id="sh-from" name="from" type="date" />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="sh-to">{t("to")}</Label>
          <Input id="sh-to" name="to" type="date" />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="sh-ref">{t("setting")}</Label>
          <Input id="sh-ref" name="setting_ref" maxLength={16} />
        </div>
        <Button type="submit" className="self-end justify-self-start">{t("apply")}</Button>
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
