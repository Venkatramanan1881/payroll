import { getTranslations } from "next-intl/server";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { api } from "@/server/api";
import type { Page, VendorOrganisation } from "@/types/api";

// shortcut: first 200 organisations, read-only; the vendor screens get their own slice.
export default async function VendorHome() {
  const t = await getTranslations("vendor");
  const r = await api<Page<VendorOrganisation>>("/v1/vendor/organisations?limit=200");
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("organisations")}</h1>
      {!r.ok ? (
        <ProblemAlert problem={r.problem} />
      ) : r.data.items.length === 0 ? (
        <p className="text-muted-foreground">{t("empty")}</p>
      ) : (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("name")}</TableHead>
              <TableHead>{t("legalName")}</TableHead>
              <TableHead>{t("state")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {r.data.items.map((o) => (
              <TableRow key={o.id}>
                <TableCell>{o.display_name}</TableCell>
                <TableCell>{o.legal_name}</TableCell>
                <TableCell>
                  <Badge variant={o.state === "active" ? "secondary" : "destructive"}>{t(o.state)}</Badge>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </section>
  );
}
