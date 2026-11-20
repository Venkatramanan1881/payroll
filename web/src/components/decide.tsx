"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";

/**
 * Approve/accept with an optional comment, or reject with a reason, for one item at `base`
 * (`${base}/approve` or `/accept`, `${base}/reject`). One Idempotency-Key per decision, so a double click decides once.
 */
export function Decide({ base, yes, onDone }: { base: string; yes: "approve" | "accept"; onDone: () => void }) {
  const t = useTranslations("decide");
  const [rejecting, setRejecting] = useState(false);
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());

  async function send(path: string, body: Record<string, string>) {
    setBusy(true);
    setProblem(null);
    const r = await bff(`${base}/${path}`, sendJson("POST", body, { "idempotency-key": key }));
    setBusy(false);
    if (!r.ok) return setProblem(r.problem);
    setKey(crypto.randomUUID());
    onDone();
  }

  const id = base.replace(/[^a-z0-9]/gi, "");
  return (
    <div className="grid gap-2">
      <div className="grid gap-1">
        <Label htmlFor={`note-${id}`}>{rejecting ? t("reason") : t("comment")}</Label>
        <Input id={`note-${id}`} value={note} onChange={(e) => setNote(e.target.value)} maxLength={1000} required={rejecting} />
      </div>
      <div className="flex flex-wrap gap-2">
        {rejecting ? (
          <>
            <Button variant="destructive" disabled={busy || !note.trim()} onClick={() => send("reject", { reason: note.trim() })}>{t("confirmReject")}</Button>
            <Button variant="ghost" disabled={busy} onClick={() => { setRejecting(false); setNote(""); }}>{t("back")}</Button>
          </>
        ) : (
          <>
            <Button disabled={busy} onClick={() => send(yes, note.trim() ? { comment: note.trim() } : {})}>{t(yes)}</Button>
            <Button variant="outline" disabled={busy} onClick={() => { setRejecting(true); setNote(""); }}>{t("reject")}…</Button>
          </>
        )}
      </div>
      <ProblemAlert problem={problem} />
    </div>
  );
}
