"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { LeaveBalances } from "@/components/leave-balances";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { EmployeeSummary, LeaveRequest, LeaveType, Page } from "@/types/api";

const PARTS = ["full", "first_half", "second_half"] as const;

/** LVE-03: a leave request for an employee in scope. Balance, notice, overlap and the rest are the API's checks. */
export function ApplyForm({ types }: { types: LeaveType[] }) {
  const t = useTranslations("leave");
  const c = useTranslations("common");
  const router = useRouter();
  const [q, setQ] = useState("");
  const [term, setTerm] = useState(""); // the search sent, a moment after typing stops
  const [employee, setEmployee] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID()); // one request per form open, however often Submit is pressed
  useEffect(() => {
    const id = setTimeout(() => setTerm(q.trim()), 300);
    return () => clearTimeout(id);
  }, [q]);
  const search = term.length >= 2 ? `/employees?limit=20&q=${encodeURIComponent(term)}` : "/employees?limit=20";
  const found = useBff<Page<EmployeeSummary>>(search);
  // Only a choice that is still listed counts: a later search can drop the person picked earlier.
  const chosen = found.data?.items.some((x) => x.id === employee) ? employee : "";

  async function submit(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const f = new FormData(e.currentTarget);
    const body = {
      employee_id: chosen,
      leave_type_id: String(f.get("leave_type_id")),
      from_date: String(f.get("from_date")),
      to_date: String(f.get("to_date")),
      from_part: String(f.get("from_part")),
      to_part: String(f.get("to_part")),
      reason: String(f.get("reason") ?? "").trim() || undefined,
    };
    setBusy(true);
    setProblem(null);
    const r = await bff<LeaveRequest>("/leave/requests/on-behalf", sendJson("POST", body, { "idempotency-key": key }));
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    router.push(`/app/leave/${r.data.id}`);
  }

  return (
    <form onSubmit={submit} className="grid gap-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="grid gap-1">
          <Label htmlFor="q">{t("search")}</Label>
          <Input id="q" type="search" value={q} onChange={(e) => {
              setQ(e.target.value);
              setEmployee(""); // a new search means choosing again; never submit for someone no longer shown
            }} maxLength={100} aria-describedby="q-help" />
          <p id="q-help" className="text-xs text-muted-foreground">{t("searchHelp")}</p>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="employee">{t("employee")}</Label>
          <NativeSelect id="employee" value={chosen} onChange={(e) => setEmployee(e.target.value)} required>
            <option value="">{t("pick")}</option>
            {(found.data?.items ?? []).map((x) => <option key={x.id} value={x.id}>{x.full_name} ({x.employee_code})</option>)}
          </NativeSelect>
        </div>
        <div className="sm:col-span-2">
          <ProblemAlert problem={found.problem} />
        </div>
      </div>
      {chosen ? (
        <div className="grid gap-1">
          <h2 className="font-medium">{t("balances")}</h2>
          <LeaveBalances employeeId={chosen} />
        </div>
      ) : null}
      <div className="grid gap-3 sm:grid-cols-3">
        <div className="grid gap-1">
          <Label htmlFor="leave_type_id">{t("type")}</Label>
          <NativeSelect id="leave_type_id" name="leave_type_id" required defaultValue="">
            <option value="" />
            {types.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
          </NativeSelect>
        </div>
        <div className="grid gap-1">
          <Label htmlFor="from_date">{t("from")}</Label>
          <Input id="from_date" name="from_date" type="date" required />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="from_part">{t("fromPart")}</Label>
          <NativeSelect id="from_part" name="from_part" defaultValue="full">
            {PARTS.map((p) => <option key={p} value={p}>{t(`part.${p}`)}</option>)}
          </NativeSelect>
        </div>
        <div />
        <div className="grid gap-1">
          <Label htmlFor="to_date">{t("to")}</Label>
          <Input id="to_date" name="to_date" type="date" required />
        </div>
        <div className="grid gap-1">
          <Label htmlFor="to_part">{t("toPart")}</Label>
          <NativeSelect id="to_part" name="to_part" defaultValue="full">
            {PARTS.map((p) => <option key={p} value={p}>{t(`part.${p}`)}</option>)}
          </NativeSelect>
        </div>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="reason">{t("reason")}</Label>
        <Textarea id="reason" name="reason" maxLength={1000} />
      </div>
      <ProblemAlert problem={problem} />
      <div className="flex gap-3">
        <Button type="submit" disabled={busy || !chosen}>{busy ? c("saving") : t("submit")}</Button>
        <Button type="button" variant="outline" onClick={() => router.back()}>{c("cancel")}</Button>
      </div>
    </form>
  );
}
