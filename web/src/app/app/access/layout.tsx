import { getTranslations } from "next-intl/server";
import { SubNav } from "@/components/sub-nav";

const SUB = [
  ["users", "/app/access"],
  ["roles", "/app/access/roles"],
  ["approvers", "/app/access/approvers"],
  ["delegations", "/app/access/delegations"],
  ["settings", "/app/access/settings"],
  ["support", "/app/access/support"],
] as const;

/** Users and access (C03) and vendor support access: each part says what it needs; the API decides every call. */
export default async function AccessLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("acc");
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <SubNav label={t("title")} links={SUB.map(([k, href]) => ({ href, text: t(`sub.${k}`) }))} />
      {children}
    </section>
  );
}
