"use client";
import { useTranslations } from "next-intl";
import { NativeSelect } from "@/components/native-select";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { COMMON_FIELDS, FIELDS_BY_TYPE, SHIFT_TYPES, TIME_FIELDS, type ShiftType } from "@/lib/shift-definition";

const asText = (v: unknown) => (v == null ? "" : String(v).slice(0, 5)); // the API's "HH:MM:SS" shows as HH:MM

/** The ShiftDefinition fields of the chosen type, named as the contract names them (read back with FormData). */
export function ShiftDefinitionFields({ type, onType, initial }: { type: ShiftType; onType: (t: ShiftType) => void; initial?: Record<string, unknown> }) {
  const t = useTranslations("sched");
  const field = (f: string) => {
    const id = `def-${f}`;
    const value = initial?.[f];
    let input: React.ReactNode;
    if ((TIME_FIELDS as readonly string[]).includes(f)) input = <Input id={id} name={f} type="time" defaultValue={asText(value)} />;
    else if (f === "required_per")
      input = (
        <NativeSelect id={id} name={f} defaultValue={String(value ?? "day")}>
          <option value="day">{t("per.day")}</option>
          <option value="week">{t("per.week")}</option>
        </NativeSelect>
      );
    else if (f === "half_day_split")
      input = (
        <NativeSelect id={id} name={f} defaultValue={String(value ?? "middle_of_shift")}>
          <option value="middle_of_shift">{t("split.middle_of_shift")}</option>
          <option value="fixed_time">{t("split.fixed_time")}</option>
        </NativeSelect>
      );
    else input = <Input id={id} name={f} type="number" min={f === "required_minutes" ? 1 : 0} defaultValue={value == null ? "" : String(value)} />;
    return (
      <div key={`${type}-${f}`} className="grid gap-1">
        <Label htmlFor={id}>{t(`f.${f}`)}</Label>
        {input}
      </div>
    );
  };
  return (
    <fieldset className="grid gap-4">
      <legend className="mb-2 font-medium">{t("definition")}</legend>
      <div className="grid max-w-xs gap-1">
        <Label htmlFor="def-type">{t("type")}</Label>
        <NativeSelect id="def-type" value={type} onChange={(e) => onType(e.target.value as ShiftType)}>
          {SHIFT_TYPES.map((x) => <option key={x} value={x}>{t(`ty.${x}`)}</option>)}
        </NativeSelect>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">{FIELDS_BY_TYPE[type].map(field)}</div>
      <div className="grid gap-3 sm:grid-cols-2">{COMMON_FIELDS.map(field)}</div>
      <p className="text-xs text-muted-foreground">{t("defaultsHelp")}{type === "night" ? ` ${t("nightNote")}` : ""}</p>
    </fieldset>
  );
}

/** The form values of the definition fields, as strings, for `definitionBody`. */
export function definitionValues(f: FormData): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [k, v] of f.entries()) if (typeof v === "string") out[k] = v;
  return out;
}
