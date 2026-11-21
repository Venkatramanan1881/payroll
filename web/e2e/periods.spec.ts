import { expect, test, type Page } from "@playwright/test";

// Web slice 7 against a freshly seeded local database: a monthly cycle from January with past periods awaiting lock;
// HR also holds the dev-only PER-04 grant (G-30). The payroll export needs a storage bucket, so it is not exercised.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

async function signIn(page: Page) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill("hr@dev.invalid");
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("HR reviews and locks a period, asks to unlock it, and confirms opening balances", async ({ page }) => {
  page.on("dialog", (d) => d.accept()); // confirmations for lock and remove
  await signIn(page);

  await page.getByRole("link", { name: "Periods" }).click();
  await expect(page.getByRole("cell", { name: "Monthly" })).toBeVisible(); // the seeded cycle
  // Unlocking is off by default (PL-06 = never): allow it with the Organisation Admin's approval.
  await page.getByLabel("Unlock", { exact: true }).selectOption("with_org_admin_approval");
  await page.getByLabel("Reason", { exact: true }).last().fill("Allow corrections after lock");
  await page.getByRole("button", { name: "Save settings" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
  await page.getByRole("row", { name: /Awaiting lock/ }).first().getByRole("link").click();
  await expect(page.getByRole("heading", { name: "Before locking" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Totals" })).toBeVisible();

  await page.getByLabel("Comment (optional)").fill("Month reviewed");
  await page.getByRole("button", { name: "Lock period" }).click();
  await expect(page.getByText("Locked", { exact: true }).first()).toBeVisible();

  await page.getByLabel("Why it must be unlocked").fill("A missed regularisation");
  await page.getByRole("button", { name: "Request unlock" }).click();
  await expect(page.getByText("A missed regularisation")).toBeVisible();

  // Opening balances: a draft batch with one entry, changed, then submitted.
  await page.getByRole("link", { name: "Opening balances" }).click();
  await page.getByRole("button", { name: "New batch" }).click();
  await expect(page.getByRole("heading", { name: "Batch" })).toBeVisible();
  await page.getByLabel("Employee", { exact: true }).fill("Asha");
  const pick = page.getByLabel("Choose an employee");
  await expect(pick.locator("option")).toHaveCount(2);
  await pick.selectOption({ index: 1 });
  await page.getByLabel("Type").selectOption({ label: "Casual leave" });
  await page.getByLabel("Value", { exact: true }).fill("2.00");
  await page.getByLabel("As of", { exact: true }).fill("2026-01-01");
  await page.getByRole("button", { name: "Add entry" }).click();
  const row = page.getByRole("row", { name: /Asha/ });
  await expect(row).toBeVisible();
  await row.getByLabel("Value").fill("3.00");
  await row.getByRole("button", { name: "Save" }).click();
  await expect(row.getByLabel("Value")).toHaveValue("3.00");
  // OB-01 default: HR confirms the batch directly.
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.getByText("Confirmed", { exact: true })).toBeVisible();
});
