// The three request types of this slice, as the API names them. Paths only: every rule stays in the API.
export type RequestKind = "permission" | "on_duty" | "comp_off";

export const KINDS: Record<RequestKind, { list: string; one: (id: string) => string; ops: string[] }> = {
  permission: { list: "/permissions/requests", one: (id) => `/permissions/requests/${id}`, ops: ["PRM-03", "PRM-04", "PRM-07", "PRM-13"] },
  on_duty: { list: "/on-duty/requests", one: (id) => `/on-duty/requests/${id}`, ops: ["PRM-09", "PRM-10", "PRM-11"] },
  comp_off: { list: "/comp-off/credits", one: (id) => `/comp-off/credits/${id}`, ops: ["CMP-05", "CMP-07"] },
};

export const isKind = (k: string): k is RequestKind => k in KINDS;
