// Session cookies (D-42): tokens live only in httpOnly cookies, never in page scripts.
export const AT = "pa_at"; // access token
export const RT = "pa_rt"; // refresh token (organisation users only)
export const KIND = "pa_kind"; // "org" | "vendor"

export type Kind = "org" | "vendor";
export type TokenPair = { access_token: string; refresh_token?: string; expires_in: number };

export type CookieSpec = { name: string; value: string; maxAge: number };
const REFRESH_DAYS = 30; // matches REFRESH_TTL_DAYS in domain-iam; the API decides, this only sizes the cookie

/** The cookies a fresh token pair becomes. The access cookie ends 30 s early so it is refreshed before the API refuses it. */
export function sessionCookies(kind: Kind, t: TokenPair): CookieSpec[] {
  const life = REFRESH_DAYS * 86400;
  const out: CookieSpec[] = [
    { name: AT, value: t.access_token, maxAge: Math.max(t.expires_in - 30, 1) },
    { name: KIND, value: kind, maxAge: kind === "org" ? life : t.expires_in },
  ];
  if (kind === "org" && t.refresh_token) out.push({ name: RT, value: t.refresh_token, maxAge: life });
  return out;
}

export const cookieOptions = (maxAge: number) => ({
  httpOnly: true,
  secure: process.env.NODE_ENV === "production",
  sameSite: "strict" as const,
  path: "/",
  maxAge,
});

export const ALL = [AT, RT, KIND];
