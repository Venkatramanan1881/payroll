import type { TokenPair } from "./session";

// The API rotates refresh tokens and refuses the old one, so parallel requests that all find the access token
// expired must share one refresh, or all but the first would sign the user out.
// shortcut: per server instance only; with several instances a rare parallel refresh can still lose. Move to a
// shared store (Redis) if that shows up.
const inflight = new Map<string, { at: number; p: Promise<TokenPair | null> }>();
const KEEP_MS = 30_000;

/**
 * Resolves to new tokens, or null when the API refused the refresh token (the session is over). Rejects when the API
 * could not be reached or failed: the refresh token may still be good, so nothing is cached and the user stays signed in.
 */
export function refreshOnce(rt: string, call: (rt: string) => Promise<TokenPair | null>, now = Date.now()): Promise<TokenPair | null> {
  for (const [k, v] of inflight) if (now - v.at > KEEP_MS) inflight.delete(k);
  const hit = inflight.get(rt);
  if (hit) return hit.p;
  const p = call(rt);
  p.catch(() => inflight.delete(rt));
  inflight.set(rt, { at: now, p });
  return p;
}

export async function callRefresh(rt: string): Promise<TokenPair | null> {
  const res = await fetch(`${apiBase()}/v1/auth/refresh`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ refresh_token: rt }),
    cache: "no-store",
  });
  if (res.ok) return (await res.json()) as TokenPair;
  if (res.status === 400 || res.status === 401 || res.status === 422) return null;
  throw new Error(`refresh failed with ${res.status}`);
}

export function apiBase(): string {
  const b = process.env.API_BASE_URL;
  if (!b) throw new Error("API_BASE_URL is required");
  return b.replace(/\/$/, "");
}
