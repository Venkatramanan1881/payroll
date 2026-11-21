import { getTranslations } from "next-intl/server";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import { lookups } from "@/server/lookups";
import type { Me } from "@/types/api";
import { OrgSettingsView } from "./org-settings";

export default async function OrganisationPage() {
  const t = await getTranslations("org");
  const [me, lk, org] = await Promise.all([api<Me>("/v1/me"), lookups(), api<{ display_name: string; legal_name: string }>("/v1/organisation")]);
  const grants = me.ok ? me.data.grants : [];
  return (
    <div className="grid gap-4">
      {org.ok ? (
        <p>
          <span className="font-medium">{org.data.display_name}</span>
          <span className="text-muted-foreground"> · {t("legalName")}: {org.data.legal_name}</span>
        </p>
      ) : null}
      <OrgSettingsView canChange={can(grants, "ORG-06")} branches={lk.branches} />
    </div>
  );
}
