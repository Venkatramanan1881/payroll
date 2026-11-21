"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { can } from "@/lib/menu";
import { useBff } from "@/lib/use-bff";
import type { Me } from "@/types/api";
import { NamedEntries } from "../named-entries";

const KINDS = ["departments", "designations", "categories", "employee-types"] as const;

/** Departments, designations, categories and employee types (ORG-07 to read; ORG-04, organisation-wide, to change). */
export default function ListsPage() {
  const t = useTranslations("org");
  const [kind, setKind] = useState<(typeof KINDS)[number]>("departments");
  const me = useBff<Me>("/me");
  return (
    <div className="grid gap-4">
      <div className="flex flex-wrap gap-2" role="group" aria-label={t("sub.lists")}>
        {KINDS.map((k) => (
          <Button key={k} size="sm" variant={k === kind ? "default" : "outline"} aria-pressed={k === kind} onClick={() => setKind(k)}>
            {t(`kinds.${k}`)}
          </Button>
        ))}
      </div>
      <NamedEntries key={kind} path={`/lists/${kind}`} second="code" canEdit={!!me.data && can(me.data.grants, "ORG-04")} label={t(`kinds.${kind}`)} />
    </div>
  );
}
