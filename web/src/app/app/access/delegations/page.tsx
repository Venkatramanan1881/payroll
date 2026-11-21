"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { wallDateTime, zonedToUtc } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Delegation, Me, Page, UserAccount } from "@/types/api";
import { userLabel } from "../access-fields";

/** AC-03 delegations (IAM-09): an approver's approvals go to a delegate for a window; refused while AC-03 is off. */
export default function DelegationsPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const list = useBff<Page<Delegation>>("/delegations");
  const users = useBff<Page<UserAccount>>(can(g, "IAM-01") ? "/users?active=true" : "");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const who = (id: string) => {
    const u = users.data?.items.find((x) => x.id === id);
    return u ? userLabel(u) : id;
  };
  // The time the states are judged at: taken on load and after each change of ours.
  const [now, setNow] = useState(() => Date.now());
  const state = (d: Delegation) => (Date.parse(d.valid_to) <= now ? "ended" : Date.parse(d.valid_from) > now ? "upcoming" : "current");
  return (
    <div className="grid max-w-4xl gap-4">
      <ProblemAlert problem={list.problem ?? problem} />
      {list.loading && !list.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {list.data && list.data.items.length === 0 ? <p className="text-muted-foreground">{t("noDelegations")}</p> : null}
      {list.data && list.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("approver")}</TableHead>
              <TableHead>{t("delegate")}</TableHead>
              <TableHead>{t("from")}</TableHead>
              <TableHead>{t("to")}</TableHead>
              <TableHead>{t("state")}</TableHead>
              <TableHead />
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((d) => (
              <TableRow key={d.id}>
                <TableCell>{who(d.approver_user_id)}</TableCell>
                <TableCell>{who(d.delegate_user_id)}</TableCell>
                <TableCell>{wallDateTime(d.valid_from)}</TableCell>
                <TableCell>{wallDateTime(d.valid_to)}</TableCell>
                <TableCell><Badge variant={state(d) === "current" ? "default" : "secondary"}>{t(state(d))}</Badge></TableCell>
                <TableCell className="text-right">
                  {state(d) !== "ended" && can(g, "IAM-09") ? (
                    <Button
                      size="sm"
                      variant="outline"
                      aria-label={`${t("end")}: ${who(d.approver_user_id)} → ${who(d.delegate_user_id)}`}
                      onClick={async () => {
                        const r = await bff(`/delegations/${d.id}/end`, { method: "POST", headers: { "idempotency-key": crypto.randomUUID() } });
                        if (!r.ok) return setProblem(r.problem);
                        setProblem(null);
                        setNow(Date.now());
                        list.reload();
                      }}
                    >
                      {t("end")}
                    </Button>
                  ) : null}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {can(g, "IAM-09") && users.data ? (
        <form
          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2"
          aria-label={t("newDelegation")}
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const f = new FormData(form);
            // shortcut: India time for every organisation, as wallDateTime shows it; use the organisation's zone when one outside India signs up.
            const instant = (k: string) => {
              const [date, time] = String(f.get(k)).split("T");
              return zonedToUtc(date, time, "Asia/Kolkata");
            };
            const r = await bff("/delegations", sendJson("POST", {
              approver_user_id: String(f.get("approver")),
              delegate_user_id: String(f.get("delegate")),
              valid_from: instant("from"),
              valid_to: instant("to"),
            }, { "idempotency-key": key }));
            if (!r.ok) return setProblem(r.problem);
            setProblem(null);
            setKey(crypto.randomUUID());
            form.reset();
            setNow(Date.now());
            list.reload();
          }}
        >
          <h2 className="text-lg font-semibold sm:col-span-2">{t("newDelegation")}</h2>
          {(["approver", "delegate"] as const).map((k) => (
            <div key={k} className="grid gap-1">
              <Label htmlFor={`nd-${k}`}>{t(k)}</Label>
              <NativeSelect id={`nd-${k}`} name={k} required defaultValue="">
                <option value="" />
                {(users.data?.items ?? []).map((u) => <option key={u.id} value={u.id}>{userLabel(u)}</option>)}
              </NativeSelect>
            </div>
          ))}
          <div className="grid gap-1">
            <Label htmlFor="nd-from">{t("fromIst")}</Label>
            <Input id="nd-from" name="from" type="datetime-local" required />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="nd-to">{t("toIst")}</Label>
            <Input id="nd-to" name="to" type="datetime-local" required />
          </div>
          <Button type="submit" className="justify-self-start">{t("addDelegation")}</Button>
        </form>
      ) : null}
    </div>
  );
}
