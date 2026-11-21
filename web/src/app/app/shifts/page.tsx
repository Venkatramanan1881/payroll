import { getTranslations } from "next-intl/server";
import Link from "next/link";
import { PageProblem } from "@/components/page-problem";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { can } from "@/lib/menu";
import { api } from "@/server/api";
import type { Me, Page, Shift } from "@/types/api";
import { Patterns } from "./patterns";

const hm = (t: string | null | undefined) => (t ? t.slice(0, 5) : "");

export default async function ShiftsPage() {
  const t = await getTranslations("sched");
  const [shifts, me] = await Promise.all([api<Page<Shift>>("/v1/shifts"), api<Me>("/v1/me")]);
  const grants = me.ok ? me.data.grants : [];
  return (
    <section className="grid gap-4">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">{t("shifts")}</h1>
        {can(grants, "SCH-01") ? (
          <Button asChild>
            <Link href="/app/shifts/new">{t("newShift")}</Link>
          </Button>
        ) : null}
      </div>
      {!shifts.ok ? (
        <PageProblem problem={shifts.problem} />
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("code")}</TableHead>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("inForce")}</TableHead>
              <TableHead>{t("versions")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {shifts.data.items.map((s) => {
              const v = s.current_version;
              const drafts = s.versions.filter((x) => x.state === "draft").length;
              return (
                <TableRow key={s.id}>
                  <TableCell className="font-mono">
                    <Link href={`/app/shifts/${s.id}`} className="underline-offset-2 hover:underline">{s.code}</Link>
                  </TableCell>
                  <TableCell>{s.name}</TableCell>
                  <TableCell>{v ? `${t(`ty.${v.shift_type}`)} ${hm(v.start_time)}${v.end_time ? `–${hm(v.end_time)}` : ""}` : t("none")}</TableCell>
                  <TableCell>
                    {s.versions.length}
                    {drafts ? <Badge variant="outline" className="ml-2">{t("st.draft")}</Badge> : null}
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      )}
      <Patterns canChange={can(grants, "SCH-04")} />
    </section>
  );
}
