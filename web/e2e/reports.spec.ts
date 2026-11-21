import { expect, test, type Page } from "@playwright/test";

// Web slice 10 against a freshly seeded local database. Report runs need the storage bucket, so they are not run
// here; the screens around them are.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

async function signOut(page: Page) {
  await page.getByRole("button", { name: "Sign out" }).click();
  await page.waitForURL(/\/sign-in/);
}

test("HR lands on the dashboard and sees HR's reports; the admin picks widgets; the Auditor reads the trail", async ({ page }) => {
  // HR: /app opens the role's dashboard; the report list holds HR's reports only.
  await signIn(page, "hr@dev.invalid");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await expect(page.getByText(/^As of /).first()).toBeVisible();
  await page.getByRole("link", { name: "Reports and audit" }).click();
  const report = page.getByLabel("Report", { exact: true });
  await expect(report.getByRole("option", { name: "R1 Daily attendance" })).toBeAttached();
  await expect(report.getByRole("option", { name: "R10 People and headcount" })).toBeAttached();
  await expect(report.getByRole("option", { name: "R27 Audit trail" })).toHaveCount(0);
  await expect(page.getByRole("link", { name: "Audit trail" })).toHaveCount(0); // HR holds no ADT-01 / ADT-02
  await expect(page.getByRole("link", { name: "Settings history" })).toHaveCount(0);
  await report.selectOption("R2");
  await page.getByLabel("Dates", { exact: true }).selectOption("period");
  await expect(page.getByLabel("Period", { exact: true })).toBeVisible();
  await signOut(page);

  // The Organisation Admin: the HR Manager dashboard drops "Joiners and leavers".
  await signIn(page, "admin@dev.invalid");
  await page.getByRole("link", { name: "Dashboard" }).click();
  const hr = page.getByRole("form", { name: "Widgets per role: HR Manager" });
  await hr.getByLabel("Joiners and leavers").uncheck();
  await hr.getByRole("button", { name: "Save widgets" }).click();
  await expect(page.getByRole("form", { name: "Widgets per role: HR Manager" }).getByRole("status")).toContainText("Saved");
  await expect(page.getByRole("form", { name: "Widgets per role: HR Manager" }).getByLabel("Joiners and leavers")).not.toBeChecked();
  await signOut(page);

  // The Auditor: the trail with an entry opened, a filter, and the settings history.
  await signIn(page, "auditor@dev.invalid");
  await page.getByRole("link", { name: "Reports and audit" }).click();
  await page.getByRole("link", { name: "Audit trail" }).click();
  await page.getByRole("button", { name: /^Details: / }).first().click();
  await expect(page.getByText("Before", { exact: true })).toBeVisible();
  await page.getByLabel("Actor kind").selectOption("user");
  await page.getByRole("button", { name: "Apply filters" }).click();
  await expect(page.getByRole("cell", { name: /^User/ }).first()).toBeVisible();
  await expect(page.getByRole("cell", { name: /^(System job|Vendor user|Device)/ })).toHaveCount(0); // the filter held
  await page.getByRole("link", { name: "Settings history" }).click();
  await expect(page.getByRole("button", { name: /^Details: / }).first()).toBeVisible();
});
