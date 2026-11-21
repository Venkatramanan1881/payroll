import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { SignOutButton } from "@/components/sign-out-button";
import { Badge } from "@/components/ui/badge";
import { landing } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me } from "@/types/api";

/** Self-service area: every signed-in organisation user may use it; the API limits each read to their own records. */
export default async function MeLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations();
  const [me, inbox] = await Promise.all([api<Me>("/v1/me"), api<{ unread_count?: number }>("/v1/notifications/inbox?unread=true")]);
  const unread = inbox.ok ? (inbox.data.unread_count ?? 0) : 0;
  const link = (href: string, label: React.ReactNode) => (
    <Link href={href} className="hover:underline">{label}</Link>
  );
  return (
    <div className="min-h-screen">
      <header className="border-b">
        <div className="mx-auto flex max-w-5xl flex-wrap items-center gap-4 px-4 py-3">
          <span className="font-semibold">{t("common.appName")}</span>
          <nav aria-label="Self-service" className="flex flex-1 flex-wrap gap-3 text-sm">
            {link("/me", t("me.nav.details"))}
            {link("/me/attendance", t("me.nav.attendance"))}
            {link("/me/schedule", t("me.nav.schedule"))}
            {link("/me/leave", t("me.nav.leave"))}
            {link("/me/notifications", (
              <>
                {t("me.nav.notifications")}{" "}
                {unread > 0 ? <Badge aria-label={t("me.unread", { count: unread })}>{unread}</Badge> : null}
              </>
            ))}
            {me.ok && landing(me.data.grants) === "/app" ? link("/app", t("me.nav.admin")) : null}
          </nav>
          <span className="text-sm text-muted-foreground">{me.ok ? (me.data.email ?? me.data.mobile) : null}</span>
          <SignOutButton to="/sign-in" />
        </div>
      </header>
      <main className="mx-auto grid max-w-5xl gap-4 px-4 py-6">{children}</main>
    </div>
  );
}
