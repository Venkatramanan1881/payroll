"use client";
import Script from "next/script";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useRef, useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { asProblem, type Problem } from "@/lib/problem";

type Gsi = {
  accounts: {
    id: {
      initialize: (o: { client_id: string; callback: (r: { credential: string }) => void }) => void;
      renderButton: (el: HTMLElement, o: Record<string, string>) => void;
    };
  };
};

// D-22: Google Identity Services gives an ID token; the API checks the vendor's domain and the vendor user.
export function GoogleButton({ clientId }: { clientId: string }) {
  const t = useTranslations("signIn");
  const router = useRouter();
  const box = useRef<HTMLDivElement>(null);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [failed, setFailed] = useState(false);

  async function exchange(idToken: string) {
    setProblem(null);
    setFailed(false);
    const res = await fetch("/bff/auth/vendor-google", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ id_token: idToken }) }).catch(() => null);
    if (res?.ok) return router.replace("/vendor");
    if (!res) return setProblem({ status: 0, code: "network" });
    if (res.status === 401) return setFailed(true);
    setProblem(asProblem(res.status, await res.json().catch(() => null)));
  }

  return (
    <>
      <Script
        src="https://accounts.google.com/gsi/client"
        strategy="afterInteractive"
        onLoad={() => {
          const g = (window as unknown as { google: Gsi }).google;
          g.accounts.id.initialize({ client_id: clientId, callback: (r) => exchange(r.credential) });
          if (box.current) g.accounts.id.renderButton(box.current, { theme: "outline", size: "large", text: "signin_with" });
        }}
      />
      <div ref={box} />
      {failed ? <p role="alert" className="text-sm text-destructive">{t("failed")}</p> : null}
      <ProblemAlert problem={problem} />
    </>
  );
}
