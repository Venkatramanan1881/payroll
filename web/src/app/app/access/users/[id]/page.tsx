"use client";
import { useTranslations } from "next-intl";
import { useParams } from "next/navigation";
import { useState } from "react";
import { PageProblem } from "@/components/page-problem";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { Me, Page, Role, UserAccount } from "@/types/api";
import { RoleChecks, ScopeFields, accessOf, userLabel } from "../../access-fields";

/** A user: roles and scope (IAM-02, If-Match) and deactivate / reactivate (IAM-01), each with a reason. */
export default function UserPage() {
  const { id } = useParams<{ id: string }>();
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const user = useBff<UserAccount>(`/users/${id}`);
  const roles = useBff<Page<Role>>(can(g, "IAM-02") && can(g, "IAM-03") ? "/roles" : "");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState(false);
  const [key, setKey] = useState(() => crypto.randomUUID());
  if (user.problem && !user.data) return <PageProblem problem={user.problem} />;
  if (!user.data) return <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} />;
  const u = user.data;

  async function send(path: string, body: unknown, headers: Record<string, string> = {}) {
    setSaved(false);
    const r = await bff(path, sendJson("POST", body, { "idempotency-key": key, ...headers }));
    if (!r.ok) return setProblem(r.problem);
    setProblem(null);
    setSaved(true);
    setKey(crypto.randomUUID());
    user.reload();
  }
  return (
    <div className="grid max-w-3xl gap-6">
      <h2 className="flex items-center gap-3 text-lg font-semibold">
        {userLabel(u)}
        <Badge variant={u.active ? "default" : "secondary"}>{u.active ? t("active") : t("inactive")}</Badge>
      </h2>
      <ProblemAlert problem={problem ?? roles.problem} />
      {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
      {can(g, "IAM-02") && roles.data ? (
        <form
          key={JSON.stringify([u.roles, u.scopes])}
          className="grid gap-3 rounded-lg border p-4 sm:grid-cols-2"
          aria-label={t("access")}
          onSubmit={(e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            send(`/users/${id}/change-access`, { ...accessOf(f), reason: String(f.get("reason") ?? "").trim() }, { "if-match": user.etag ?? "" });
          }}
        >
          <h3 className="font-semibold sm:col-span-2">{t("access")}</h3>
          <div className="sm:col-span-2"><RoleChecks roles={roles.data.items} chosen={u.roles.map((r) => r.id)} /></div>
          <ScopeFields prefix="ua" initial={u.scopes[0]} />
          <div className="grid gap-1 sm:col-span-2">
            <Label htmlFor="ua-reason">{t("reason")}</Label>
            <Input id="ua-reason" name="reason" required maxLength={1000} />
          </div>
          <Button type="submit" className="justify-self-start" disabled={!user.etag}>{t("saveAccess")}</Button>
        </form>
      ) : (
        <p className="text-sm">{u.roles.map((r) => r.name).join(", ")} · {u.scopes.map((s) => t(`sk.${s.scope_kind}`)).join(", ")}</p>
      )}
      {can(g, "IAM-01") ? (
        <form
          key={`active-${u.active}`}
          className="grid gap-3 rounded-lg border p-4"
          aria-label={u.active ? t("deactivate") : t("reactivate")}
          onSubmit={(e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            send(`/users/${id}/${u.active ? "deactivate" : "reactivate"}`, { reason: String(f.get("reason") ?? "").trim() });
          }}
        >
          {u.active ? <p className="text-sm text-muted-foreground">{t("activeHelp")}</p> : null}
          <div className="grid gap-1">
            <Label htmlFor="ud-reason">{t("reason")}</Label>
            <Input id="ud-reason" name="reason" required maxLength={1000} />
          </div>
          <Button type="submit" variant={u.active ? "destructive" : "default"} className="justify-self-start">{u.active ? t("deactivate") : t("reactivate")}</Button>
        </form>
      ) : null}
    </div>
  );
}
