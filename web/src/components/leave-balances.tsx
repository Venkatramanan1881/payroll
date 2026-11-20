"use client";
import { useTranslations } from "next-intl";
import { ProblemAlert } from "@/components/problem-alert";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useBff } from "@/lib/use-bff";
import type { LeaveBalance } from "@/types/api";

/** An employee's balances exactly as the API reports them (LVE-08); the browser adds nothing up. */
export function LeaveBalances({ employeeId }: { employeeId: string }) {
  const t = useTranslations("leave");
  const c = useTranslations("common");
  const { data, problem, loading } = useBff<{ balances: LeaveBalance[] }>(`/leave/employees/${employeeId}/balances`);
  if (loading && !data) return <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} />;
  if (problem) return <ProblemAlert problem={problem} />;
  const rows = data?.balances ?? [];
  if (rows.length === 0) return <p className="text-sm text-muted-foreground">{t("noBalances")}</p>;
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{t("type")}</TableHead>
          <TableHead className="text-right">{t("balance")}</TableHead>
          <TableHead className="text-right">{t("pending")}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((b) => (
          <TableRow key={b.leave_type_id}>
            <TableCell>{b.leave_type_name}</TableCell>
            <TableCell className="text-right">{b.balance}</TableCell>
            <TableCell className="text-right">{b.pending_days}</TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
