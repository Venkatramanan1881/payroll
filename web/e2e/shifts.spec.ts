import { expect, test, type Page } from "@playwright/test";

// Web slice 5 against a freshly seeded local database: HR sets up a night shift, a weekly-off pattern, Asha's schedule,
// a roster cell and a holiday; Asha sees her schedule.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

const indiaDate = (days: number) => new Date(Date.now() + 330 * 60_000 + days * 86_400_000).toISOString().slice(0, 10);

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("HR sets up shifts, weekly offs, a schedule, the roster and holidays; the employee sees their schedule", async ({ browser, baseURL }) => {
  const hr = await (await browser.newContext({ baseURL })).newPage();
  await signIn(hr, "hr@dev.invalid");

  // A night shift: create (draft), edit the draft, publish.
  await hr.getByRole("link", { name: "Shifts" }).click();
  await hr.getByRole("link", { name: "New shift" }).click();
  await hr.getByLabel("Code").fill("NT");
  await hr.getByLabel("Name").fill("Night 22-06");
  // Next month: a version may not start inside a locked period (another test locks a past one).
  const next = new Date(Date.now() + 330 * 60_000);
  next.setUTCMonth(next.getUTCMonth() + 1, 1);
  await hr.getByLabel("Effective from").fill(next.toISOString().slice(0, 10));
  await hr.getByLabel("Shift type").selectOption("night");
  await hr.getByLabel("Start", { exact: true }).fill("22:00");
  await hr.getByLabel("End", { exact: true }).fill("06:00");
  await hr.getByRole("button", { name: "Create shift" }).click();
  await expect(hr.getByRole("heading", { name: "NT · Night 22-06" })).toBeVisible();
  await hr.getByRole("link", { name: "Edit draft" }).click();
  await hr.getByLabel("Start", { exact: true }).fill("21:30");
  await hr.getByRole("button", { name: "Save" }).click();
  await expect(hr.getByRole("status")).toContainText("Saved");
  await hr.getByRole("button", { name: "Publish" }).click();
  await expect(hr.getByText("Scheduled").first()).toBeVisible();
  await expect(hr.getByText("Published versions cannot change")).toBeVisible();

  // A weekly-off pattern: Sundays.
  await hr.getByRole("link", { name: "Shifts" }).click();
  await hr.getByLabel("Name", { exact: true }).fill("Sundays off");
  await hr.getByLabel("Sun").check();
  await hr.getByRole("button", { name: "Add pattern" }).click();
  await expect(hr.getByRole("cell", { name: "Sundays off" })).toBeVisible();

  // Asha's schedule: the night shift from next month, Sundays off.
  await hr.getByRole("link", { name: "Employees" }).click();
  await hr.getByRole("link", { name: "Asha" }).click();
  await hr.getByLabel("Shift", { exact: true }).selectOption({ label: "NT · Night 22-06" });
  await hr.getByLabel("Weekly offs").selectOption({ label: "Sundays off" });
  await hr.getByLabel("From", { exact: true }).fill(indiaDate(40)); // after the shift starts next month
  await hr.getByLabel("Reason").fill("Moves to nights");
  await hr.getByRole("button", { name: "Assign schedule" }).click();
  await expect(hr.getByRole("cell", { name: "Moves to nights" })).toBeVisible();

  // One roster cell: Meena off on the week's first day.
  await hr.getByRole("link", { name: "Roster" }).click();
  const cell = hr.getByRole("combobox", { name: /^Meena / }).first();
  await cell.selectOption("off");
  await hr.getByRole("button", { name: /Save changes/ }).click();
  await expect(hr.getByRole("button", { name: /Save changes/ })).toHaveCount(0);
  await expect(cell).toHaveValue("off");

  // A holiday calendar with one holiday.
  await hr.getByRole("link", { name: "Holidays" }).click();
  await hr.getByLabel("New calendar").fill("Tamil Nadu");
  await hr.getByRole("button", { name: "Add calendar" }).click();
  await hr.getByRole("link", { name: "Tamil Nadu" }).click();
  const year = indiaDate(0).slice(0, 4);
  await hr.getByLabel("Date", { exact: true }).fill(`${year}-12-25`);
  await hr.getByLabel("Name", { exact: true }).last().fill("Christmas");
  await hr.getByLabel("Reason", { exact: true }).fill("Gazette");
  await hr.getByRole("button", { name: "Add holiday" }).click();
  await expect(hr.getByRole("cell", { name: "Christmas" })).toBeVisible();

  // Asha's own schedule: still the general shift for the next two weeks.
  const asha = await (await browser.newContext({ baseURL })).newPage();
  await signIn(asha, "asha@dev.invalid");
  await asha.getByRole("link", { name: "My schedule" }).click();
  await expect(asha.getByRole("cell", { name: "General 09-18" }).first()).toBeVisible();
});
