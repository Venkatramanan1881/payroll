"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Branch, Me, Page } from "@/types/api";

/** Branches in scope (ORG-07), and a new branch for ORG-01 (the API also requires organisation-wide scope). */
export default function BranchesPage() {
  const t = useTranslations("org");
  const c = useTranslations("common");
  const list = useBff<Page<Branch>>("/branches");
  const me = useBff<Me>("/me");
  return (
    <div className="grid max-w-4xl gap-4">
      <ProblemAlert problem={list.problem} />
      {list.loading && !list.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {list.data && list.data.items.length === 0 ? <p className="text-muted-foreground">{t("noBranches")}</p> : null}
      {list.data && list.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("code")}</TableHead>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("tz")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((b) => (
              <TableRow key={b.id}>
                <TableCell>{b.code}</TableCell>
                <TableCell><Link className="underline-offset-2 hover:underline" href={`/app/organisation/branches/${b.id}`}>{b.name}</Link></TableCell>
                <TableCell>{b.tz ?? ""}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {me.data && can(me.data.grants, "ORG-01") ? <NewBranch onCreated={list.reload} /> : null}
    </div>
  );
}

function NewBranch({ onCreated }: { onCreated: () => void }) {
  const t = useTranslations("org");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const zones = typeof Intl.supportedValuesOf === "function" ? Intl.supportedValuesOf("timeZone") : [];
  return (
    <form
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const form = e.currentTarget;
        const f = new FormData(form);
        const text = (k: string) => String(f.get(k) ?? "").trim();
        const r = await bff("/branches", sendJson("POST", {
          name: text("name"),
          code: text("code"),
          address: text("address") || undefined,
          tz: text("tz"),
          time_zone_valid_from: text("tz_from"),
        }, { "idempotency-key": key }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setKey(crypto.randomUUID());
        form.reset();
        onCreated();
      }}
    >
      <h2 className="text-lg font-semibold sm:col-span-3">{t("newBranch")}</h2>
      <div className="grid gap-1">
        <Label htmlFor="nb-name">{t("name")}</Label>
        <Input id="nb-name" name="name" required maxLength={120} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="nb-code">{t("code")}</Label>
        <Input id="nb-code" name="code" required maxLength={20} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="nb-address">{t("address")}</Label>
        <Input id="nb-address" name="address" maxLength={500} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="nb-tz">{t("tz")}</Label>
        <Input id="nb-tz" name="tz" required list="nb-zones" placeholder="Asia/Kolkata" />
        <datalist id="nb-zones">{zones.map((z) => <option key={z} value={z} />)}</datalist>
      </div>
      <div className="grid gap-1">
        <Label htmlFor="nb-from">{t("tzFrom")}</Label>
        <Input id="nb-from" name="tz_from" type="date" required />
      </div>
      <Button type="submit" className="self-end justify-self-start">{t("addBranch")}</Button>
      <div className="sm:col-span-3"><ProblemAlert problem={problem} /></div>
    </form>
  );
}
