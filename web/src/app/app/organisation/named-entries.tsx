"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { NamedEntry, Page } from "@/types/api";

/**
 * Lists (ORG-04) and contractors (ORG-05) work alike: a name and a second field (`code` or `contact`), add, rename with
 * If-Match, retire with a reason. There is no delete; retired entries stay on history and show on request.
 */
export function NamedEntries({ path, second, canEdit, label }: { path: string; second: "code" | "contact"; canEdit: boolean; label: string }) {
  const t = useTranslations("org");
  const c = useTranslations("common");
  const [retired, setRetired] = useState(false);
  const list = useBff<Page<NamedEntry>>(`${path}${retired ? "?include_retired=true" : ""}`);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const max = second === "code" ? 20 : 500;

  async function send(url: string, init: RequestInit) {
    const r = await bff(url, init);
    if (!r.ok) {
      setProblem(r.problem);
      return false;
    }
    setProblem(null);
    list.reload();
    return true;
  }
  return (
    <div className="grid max-w-4xl gap-3">
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={retired} onChange={(e) => setRetired(e.target.checked)} />
        {t("showRetired")}
      </label>
      <ProblemAlert problem={list.problem ?? problem} />
      {list.loading && !list.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {list.data && list.data.items.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {list.data && list.data.items.length > 0 ? (
        <Table aria-label={label}>
          <TableHeader>
            <TableRow>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t(second)}</TableHead>
              <TableHead>{t("state")}</TableHead>
              {canEdit ? <TableHead /> : null}
            </TableRow>
          </TableHeader>
          <TableBody>
            {list.data.items.map((x) => (
              <TableRow key={`${x.id}-${x.updated_at}`}>
                {canEdit && !x.retired_at ? (
                  <>
                    <TableCell>
                      <Input form={`ne-${x.id}`} name="name" required maxLength={second === "code" ? 120 : 200} defaultValue={x.name} aria-label={`${t("name")}: ${x.name}`} />
                    </TableCell>
                    <TableCell>
                      <Input form={`ne-${x.id}`} name="second" maxLength={max} defaultValue={x[second] ?? ""} aria-label={`${t(second)}: ${x.name}`} />
                    </TableCell>
                  </>
                ) : (
                  <>
                    <TableCell>{x.name}</TableCell>
                    <TableCell>{x[second] ?? ""}</TableCell>
                  </>
                )}
                <TableCell><Badge variant={x.retired_at ? "secondary" : "default"}>{x.retired_at ? t("retired") : t("active")}</Badge></TableCell>
                {canEdit ? (
                  <TableCell className="text-right">
                    {!x.retired_at ? (
                      <form
                        id={`ne-${x.id}`}
                        className="flex justify-end gap-2"
                        onSubmit={(e) => {
                          e.preventDefault();
                          const f = new FormData(e.currentTarget);
                          const v = String(f.get("second") ?? "").trim();
                          send(`${path}/${x.id}`, sendJson("PATCH", { name: String(f.get("name") ?? "").trim(), [second]: v || null }, { "if-match": `W/"${x.updated_at}"` }));
                        }}
                      >
                        <Button type="submit" size="sm" variant="outline" aria-label={`${t("rename")}: ${x.name}`}>{t("rename")}</Button>
                        <Button
                          type="button"
                          size="sm"
                          variant="ghost"
                          aria-label={`${t("retire")}: ${x.name}`}
                          onClick={() => {
                            const reason = window.prompt(t("retireReason"))?.trim();
                            if (reason) send(`${path}/${x.id}/retire`, sendJson("POST", { reason }, { "idempotency-key": crypto.randomUUID() }));
                          }}
                        >
                          {t("retire")}
                        </Button>
                      </form>
                    ) : null}
                  </TableCell>
                ) : null}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {canEdit ? (
        <form
          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
          aria-label={`${t("add")}: ${label}`}
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const f = new FormData(form);
            const v = String(f.get("second") ?? "").trim();
            const ok = await send(path, sendJson("POST", { name: String(f.get("name") ?? "").trim(), [second]: v || undefined }, { "idempotency-key": key }));
            if (!ok) return;
            setKey(crypto.randomUUID());
            form.reset();
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="ne-new-name">{t("name")}</Label>
            <Input id="ne-new-name" name="name" required maxLength={second === "code" ? 120 : 200} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="ne-new-second">{t(second)}</Label>
            <Input id="ne-new-second" name="second" maxLength={max} />
          </div>
          <Button type="submit" className="self-end justify-self-start">{t("add")}</Button>
        </form>
      ) : null}
    </div>
  );
}
