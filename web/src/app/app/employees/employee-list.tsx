"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import type { Option } from "@/server/lookups";
import type { EmployeeSummary, Page } from "@/types/api";

const STATUSES = ["on_probation", "active", "on_notice", "exited"] as const;

export function EmployeeList({ branches, departments }: { branches: Option[]; departments: Option[] }) {
  const t = useTranslations("employees");
  const c = useTranslations("common");
  const [filters, setFilters] = useState({ q: "", employment_status: "", branch_id: "", sort: "employee_code" });
  const [rows, setRows] = useState<EmployeeSummary[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [problem, setProblem] = useState<Problem | null>(null);
  const latest = useRef(0); // only the newest request may change the table
  const name = (list: Option[], id: string | null) => list.find((o) => o.id === id)?.name ?? c("none");

  const load = useCallback(
    async (after: string | null) => {
      setLoading(true);
      setProblem(null);
      const qs = new URLSearchParams({ limit: "50" });
      for (const [k, v] of Object.entries(filters)) if (v) qs.set(k, v);
      if (after) qs.set("cursor", after);
      const mine = ++latest.current;
      const r = await bff<Page<EmployeeSummary>>(`/employees?${qs}`);
      if (mine !== latest.current) return;
      setLoading(false);
      if (!r.ok) return setProblem(r.problem);
      setRows((old) => (after ? [...old, ...r.data.items] : r.data.items));
      setCursor(r.data.next_cursor);
    },
    [filters],
  );

  useEffect(() => {
    const id = setTimeout(() => load(null), 250); // let typing settle before searching
    return () => clearTimeout(id);
  }, [load]);

  const set = (k: keyof typeof filters) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) =>
    setFilters((f) => ({ ...f, [k]: e.target.value }));

  return (
    <div className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-4">
        <div className="grid gap-1">
          <Label htmlFor="q">{c("search")}</Label>
          <Input id="q" type="search" placeholder={t("searchPlaceholder")} value={filters.q} onChange={set("q")} maxLength={100} />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="status">{t("status")}</Label>
          <NativeSelect id="status" value={filters.employment_status} onChange={set("employment_status")}>
            <option value="">{c("all")}</option>
            {STATUSES.map((s) => (
              <option key={s} value={s}>{t(`st.${s}`)}</option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="branch">{t("branch")}</Label>
          <NativeSelect id="branch" value={filters.branch_id} onChange={set("branch_id")}>
            <option value="">{c("all")}</option>
            {branches.map((b) => (
              <option key={b.id} value={b.id}>{b.name}</option>
            ))}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="sort">{t("sort")}</Label>
          <NativeSelect id="sort" value={filters.sort} onChange={set("sort")}>
            <option value="employee_code">{t("sortCode")}</option>
            <option value="full_name">{t("sortName")}</option>
          </NativeSelect>
        </div>
      </div>

      <ProblemAlert problem={problem} />
      {problem ? (
        <Button variant="outline" className="justify-self-start" onClick={() => load(null)}>{c("retry")}</Button>
      ) : null}

      {!problem && !loading && rows.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {rows.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("code")}</TableHead>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("branch")}</TableHead>
              <TableHead>{t("department")}</TableHead>
              <TableHead>{t("status")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((e) => (
              <TableRow key={e.id}>
                <TableCell className="font-mono">{e.employee_code}</TableCell>
                <TableCell>
                  <Link href={`/app/employees/${e.id}`} className="underline-offset-2 hover:underline">{e.full_name}</Link>
                </TableCell>
                <TableCell>{name(branches, e.master_branch_id)}</TableCell>
                <TableCell>{name(departments, e.department_id)}</TableCell>
                <TableCell>
                  <Badge variant={e.employment_status === "exited" ? "outline" : "secondary"}>{t(`st.${e.employment_status}`)}</Badge>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {loading ? (
        <div className="grid gap-2" aria-busy="true" aria-label={c("loading")}>
          <Skeleton className="h-8" />
          <Skeleton className="h-8" />
          <Skeleton className="h-8" />
        </div>
      ) : null}
      {cursor && !loading ? (
        <Button variant="outline" className="justify-self-center" onClick={() => load(cursor)}>{c("loadMore")}</Button>
      ) : null}
    </div>
  );
}
