import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { SignInForm } from "./sign-in-form";

export default async function SignInPage() {
  const t = await getTranslations("signIn");
  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col justify-center gap-4 p-4">
      <Card>
        <CardHeader>
          <CardTitle>
            <h1>{t("title")}</h1>
          </CardTitle>
        </CardHeader>
        <CardContent>
          <SignInForm phoneEnabled={Boolean(process.env.NEXT_PUBLIC_FIREBASE_API_KEY)} />
        </CardContent>
      </Card>
      <Link href="/vendor/sign-in" className="text-center text-sm text-muted-foreground underline">
        {t("vendorLink")}
      </Link>
    </main>
  );
}
