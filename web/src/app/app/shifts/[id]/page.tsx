import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me, Shift } from "@/types/api";
import { NewVersion } from "./new-version";

const hm = (t: string | null | undefined) => (t ? t.slice(0, 5) : "");

/** A shift and its dated versions (G5): drafts can be edited and published; published ones are frozen. */
export default async function ShiftPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const t = await getTranslations("sched");
  const [s, me] = await Promise.all([api<Shift>(`/v1/shifts/${encodeURIComponent(id)}`), api<Me>("/v1/me")]);
  if (!s.ok) return <PageProblem problem={s.problem} />;
  const change = me.ok && can(me.data.grants, "SCH-01");
  const versions = [...s.data.versions].sort((a, b) => b.effective_from.localeCompare(a.effective_from));
  return (
    <section className="grid max-w-4xl gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <Link href="/app/shifts" className="text-sm underline">{t("shifts")}</Link>
        <h1 className="flex-1 text-xl font-semibold">{s.data.code} · {s.data.name}</h1>
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{t("effectiveFrom")}</TableHead>
            <TableHead>{t("state")}</TableHead>
            <TableHead>{t("type")}</TableHead>
            <TableHead>{t("times")}</TableHead>
            <TableHead />
          </TableRow>
        </TableHeader>
        <TableBody>
          {versions.map((v) => (
            <TableRow key={v.id}>
              <TableCell>{v.effective_from}</TableCell>
              <TableCell><Badge variant={v.state === "in_force" ? "default" : "secondary"}>{t(`st.${v.state}`)}</Badge></TableCell>
              <TableCell>{t(`ty.${v.shift_type}`)}</TableCell>
              <TableCell>{hm(v.start_time)}{v.end_time ? `–${hm(v.end_time)}` : ""}</TableCell>
              <TableCell>
                <Link href={`/app/shifts/${s.data.id}/versions/${v.id}`} className="text-sm underline">
                  {v.state === "draft" && change ? t("edit") : t("definition")}
                </Link>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {change ? <NewVersion shiftId={s.data.id} /> : null}
    </section>
  );
}
