import { cookies } from "next/headers";
import { NextResponse, type NextRequest } from "next/server";
import { apiBase } from "@/server/refresh";
import { sameOrigin } from "@/server/rules";
import { ALL, AT, KIND, cookieOptions, sessionCookies, type Kind, type TokenPair } from "@/server/session";

// Sign-in steps pass straight through to the API; on success the tokens become httpOnly cookies and the browser
// only learns "ok". Step tokens (challenge, organisation choice) are short-lived and must reach the browser.
const STEPS: Record<string, { path: string; kind: Kind }> = {
  password: { path: "/v1/auth/password/sign-in", kind: "org" },
  "second-factor": { path: "/v1/auth/second-factor/verify", kind: "org" },
  "organisation-choice": { path: "/v1/auth/organisation-choice", kind: "org" },
  firebase: { path: "/v1/auth/firebase/sign-in", kind: "org" },
  "vendor-google": { path: "/v1/vendor/auth/google/sign-in", kind: "vendor" },
};

export async function POST(req: NextRequest, { params }: { params: Promise<{ step: string }> }) {
  if (!sameOrigin(req.method, req.headers.get("origin"), req.headers.get("host")))
    return NextResponse.json({ status: 403, code: "FORBIDDEN" }, { status: 403 });
  const { step } = await params;
  const jar = await cookies();

  if (step === "sign-out") {
    const at = jar.get(AT)?.value;
    if (at && jar.get(KIND)?.value === "org")
      await fetch(`${apiBase()}/v1/auth/sign-out`, { method: "POST", headers: { authorization: `Bearer ${at}` }, cache: "no-store" }).catch(() => undefined);
    const res = NextResponse.json({ ok: true });
    for (const n of ALL) res.cookies.set(n, "", cookieOptions(0));
    return res;
  }

  const s = STEPS[step];
  if (!s) return NextResponse.json({ status: 404, code: "NOT_FOUND" }, { status: 404 });
  const up = await fetch(`${apiBase()}${s.path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: await req.text(),
    cache: "no-store",
  });
  const body = await up.json().catch(() => ({}));
  if (!up.ok) return NextResponse.json(body, { status: up.status, headers: { "content-type": "application/problem+json" } });

  const res = NextResponse.json({ ok: true });
  for (const n of ALL) res.cookies.set(n, "", cookieOptions(0)); // never mix an old session with the new one
  for (const c of sessionCookies(s.kind, body as TokenPair)) res.cookies.set(c.name, c.value, cookieOptions(c.maxAge));
  return res;
}
