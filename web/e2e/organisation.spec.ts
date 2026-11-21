import { expect, test, type Page } from "@playwright/test";

// Web slice 8 against a freshly seeded local database. HR holds the dev-only ORG-01 / ORG-04 / ORG-05 grants (G-11).
// The browser stands at a set position inside the new fence, so "Test from here" answers "inside".
const password = process.env.E2E_PASSWORD;
test.skip(!password, "E2E_PASSWORD not set");
test.use({ geolocation: { latitude: 13.0827, longitude: 80.2707 }, permissions: ["geolocation"] });

async function signIn(page: Page) {
  await page.goto("/sign-in");
  await page.getByLabel("Email").fill("hr@dev.invalid");
  await page.getByLabel("Password").fill(password!);
  await page.getByRole("button", { name: "Sign in" }).click();
}

// A day last year: one settings version per date, so pick at random. A past date (no seeded period covers last year)
// rather than a future one, which would hold back the period settings test until it takes effect.
function lastYearDate() {
  const d = new Date(Date.UTC(new Date().getUTCFullYear() - 1, 0, 2 + Math.floor(Math.random() * 360)));
  return d.toISOString().slice(0, 10);
}

test("HR keeps organisation settings, a branch with a fence and IP list, lists and contractors", async ({ page }) => {
  const n = Date.now().toString().slice(-6);
  page.on("dialog", (d) => d.accept("No longer used")); // the retire prompt asks for a reason
  await signIn(page);

  // Settings: a new version from a day last year.
  await page.getByRole("link", { name: "Organisation" }).click();
  await page.getByLabel("Financial year").selectOption("april_to_march");
  await page.getByLabel("Week starts on").selectOption("monday");
  await page.getByLabel("Employee codes").selectOption("manual");
  await page.getByLabel("Employees edit their contact details").selectOption("off");
  await page.getByLabel("Employee code on rejoining").selectOption("new_code");
  await page.getByLabel("Branches an employee may punch at").selectOption("master_only");
  await page.getByLabel("Punch at another branch").selectOption("record_and_flag");
  await page.getByLabel("Cost of work at another branch").selectOption("branch_worked");
  await page.getByLabel("English").check();
  await page.getByLabel("Currency").fill("INR");
  await page.getByLabel("Probation (months)").fill("6");
  await page.getByLabel("Applies from").fill(lastYearDate());
  await page.getByLabel("Reason").fill(`Settings review ${n}`);
  await page.getByRole("button", { name: "Save version" }).click();
  await expect(page.getByRole("status")).toContainText("Saved");
  await expect(page.getByRole("cell", { name: `Settings review ${n}` })).toBeVisible();

  // A branch with a time zone.
  await page.getByRole("link", { name: "Branches" }).click();
  await page.getByLabel("Name").fill(`Velachery ${n}`);
  await page.getByLabel("Code").fill(`V${n}`);
  await page.getByLabel("Time zone", { exact: true }).fill("Asia/Kolkata");
  await page.getByLabel("Time zone applies from").fill("2026-01-01");
  await page.getByRole("button", { name: "Add branch" }).click();
  await page.getByRole("link", { name: `Velachery ${n}` }).click();
  await expect(page.getByText("Asia/Kolkata: 2026-01-01")).toBeVisible();

  // A geo-fence around the browser's position: tested from here, then switched on.
  const add = page.getByRole("form", { name: "Add fence" });
  await add.getByLabel("Latitude").fill("13.0827");
  await add.getByLabel("Longitude").fill("80.2707");
  await add.getByLabel("Radius (m)").fill("200");
  await add.getByRole("button", { name: "Add fence" }).click();
  await expect(page.getByText("Off", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Test from here" }).click();
  await expect(page.getByRole("status")).toContainText("Inside the fence");
  await page.getByRole("button", { name: "Switch on" }).click();
  await expect(page.getByText("On", { exact: true })).toBeVisible();

  // Details first: the IP list is versioned with the branch, so it must follow the new version.
  await page.getByLabel("Address").fill("1 Velachery Main Road");
  const reread = page.waitForResponse((r) => r.request().method() === "GET" && /\/bff\/api\/branches\/[^/]+$/.test(new URL(r.url()).pathname));
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await reread; // the branch, read again after its change

  // The allowed web IP list.
  await page.getByLabel("Networks").fill("203.0.113.0/24\n198.51.100.7/32");
  await page.getByLabel("Reason").fill("Office network");
  await page.getByRole("button", { name: "Save IP list" }).click();
  await expect(page.getByRole("status").last()).toContainText("Saved");

  // Lists: add a designation, rename it, retire it.
  await page.getByRole("link", { name: "Lists" }).click();
  await page.getByRole("button", { name: "Designations" }).click();
  await expect(page.getByLabel("Name: Baker")).toBeVisible();
  await page.getByLabel("Name", { exact: true }).fill(`Packer ${n}`);
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await page.getByLabel(`Name: Packer ${n}`).fill(`Senior packer ${n}`);
  await page.getByRole("button", { name: `Save: Packer ${n}` }).click();
  await page.getByRole("button", { name: `Retire: Senior packer ${n}` }).click();
  await expect(page.getByLabel(`Name: Senior packer ${n}`)).toHaveCount(0);
  await page.getByLabel("Show retired").check();
  await expect(page.getByRole("row", { name: new RegExp(`Senior packer ${n}.*Retired`) })).toBeVisible();

  // Contractors.
  await page.getByRole("link", { name: "Contractors" }).click();
  await expect(page.getByRole("form", { name: "Add: Contractors" })).toBeVisible(); // the lists form has left
  await page.getByLabel("Name", { exact: true }).fill(`Acme Labour ${n}`);
  await page.getByLabel("Contact", { exact: true }).fill("Ravi, 98400 00000");
  await page.getByRole("button", { name: "Add", exact: true }).click();
  await expect(page.getByLabel(`Name: Acme Labour ${n}`)).toHaveValue(`Acme Labour ${n}`);
});
