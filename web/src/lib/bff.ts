import { asProblem, type Problem } from "./problem";

export type BffResult<T> = { ok: true; data: T; etag: string | null } | { ok: false; problem: Problem };

/** Browser calls go to the BFF (`/bff/api/<path>` → API `/v1/<path>`); the session cookie travels on its own. */
export async function bff<T>(path: string, init: RequestInit = {}): Promise<BffResult<T>> {
  let res: Response;
  try {
    res = await fetch(`/bff/api${path}`, { ...init, headers: { accept: "application/json", ...init.headers } });
  } catch {
    return { ok: false, problem: { status: 0, code: "network" } };
  }
  const body = res.status === 204 ? null : await res.json().catch(() => null);
  if (res.status === 401) window.location.assign(window.location.pathname.startsWith("/vendor") ? "/vendor/sign-in" : "/sign-in");
  return res.ok ? { ok: true, data: body as T, etag: res.headers.get("etag") } : { ok: false, problem: asProblem(res.status, body) };
}

export const sendJson = (method: string, body: unknown, headers: Record<string, string> = {}): RequestInit => ({
  method,
  headers: { "content-type": "application/json", ...headers },
  body: JSON.stringify(body),
});
