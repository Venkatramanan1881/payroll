// Times are stored as instants; people read and type them as wall-clock times in a branch's or schedule's time zone.
// These helpers only convert for display and input; no attendance rule lives here.

function parts(ms: number, tz: string) {
  const p = new Intl.DateTimeFormat("en-US", {
    timeZone: tz, hourCycle: "h23", year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", second: "2-digit",
  }).formatToParts(new Date(ms));
  const g = (k: string) => Number(p.find((x) => x.type === k)?.value);
  return Date.UTC(g("year"), g("month") - 1, g("day"), g("hour"), g("minute"), g("second"));
}

/** The instant (ISO, UTC) of wall-clock `date` `time` (YYYY-MM-DD, HH:MM) in time zone `tz`. */
export function zonedToUtc(date: string, time: string, tz: string): string {
  const wall = Date.parse(`${date}T${time}:00Z`);
  let t = wall - (parts(wall, tz) - wall);
  t = wall - (parts(t, tz) - t); // second pass settles times near a DST change
  return new Date(t).toISOString();
}

/** HH:MM of an instant in `tz`; empty for null. */
export function wallTime(iso: string | null | undefined, tz: string): string {
  if (!iso) return "";
  return new Intl.DateTimeFormat("en-GB", { timeZone: tz, hour: "2-digit", minute: "2-digit", hourCycle: "h23" }).format(new Date(iso));
}

/** Date and time of an instant in `tz`, for audit-style lines. */
export function wallDateTime(iso: string | null | undefined, tz = "Asia/Kolkata"): string {
  if (!iso) return "";
  return new Intl.DateTimeFormat("en-GB", { timeZone: tz, dateStyle: "medium", timeStyle: "short" }).format(new Date(iso));
}

/** Today's date (YYYY-MM-DD) in `tz`, shifted by `days`. */
export function dateIn(tz: string, days = 0, now = Date.now()): string {
  const local = new Date(parts(now, tz));
  local.setUTCDate(local.getUTCDate() + days);
  return local.toISOString().slice(0, 10);
}

export const hours = (minutes: number | null | undefined) => (minutes == null ? "" : (minutes / 60).toFixed(2));
