import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { SignOutButton } from "@/components/sign-out-button";
import { buildMenu } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me, Modules } from "@/types/api";

// Organisation admin shell: menu items from the user's grants and the organisation's modules (TSD §12).
export default async function AppLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations();
  const [me, mods] = await Promise.all([api<Me>("/v1/me"), api<Modules>("/v1/me/modules")]);
  const menu = me.ok && mods.ok ? buildMenu(me.data.grants, mods.data.items) : [];
  return (
    <div className="min-h-screen">
      <header className="border-b">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center gap-4 px-4 py-3">
          <Link href="/app" className="font-semibold">{t("common.appName")}</Link>
          <nav aria-label="Main" className="flex flex-1 flex-wrap gap-3 text-sm">
            {menu.map((i) => (
              <Link key={i.key} href={i.href} className="hover:underline">{t(`nav.${i.key}`)}</Link>
            ))}
            {me.ok && me.data.employee_id ? <Link href="/me" className="hover:underline">{t("me.title")}</Link> : null}
          </nav>
          <span className="text-sm text-muted-foreground">{me.ok ? (me.data.email ?? me.data.mobile) : null}</span>
          <SignOutButton to="/sign-in" />
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">{children}</main>
    </div>
  );
}
