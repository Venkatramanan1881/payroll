"use client";
import { useTranslations } from "next-intl";
import { Fragment, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { wallDateTime } from "@/lib/time";
import type { AuditEntry } from "@/types/api";

/** Audit entries, newest first, each with its before and after on request (read-only: the trail is insert-only). */
export function AuditTable({ items }: { items: AuditEntry[] }) {
  const t = useTranslations("rpt");
  const [open, setOpen] = useState<string | null>(null);
  if (items.length === 0) return <p className="text-muted-foreground">{t("noEntries")}</p>;
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{t("when")}</TableHead>
          <TableHead>{t("actor")}</TableHead>
          <TableHead>{t("action")}</TableHead>
          <TableHead>{t("target")}</TableHead>
          <TableHead>{t("reason")}</TableHead>
          <TableHead />
        </TableRow>
      </TableHeader>
      <TableBody>
        {items.map((a) => (
          <Fragment key={a.id}>
            <TableRow>
              <TableCell className="whitespace-nowrap">{wallDateTime(a.occurred_at)}</TableCell>
              <TableCell>
                {t(`ak.${a.actor_kind}`)}
                {a.actor_id ? <code className="block text-xs text-muted-foreground">{a.actor_id}</code> : null}
                {a.support_grant_id ? <Badge variant="outline" className="ml-2">{t("vendorAccess")}</Badge> : null}
              </TableCell>
              <TableCell>{a.action}</TableCell>
              <TableCell>
                {a.target_type}
                {a.target_id ? <code className="block text-xs text-muted-foreground">{a.target_id}</code> : null}
              </TableCell>
              <TableCell>{a.reason ?? ""}</TableCell>
              <TableCell className="text-right">
                <Button
                  size="sm"
                  variant="ghost"
                  aria-expanded={open === a.id}
                  aria-label={`${t("details")}: ${a.action}, ${wallDateTime(a.occurred_at)}`}
                  onClick={() => setOpen(open === a.id ? null : a.id)}
                >
                  {t("details")}
                </Button>
              </TableCell>
            </TableRow>
            {open === a.id ? (
              <TableRow>
                <TableCell colSpan={6}>
                  <div className="grid gap-2 md:grid-cols-2">
                    {(["before", "after"] as const).map((k) => (
                      <div key={k} className="grid gap-1">
                        <p className="text-sm font-medium">{t(k)}</p>
                        <pre className="max-h-64 overflow-auto rounded bg-muted p-2 text-xs">{JSON.stringify(k === "before" ? a.before_value : a.after_value, null, 2) ?? "null"}</pre>
                      </div>
                    ))}
                  </div>
                </TableCell>
              </TableRow>
            ) : null}
          </Fragment>
        ))}
      </TableBody>
    </Table>
  );
}
