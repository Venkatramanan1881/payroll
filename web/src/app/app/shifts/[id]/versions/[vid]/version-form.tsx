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
import { definitionBody, SHIFT_TYPES, type ShiftType } from "@/lib/shift-definition";
import type { ShiftVersion } from "@/types/api";

/** A version's definition: editable while a draft (PATCH with If-Match), then published; read-only afterwards. */
export function VersionForm({ version, etag, editable }: { version: ShiftVersion; etag: string; editable: boolean }) {
  const t = useTranslations("sched");
  const c = useTranslations("common");
  const router = useRouter();
  const known = (SHIFT_TYPES as readonly string[]).includes(version.shift_type);
  const [type, setType] = useState<ShiftType>(known ? (version.shift_type as ShiftType) : "fixed");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false); // Publish sends the saved draft, so unsaved edits must be saved first
  const [key] = useState(() => crypto.randomUUID());

  if (!known) return <p className="text-sm text-muted-foreground">{t(`ty.${version.shift_type}`)}</p>;
  return (
    <form
      className="grid gap-4"
      onChange={() => setDirty(true)}
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        setBusy(true);
        setProblem(null);
        setStatus(null);
        const body = { effective_from: String(f.get("effective_from")), ...definitionBody(type, definitionValues(f), true) };
        const r = await bff<ShiftVersion>(`/shift-versions/${version.id}`, sendJson("PATCH", body, { "if-match": etag }));
        setBusy(false);
        if (!r.ok) return setProblem(r.problem);
        setStatus(t("saved"));
        setDirty(false);
        router.refresh(); // new ETag and values from the server
      }}
    >
      <fieldset disabled={!editable || busy} className="grid gap-4">
        <div className="grid max-w-xs gap-1">
          <Label htmlFor="v-from">{t("effectiveFrom")}</Label>
          <Input id="v-from" name="effective_from" type="date" required defaultValue={version.effective_from} />
        </div>
        <ShiftDefinitionFields type={type} onType={(x) => { setType(x); setDirty(true); }} initial={version} />
      </fieldset>
      <ProblemAlert problem={problem} />
      {status ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{status}</p> : null}
      {editable ? (
        <div className="flex gap-3">
          <Button type="submit" disabled={busy}>{busy ? c("saving") : c("save")}</Button>
          <Button
            type="button"
            variant="secondary"
            disabled={busy || dirty}
            onClick={async () => {
              setBusy(true);
              setProblem(null);
              const r = await bff(`/shift-versions/${version.id}/publish`, sendJson("POST", {}, { "idempotency-key": key }));
              setBusy(false);
              if (!r.ok) return setProblem(r.problem);
              setStatus(t("published"));
              router.refresh();
            }}
          >
            {t("publish")}
          </Button>
          {dirty ? <span className="self-center text-xs text-muted-foreground">{t("saveFirst")}</span> : null}
        </div>
      ) : null}
    </form>
  );
}
