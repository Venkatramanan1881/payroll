"use client";
import { useTranslations } from "next-intl";
import { Alert, AlertDescription } from "@/components/ui/alert";
import type { Problem } from "@/lib/problem";

/** One way to show an API refusal: the known message, plus the API's own detail when it gives one. */
export function ProblemAlert({ problem }: { problem: Problem | null }) {
  const t = useTranslations("problem");
  if (!problem) return null;
  const code = problem.status === 412 ? "PRECONDITION_FAILED" : problem.code;
  return (
    <Alert variant="destructive" role="alert">
      <AlertDescription>
        {t.has(code) ? t(code) : t("other", { code })}
        {problem.detail && code !== "network" ? <span className="block text-sm opacity-80">{problem.detail}</span> : null}
      </AlertDescription>
    </Alert>
  );
}
