import { NextResponse, type NextRequest } from "next/server";
import { callRefresh, refreshOnce } from "@/server/refresh";
import { areaOf } from "@/server/rules";
import { ALL, AT, KIND, RT, cookieOptions, sessionCookies, type CookieSpec } from "@/server/session";

// Runs before every page and BFF call: renews an expired access token from the refresh cookie (once, shared), and
// sends a request without the right session to its sign-in page. The API still authorises every call.
export async function proxy(req: NextRequest) {
  const path = req.nextUrl.pathname;
  let at = req.cookies.get(AT)?.value;
  const kind = req.cookies.get(KIND)?.value;
  const rt = req.cookies.get(RT)?.value;
  let fresh: CookieSpec[] = [];
  let clear = false;

  if (!at && rt && kind === "org") {
    let t;
    try {
      t = await refreshOnce(rt, callRefresh);
    } catch {
      // The API is down or failing, not refusing: keep the session and say so instead of signing the user out.
      return new NextResponse("The service is briefly unavailable. Please try again in a moment.", { status: 503, headers: { "retry-after": "5" } });
    }
    if (t) {
      fresh = sessionCookies("org", t);
      at = t.access_token;
      for (const c of fresh) req.cookies.set(c.name, c.value);
    } else {
      clear = true;
      for (const n of ALL) req.cookies.delete(n);
    }
  }

  const area = areaOf(path);
  const has = (k: string) => Boolean(at) && !clear && kind === k;
  let res: NextResponse;
  if (area === "org" && !has("org")) res = NextResponse.redirect(new URL("/sign-in", req.url));
  else if (area === "vendor" && !has("vendor")) res = NextResponse.redirect(new URL("/vendor/sign-in", req.url));
  else res = NextResponse.next({ request: { headers: req.headers } });

  for (const c of fresh) res.cookies.set(c.name, c.value, cookieOptions(c.maxAge));
  if (clear) for (const n of ALL) res.cookies.set(n, "", cookieOptions(0));
  return res;
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
