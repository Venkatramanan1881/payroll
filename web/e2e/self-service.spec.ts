import { expect, test, type Page } from "@playwright/test";

// Web slice 4 against a freshly seeded local database: Asha (employee) uses self-service; Meena (the HR user) approves.
// Notifications are made by the dispatch job; with E2E_JOB_SECRET and E2E_API_URL set, the test runs it like Cloud
// Scheduler would.
const password = process.env.E2E_PASSWORD;
const jobSecret = process.env.E2E_JOB_SECRET;
const apiUrl = process.env.E2E_API_URL ?? "http://localhost:8080";
test.skip(!password, "E2E_PASSWORD not set");

const indiaDate = (days: number) => new Date(Date.now() + 330 * 60_000 + days * 86_400_000).toISOString().slice(0, 10);

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("an employee reads their days, asks for a correction, applies for leave and hears back", async ({ browser, baseURL }) => {
  const d = indiaDate(-1);
  const asha = await (await browser.newContext({ baseURL })).newPage();
  await signIn(asha, "asha@dev.invalid");
  await expect(asha.getByRole("heading", { name: "My details" })).toBeVisible();

  // The month calendar and yesterday's day.
  await asha.getByRole("link", { name: "My attendance" }).click();
  if (d.slice(0, 7) !== indiaDate(0).slice(0, 7)) await asha.getByRole("button", { name: "Previous month" }).click();
  await asha.locator(`a[href="/me/attendance/${d}"]`).click();
  await expect(asha.getByRole("heading", { name: d })).toBeVisible();
  await expect(asha.getByRole("cell", { name: "09:20" })).toBeVisible();

  // Ask for the late arrival to be corrected.
  await asha.getByLabel("Correction type").selectOption("late_arrival");
  await asha.getByLabel("In time").fill("09:00");
  await asha.getByLabel("Reason").fill("Bus breakdown");
  await asha.getByRole("button", { name: "Ask for a correction" }).click();
  await expect(asha.getByRole("status")).toContainText("Sent for approval");
  await asha.getByRole("link", { name: "My attendance" }).first().click();
  await expect(asha.getByRole("row", { name: /Bus breakdown/ })).toContainText("Waiting");

  // Leave: balances, then apply.
  await asha.getByRole("link", { name: "My leave" }).click();
  await expect(asha.getByRole("cell", { name: "Sick leave" }).first()).toBeVisible(); // balances (and earlier requests)
  await asha.getByRole("link", { name: "Apply for leave" }).click();
  await asha.getByLabel("Leave type").selectOption({ label: "Sick leave" });
  const leaveDay = indiaDate(8);
  await asha.getByLabel("From", { exact: true }).fill(leaveDay);
  await asha.getByLabel("To", { exact: true }).fill(leaveDay);
  await asha.getByLabel("Reason", { exact: true }).fill("Dentist");
  await asha.getByRole("button", { name: "Submit request" }).click();
  await expect(asha.getByRole("heading", { name: "Leave request: Asha" })).toBeVisible();
  await expect(asha.getByRole("button", { name: "Cancel request…" })).toBeVisible(); // the requester may cancel
  await expect(asha.getByRole("button", { name: "Approve" })).toHaveCount(0); // never one's own

  // Meena approves it.
  const hr = await (await browser.newContext({ baseURL })).newPage();
  await signIn(hr, "hr@dev.invalid");
  await hr.getByRole("link", { name: "Leave", exact: true }).click();
  await hr.getByRole("row", { name: /Sick leave/ }).getByRole("link").click();
  await hr.getByRole("button", { name: "Approve" }).click();
  await expect(hr.getByText("Approved", { exact: true }).first()).toBeVisible();

  // The approval reaches Asha's notifications once the dispatch job has run.
  test.skip(!jobSecret, "E2E_JOB_SECRET not set: notifications not dispatched");
  const job = await asha.request.post(`${apiUrl}/internal/jobs/notifications-dispatch`, { headers: { authorization: `Bearer ${jobSecret}` } });
  expect(job.ok(), await job.text()).toBe(true);
  await asha.getByRole("link", { name: /Notifications/ }).click();
  await expect(asha.getByRole("listitem").first()).toBeVisible();
  await asha.getByRole("button", { name: "Mark all read" }).click();
  await expect(asha.getByRole("button", { name: "Mark all read" })).toBeDisabled();
});
