import { getTranslations } from "next-intl/server";
import { SubNav } from "@/components/sub-nav";

const SUB = [
  ["settings", "/app/organisation"],
  ["branches", "/app/organisation/branches"],
  ["lists", "/app/organisation/lists"],
  ["contractors", "/app/organisation/contractors"],
] as const;

/** Organisation structure and settings (C01): each part is readable with ORG-07; the API decides every call. */
export default async function OrganisationLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("org");
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <SubNav label={t("title")} links={SUB.map(([k, href]) => ({ href, text: t(`sub.${k}`) }))} />
      {children}
    </section>
  );
}
