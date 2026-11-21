import { getTranslations } from "next-intl/server";
import Link from "next/link";

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
      <nav aria-label={t("title")} className="flex flex-wrap gap-4 border-b pb-2 text-sm">
        {SUB.map(([k, href]) => <Link key={k} href={href} className="hover:underline">{t(`sub.${k}`)}</Link>)}
      </nav>
      {children}
    </section>
  );
}
