import { describe, expect, it, vi } from "vitest";
import { allowedApiPath, areaOf, sameOrigin } from "./rules";
import { refreshOnce } from "./refresh";
import { sessionCookies } from "./session";
import { buildMenu, landing } from "@/lib/menu";
import { asProblem, fieldErrors } from "@/lib/problem";

describe("proxy guard", () => {
  it("keeps vendor and organisation sessions apart", () => {
    expect(allowedApiPath("org", "/v1/employees")).toBe(true);
    expect(allowedApiPath("org", "/v1/vendor/organisations")).toBe(false);
    expect(allowedApiPath("vendor", "/v1/vendor/organisations")).toBe(true);
    expect(allowedApiPath("vendor", "/v1/employees")).toBe(false);
    expect(allowedApiPath(undefined, "/v1/employees")).toBe(false);
  });
  it("refuses traversal, odd paths and the sign-in routes", () => {
    expect(allowedApiPath("org", "/v1/../internal/jobs/x")).toBe(false);
    expect(allowedApiPath("org", "/internal/jobs/x")).toBe(false);
    expect(allowedApiPath("org", "/v1//vendor/x")).toBe(false);
    expect(allowedApiPath("org", "/v1/auth/refresh")).toBe(false);
    expect(allowedApiPath("org", "/v1/employees?x=1")).toBe(false); // query travels separately
    expect(allowedApiPath("org", "/v1/./vendor/organisations")).toBe(false);
    expect(allowedApiPath("org", "/v1/./auth/refresh")).toBe(false);
    expect(allowedApiPath("org", "/v1/employees/.")).toBe(false);
  });
  it("needs a same-site Origin on writes", () => {
    expect(sameOrigin("GET", null, "app.test")).toBe(true);
    expect(sameOrigin("POST", "https://app.test", "app.test")).toBe(true);
    expect(sameOrigin("POST", "https://evil.test", "app.test")).toBe(false);
    expect(sameOrigin("PATCH", null, "app.test")).toBe(false);
    expect(sameOrigin("DELETE", "not a url", "app.test")).toBe(false);
  });
  it("maps pages to areas", () => {
    expect(areaOf("/vendor/sign-in")).toBe("public");
    expect(areaOf("/vendor/organisations")).toBe("vendor");
    expect(areaOf("/app/employees")).toBe("org");
    expect(areaOf("/me")).toBe("org");
    expect(areaOf("/apple")).toBe("public");
  });
});

describe("refresh", () => {
  it("shares one refresh between parallel requests", async () => {
    const call = vi.fn(async () => ({ access_token: "a2", refresh_token: "r2", expires_in: 900 }));
    const [x, y] = await Promise.all([refreshOnce("r1", call), refreshOnce("r1", call)]);
    expect(call).toHaveBeenCalledTimes(1);
    expect(x).toEqual(y);
  });
  it("keeps the session when the API is down, and does not remember the failure", async () => {
    await expect(refreshOnce("r-down", async () => { throw new Error("down"); })).rejects.toThrow("down");
    await Promise.resolve();
    expect(await refreshOnce("r-down", async () => ({ access_token: "a", expires_in: 900 }))).toEqual({ access_token: "a", expires_in: 900 });
    expect(await refreshOnce("r-refused", async () => null)).toBeNull();
  });
  it("vendor sessions get no refresh cookie", () => {
    expect(sessionCookies("vendor", { access_token: "a", refresh_token: "r", expires_in: 3600 }).map((c) => c.name)).toEqual(["pa_at", "pa_kind"]);
    const org = sessionCookies("org", { access_token: "a", refresh_token: "r", expires_in: 900 });
    expect(org.find((c) => c.name === "pa_at")?.maxAge).toBe(870);
  });
});

describe("menus", () => {
  const mods = [{ module_code: "C02", enabled: true }];
  it("shows an item only with its module on and a non-self grant", () => {
    expect(buildMenu([{ operation_code: "WRK-08", grant_kind: "scoped" }], mods).map((i) => i.key)).toEqual(["employees"]);
    expect(buildMenu([{ operation_code: "WRK-08", grant_kind: "self" }], mods)).toEqual([]);
    expect(buildMenu([{ operation_code: "WRK-08", grant_kind: "all" }], [{ module_code: "C02", enabled: false }])).toEqual([]);
  });
  it("sends self-only users to self-service and managers to their inbox", () => {
    expect(landing([{ operation_code: "WRK-08", grant_kind: "self" }])).toBe("/me");
    expect(landing([{ operation_code: "WRK-01", grant_kind: "scoped" }])).toBe("/app");
    expect(landing([{ operation_code: "ATT-03", grant_kind: "approve" }])).toBe("/app"); // a manager reaches the inbox
    expect(buildMenu([{ operation_code: "WRK-08", grant_kind: "approve" }], mods)).toEqual([]);
    const c06 = [{ module_code: "C06", enabled: true }];
    expect(buildMenu([{ operation_code: "ATT-03", grant_kind: "approve" }], c06).map((i) => i.key)).toEqual(["approvals"]);
    expect(buildMenu([{ operation_code: "ATT-01", grant_kind: "self" }], c06)).toEqual([]);
  });
});

describe("problems", () => {
  it("collects field errors and keeps unknown bodies safe", () => {
    const p = asProblem(422, { code: "VALIDATION_FAILED", errors: [{ field: "mobile", code: "PATTERN", message: "bad" }, { code: "X" }] });
    expect(fieldErrors(p)).toEqual({ mobile: "bad" });
    expect(asProblem(502, "oops").code).toBe("INTERNAL");
  });
});
