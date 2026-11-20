import { getTranslations } from "next-intl/server";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { GoogleButton } from "./google-button";

export default async function VendorSignInPage() {
  const t = await getTranslations("signIn");
  const clientId = process.env.NEXT_PUBLIC_VENDOR_GOOGLE_CLIENT_ID;
  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col justify-center p-4">
      <Card>
        <CardHeader>
          <CardTitle>
            <h1>{t("vendorTitle")}</h1>
          </CardTitle>
        </CardHeader>
        <CardContent className="grid gap-3">
          <p className="text-sm text-muted-foreground">{clientId ? t("vendorHelp") : t("vendorUnavailable")}</p>
          {clientId ? <GoogleButton clientId={clientId} /> : null}
        </CardContent>
      </Card>
    </main>
  );
}
