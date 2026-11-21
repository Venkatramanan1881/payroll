"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import type { ShiftVersion } from "@/types/api";

/** A new draft version on a new date; the API copies the shift's latest version into it. It is then edited and published. */
export function NewVersion({ shiftId }: { shiftId: string }) {
  const t = useTranslations("sched");
  const router = useRouter();
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  return (
    <form
      className="grid max-w-xl gap-3 rounded-lg border p-4"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        setBusy(true);
        setProblem(null);
        const r = await bff<ShiftVersion>(`/shifts/${shiftId}/versions`, sendJson("POST", { effective_from: String(f.get("effective_from")), definition: {} }, { "idempotency-key": key }));
        setBusy(false);
        if (!r.ok) return setProblem(r.problem);
        router.push(`/app/shifts/${shiftId}/versions/${r.data.id}`);
      }}
    >
      <h2 className="font-medium">{t("newVersion")}</h2>
      <p className="text-sm text-muted-foreground">{t("newVersionHelp")}</p>
      <Label htmlFor="nv-from">{t("effectiveFrom")}</Label>
      <Input id="nv-from" name="effective_from" type="date" required className="max-w-xs" />
      <ProblemAlert problem={problem} />
      <Button type="submit" disabled={busy} className="justify-self-start">{t("newVersion")}</Button>
    </form>
  );
}
