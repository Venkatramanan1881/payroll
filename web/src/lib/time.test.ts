import { describe, expect, it } from "vitest";
import { dateIn, wallTime, zonedToUtc } from "./time";

describe("zoned times", () => {
  it("turns an India wall time into the right instant and back", () => {
    expect(zonedToUtc("2026-11-19", "09:20", "Asia/Kolkata")).toBe("2026-11-19T03:50:00.000Z");
    expect(wallTime("2026-11-19T12:00:00Z", "Asia/Kolkata")).toBe("17:30");
    expect(zonedToUtc("2026-11-19", "02:00", "Asia/Kolkata")).toBe("2026-11-18T20:30:00.000Z"); // previous UTC day
  });
  it("follows daylight saving where a zone has it", () => {
    expect(zonedToUtc("2026-07-01", "09:00", "Europe/London")).toBe("2026-07-01T08:00:00.000Z");
    expect(zonedToUtc("2026-12-01", "09:00", "Europe/London")).toBe("2026-12-01T09:00:00.000Z");
  });
  it("knows yesterday in India just after midnight there", () => {
    const justAfterMidnightIst = Date.parse("2026-11-19T18:40:00Z"); // 00:10 on the 20th in India
    expect(dateIn("Asia/Kolkata", 0, justAfterMidnightIst)).toBe("2026-11-20");
    expect(dateIn("Asia/Kolkata", -1, justAfterMidnightIst)).toBe("2026-11-19");
  });
});
