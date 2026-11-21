import { expect, test, type Page } from "@playwright/test";

// Web slice 6 against a freshly seeded local database: Asha applies for a permission and an on-duty day; Meena (the
// HR user, Asha's manager) approves one and rejects the other; Asha sees both outcomes.
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");

const indiaDate = (days: number) => new Date(Date.now() + 330 * 60_000 + days * 86_400_000).toISOString().slice(0, 10);

async function signIn(page: Page, email: string) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

test("permission and on-duty requests go from the employee to the approver and back", async ({ browser, baseURL }) => {
  const asha = await (await browser.newContext({ baseURL })).newPage();
  await signIn(asha, "asha@dev.invalid");
  await asha.getByRole("link", { name: "My requests" }).click();

  const perm = asha.locator("form", { hasText: "Apply for a permission" });
  await perm.getByLabel("Type").selectOption({ label: "Personal permission" });
  await perm.getByLabel("Date").fill(indiaDate(-1));
  await perm.getByLabel("From time").fill("09:00");
  await perm.getByLabel("To time").fill("10:00");
  await perm.getByLabel("Reason").fill("Bank visit");
  await perm.getByRole("button", { name: "Submit" }).click();
  await expect(perm.getByRole("status")).toContainText("Sent for approval");

  const od = asha.locator("form", { hasText: "Apply for on-duty" });
  await od.getByLabel("From", { exact: true }).fill(indiaDate(12)); // clear of the seeded leave
  await od.getByLabel("Purpose").fill("Client visit");
  await od.getByLabel("Place").fill("Guindy");
  await od.getByRole("button", { name: "Submit" }).click();
  await expect(od.getByRole("status")).toContainText("Sent for approval");

  const hr = await (await browser.newContext({ baseURL })).newPage();
  await signIn(hr, "hr@dev.invalid");
  await hr.getByRole("link", { name: "Requests" }).click();
  await hr.getByRole("button", { name: "Permissions" }).click();
  await hr.getByRole("row", { name: /Asha/ }).getByRole("link").click();
  await expect(hr.getByText("Bank visit")).toBeVisible();
  await hr.getByRole("button", { name: "Approve" }).click();
  await expect(hr.getByText("Approved", { exact: true }).first()).toBeVisible();

  await hr.getByRole("link", { name: "Requests" }).first().click();
  await hr.getByRole("button", { name: "On-duty" }).click();
  await hr.getByRole("row", { name: /Asha/ }).getByRole("link").click();
  await hr.getByRole("button", { name: "Reject…" }).click();
  await hr.getByLabel("Reason for rejecting").fill("Covered by the team");
  await hr.getByRole("button", { name: "Confirm rejection" }).click();
  await expect(hr.getByText("Rejected", { exact: true }).first()).toBeVisible();

  await asha.reload();
  await expect(asha.getByRole("row", { name: /09:00–10:00/ })).toContainText("Approved");
  await expect(asha.getByRole("row", { name: /Full day/ })).toContainText("Rejected");
});
