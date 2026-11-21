"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { ProblemAlert } from "@/components/problem-alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { bff, sendJson } from "@/lib/bff";
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { InboxItem, Page } from "@/types/api";

// Where a notification's link opens: the owner's own request in self-service, an approver's in the admin area.
// Kinds without a page yet show no link.
const HREF: Record<InboxItem["recipient_kind"], Record<string, (id: string) => string>> = {
  employee: { leave: (id) => `/me/leave/${id}`, regularisation: () => "/me/attendance" },
  approver: { leave: (id) => `/app/leave/${id}`, regularisation: (id) => `/app/attendance/regularisations/${id}` },
};

/** The caller's in-app notifications (already in their language, from the API), with mark-read. */
export default function NotificationsPage() {
  const t = useTranslations("me");
  const c = useTranslations("common");
  const router = useRouter();
  const [unread, setUnread] = useState(false);
  const { data, problem, loading, reload } = useBff<Page<InboxItem> & { unread_count?: number }>(`/notifications/inbox${unread ? "?unread=true" : ""}`);
  const [busy, setBusy] = useState(false);
  const [markProblem, setMarkProblem] = useState<Problem | null>(null);
  const rows = data?.items ?? null;

  async function mark(body: { ids: string[] } | { all: true }) {
    setBusy(true);
    setMarkProblem(null);
    const r = await bff("/notifications/inbox/mark-read", sendJson("POST", body));
    setBusy(false);
    if (!r.ok) return setMarkProblem(r.problem);
    reload();
    router.refresh(); // the unread count in the header
  }

  return (
    <>
      <div className="flex flex-wrap items-center gap-3">
        <h1 className="flex-1 text-xl font-semibold">{t("nav.notifications")}</h1>
        <label className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={unread} onChange={(e) => setUnread(e.target.checked)} /> {t("onlyUnread")}
        </label>
        <Button variant="outline" size="sm" disabled={busy || !data?.unread_count} onClick={() => mark({ all: true })}>{t("markAllRead")}</Button>
      </div>
      <ProblemAlert problem={problem ?? markProblem} />
      {loading && !rows ? <Skeleton className="h-24" aria-busy="true" aria-label={c("loading")} /> : null}
      {rows && rows.length === 0 ? <p className="text-muted-foreground">{t("inboxEmpty")}</p> : null}
      <ul className="grid gap-2">
        {rows?.map((n) => {
          const href = n.link ? HREF[n.recipient_kind]?.[n.link.kind]?.(n.link.id) : undefined;
          return (
            <li key={n.id}>
              <Card className={n.read_at ? "opacity-70" : undefined}>
                <CardContent className="grid gap-1 pt-4">
                  <div className="flex flex-wrap items-center gap-2">
                    {!n.read_at ? <Badge>●</Badge> : null}
                    <span className="flex-1 font-medium">{n.title}</span>
                    <span className="text-xs text-muted-foreground">{wallDateTime(n.created_at)}</span>
                  </div>
                  <p className="text-sm">{n.body}</p>
                  <div className="flex gap-3 text-sm">
                    {href ? <Link className="underline" href={href} onClick={() => !n.read_at && mark({ ids: [n.id] })}>{t("open")}</Link> : null}
                    {!n.read_at ? <button className="underline" disabled={busy} onClick={() => mark({ ids: [n.id] })}>{t("markRead")}</button> : null}
                  </div>
                </CardContent>
              </Card>
            </li>
          );
        })}
      </ul>
    </>
  );
}
