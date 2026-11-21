import { expect, test, type Page } from "@playwright/test";

// Web slice 9 against a freshly seeded local database: the Organisation Admin (admin@dev.invalid) also holds the
// dev-only IAM-06 / IAM-09 / PLT-15 (G-08, G-10, G-03), and the dev vendor has asked for support access.
// Named to run last: the settings changes below are dated ahead, and a future settings version holds back
// period-settings changes until it takes effect.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

/** A datetime-local value `minutes` from now, in India time (the screens enter and show India time). */
function indiaInput(minutes: number) {
  return new Date(Date.now() + (minutes + 330) * 60_000).toISOString().slice(0, 16);
}

/** India's date `days` from today. */
function indiaDate(days: number) {
  return new Date(Date.now() + 330 * 60_000 + days * 86_400_000).toISOString().slice(0, 10);
}

test("the Organisation Admin manages users, roles, delegations and support access; HR sets approvers and visibility", async ({ page }) => {
  const n = Date.now().toString().slice(-6);
  const clerk = `clerk${n}@dev.invalid`;
  await signIn(page, "admin@dev.invalid");

  // A user: created, given another role and one branch, then deactivated.
  await page.getByRole("link", { name: "Users and access" }).click();
  const add = page.getByRole("form", { name: "New user" });
  await add.getByLabel("Email").fill(clerk);
  await add.getByLabel("Initial password").fill(`Clerk-${n}-pass`);
  await add.getByLabel("Employee", { exact: true }).check();
  await add.getByRole("button", { name: "Add user" }).click();
  await page.getByRole("link", { name: clerk }).click();
  const access = page.getByRole("form", { name: "Roles and scope" });
  await access.getByLabel("Manager", { exact: true }).check();
  await access.getByLabel("Scope").selectOption("branch");
  await access.getByLabel("Branch").selectOption({ label: "Head office" });
  await access.getByLabel("Reason").fill("Now leads the packing line");
  await access.getByRole("button", { name: "Save access" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
  await page.reload(); // the branch must show once the branch list arrives
  await expect(access.getByLabel("Manager", { exact: true })).toBeChecked();
  await expect(access.getByLabel("Branch").locator("option:checked")).toHaveText("Head office");
  const deactivate = page.getByRole("form", { name: "Deactivate" });
  await deactivate.getByLabel("Reason").fill("Left the company");
  await deactivate.getByRole("button", { name: "Deactivate" }).click();
  await expect(page.getByText("Inactive", { exact: true })).toBeVisible();

  // A custom role with one grant, still there after a reload.
  await page.getByRole("link", { name: "Roles", exact: true }).click();
  await page.getByRole("textbox", { name: "New custom role" }).fill(`Night auditor ${n}`);
  await page.getByRole("button", { name: "Add role" }).click();
  await page.getByRole("link", { name: `Night auditor ${n}` }).click();
  await page.getByRole("button", { name: "Add grant" }).click();
  await page.getByLabel("Operation 1").fill("ATT-01");
  await page.getByLabel("Reason").fill("Reads attendance at night");
  await page.getByRole("button", { name: "Save grants" }).click();
  await expect(page.getByRole("button", { name: "Remove: ATT-01" })).toBeVisible();
  await page.reload();
  await expect(page.getByLabel("Operation 1")).toHaveValue("ATT-01");

  // HR's approvals delegated to the admin for two days, then ended.
  await page.getByRole("link", { name: "Delegations" }).click();
  await page.getByLabel("Approver").selectOption({ label: "hr@dev.invalid" });
  await page.getByLabel("Delegate").selectOption({ label: "admin@dev.invalid" });
  await page.getByLabel("From (India time)").fill(indiaInput(-5));
  await page.getByLabel("To (India time)").fill(indiaInput(2 * 24 * 60));
  await page.getByRole("button", { name: "Delegate", exact: true }).click();
  await expect(page.getByText("Current", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "End now: hr@dev.invalid → admin@dev.invalid" }).click();
  await expect(page.getByText("Ended", { exact: true })).toBeVisible();

  // The vendor's support request: approved, then ended early.
  await page.getByRole("link", { name: "Support access" }).click();
  await expect(page.getByText("Dev seed: look into a payroll export question")).toBeVisible();
  await page.getByRole("button", { name: "Approve" }).click();
  await expect(page.getByText("Active", { exact: true })).toBeVisible();
  await page.getByLabel("Why end it now?").fill("Question answered");
  await page.getByRole("button", { name: "End now" }).click();
  await expect(page.getByText("Ended", { exact: true })).toBeVisible();
  // Access settings (IAM-06): delegation stays on, saved as a version from tomorrow.
  await page.getByRole("link", { name: "Settings" }).click();
  const acs = page.getByRole("form", { name: "Access settings" });
  await acs.getByLabel("Approvers may delegate (AC-03)").selectOption("on");
  await acs.getByLabel("Applies from (today or later)").fill(indiaDate(1));
  await acs.getByLabel("Reason").fill("Delegation confirmed for the new year");
  await acs.getByRole("button", { name: "Save access settings" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
  await page.getByRole("button", { name: "Sign out" }).click();
  await page.waitForURL(/\/sign-in/); // let the sign-out finish before signing in again

  // HR: leave goes to the reporting manager, then HR; deduction reasons hidden from the day after tomorrow.
  await signIn(page, "hr@dev.invalid");
  await page.getByRole("link", { name: "Users and access" }).click();
  await page.getByRole("link", { name: "Approvers" }).click();
  await page.getByLabel("Level 1").selectOption("reporting_manager");
  await page.getByLabel("Level 2 (optional)").selectOption("hr");
  await page.getByLabel("Reason").fill("Two-step leave approval");
  await page.getByRole("button", { name: "Save approvers" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
  await expect(page.getByRole("row", { name: "Leave Reporting manager HR" })).toBeVisible();

  await page.getByRole("link", { name: "Settings" }).click();
  const vis = page.getByRole("form", { name: "What employees see about themselves (AC-05)" });
  await vis.getByLabel("Why a deduction was made").uncheck();
  await vis.getByLabel("Applies from (today or later)").fill(indiaDate(2)); // after the admin's version: one per date
  await vis.getByLabel("Reason").fill("Payroll explains deductions in person");
  await vis.getByRole("button", { name: "Save visibility" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
});
