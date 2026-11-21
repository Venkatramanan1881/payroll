"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import type { RequestKind } from "@/lib/requests";
import type { CompOffCredit, OnDutyRequest, PermissionRequest } from "@/types/api";

type Row = PermissionRequest | OnDutyRequest | CompOffCredit;

/** One table for the three request types; `base` is where a row opens (`${base}/${kind}/${id}`). */
export function RequestTable({ kind, rows, base, showEmployee = true }: { kind: RequestKind; rows: Row[]; base: string; showEmployee?: boolean }) {
  const t = useTranslations("req");
  const r = useTranslations("regularisation");
  const state = (s: string) => (kind === "comp_off" ? t(`cs.${s}`) : r(`rs.${s}`));
  const cols: [string, (x: Row) => React.ReactNode][] =
    kind === "permission"
      ? [
          [t("type"), (x) => (x as PermissionRequest).permission_type_name],
          [t("date"), (x) => (x as PermissionRequest).permission_date],
          [t("time"), (x) => { const p = x as PermissionRequest; return `${p.start_time}–${p.end_time} (${t("minutes", { count: p.minutes })})`; }],
        ]
      : kind === "on_duty"
        ? [
            [t("kind"), (x) => t(`dk.${(x as OnDutyRequest).duration_kind}`)],
            [t("dates"), (x) => { const o = x as OnDutyRequest; return `${o.from_date}${o.to_date !== o.from_date ? ` – ${o.to_date}` : ""}${o.from_time ? ` ${o.from_time}–${o.to_time ?? ""}` : ""}`; }],
            [t("place"), (x) => (x as OnDutyRequest).place],
          ]
        : [
            [t("worked"), (x) => (x as CompOffCredit).worked_date],
            [t("fraction"), (x) => (x as CompOffCredit).fraction],
            [t("expires"), (x) => (x as CompOffCredit).expires_on ?? ""],
          ];
  return (
    <Table>
      <TableHeader>
        <TableRow>
          {showEmployee ? <TableHead>{t("employee")}</TableHead> : null}
          {cols.map(([h]) => <TableHead key={h}>{h}</TableHead>)}
          <TableHead>{t("state")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((x) => (
          <TableRow key={x.id}>
            {showEmployee ? (
              <TableCell>
                <Link href={`${base}/${kind}/${x.id}`} className="underline-offset-2 hover:underline">{x.employee_name}</Link>{" "}
                <span className="font-mono text-xs text-muted-foreground">{x.employee_code}</span>
              </TableCell>
            ) : null}
            {cols.map(([h, cell], i) => (
              <TableCell key={h}>
                {!showEmployee && i === 0 ? <Link href={`${base}/${kind}/${x.id}`} className="underline-offset-2 hover:underline">{cell(x)}</Link> : cell(x)}
              </TableCell>
            ))}
            <TableCell>
              <Badge variant="secondary">{state(x.state)}</Badge>
              {x.escalated ? <Badge variant="outline" className="ml-1">{r("escalated")}</Badge> : null}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
