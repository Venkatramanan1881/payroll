"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
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
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Me, Page, Role, UserAccount } from "@/types/api";
import { RoleChecks, ScopeFields, accessOf, userLabel } from "./access-fields";

/** User accounts (IAM-01, organisation-wide), and a new user for IAM-01 + IAM-02. */
export default function UsersPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  const users = useBff<Page<UserAccount>>("/users");
  const roles = useBff<Page<Role>>(can(g, "IAM-02") && can(g, "IAM-03") ? "/roles" : "");
  return (
    <div className="grid max-w-5xl gap-4">
      <ProblemAlert problem={users.problem ?? roles.problem} />
      {users.loading && !users.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {users.data && users.data.items.length === 0 ? <p className="text-muted-foreground">{t("noUsers")}</p> : null}
      {users.data && users.data.items.length > 0 ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("signIn")}</TableHead>
              <TableHead>{t("roles")}</TableHead>
              <TableHead>{t("scope")}</TableHead>
              <TableHead>{t("state")}</TableHead>
              <TableHead>{t("lastSignIn")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {users.data.items.map((u) => (
              <TableRow key={u.id}>
                <TableCell><Link className="underline-offset-2 hover:underline" href={`/app/access/users/${u.id}`}>{userLabel(u)}</Link></TableCell>
                <TableCell>{u.roles.map((r) => r.name).join(", ")}</TableCell>
                <TableCell>{u.scopes.map((s) => t(`sk.${s.scope_kind}`)).join(", ")}</TableCell>
                <TableCell><Badge variant={u.active ? "default" : "secondary"}>{u.active ? t("active") : t("inactive")}</Badge></TableCell>
                <TableCell>{u.last_sign_in_at ? wallDateTime(u.last_sign_in_at) : t("never")}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {can(g, "IAM-01") && can(g, "IAM-02") && roles.data ? <NewUser roles={roles.data.items} onCreated={users.reload} /> : null}
    </div>
  );
}

function NewUser({ roles, onCreated }: { roles: Role[]; onCreated: () => void }) {
  const t = useTranslations("acc");
  const [kind, setKind] = useState<UserAccount["login_kind"]>("password");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const [form, setForm] = useState(0); // a new form after each user
  return (
    <form
      key={form}
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      aria-label={t("newUser")}
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const text = (k: string) => String(f.get(k) ?? "").trim();
        const r = await bff("/users", sendJson("POST", {
          login_kind: kind,
          ...(kind === "mobile_otp" ? { mobile: text("mobile") } : { email: text("email"), initial_password: String(f.get("initial_password") ?? "") }),
          ...accessOf(f),
        }, { "idempotency-key": key }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setKey(crypto.randomUUID());
        setForm((n) => n + 1);
        onCreated();
      }}
    >
      <h2 className="text-lg font-semibold sm:col-span-3">{t("newUser")}</h2>
      <div className="grid gap-1">
        <Label htmlFor="nu-kind">{t("loginKind")}</Label>
        <NativeSelect id="nu-kind" value={kind} onChange={(e) => setKind(e.target.value as UserAccount["login_kind"])}>
          <option value="password">{t("lk.password")}</option>
          <option value="mobile_otp">{t("lk.mobile_otp")}</option>
        </NativeSelect>
      </div>
      {kind === "mobile_otp" ? (
        <div className="grid gap-1">
          <Label htmlFor="nu-mobile">{t("mobile")}</Label>
          <Input id="nu-mobile" name="mobile" type="tel" required pattern="\+[1-9][0-9]{6,14}" />
        </div>
      ) : (
        <>
          <div className="grid gap-1">
            <Label htmlFor="nu-email">{t("email")}</Label>
            <Input id="nu-email" name="email" type="email" required maxLength={254} autoComplete="off" />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="nu-password">{t("initialPassword")}</Label>
            <Input id="nu-password" name="initial_password" type="password" required minLength={8} maxLength={200} autoComplete="new-password" />
          </div>
        </>
      )}
      <div className="sm:col-span-3"><RoleChecks roles={roles} /></div>
      <ScopeFields prefix="nu" />
      <Button type="submit" className="self-end justify-self-start">{t("addUser")}</Button>
      <div className="sm:col-span-3"><ProblemAlert problem={problem} /></div>
    </form>
  );
}
