// RFC 9457 problem bodies from the API (common.yaml Problem). The web app shows them; it never re-decides them.
export type Problem = {
  status: number;
  code: string;
  title?: string;
  detail?: string;
  errors?: { field?: string; code: string; message?: string }[];
  [k: string]: unknown;
};

export function asProblem(status: number, body: unknown): Problem {
  const b = (body && typeof body === "object" ? body : {}) as Partial<Problem>;
  return { ...b, status, code: typeof b.code === "string" ? b.code : status >= 500 ? "INTERNAL" : "UNKNOWN" };
}

/** Field name → message, for errors that name a field. The rest go to the form banner. */
export function fieldErrors(p: Problem | null): Record<string, string> {
  const out: Record<string, string> = {};
  for (const e of p?.errors ?? []) if (e.field && !out[e.field]) out[e.field] = e.message ?? e.code;
  return out;
}
