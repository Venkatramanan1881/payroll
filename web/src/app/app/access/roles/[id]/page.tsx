"use client";
import { useTranslations } from "next-intl";
import { useParams } from "next/navigation";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { PageProblem } from "@/components/page-problem";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can, type GrantKind } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Me, Role, RoleGrant } from "@/types/api";

const KINDS: GrantKind[] = ["all", "scoped", "self", "approve"];

/** A role: rename a custom one (IAM-04) and replace its grants as a whole with a reason (IAM-03); both If-Match. */
export default function RolePage() {
  const { id } = useParams<{ id: string }>();
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const role = useBff<Role>(`/roles/${id}`);
  const [problem, setProblem] = useState<Problem | null>(null);
  if (role.problem && !role.data) return <PageProblem problem={role.problem} />;
  if (!role.data) return <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} />;
  const r = role.data;
  return (
    <div className="grid max-w-3xl gap-6">
      <h2 className="flex items-center gap-3 text-lg font-semibold">
        {r.name}
        <Badge variant="secondary">{r.system_code ? t("system") : t("custom")}</Badge>
      </h2>
      <ProblemAlert problem={problem} />
      {!r.system_code && can(g, "IAM-04") ? (
        <form
          key={`name-${r.updated_at}`}
          className="flex flex-wrap items-end gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            const name = String(new FormData(e.currentTarget).get("name") ?? "").trim();
            const res = await bff(`/roles/${id}`, sendJson("PATCH", { name }, { "if-match": role.etag ?? "" }));
            if (!res.ok) return setProblem(res.problem);
            setProblem(null);
            role.reload();
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="rr-name">{t("name")}</Label>
            <Input id="rr-name" name="name" required maxLength={80} defaultValue={r.name} />
          </div>
          <Button type="submit" variant="outline" disabled={!role.etag}>{t("rename")}</Button>
        </form>
      ) : null}
      <Grants
        key={JSON.stringify(r.grants ?? [])}
        role={r}
        canEdit={can(g, "IAM-03")}
        onSave={async (grants, reason) => {
          const res = await bff(`/roles/${id}/change-grants`, sendJson("POST", { grants, reason }, { "idempotency-key": crypto.randomUUID(), "if-match": role.etag ?? "" }));
          if (!res.ok) {
            setProblem(res.problem);
            return false;
          }
          setProblem(null);
          role.reload();
          return true;
        }}
      />
    </div>
  );
}

function Grants({ role, canEdit, onSave }: { role: Role; canEdit: boolean; onSave: (g: RoleGrant[], reason: string) => Promise<boolean> }) {
  const t = useTranslations("acc");
  const [rows, setRows] = useState<RoleGrant[]>(role.grants ?? []);
  const [busy, setBusy] = useState(false);
  const set = (i: number, patch: Partial<RoleGrant>) => setRows((rs) => rs.map((x, j) => (j === i ? { ...x, ...patch } : x)));
  if (!canEdit) {
    return (
      <div className="grid gap-2">
        <h3 className="font-semibold">{t("grants")}</h3>
        {rows.length ? (
          <ul className="text-sm">{rows.map((x) => <li key={x.operation_code + x.grant_kind}>{x.operation_code}: {t(`gk.${x.grant_kind}`)}{x.setting_ref ? ` (${x.setting_ref})` : ""}</li>)}</ul>
        ) : <p className="text-sm text-muted-foreground">{t("noGrants")}</p>}
      </div>
    );
  }
  return (
    <form
      className="grid gap-3"
      aria-label={t("grants")}
      onSubmit={async (e) => {
        e.preventDefault();
        const reason = String(new FormData(e.currentTarget).get("reason") ?? "").trim();
        setBusy(true);
        const grants = rows.map((x) => ({ operation_code: x.operation_code.trim().toUpperCase(), grant_kind: x.grant_kind, setting_ref: x.setting_ref?.trim() || null }));
        await onSave(grants, reason);
        setBusy(false);
      }}
    >
      <h3 className="font-semibold">{t("grants")}</h3>
      <p className="text-sm text-muted-foreground">{t("grantsHelp")}</p>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t("operation")}</TableHead>
            <TableHead>{t("kind")}</TableHead>
            <TableHead>{t("condition")}</TableHead>
            <TableHead />
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((x, i) => (
            <TableRow key={i}>
              <TableCell>
                <Input value={x.operation_code} required pattern="[A-Za-z]{3}-[0-9]{2}[a-z]?" maxLength={7} aria-label={`${t("operation")} ${i + 1}${x.operation_code ? `: ${x.operation_code}` : ""}`} onChange={(e) => set(i, { operation_code: e.target.value })} />
              </TableCell>
              <TableCell>
                <NativeSelect value={x.grant_kind} aria-label={`${t("kind")}: ${x.operation_code || i + 1}`} onChange={(e) => set(i, { grant_kind: e.target.value as GrantKind })}>
                  {KINDS.map((k) => <option key={k} value={k}>{t(`gk.${k}`)}</option>)}
                </NativeSelect>
              </TableCell>
              <TableCell>
                <Input value={x.setting_ref ?? ""} maxLength={20} aria-label={`${t("condition")}: ${x.operation_code || i + 1}`} onChange={(e) => set(i, { setting_ref: e.target.value })} />
              </TableCell>
              <TableCell className="text-right">
                <Button type="button" size="sm" variant="ghost" aria-label={`${t("remove")}: ${x.operation_code || i + 1}`} onClick={() => setRows((rs) => rs.filter((_, j) => j !== i))}>
                  {t("remove")}
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <Button type="button" variant="outline" className="justify-self-start" onClick={() => setRows((rs) => [...rs, { operation_code: "", grant_kind: "scoped", setting_ref: null }])}>
        {t("addGrant")}
      </Button>
      <div className="grid gap-1">
        <Label htmlFor="rg-reason">{t("reason")}</Label>
        <Input id="rg-reason" name="reason" required maxLength={1000} />
      </div>
      <Button type="submit" className="justify-self-start" disabled={busy}>{t("saveGrants")}</Button>
    </form>
  );
}
