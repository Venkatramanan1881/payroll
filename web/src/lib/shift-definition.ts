// scheduling.yaml ShiftDefinition: which fields each shift type uses, and the body the API takes. The form only
// collects values; every rule about them (ordering, windows, night shifts) is the API's.

export const SHIFT_TYPES = ["fixed", "night", "flexible", "flexible_core", "open"] as const; // "split" waits for OQ-C05-01
export type ShiftType = (typeof SHIFT_TYPES)[number];

export const TIME_FIELDS = ["start_time", "end_time", "core_start", "core_end", "flex_earliest_in", "flex_latest_out", "half_day_split_time", "missed_punch_cutoff"] as const;
const NUMBER_FIELDS = ["required_minutes", "window_before_min", "window_after_min"] as const;
// Columns that are NOT NULL with a default (migrations 0003, 0008): never sent as null; left empty, the value stays.
const DEFAULTED = ["window_before_min", "window_after_min", "half_day_split", "missed_punch_cutoff", "required_per"];

/** Fields shown for a type, besides the common ones (window, half-day split, cutoff). */
export const FIELDS_BY_TYPE: Record<ShiftType, readonly string[]> = {
  fixed: ["start_time", "end_time"],
  night: ["start_time", "end_time"],
  flexible: ["required_minutes", "required_per", "flex_earliest_in", "flex_latest_out"],
  flexible_core: ["required_minutes", "required_per", "core_start", "core_end", "flex_earliest_in", "flex_latest_out"],
  open: ["required_minutes", "required_per"],
};
export const COMMON_FIELDS = ["window_before_min", "window_after_min", "half_day_split", "half_day_split_time", "missed_punch_cutoff"] as const;

export type Definition = Record<string, string | number | boolean | null>;

/**
 * The definition from the form values of the fields shown for `type`. Creating omits empty fields (the API's defaults
 * apply). Editing a draft sends null for an emptied optional field, and clears the optional times and minutes of the
 * type it replaced; defaulted fields are never nulled.
 */
export function definitionBody(type: ShiftType, values: Record<string, string>, editing: boolean): Definition {
  const out: Definition = { shift_type: type, is_night_shift: type === "night" };
  if (editing) for (const f of [...TIME_FIELDS, ...NUMBER_FIELDS]) if (!DEFAULTED.includes(f)) out[f] = null;
  for (const f of [...FIELDS_BY_TYPE[type], ...COMMON_FIELDS]) {
    const raw = (values[f] ?? "").trim();
    if (!raw) {
      if (editing && !DEFAULTED.includes(f)) out[f] = null;
      continue;
    }
    out[f] = (NUMBER_FIELDS as readonly string[]).includes(f) ? Number(raw) : raw;
  }
  return out;
}
