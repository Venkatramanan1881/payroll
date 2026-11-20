import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { asProblem, type Problem } from "@/lib/problem";
import { apiBase } from "./refresh";
import { AT, KIND } from "./session";

export type ApiResult<T> = { ok: true; data: T; etag: string | null } | { ok: false; problem: Problem };

/** Server components read the API with the session cookie. A 401 here means the session ended: back to sign-in. */
export async function api<T>(path: string): Promise<ApiResult<T>> {
  const jar = await cookies();
  const at = jar.get(AT)?.value;
  const vendor = jar.get(KIND)?.value === "vendor";
  if (!at) redirect(vendor ? "/vendor/sign-in" : "/sign-in");
  const res = await fetch(`${apiBase()}${path}`, { headers: { authorization: `Bearer ${at}` }, cache: "no-store" });
  if (res.status === 401) redirect(vendor ? "/vendor/sign-in" : "/sign-in");
  const body = await res.json().catch(() => null);
  return res.ok ? { ok: true, data: body as T, etag: res.headers.get("etag") } : { ok: false, problem: asProblem(res.status, body) };
}

