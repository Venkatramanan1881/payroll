import { expect, test, type Page } from "@playwright/test";

// Web slice 2 against the local dev seed: HR is also Meena, Asha's manager; Asha has punches yesterday at 09:20 and
// 17:30 India time. Run on a freshly seeded database (each run adds a correction and a request).
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

const yesterdayInIndia = () => new Date(Date.now() + 330 * 60_000 - 86_400_000).toISOString().slice(0, 10);

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("HR reads and corrects a day; a manager approves a regularisation", async ({ browser, baseURL }) => {
  const d = yesterdayInIndia();
  const hr = await (await browser.newContext({ baseURL })).newPage();
  await signIn(hr, "hr@dev.invalid");
  await expect(hr.getByRole("link", { name: "Attendance" })).toBeVisible();

  // The board for yesterday, then Asha's day with her punches and the explanation.
  await hr.getByRole("link", { name: "Attendance" }).click();
  await expect(hr.getByLabel("Date")).toHaveValue(d);
  await hr.getByRole("link", { name: "Asha" }).click();
  await expect(hr.getByRole("heading", { name: `Asha — ${d}` })).toBeVisible();
  await expect(hr.getByRole("cell", { name: "09:20" })).toBeVisible();
  await expect(hr.getByRole("cell", { name: "17:30" })).toBeVisible();
  await expect(hr.getByRole("heading", { name: "How this was worked out" })).toBeVisible();

  // HR correction: the late arrival is corrected to 09:00; the server keeps the old result as history.
  await hr.getByLabel("Correction type").selectOption("late_arrival");
  await hr.getByLabel("In time").fill("09:00");
  await hr.getByLabel("Reason", { exact: true }).first().fill("Gate register shows 09:00");
  await hr.getByRole("button", { name: "Record" }).first().click();
  await expect(hr.getByRole("status")).toContainText("Recorded");
  await expect(hr.getByText(/Earlier results \(\d+\)/)).toBeVisible();

  // Asha asks for an early check-out to be regularised (no self-service screen yet: through the BFF).
  const asha = await (await browser.newContext({ baseURL })).newPage();
  await signIn(asha, "asha@dev.invalid");
  await expect(asha.getByRole("heading", { name: "My details" })).toBeVisible();
  const res = await asha.request.post("/bff/api/attendance/regularisation-requests", {
    headers: { origin: baseURL!, "idempotency-key": crypto.randomUUID() },
    data: { attendance_date: d, request_type: "early_checkout", proposed_out_at: `${d}T12:30:00Z`, reason: "Client visit" },
  });
  expect(res.status(), await res.text()).toBe(201);

  // Meena (the HR user) finds it waiting and approves it.
  await hr.getByRole("link", { name: "Approvals" }).click();
  await expect(hr.getByRole("tab", { name: "Waiting for me" })).toHaveAttribute("aria-selected", "true");
  await hr.getByRole("row", { name: /Asha/ }).getByRole("link").click();
  await expect(hr.getByText("Client visit")).toBeVisible();
  await hr.getByRole("button", { name: "Approve" }).click();
  await expect(hr.getByText("Approved", { exact: true }).first()).toBeVisible();
});
