"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { Label } from "@/components/ui/label";
import { useBff } from "@/lib/use-bff";
import type { Branch, ListEntry, Page, Role, UserAccount, UserScope } from "@/types/api";

/** How a user is shown: their sign-in (users carry no name of their own). */
export const userLabel = (u: Pick<UserAccount, "email" | "mobile" | "id">) => u.email ?? u.mobile ?? u.id;

/** Role checkboxes (`role_ids` in the form). */
export function RoleChecks({ roles, chosen = [] }: { roles: Role[]; chosen?: string[] }) {
  const t = useTranslations("acc");
  return (
    <fieldset className="grid gap-1">
      <legend className="text-sm font-medium">{t("roles")}</legend>
      <div className="flex flex-wrap gap-x-4 gap-y-1">
        {roles.map((r) => (
          <label key={r.id} className="flex items-center gap-1 text-sm">
            <input type="checkbox" name="role_ids" value={r.id} defaultChecked={chosen.includes(r.id)} />
            {r.name}
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/**
 * One AC-07 scope: the whole organisation, a branch or a department (`scope_kind`, `branch_id`, `department_id`).
 * shortcut: one scope per user while combined scopes are OQ-C03-04; offer several once that is decided.
 */
export function ScopeFields({ initial, prefix }: { initial?: UserScope; prefix: string }) {
  const t = useTranslations("acc");
  const [kind, setKind] = useState<UserScope["scope_kind"]>(initial?.scope_kind ?? "all");
  // Controlled: the options load after the first render, and a defaultValue would not pick them up.
  const [target, setTarget] = useState((initial?.scope_kind === "branch" ? initial.branch_id : initial?.department_id) ?? "");
  const branches = useBff<Page<Branch>>(kind === "branch" ? "/branches?limit=200&sort=name" : "");
  const departments = useBff<Page<ListEntry>>(kind === "department" ? "/lists/departments?limit=200" : "");
  const options = kind === "branch" ? branches.data?.items : kind === "department" ? departments.data?.items : undefined;
  const loaded = kind === "branch" ? !branches.loading : !departments.loading;
  return (
    <>
      <div className="grid gap-1">
        <Label htmlFor={`${prefix}-scope`}>{t("scope")}</Label>
        <NativeSelect id={`${prefix}-scope`} name="scope_kind" value={kind} onChange={(e) => { setKind(e.target.value as UserScope["scope_kind"]); setTarget(""); }}>
          {(["all", "branch", "department"] as const).map((k) => <option key={k} value={k}>{t(`sk.${k}`)}</option>)}
        </NativeSelect>
      </div>
      {kind !== "all" ? (
        <div className="grid gap-1">
          <Label htmlFor={`${prefix}-target`}>{t(kind)}</Label>
          <NativeSelect id={`${prefix}-target`} name="scope_target" required value={target} onChange={(e) => setTarget(e.target.value)}>
            <option value="" />
            {(options ?? []).map((o) => <option key={o.id} value={o.id}>{o.name}</option>)}
          </NativeSelect>
          {loaded && !options?.length ? <p className="text-sm text-muted-foreground">{t("noOptions")}</p> : null}
        </div>
      ) : null}
    </>
  );
}

/** The role ids and the one scope a form holds, as identity.yaml takes them. */
export function accessOf(f: FormData) {
  const kind = String(f.get("scope_kind")) as UserScope["scope_kind"];
  const target = String(f.get("scope_target") ?? "");
  const scope = kind === "branch" ? { scope_kind: kind, branch_id: target } : kind === "department" ? { scope_kind: kind, department_id: target } : { scope_kind: kind };
  return { role_ids: f.getAll("role_ids").map(String), scopes: [scope] };
}
