"use client";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { NativeSelect } from "@/components/native-select";
import { ProblemAlert } from "@/components/problem-alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { bff, sendJson } from "@/lib/bff";
import { can } from "@/lib/menu";
import type { Problem } from "@/lib/problem";
import { useBff } from "@/lib/use-bff";
import type { ApproverRule, ApproverSource, Me, Page, UserAccount } from "@/types/api";
import { userLabel } from "../access-fields";

const TYPES = ["leave", "permission", "on_duty", "regularisation", "comp_off", "shift_change", "punch_flag", "period_unlock", "settings_change", "opening_balance"];
const SOURCES: ApproverSource[] = ["reporting_manager", "named_user", "department_head", "hr"];

/** Who approves each request type, level 1 and an optional level 2 (IAM-05, If-Match on the rules). */
export default function ApproversPage() {
  const t = useTranslations("acc");
  const c = useTranslations("common");
  const rules = useBff<{ items: ApproverRule[] }>("/approver-rules");
  const me = useBff<Me>("/me");
  const g = me.data?.grants ?? [];
  // Named users are picked from the user list where the caller may read it (IAM-01); otherwise only their id shows.
  const users = useBff<Page<UserAccount>>(can(g, "IAM-01") ? "/users?active=true" : "");
  const [type, setType] = useState(TYPES[0]);
  const named = (id: string | null) => {
    const u = users.data?.items.find((x) => x.id === id);
    return u ? userLabel(u) : (id ?? "");
  };
  const rule = (rt: string, level: number) => rules.data?.items.find((r) => r.request_type === rt && r.level === level);
  // A level with no rule uses its default (admin.rs: level 1 reporting manager, level 2 HR).
  const shown = (r: ApproverRule | undefined, level: 1 | 2) =>
    r ? `${t(`src.${r.source}`)}${r.source === "named_user" ? `: ${named(r.named_user_id)}` : ""}` : t(`default${level}`);
  return (
    <div className="grid max-w-4xl gap-4">
      <ProblemAlert problem={rules.problem} />
      {rules.loading && !rules.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {rules.data ? (
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("requestType")}</TableHead>
              <TableHead>{t("level1")}</TableHead>
              <TableHead>{t("level2")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {TYPES.map((rt) => (
              <TableRow key={rt}>
                <TableCell>{t(`rt.${rt}`)}</TableCell>
                <TableCell>{shown(rule(rt, 1), 1)}</TableCell>
                <TableCell>{shown(rule(rt, 2), 2)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      ) : null}
      {rules.data && can(g, "IAM-05") ? (
        <div className="grid gap-3 rounded-lg border p-4">
          <div className="grid max-w-xs gap-1">
            <Label htmlFor="ar-type">{t("editRule")}</Label>
            <NativeSelect id="ar-type" value={type} onChange={(e) => setType(e.target.value)}>
              {TYPES.map((rt) => <option key={rt} value={rt}>{t(`rt.${rt}`)}</option>)}
            </NativeSelect>
          </div>
          <RuleForm
            key={type}
            type={type}
            first={rule(type, 1)}
            second={rule(type, 2)}
            users={users.data?.items}
            etag={rules.etag}
            onSaved={rules.reload}
          />
        </div>
      ) : null}
    </div>
  );
}

function RuleForm({ type, first, second, users, etag, onSaved }: {
  type: string; first?: ApproverRule; second?: ApproverRule; users?: UserAccount[]; etag: string | null; onSaved: () => void;
}) {
  const t = useTranslations("acc");
  const [src, setSrc] = useState<(ApproverSource | "")[]>([first?.source ?? "", second?.source ?? ""]);
  const [named, setNamed] = useState([first?.named_user_id ?? "", second?.named_user_id ?? ""]); // controlled: users load later
  // Named users need the user list (IAM-01); without it the choice is not offered (HR: G-08 open).
  const sources = SOURCES.filter((s) => s !== "named_user" || users || src.includes("named_user"));
  const [problem, setProblem] = useState<Problem | null>(null);
  const [saved, setSaved] = useState(false);
  const [key, setKey] = useState(() => crypto.randomUUID());
  return (
    <form
      className="grid gap-3 sm:grid-cols-2"
      aria-label={`${t("editRule")} ${t(`rt.${type}`)}`}
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const levels = [1, 2].flatMap((level) => {
          const source = src[level - 1];
          if (!source) return [];
          return [{ level, source, ...(source === "named_user" ? { named_user_id: named[level - 1] } : {}) }];
        });
        setSaved(false);
        const r = await bff("/approver-rules/change", sendJson("POST", { request_type: type, levels, reason: String(f.get("reason") ?? "").trim() }, {
          "idempotency-key": key,
          "if-match": etag ?? "",
        }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setSaved(true);
        setKey(crypto.randomUUID());
        onSaved();
      }}
    >
      {[first, second].map((r, i) => (
        <div key={i} className="grid gap-2">
          <div className="grid gap-1">
            <Label htmlFor={`ar-src-${i + 1}`}>{i === 0 ? t("level1") : t("level2")}</Label>
            <NativeSelect id={`ar-src-${i + 1}`} value={src[i]} required={i === 0} onChange={(e) => setSrc((s) => s.map((x, j) => (j === i ? (e.target.value as ApproverSource | "") : x)))}>
              <option value="">{i === 0 ? "" : t("default2")}</option>
              {sources.map((s) => <option key={s} value={s}>{t(`src.${s}`)}</option>)}
            </NativeSelect>
          </div>
          {src[i] === "named_user" ? (
            <div className="grid gap-1">
              <Label htmlFor={`ar-named-${i + 1}`}>{t("namedUser")}</Label>
              <NativeSelect id={`ar-named-${i + 1}`} required value={named[i]} onChange={(e) => setNamed((n) => n.map((x, j) => (j === i ? e.target.value : x)))}>
                <option value="" />
                {r?.named_user_id && !users?.some((u) => u.id === r.named_user_id) ? <option value={r.named_user_id}>{r.named_user_id}</option> : null}
                {(users ?? []).map((u) => <option key={u.id} value={u.id}>{userLabel(u)}</option>)}
              </NativeSelect>
            </div>
          ) : null}
        </div>
      ))}
      <div className="grid gap-1 sm:col-span-2">
        <Label htmlFor="ar-reason">{t("reason")}</Label>
        <Input id="ar-reason" name="reason" required maxLength={1000} />
      </div>
      <Button type="submit" className="justify-self-start" disabled={!etag}>{t("saveRule")}</Button>
      <div className="sm:col-span-2">
        <ProblemAlert problem={problem} />
        {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
      </div>
    </form>
  );
}
