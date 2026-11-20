import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { landing } from "@/lib/menu";
import { api } from "@/server/api";
import { AT, KIND } from "@/server/session";
import type { Me } from "@/types/api";

// Where a signed-in user starts: vendor console, organisation admin, or self-service (from their grants).
export default async function Home() {
  const jar = await cookies();
  if (!jar.get(AT)) redirect("/sign-in");
  if (jar.get(KIND)?.value === "vendor") redirect("/vendor");
  const me = await api<Me>("/v1/me");
  redirect(me.ok ? landing(me.data.grants) : "/sign-in");
}
