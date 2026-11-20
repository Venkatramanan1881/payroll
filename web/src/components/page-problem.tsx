import { notFound } from "next/navigation";
import { ProblemAlert } from "@/components/problem-alert";
import type { Problem } from "@/lib/problem";

/** A page whose main read was refused: 404 becomes the not-found page; anything else is shown as the API said it. */
export function PageProblem({ problem }: { problem: Problem }) {
  if (problem.status === 404) notFound();
  return <ProblemAlert problem={problem} />;
}
