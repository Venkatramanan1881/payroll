import { getTranslations } from "next-intl/server";
import Link from "next/link";

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
      <nav aria-label={t("title")} className="flex flex-wrap gap-4 border-b pb-2 text-sm">
        {SUB.map(([k, href]) => <Link key={k} href={href} className="hover:underline">{t(`sub.${k}`)}</Link>)}
      </nav>
      {children}
    </section>
  );
}
