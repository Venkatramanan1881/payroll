"use client";
import { useTranslations } from "next-intl";
import { Button } from "@/components/ui/button";

export function SignOutButton({ to }: { to: string }) {
  const t = useTranslations("common");
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={async () => {
        await fetch("/bff/auth/sign-out", { method: "POST" }).catch(() => undefined);
        window.location.assign(to);
      }}
    >
      {t("signOut")}
    </Button>
  );
}
