"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import { can } from "@/lib/menu";
import type { Me, ObBatch, Page } from "@/types/api";

/** Opening-balance batches (OBL-04 to read, OBL-01 to start one). Entries are added on the batch's page. */
export default function OpeningBalancesPage() {
  const t = useTranslations("ob");
  const c = useTranslations("common");
  const router = useRouter();
  const { data, problem, loading } = useBff<Page<ObBatch>>("/opening-balances/batches");
  const [err, setErr] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  const me = useBff<Me>("/me");
  return (
    <section className="grid max-w-3xl gap-4">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("title")}</h1>
        {me.data && can(me.data.grants, "OBL-01") ? <Button
          onClick={async () => {
            const r = await bff<ObBatch>("/opening-balances/batches", sendJson("POST", {}, { "idempotency-key": key }));
            if (!r.ok) return setErr(r.problem);
            router.push(`/app/opening-balances/${r.data.id}`);
          }}
        >
          {t("newBatch")}
        </Button> : null}
      </div>
      <ProblemAlert problem={problem ?? err} />
      {loading && !data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {data && data.items.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {data && data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("created")}</TableHead>
              <TableHead>{t("state")}</TableHead>
              <TableHead className="text-right">{t("entries")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.items.map((b) => (
              <TableRow key={b.id}>
                <TableCell><Link className="underline-offset-2 hover:underline" href={`/app/opening-balances/${b.id}`}>{wallDateTime(b.created_at)}</Link></TableCell>
                <TableCell><Badge variant={b.state === "confirmed" ? "default" : "secondary"}>{t(`bs.${b.state}`)}</Badge></TableCell>
                <TableCell className="text-right">{b.entry_count}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
    </section>
  );
}
