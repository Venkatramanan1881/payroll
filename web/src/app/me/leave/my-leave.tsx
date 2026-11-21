"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useBff } from "@/lib/use-bff";
import type { LeaveRequest, Page } from "@/types/api";

/** The employee's own leave requests, newest first; each opens its detail page (cancel lives there). */
export function MyLeave({ employeeId }: { employeeId: string }) {
  const t = useTranslations("leave");
  const m = useTranslations("me");
  const r = useTranslations("regularisation");
  const c = useTranslations("common");
  const { data, problem, loading } = useBff<Page<LeaveRequest>>(`/leave/requests?employee_id=${employeeId}`);
  const rows = data?.items ?? null;
  if (loading && !rows) return <Skeleton className="h-8" aria-busy="true" aria-label={c("loading")} />;
  if (problem) return <ProblemAlert problem={problem} />;
  if (!rows || rows.length === 0) return <p className="text-muted-foreground">{m("noLeave")}</p>;
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{t("type")}</TableHead>
          <TableHead>{t("dates")}</TableHead>
          <TableHead className="text-right">{t("days")}</TableHead>
          <TableHead>{t("state")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((q) => (
          <TableRow key={q.id}>
            <TableCell>
              <Link href={`/me/leave/${q.id}`} className="underline-offset-2 hover:underline">{q.leave_type_name}</Link>
            </TableCell>
            <TableCell>
              {q.from_date}
              {q.from_part !== "full" ? ` (${t(`part.${q.from_part}`)})` : ""}
              {q.to_date !== q.from_date ? ` – ${q.to_date}${q.to_part !== "full" ? ` (${t(`part.${q.to_part}`)})` : ""}` : ""}
            </TableCell>
            <TableCell className="text-right">{q.total_days}</TableCell>
            <TableCell><Badge variant="secondary">{r(`rs.${q.state}`)}</Badge></TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
