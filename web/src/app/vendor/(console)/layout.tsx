import { getTranslations } from "next-intl/server";
import { SignOutButton } from "@/components/sign-out-button";

export default async function VendorLayout({ children }: { children: React.ReactNode }) {
  const t = await getTranslations("signIn");
  return (
    <div className="min-h-screen">
      <header className="border-b">
        <div className="mx-auto flex max-w-6xl items-center gap-4 px-4 py-3">
          <span className="flex-1 font-semibold">{t("vendorTitle")}</span>
          <SignOutButton to="/vendor/sign-in" />
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">{children}</main>
    </div>
  );
}
