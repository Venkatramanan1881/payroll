"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useRef, useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { asProblem, type Problem } from "@/lib/problem";
import { signInWithFirebase, type PhoneStep } from "./firebase-phone";

type Candidate = { candidate_ref: string; display_name: string };
type Step =
  | { s: "start" }
  | { s: "code"; challenge: string; otpauth?: string }
  | { s: "choose"; choice: string; candidates: Candidate[] };

/** Posts one sign-in step to the BFF and moves to the next step the API asks for. */
async function post(step: string, body: unknown): Promise<{ ok: true } | { ok: false; problem: Problem }> {
  try {
    const res = await fetch(`/bff/auth/${step}`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
    const b = await res.json().catch(() => null);
    return res.ok ? { ok: true } : { ok: false, problem: asProblem(res.status, b) };
  } catch {
    return { ok: false, problem: { status: 0, code: "network" } };
  }
}

export function SignInForm({ phoneEnabled }: { phoneEnabled: boolean }) {
  const t = useTranslations("signIn");
  const router = useRouter();
  const [mode, setMode] = useState<"admin" | "employee">("admin");
  const [step, setStep] = useState<Step>({ s: "start" });
  const [problem, setProblem] = useState<Problem | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const phone = useRef<PhoneStep | null>(null);
  const [smsSent, setSmsSent] = useState(false);

  async function run(stepName: string, body: unknown) {
    setBusy(true);
    setProblem(null);
    setMessage(null);
    const r = await post(stepName, body);
    setBusy(false);
    if (r.ok) return router.replace("/");
    const p = r.problem;
    if (p.code === "SECOND_FACTOR_REQUIRED" && typeof p.challenge_token === "string")
      return setStep({ s: "code", challenge: p.challenge_token, otpauth: typeof p.otpauth_uri === "string" ? p.otpauth_uri : undefined });
    if (p.code === "ORGANISATION_CHOICE_REQUIRED" && typeof p.choice_token === "string")
      return setStep({ s: "choose", choice: p.choice_token, candidates: (p.candidates as Candidate[]) ?? [] });
    if (p.code === "UNAUTHENTICATED") return setMessage(t("failed"));
    if (p.code === "OTP_INVALID") return setMessage(t("codeInvalid"));
    setProblem(p);
  }

  const errors = (
    <>
      {message ? (
        <p role="alert" className="text-sm text-destructive">
          {message}
        </p>
      ) : null}
      <ProblemAlert problem={problem} />
    </>
  );

  if (step.s === "code")
    return (
      <form className="grid gap-3" onSubmit={(e) => { e.preventDefault(); run("second-factor", { challenge_token: step.challenge, code: String(new FormData(e.currentTarget).get("code")) }); }}>
        <h2 className="font-medium">{t("codeTitle")}</h2>
        <p className="text-sm text-muted-foreground">{step.otpauth ? t("enrolHelp") : t("codeHelp")}</p>
        {/* shortcut: the enrolment key is shown as text; add a QR code when the admin screens get their design pass. */}
        {step.otpauth ? <code className="break-all rounded bg-muted p-2 text-xs">{new URL(step.otpauth).searchParams.get("secret")}</code> : null}
        <Label htmlFor="code">{t("code")}</Label>
        <Input id="code" name="code" inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6,8}" required autoFocus />
        {errors}
        <Button type="submit" disabled={busy}>{t("verify")}</Button>
      </form>
    );

  if (step.s === "choose")
    return (
      <div className="grid gap-3">
        <h2 className="font-medium">{t("chooseTitle")}</h2>
        <p className="text-sm text-muted-foreground">{t("chooseHelp")}</p>
        {step.candidates.map((c) => (
          <Button key={c.candidate_ref} variant="outline" disabled={busy} onClick={() => run("organisation-choice", { choice_token: step.choice, candidate_ref: c.candidate_ref })}>
            {c.display_name}
          </Button>
        ))}
        {errors}
      </div>
    );

  return (
    <div className="grid gap-4">
      <div role="tablist" className="grid grid-cols-2 gap-2">
        {(["admin", "employee"] as const).map((m) => (
          <Button key={m} role="tab" aria-selected={mode === m} variant={mode === m ? "default" : "outline"} onClick={() => { setMode(m); setProblem(null); setMessage(null); }}>
            {t(m)}
          </Button>
        ))}
      </div>
      {mode === "admin" ? (
        <form className="grid gap-3" onSubmit={(e) => { e.preventDefault(); const f = new FormData(e.currentTarget); run("password", { email: f.get("email"), password: f.get("password"), device_label: "web" }); }}>
          <Label htmlFor="email">{t("email")}</Label>
          <Input id="email" name="email" type="email" autoComplete="username" required />
          <Label htmlFor="password">{t("password")}</Label>
          <Input id="password" name="password" type="password" autoComplete="current-password" required />
          {errors}
          <Button type="submit" disabled={busy}>{t("submit")}</Button>
        </form>
      ) : !phoneEnabled ? (
        <p className="text-sm text-muted-foreground">{t("phoneUnavailable")}</p>
      ) : (
        <form
          className="grid gap-3"
          onSubmit={async (e) => {
            e.preventDefault();
            const f = new FormData(e.currentTarget);
            setBusy(true);
            setProblem(null);
            setMessage(null);
            try {
              if (!smsSent) {
                phone.current = await signInWithFirebase(String(f.get("mobile")), "recaptcha");
                setSmsSent(true);
              } else {
                const token = await phone.current!.confirm(String(f.get("sms")));
                setBusy(false);
                return run("firebase", { firebase_id_token: token, device_label: "web" });
              }
            } catch {
              setMessage(t("failed"));
            }
            setBusy(false);
          }}
        >
          <Label htmlFor="mobile">{t("mobile")}</Label>
          <Input id="mobile" name="mobile" type="tel" autoComplete="tel" pattern="\+[1-9][0-9]{6,14}" required readOnly={smsSent} />
          {smsSent ? (
            <>
              <Label htmlFor="sms">{t("smsCode")}</Label>
              <Input id="sms" name="sms" inputMode="numeric" autoComplete="one-time-code" required autoFocus />
            </>
          ) : null}
          <div id="recaptcha" />
          {errors}
          <Button type="submit" disabled={busy}>{smsSent ? t("submit") : t("sendCode")}</Button>
        </form>
      )}
    </div>
  );
}
