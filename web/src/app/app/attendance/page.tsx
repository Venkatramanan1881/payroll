import { getTranslations } from "next-intl/server";
import { lookups } from "@/server/lookups";
import { DailyBoard } from "./daily-board";

export default async function AttendancePage() {
  const t = await getTranslations("attendance");
  const lk = await lookups();
  return (
    <section className="grid gap-4">
      <h1 className="text-xl font-semibold">{t("title")}</h1>
      <DailyBoard branches={lk.branches} departments={lk.departments} />
    </section>
  );
}
