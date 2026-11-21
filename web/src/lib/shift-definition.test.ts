import { describe, expect, it } from "vitest";
import { definitionBody } from "./shift-definition";

describe("shift definition body", () => {
  it("sends only the shown fields, typed, and omits empties when creating", () => {
    const body = definitionBody("night", { start_time: "22:00", end_time: "06:00", window_before_min: "120", core_start: "10:00", half_day_split: "" }, false);
    expect(body).toEqual({ shift_type: "night", is_night_shift: true, start_time: "22:00", end_time: "06:00", window_before_min: 120 });
  });
  it("clears emptied optional fields when editing a draft, never the defaulted NOT NULL ones", () => {
    const body = definitionBody("flexible", { required_minutes: "480", required_per: "day", flex_earliest_in: "", window_before_min: "", missed_punch_cutoff: "" }, true);
    expect(body.required_minutes).toBe(480);
    expect(body.flex_earliest_in).toBeNull();
    expect(body.start_time).toBeNull(); // left over from a fixed type, cleared
    expect("window_before_min" in body).toBe(false);
    expect("missed_punch_cutoff" in body).toBe(false);
    expect("half_day_split" in body).toBe(false);
  });
});
