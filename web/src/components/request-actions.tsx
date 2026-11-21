"use client";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { Decide } from "@/components/decide";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";

/** Approve / reject and cancel for one request at `path`, shown only when the API says the caller may. */
export function RequestActions({ path, decide, cancel }: { path: string; decide: boolean; cancel: boolean }) {
  const router = useRouter();
  if (!decide && !cancel) return null;
  return (
    <Card>
      <CardContent className="grid gap-6 pt-6">
        {decide ? <Decide base={path} yes="approve" onDone={() => router.refresh()} /> : null}
        {cancel ? <Cancel path={path} onDone={() => router.refresh()} /> : null}
      </CardContent>
    </Card>
  );
}

function Cancel({ path, onDone }: { path: string; onDone: () => void }) {
  const t = useTranslations("req");
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState("");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key] = useState(() => crypto.randomUUID());
  if (!open) return <Button variant="outline" className="justify-self-start" onClick={() => setOpen(true)}>{t("cancel")}…</Button>;
  return (
    <div className="grid gap-2">
      <Label htmlFor="cancel-reason">{t("cancelReason")}</Label>
      <Input id="cancel-reason" value={reason} onChange={(e) => setReason(e.target.value)} maxLength={1000} required />
      <Button
        variant="destructive"
        className="justify-self-start"
        disabled={busy || !reason.trim()}
        onClick={async () => {
          setBusy(true);
          setProblem(null);
          const r = await bff(`${path}/cancel`, sendJson("POST", { reason: reason.trim() }, { "idempotency-key": key }));
          setBusy(false);
          if (!r.ok) return setProblem(r.problem);
          onDone();
        }}
      >
        {t("confirmCancel")}
      </Button>
      <ProblemAlert problem={problem} />
    </div>
  );
}
