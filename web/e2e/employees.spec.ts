import { expect, test } from "@playwright/test";

// Web slice 1 smoke flow against a running API and web app (see web/README.md). Credentials come from the
// environment, from the local dev seed; the test is skipped without them.
const email = process.env.E2E_EMAIL;
const password = process.env.E2E_PASSWORD;

test.skip(!email || !password, "E2E_EMAIL / E2E_PASSWORD not set");

test("HR signs in, adds and edits an employee, signs out", async ({ page, context }) => {
  await page.goto("/app/employees");
  await expect(page).toHaveURL(/\/sign-in$/); // no session → sign-in

  await page.getByLabel("Email").fill(email!);
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible(); // HR lands on their dashboard (DB-R1)
  await page.getByRole("link", { name: "Employees" }).click();
  await expect(page.getByRole("heading", { name: "Employees" })).toBeVisible();

  // Tokens stay in httpOnly cookies, out of reach of page scripts.
  const cookies = await context.cookies();
  expect(cookies.find((c) => c.name === "pa_at")?.httpOnly).toBe(true);
  expect(await page.evaluate(() => document.cookie)).not.toContain("pa_at");

  // An expired access token is renewed from the refresh cookie without signing the user out.
  const oldRefresh = cookies.find((c) => c.name === "pa_rt")?.value;
  await context.clearCookies({ name: "pa_at" });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Employees" })).toBeVisible();
  const renewed = await context.cookies();
  expect(renewed.find((c) => c.name === "pa_at")).toBeTruthy();
  expect(renewed.find((c) => c.name === "pa_rt")?.value).not.toBe(oldRefresh);

  const name = `E2E ${Date.now()}`;
  await page.getByRole("link", { name: "New employee" }).click();
  await page.getByLabel("Name", { exact: true }).fill(name);
  await page.getByLabel("Joining date").fill("2026-11-01");
  await page.getByLabel("Branch").selectOption({ label: "Head office (HQ)" });
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByRole("heading", { name })).toBeVisible();

  await page.getByRole("link", { name: "Edit" }).click();
  await page.getByLabel("Email").fill("e2e@example.com");
  await page.getByRole("button", { name: "Save" }).click();
  await expect(page.getByText("e2e@example.com")).toBeVisible();

  await page.getByRole("link", { name: "Back" }).click();
  await page.getByLabel("Search").fill(name);
  await expect(page.getByRole("link", { name })).toBeVisible();

  // The proxy refuses another site's writes and the vendor API for an organisation session.
  const csrf = await page.request.post("/bff/api/employees", { headers: { origin: "https://evil.example" }, data: {} });
  expect(csrf.status()).toBe(403);
  expect((await page.request.get("/bff/api/vendor/organisations")).status()).toBe(404);

  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page).toHaveURL(/\/sign-in$/);
  await page.goto("/app/employees");
  await expect(page).toHaveURL(/\/sign-in$/);
});

test("a wrong password is refused without saying which part was wrong", async ({ page }) => {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill(email!);
  await page.getByLabel("Password").fill("not-the-password");
  await page.getByRole("button", { name: "Sign in" }).click();
  await expect(page.getByText("The details do not match")).toBeVisible();
});
