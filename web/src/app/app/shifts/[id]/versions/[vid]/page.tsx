import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me, ShiftVersion } from "@/types/api";
import { VersionForm } from "./version-form";

export default async function ShiftVersionPage({ params }: { params: Promise<{ id: string; vid: string }> }) {
  const { id, vid } = await params;
  const t = await getTranslations("sched");
  const [v, me] = await Promise.all([api<ShiftVersion>(`/v1/shift-versions/${encodeURIComponent(vid)}`), api<Me>("/v1/me")]);
  if (!v.ok) return <PageProblem problem={v.problem} />;
  const editable = v.data.state === "draft" && me.ok && can(me.data.grants, "SCH-01");
  return (
    <section className="grid max-w-3xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href={`/app/shifts/${id}`} className="text-sm underline">{t("versions")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{t("effectiveFrom")} {v.data.effective_from}</h1>
        <Badge variant="secondary">{t(`st.${v.data.state}`)}</Badge>
      </div>
      {!editable && v.data.state !== "draft" ? <p className="text-sm text-muted-foreground">{t("frozen")}</p> : null}
      <VersionForm version={v.data} etag={v.etag ?? ""} editable={editable} />
    </section>
  );
}
