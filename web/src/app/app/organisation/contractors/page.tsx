"use client";
import { useTranslations } from "next-intl";
import { can } from "@/lib/menu";
import { useBff } from "@/lib/use-bff";
import type { Me } from "@/types/api";
import { NamedEntries } from "../named-entries";

/** Contractors (ORG-07 to read; ORG-05, organisation-wide, to change), chosen later on the employee record. */
export default function ContractorsPage() {
  const t = useTranslations("org");
  const me = useBff<Me>("/me");
  return (
    <div className="grid gap-3">
      <p className="text-sm text-muted-foreground">{t("contractorsHelp")}</p>
      <NamedEntries path="/contractors" second="contact" canEdit={!!me.data && can(me.data.grants, "ORG-05")} label={t("sub.contractors")} />
    </div>
  );
}
