"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Branch, Dashboard, Me, Page, Widget, WidgetChoice } from "@/types/api";

/** The caller's role dashboard (DSH-01; counts within their scope), and the widgets per role for DSH-02 holders. */
export default function DashboardPage() {
  const t = useTranslations("dsh");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const [branch, setBranch] = useState("");
  const branches = useBff<Page<Branch>>("/branches?limit=200&sort=name");
  const dash = useBff<Dashboard>(`/dashboards/me${branch ? `?branch_id=${branch}` : ""}`);
  return (
    <section className="grid gap-4">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("title")}</h1>
        {branches.data?.items.length ? (
          <div className="grid gap-1">
            <Label htmlFor="db-branch">{t("branch")}</Label>
            <NativeSelect id="db-branch" value={branch} onChange={(e) => setBranch(e.target.value)}>
              <option value="">{t("allBranches")}</option>
              {branches.data.items.map((b) => <option key={b.id} value={b.id}>{b.name}</option>)}
            </NativeSelect>
          </div>
        ) : null}
      </div>
      <ProblemAlert problem={dash.problem} />
      {dash.loading && !dash.data ? <Skeleton className="h-32" aria-busy="true" aria-label={c("loading")} /> : null}
      {dash.data && dash.data.widgets.length === 0 ? <p className="text-muted-foreground">{t("empty")}</p> : null}
      {dash.data ? (
        <div className="grid gap-4 md:grid-cols-2">
          {dash.data.widgets.map((w) => <WidgetCard key={w.code} widget={w} />)}
        </div>
      ) : null}
      {can(g, "DSH-02") ? <Choices /> : null}
    </section>
  );
}

const label = (k: string) => k.replaceAll("_", " ");
const shown = (k: string) => k !== "id" && !k.endsWith("_id"); // ids are for links, not for reading
const cell = (v: unknown) => (v === null || v === undefined ? "" : typeof v === "object" ? JSON.stringify(v) : String(v));

/** Widgets differ in shape (reports.yaml: "widget-specific counts and rows"): figures as figures, lists as tables. */
function WidgetCard({ widget }: { widget: Widget }) {
  const t = useTranslations("dsh");
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base"><h2>{t.has(`w.${widget.code}`) ? t(`w.${widget.code}`) : label(widget.code)}</h2></CardTitle>
        <p className="text-xs text-muted-foreground">{t("asOf", { time: wallDateTime(widget.as_of) })}</p>
      </CardHeader>
      <CardContent className="grid gap-3">
        {Object.entries(widget.data ?? {}).filter(([k]) => shown(k)).map(([k, v]) => <Value key={k} name={k} value={v} />)}
      </CardContent>
    </Card>
  );
}

function Value({ name, value }: { name: string; value: unknown }) {
  const t = useTranslations("dsh");
  if (Array.isArray(value)) {
    if (value.length === 0) return <p className="text-sm"><span className="text-muted-foreground">{label(name)}:</span> {t("none")}</p>;
    if (typeof value[0] !== "object" || value[0] === null) return <p className="text-sm"><span className="text-muted-foreground">{label(name)}:</span> {value.map(cell).join(", ")}</p>;
    const cols = Object.keys(value[0] as object).filter(shown);
    return (
      <Table aria-label={label(name)}>
        <TableHeader>
          <TableRow>{cols.map((col) => <TableHead key={col}>{label(col)}</TableHead>)}</TableRow>
        </TableHeader>
        <TableBody>
          {value.map((row, i) => (
            <TableRow key={i}>{cols.map((col) => <TableCell key={col}>{cell(((row ?? {}) as Record<string, unknown>)[col])}</TableCell>)}</TableRow>
          ))}
        </TableBody>
      </Table>
    );
  }
  if (value && typeof value === "object") {
    return (
      <div className="grid gap-1">
        <p className="text-sm text-muted-foreground">{label(name)}</p>
        {Object.entries(value).filter(([k]) => shown(k)).map(([k, v]) => <Value key={k} name={k} value={v} />)}
      </div>
    );
  }
  return (
    <p className="flex items-baseline justify-between gap-3">
      <span className="text-sm text-muted-foreground">{label(name)}</span>
      <span className={typeof value === "number" ? "text-2xl font-semibold" : "text-sm"}>{cell(value)}</span>
    </p>
  );
}

/** DB-01: the widgets each role shows, from the ones offered to it (DSH-02; If-Match on the choices). */
function Choices() {
  const t = useTranslations("dsh");
  const list = useBff<{ items: WidgetChoice[] }>("/dashboards/widget-choices");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState<string | null>(null);
  return (
    <div className="grid gap-3">
      <h2 className="text-lg font-semibold">{t("choices")}</h2>
      <p className="text-sm text-muted-foreground">{t("choicesHelp")}</p>
      <ProblemAlert problem={list.problem ?? problem} />
      <div className="grid gap-3 md:grid-cols-2">
        {list.data?.items.filter((r) => r.offered.length > 0).map((r) => (
          <form
            key={`${r.role_id}-${r.chosen.join(",")}`}
            className="grid gap-2 rounded-lg border p-4"
            aria-label={`${t("choices")}: ${r.role_name ?? r.role_id}`}
            onSubmit={async (e) => {
              e.preventDefault();
              const widgets = new FormData(e.currentTarget).getAll("widgets").map(String);
              setSaved(null);
              const res = await bff(`/dashboards/widget-choices/${r.role_id}`, sendJson("PUT", { widgets }, { "if-match": list.etag ?? "" }));
              if (!res.ok) {
                if (res.problem.status === 412) list.reload(); // someone saved first: read the choices again
                return setProblem(res.problem);
              }
              setProblem(null);
              setSaved(r.role_id);
              list.reload();
            }}
          >
            <h3 className="flex items-center gap-2 font-medium">
              {r.role_name ?? r.role_id}
              {r.default_applied ? <Badge variant="secondary">{t("defaultApplied")}</Badge> : null}
            </h3>
            {r.offered.map((w) => (
              <label key={w} className="flex items-center gap-2 text-sm">
                <input type="checkbox" name="widgets" value={w} defaultChecked={r.chosen.includes(w)} />
                {t.has(`w.${w}`) ? t(`w.${w}`) : label(w)}
              </label>
            ))}
            <Button type="submit" size="sm" className="justify-self-start" disabled={!list.etag || list.loading}>{t("saveChoice")}</Button>
            <p role="status" className="text-sm text-green-700 dark:text-green-400">{saved === r.role_id ? t("saved") : ""}</p>
          </form>
        ))}
      </div>
    </div>
  );
}
