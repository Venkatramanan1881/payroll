import type { Kind } from "./session";

/** Which API paths a session may reach through the proxy: vendor sessions only /v1/vendor/..., organisation sessions never. */
export function allowedApiPath(kind: Kind | undefined, path: string): boolean {
  // `.` and `..` segments would be collapsed by URL parsing upstream, after this check: refuse them outright.
  if (!kind || !/^\/v1\/[A-Za-z0-9\-_/.]*$/.test(path) || /(^|\/)\.\.?(\/|$)/.test(path) || path.includes("//")) return false;
  const vendor = path === "/v1/vendor" || path.startsWith("/v1/vendor/");
  if (path.startsWith("/v1/auth/")) return false; // sign-in steps go through /bff/auth only
  return kind === "vendor" ? vendor : !vendor;
}

/** CSRF: a state-changing request must come from this site. SameSite=Strict cookies are the first line; this is the second. */
export function sameOrigin(method: string, origin: string | null, host: string | null): boolean {
  if (["GET", "HEAD", "OPTIONS"].includes(method.toUpperCase())) return true;
  if (!origin || !host) return false;
  try {
    return new URL(origin).host === host;
  } catch {
    return false;
  }
}

/** Which area a page path belongs to, and so which session it needs. */
export function areaOf(path: string): "vendor" | "org" | "public" {
  if (path === "/vendor/sign-in") return "public";
  if (path === "/vendor" || path.startsWith("/vendor/")) return "vendor";
  if (path === "/app" || path.startsWith("/app/") || path === "/me" || path.startsWith("/me/")) return "org";
  return "public";
}
