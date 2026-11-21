// Menus are a convenience built from the API's answers (/v1/me grants, /v1/me/modules); the API still decides every call.
export type GrantKind = "all" | "scoped" | "self" | "approve";
export type Grant = { operation_code: string; grant_kind: GrantKind };
export type ModuleState = { module_code: string; enabled: boolean };

/** Shown when the module is on and the user holds any of `ops` with one of `kinds` (default: all or scoped). */
export type MenuItem = { key: string; href: string; module: string | string[]; ops: string[]; kinds?: GrantKind[] };

const WORK: GrantKind[] = ["all", "scoped"];
const DECIDE: GrantKind[] = ["all", "scoped", "approve"];

// Each slice adds its items here.
export const ORG_MENU: MenuItem[] = [
  { key: "employees", href: "/app/employees", module: "C02", ops: ["WRK-08"] },
  { key: "attendance", href: "/app/attendance", module: "C06", ops: ["ATT-01"] },
  { key: "approvals", href: "/app/attendance/regularisations", module: "C06", ops: ["ATT-03", "ATT-04", "ATT-05"], kinds: DECIDE },
  { key: "flags", href: "/app/attendance/flags", module: "C04", ops: ["TCP-06"], kinds: DECIDE },
  { key: "leave", href: "/app/leave", module: "C08", ops: ["LVE-08", "LVE-04", "LVE-05"], kinds: DECIDE },
  { key: "shifts", href: "/app/shifts", module: "C05", ops: ["SCH-03"] },
  { key: "roster", href: "/app/roster", module: "C05", ops: ["SCH-03"] },
  { key: "holidays", href: "/app/holidays", module: "C07", ops: ["SCH-05"] },
  { key: "requests", href: "/app/requests", module: ["C09", "M02"], ops: ["PRM-03", "PRM-04", "PRM-07", "PRM-09", "PRM-10", "PRM-11", "PRM-13", "CMP-05", "CMP-07"], kinds: DECIDE },
];

export function holds(grants: Grant[], ops: string[], kinds: GrantKind[] = WORK): boolean {
  return grants.some((g) => ops.includes(g.operation_code) && kinds.includes(g.grant_kind));
}

export function buildMenu(grants: Grant[], modules: ModuleState[], items = ORG_MENU): MenuItem[] {
  const on = new Set(modules.filter((m) => m.enabled).map((m) => m.module_code));
  return items.filter((i) => [i.module].flat().some((m) => on.has(m)) && holds(grants, i.ops, i.kinds));
}

/** Anyone who works on others' records or decides their requests uses /app; everyone else uses self-service. */
export function landing(grants: Grant[]): "/app" | "/me" {
  return grants.some((g) => DECIDE.includes(g.grant_kind)) ? "/app" : "/me";
}

/** Whether to offer a control for `op` (all or scoped grant). The API still decides. */
export const can = (grants: Grant[], op: string, kinds: GrantKind[] = WORK) => holds(grants, [op], kinds);
