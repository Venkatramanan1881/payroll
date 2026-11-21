"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useState } from "react";
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
import type { Me, Page, Role } from "@/types/api";

/** Roles (IAM-03), and a new custom role for IAM-04 (refused while AC-06 is off). */
export default function RolesPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const me = useBff<Me>("/me");
  const roles = useBff<Page<Role>>("/roles");
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  return (
    <div className="grid max-w-3xl gap-4">
      <ProblemAlert problem={roles.problem ?? problem} />
      {roles.loading && !roles.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {roles.data ? (
        <ul className="grid gap-2">
          {roles.data.items.map((r) => (
            <li key={r.id} className="flex items-center gap-3">
              <Link className="underline-offset-2 hover:underline" href={`/app/access/roles/${r.id}`}>{r.name}</Link>
              <Badge variant="secondary">{r.system_code ? t("system") : t("custom")}</Badge>
            </li>
          ))}
        </ul>
      ) : null}
      {me.data && can(me.data.grants, "IAM-04") ? (
        <form
          className="flex flex-wrap items-end gap-3 rounded-lg border p-4"
          aria-label={t("newRole")}
          onSubmit={async (e) => {
            e.preventDefault();
            const form = e.currentTarget;
            const name = String(new FormData(form).get("name") ?? "").trim();
            const r = await bff("/roles", sendJson("POST", { name }, { "idempotency-key": key }));
            if (!r.ok) return setProblem(r.problem);
            setProblem(null);
            setKey(crypto.randomUUID());
            form.reset();
            roles.reload();
          }}
        >
          <div className="grid gap-1">
            <Label htmlFor="nr-name">{t("newRole")}</Label>
            <Input id="nr-name" name="name" required maxLength={80} />
          </div>
          <Button type="submit">{t("addRole")}</Button>
        </form>
      ) : null}
    </div>
  );
}
