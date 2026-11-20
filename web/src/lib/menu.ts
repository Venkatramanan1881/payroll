// Menus are a convenience built from the API's answers (/v1/me grants, /v1/me/modules); the API still decides every call.
export type Grant = { operation_code: string; grant_kind: "all" | "scoped" | "self" | "approve" };
export type ModuleState = { module_code: string; enabled: boolean };

export type MenuItem = { key: string; href: string; module: string; op: string };

// Each slice adds its items here.
export const ORG_MENU: MenuItem[] = [{ key: "employees", href: "/app/employees", module: "C02", op: "WRK-08" }];

// Acting on others' records needs an `all` or `scoped` grant; `self` and `approve` grants do not open admin screens.
const holds = (grants: Grant[], op: string) => grants.some((g) => g.operation_code === op && (g.grant_kind === "all" || g.grant_kind === "scoped"));

export function buildMenu(grants: Grant[], modules: ModuleState[], items = ORG_MENU): MenuItem[] {
  const on = new Set(modules.filter((m) => m.enabled).map((m) => m.module_code));
  return items.filter((i) => on.has(i.module) && holds(grants, i.op));
}

/** A user with any `all` or `scoped` grant works in /app; everyone else uses self-service. */
export function landing(grants: Grant[]): "/app" | "/me" {
  return grants.some((g) => g.grant_kind === "all" || g.grant_kind === "scoped") ? "/app" : "/me";
}

export const can = (grants: Grant[], op: string) => holds(grants, op);
