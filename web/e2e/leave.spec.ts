import { expect, test, type Page } from "@playwright/test";

// Web slice 3 against a freshly seeded local database: Asha has a pending CL request that waits for Meena (the HR
// user), and HR holds the dev-only LVE-03 / LVE-06 grants.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

const indiaDate = (days: number) => new Date(Date.now() + 330 * 60_000 + days * 86_400_000).toISOString().slice(0, 10);

async function signIn(page: Page) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill("hr@dev.invalid");
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("HR approves leave from the inbox, applies on behalf, cancels, and sees balances", async ({ page }) => {
  await signIn(page);
  await page.getByRole("link", { name: "Leave" }).click();

  // Asha's pending request waits for Meena (the HR user is Meena).
  await expect(page.getByRole("tab", { name: "Waiting for me" })).toHaveAttribute("aria-selected", "true");
  await page.getByRole("row", { name: /Casual leave/ }).getByRole("link", { name: "Asha" }).click();
  await expect(page.getByText("Family function")).toBeVisible();
  await page.getByRole("button", { name: "Approve" }).click();
  await expect(page.getByText("Approved", { exact: true }).first()).toBeVisible();

  // Apply a half day of sick leave on her behalf.
  await page.getByRole("link", { name: "Leave", exact: true }).first().click();
  await page.getByRole("link", { name: "Apply on behalf" }).click();
  await page.getByLabel("Find employee").fill("Asha");
  // Wait for the search answer (only Asha), then pick her.
  await expect(page.getByLabel("Employee", { exact: true }).locator("option")).toHaveCount(2);
  await page.getByLabel("Employee", { exact: true }).selectOption({ index: 1 });
  await expect(page.getByLabel("Employee", { exact: true })).toContainText("Asha");
  await expect(page.getByRole("cell", { name: "Sick leave" })).toBeVisible();
  await page.getByLabel("Leave type").selectOption({ label: "Sick leave" });
  const d = indiaDate(6);
  await page.getByLabel("From", { exact: true }).fill(d);
  await page.getByLabel("To", { exact: true }).fill(d);
  await page.getByLabel("First day").selectOption("first_half");
  await page.getByLabel("Last day").selectOption("first_half");
  await page.getByLabel("Reason", { exact: true }).fill("Doctor's appointment");
  await page.getByRole("button", { name: "Submit request" }).click();
  await expect(page.getByRole("heading", { name: "Leave request: Asha" })).toBeVisible();
  await expect(page.getByText("Doctor's appointment")).toBeVisible();

  // Cancel it with a reason.
  await page.getByRole("button", { name: "Cancel request…" }).click();
  await page.getByLabel("Reason for cancelling").fill("Appointment moved");
  await page.getByRole("button", { name: "Confirm cancellation" }).click();
  await expect(page.getByText("Cancelled", { exact: true }).first()).toBeVisible();

  // Her balances on the employee page.
  await page.getByRole("link", { name: "Employees" }).click();
  await page.getByRole("link", { name: "Asha" }).click();
  await expect(page.getByRole("heading", { name: "Leave balances" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Casual leave" })).toBeVisible();
});
