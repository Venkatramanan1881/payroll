import { api } from "./api";
import type { Branch, ListEntry, Page } from "@/types/api";

export type Option = { id: string; name: string; tz?: string };
const opt = (x: { id: string; name: string; code?: string | null; tz?: string | null }): Option => ({
  id: x.id,
  name: x.code ? `${x.name} (${x.code})` : x.name,
  ...(x.tz ? { tz: x.tz } : {}),
});

// shortcut: the first 200 of each list; add search-as-you-type pickers when an organisation outgrows that.
export async function lookups() {
  const [branches, departments, designations, categories, types] = await Promise.all([
    api<Page<Branch>>("/v1/branches?limit=200&sort=name"),
    ...["department", "designation", "category", "employee_type"].map((k) => api<Page<ListEntry>>(`/v1/lists/${k}?limit=200`)),
  ]);
  const list = (r: typeof departments) => (r.ok ? r.data.items.map(opt) : []);
  return {
    branches: branches.ok ? branches.data.items.map(opt) : [],
    departments: list(departments),
    designations: list(designations),
    categories: list(categories),
    employeeTypes: list(types),
  };
}
export type Lookups = Awaited<ReturnType<typeof lookups>>;
