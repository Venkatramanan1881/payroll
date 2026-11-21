"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { definitionValues, ShiftDefinitionFields } from "@/components/shift-definition-fields";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { definitionBody, type ShiftType } from "@/lib/shift-definition";
import type { Shift } from "@/types/api";

/** SCH-01: a new shift with its first (draft) version. Publishing is a separate step on the shift's page. */
export default function NewShiftPage() {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const router = useRouter();
  const [type, setType] = useState<ShiftType>("fixed");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  return (
    <section className="grid max-w-3xl gap-4">
      <h1 className="text-xl font-semibold">{t("newShift")}</h1>
      <form
        className="grid gap-4"
        onSubmit={async (e) => {
          e.preventDefault();
          const f = new FormData(e.currentTarget);
          setBusy(true);
          setProblem(null);
          const r = await bff<Shift>("/shifts", sendJson("POST", {
            code: String(f.get("code") ?? "").trim(),
            name: String(f.get("name") ?? "").trim(),
            effective_from: String(f.get("effective_from")),
            definition: definitionBody(type, definitionValues(f), false),
          }, { "idempotency-key": key }));
          setBusy(false);
          if (!r.ok) return setProblem(r.problem);
          router.push(`/app/shifts/${r.data.id}`);
        }}
      >
        <div className="grid gap-3 sm:grid-cols-3">
          <div className="grid gap-1">
            <Label htmlFor="code">{t("code")}</Label>
            <Input id="code" name="code" required maxLength={20} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="name">{t("name")}</Label>
            <Input id="name" name="name" required maxLength={100} />
          </div>
          <div className="grid gap-1">
            <Label htmlFor="effective_from">{t("effectiveFrom")}</Label>
            <Input id="effective_from" name="effective_from" type="date" required />
          </div>
        </div>
        <ShiftDefinitionFields type={type} onType={setType} />
        <ProblemAlert problem={problem} />
        <div className="flex gap-3">
          <Button type="submit" disabled={busy}>{busy ? c("saving") : t("create")}</Button>
          <Button type="button" variant="outline" onClick={() => router.back()}>{c("cancel")}</Button>
        </div>
      </form>
    </section>
  );
}
