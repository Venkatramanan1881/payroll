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
import type { Problem } from "@/lib/problem";
import { wallDateTime } from "@/lib/time";
import { useBff } from "@/lib/use-bff";
import type { Option } from "@/server/lookups";
import type { OrgSettings, OrgSettingsVersion, Page } from "@/types/api";

// The choices organisation.yaml OrganisationSettings allows; the API validates the document.
const CHOICES: Record<string, string[]> = {
  week_start: ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"],
  employee_code_mode: ["automatic", "manual"],
  contact_self_edit: ["off", "on_with_approval", "on_without_approval"],
  rejoin_code: ["new_code", "reuse_old_code"],
  allowed_branches_mode: ["master_only", "chosen_branches", "all_branches"],
  non_allowed_branch_punch: ["block", "record_and_flag"],
  other_branch_costing: ["branch_worked", "master_branch"],
};
const LANGUAGES = ["en", "ta", "hi"];
const str = (v: unknown) => (typeof v === "string" || typeof v === "number" ? String(v) : "");

/** Organisation settings (OR-01..04, EM-01..08): the version in force, all versions, and a new version for ORG-06. */
export function OrgSettingsView({ canChange, branches }: { canChange: boolean; branches: Option[] }) {
  const t = useTranslations("org");
  const c = useTranslations("common");
  const current = useBff<OrgSettingsVersion>("/organisation-settings");
  const versions = useBff<Page<OrgSettingsVersion>>("/organisation-settings/versions");
  // Kept here: a back-dated version becomes the one in force, and the form below starts again from it.
  const [saved, setSaved] = useState(false);
  const reload = () => {
    setSaved(true);
    current.reload();
    versions.reload();
  };
  return (
    <div className="grid gap-6">
      {/* No version yet answers 404: the form below then starts empty. */}
      <ProblemAlert problem={current.problem?.status === 404 ? null : current.problem} />
      {current.loading && !current.data ? <Skeleton className="h-16" aria-busy="true" aria-label={c("loading")} /> : null}
      {current.data ? <p className="text-sm text-muted-foreground">{t("inForce", { date: current.data.effective_from })}</p> : null}
      {current.problem?.status === 404 ? <p className="text-sm text-muted-foreground">{t("noSettings")}</p> : null}
      {/* Shown once the first answer is in; it starts again from a new version in force. */}
      {canChange && (current.data || current.problem?.status === 404) ? (
        <NewVersion key={current.data?.id ?? "none"} base={current.data?.settings ?? {}} branches={branches} onSaving={() => setSaved(false)} onSaved={reload} />
      ) : null}
      {saved ? <p role="status" className="text-sm text-green-700 dark:text-green-400">{t("saved")}</p> : null}
      <div className="grid gap-2">
        <h2 className="text-lg font-semibold">{t("versions")}</h2>
        <ProblemAlert problem={versions.problem} />
        {versions.data ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("from")}</TableHead>
                <TableHead>{t("reason")}</TableHead>
                <TableHead>{t("savedAt")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {versions.data.items.map((v) => (
                <TableRow key={v.id}>
                  <TableCell>{v.effective_from}</TableCell>
                  <TableCell>{v.reason ?? ""}</TableCell>
                  <TableCell>{wallDateTime(v.created_at)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : null}
      </div>
    </div>
  );
}

function NewVersion({ base, branches, onSaving, onSaved }: { base: OrgSettings; branches: Option[]; onSaving: () => void; onSaved: () => void }) {
  const t = useTranslations("org");
  const fy = (base.financial_year ?? {}) as { kind?: string; custom_start_month?: number };
  const [fyKind, setFyKind] = useState(fy.kind ?? "");
  const [codes, setCodes] = useState(str(base.employee_code_mode));
  const [problem, setProblem] = useState<Problem | null>(null);
  const [key, setKey] = useState(() => crypto.randomUUID());
  const prefixes = (base.branch_code_prefixes ?? {}) as Record<string, string>;
  const langs = Array.isArray(base.languages) ? (base.languages as string[]) : [];
  const fields = Array.isArray(base.mandatory_fields) ? (base.mandatory_fields as string[]) : [];
  return (
    <form
      className="grid gap-3 rounded-lg border p-4 sm:grid-cols-3"
      onSubmit={async (e) => {
        e.preventDefault();
        const f = new FormData(e.currentTarget);
        const text = (k: string) => String(f.get(k) ?? "").trim();
        const num = (k: string) => (text(k) === "" ? undefined : Number(text(k)));
        const settings: OrgSettings = {
          financial_year: { kind: fyKind, custom_start_month: fyKind === "custom" ? num("custom_start_month") : undefined },
          languages: LANGUAGES.filter((l) => f.get(`lang-${l}`)),
          currency: text("currency"),
          number_format: text("number_format") || undefined,
          mandatory_fields: text("mandatory_fields").split(",").map((x) => x.trim()).filter(Boolean),
          probation_months: num("probation_months"),
          confirmation_reminder_days: num("confirmation_reminder_days"),
        };
        for (const k of Object.keys(CHOICES)) settings[k] = text(k);
        if (codes === "automatic") {
          const p = Object.fromEntries(branches.map((b) => [b.id, text(`prefix-${b.id}`)]).filter(([, v]) => v));
          if (Object.keys(p).length) settings.branch_code_prefixes = p;
        }
        onSaving();
        const r = await bff("/organisation-settings/versions", sendJson("POST", {
          effective_from: text("effective_from"),
          reason: text("reason"),
          schema_version: 1,
          settings,
        }, { "idempotency-key": key }));
        if (!r.ok) return setProblem(r.problem);
        setProblem(null);
        setKey(crypto.randomUUID());
        onSaved();
      }}
    >
      <h2 className="text-lg font-semibold sm:col-span-3">{t("newVersion")}</h2>
      <p className="text-sm text-muted-foreground sm:col-span-3">{t("newVersionHelp")}</p>
      <div className="grid gap-1">
        <Label htmlFor="os-fy">{t("s.financial_year")}</Label>
        <NativeSelect id="os-fy" value={fyKind} required onChange={(e) => setFyKind(e.target.value)}>
          <option value="" />
          {["calendar_year", "april_to_march", "custom"].map((o) => <option key={o} value={o}>{t(`v.${o}`)}</option>)}
        </NativeSelect>
      </div>
      {fyKind === "custom" ? (
        <div className="grid gap-1">
          <Label htmlFor="os-fym">{t("s.custom_start_month")}</Label>
          <Input id="os-fym" name="custom_start_month" type="number" min={1} max={12} required defaultValue={fy.custom_start_month ?? ""} />
        </div>
      ) : null}
      {Object.entries(CHOICES).map(([k, opts]) => (
        <div key={k} className="grid gap-1">
          <Label htmlFor={`os-${k}`}>{t(`s.${k}`)}</Label>
          <NativeSelect
            id={`os-${k}`}
            name={k}
            required
            defaultValue={str(base[k])}
            onChange={k === "employee_code_mode" ? (e) => setCodes(e.target.value) : undefined}
          >
            <option value="" />
            {opts.map((o) => <option key={o} value={o}>{t(`v.${o}`)}</option>)}
          </NativeSelect>
        </div>
      ))}
      <fieldset className="grid gap-1">
        <legend className="text-sm font-medium">{t("s.languages")}</legend>
        <div className="flex gap-4">
          {LANGUAGES.map((l) => (
            <label key={l} className="flex items-center gap-1 text-sm">
              <input type="checkbox" name={`lang-${l}`} defaultChecked={langs.includes(l)} />
              {t(`lang.${l}`)}
            </label>
          ))}
        </div>
      </fieldset>
      <div className="grid gap-1">
        <Label htmlFor="os-cur">{t("s.currency")}</Label>
        <Input id="os-cur" name="currency" required pattern="[A-Z]{3}" maxLength={3} defaultValue={str(base.currency)} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="os-nf">{t("s.number_format")}</Label>
        <Input id="os-nf" name="number_format" maxLength={30} defaultValue={str(base.number_format)} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="os-prob">{t("s.probation_months")}</Label>
        <Input id="os-prob" name="probation_months" type="number" min={0} max={60} required defaultValue={str(base.probation_months)} />
      </div>
      <div className="grid gap-1">
        <Label htmlFor="os-rem">{t("s.confirmation_reminder_days")}</Label>
        <Input id="os-rem" name="confirmation_reminder_days" type="number" min={0} defaultValue={str(base.confirmation_reminder_days)} />
      </div>
      <div className="grid gap-1 sm:col-span-2">
        <Label htmlFor="os-mf">{t("s.mandatory_fields")}</Label>
        <Input id="os-mf" name="mandatory_fields" defaultValue={fields.join(", ")} />
      </div>
      {codes === "automatic" && branches.length ? (
        <fieldset className="grid gap-2 sm:col-span-3 sm:grid-cols-3">
          <legend className="text-sm font-medium">{t("s.branch_code_prefixes")}</legend>
          {branches.map((b) => (
            <div key={b.id} className="grid gap-1">
              <Label htmlFor={`os-p-${b.id}`}>{b.name}</Label>
              <Input id={`os-p-${b.id}`} name={`prefix-${b.id}`} maxLength={10} defaultValue={prefixes[b.id] ?? ""} />
            </div>
          ))}
        </fieldset>
      ) : null}
      <div className="grid gap-1">
        <Label htmlFor="os-from">{t("from")}</Label>
        <Input id="os-from" name="effective_from" type="date" required />
      </div>
      <div className="grid gap-1 sm:col-span-2">
        <Label htmlFor="os-reason">{t("reason")}</Label>
        <Input id="os-reason" name="reason" required maxLength={1000} />
      </div>
      <Button type="submit" className="justify-self-start">{t("saveVersion")}</Button>
      <div className="sm:col-span-3">
        <ProblemAlert problem={problem} />
      </div>
    </form>
  );
}
