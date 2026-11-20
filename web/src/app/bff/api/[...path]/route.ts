import { cookies } from "next/headers";
import { NextResponse, type NextRequest } from "next/server";
import { apiBase } from "@/server/refresh";
import { allowedApiPath, sameOrigin } from "@/server/rules";
import { ALL, AT, KIND, cookieOptions, type Kind } from "@/server/session";

// The browser's only door to the API: adds the bearer token from the httpOnly cookie. No business rules here.
const PASS_IN = ["content-type", "if-match", "idempotency-key", "accept"];
const PASS_OUT = ["content-type", "etag", "location", "retry-after", "content-disposition"];

async function forward(req: NextRequest, { params }: { params: Promise<{ path: string[] }> }) {
  const problem = (status: number, code: string) =>
    NextResponse.json({ status, code, title: code }, { status, headers: { "content-type": "application/problem+json" } });
  if (!sameOrigin(req.method, req.headers.get("origin"), req.headers.get("host"))) return problem(403, "FORBIDDEN");

  const jar = await cookies();
  const at = jar.get(AT)?.value;
  const kind = jar.get(KIND)?.value as Kind | undefined;
  const segs = (await params).path;
  const path = "/v1/" + segs.join("/");
  if (!at) return problem(401, "UNAUTHENTICATED");
  // A segment holding "/" (sent as %2F) would mean a different path to the API than the one checked.
  if (segs.some((s) => s === "" || s.includes("/")) || !allowedApiPath(kind, path)) return problem(404, "NOT_FOUND");

  const headers = new Headers({ authorization: `Bearer ${at}` });
  for (const h of PASS_IN) {
    const v = req.headers.get(h);
    if (v) headers.set(h, v);
  }
  const url = `${apiBase()}/v1/${segs.map(encodeURIComponent).join("/")}${req.nextUrl.search}`;
  const body = ["GET", "HEAD"].includes(req.method) ? undefined : await req.arrayBuffer();
  const up = await fetch(url, { method: req.method, headers, body, cache: "no-store", redirect: "manual" });

  const out = new Headers();
  for (const h of PASS_OUT) {
    const v = up.headers.get(h);
    if (v) out.set(h, v);
  }
  const res = new NextResponse(up.status === 204 ? null : up.body, { status: up.status, headers: out });
  // The proxy already tried a refresh; a 401 now means the session is over.
  if (up.status === 401) for (const n of ALL) res.cookies.set(n, "", cookieOptions(0));
  return res;
}

export { forward as GET, forward as POST, forward as PATCH, forward as PUT, forward as DELETE };
