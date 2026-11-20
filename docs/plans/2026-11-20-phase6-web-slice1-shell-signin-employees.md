# Phase 6 Web Slice 1: Shell, sign-in, employees — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The first working web app.
- One Next.js app with three areas: vendor console (`/vendor`), organisation admin (`/app`) and employee self-service (`/me`), per TSD §12.
- Sign-in for all three kinds of user.
- Menus built from the user's grants and the organisation's modules.
- Translation plumbing (English now).
- The first real workflow: employees.

**Decisions (D-42, 2026-11-20):**
- **Tokens stay on the server.** Next.js route handlers keep the access and refresh tokens in httpOnly cookies and forward calls to the Rust API. This is a thin BFF with no business rules.
- **UI:** shadcn/ui on Tailwind.
- **Languages:** English now; every string goes through translation files (next-intl) so Tamil and Hindi can be added later.

**Spec:**
- TSD v0.2 §12 (web app).
- `docs/api/openapi/identity.yaml`: sign-in, refresh, sign-out, `/v1/me`, `/v1/me/modules`.
- `vendor.yaml`: `/v1/vendor/auth/google/sign-in`.
- `workforce.yaml`: `/v1/employees*`.
- `organisation.yaml`: `/v1/branches`, `/v1/lists/{kind}`, for the create form.
- API_CONVENTIONS (RFC 9457 problems, ETag/If-Match, Idempotency-Key, cursor paging).

**Backend addition:** `GET /v1/employees` was in the approved contract but not yet built; it is added here (WRK-08). It is scoped, filtered and sorted, with an opaque offset cursor. A local-only `dev_seed` example lets the web app be run against the throwaway Postgres.

## Layout
- **Folder:** `web/` at the repo root, next to `backend/`.
- **Stack:** Next.js (App Router, TypeScript strict), Tailwind, shadcn/ui, next-intl, Vitest for logic, Playwright for one smoke flow.
- **Configuration:** server-only env `API_BASE_URL`. The Firebase web config and the vendor Google client id are public (`NEXT_PUBLIC_*`) and are not secrets. Nothing secret is in the repo.

## Rules
- **BFF (`web/src/server/`):**
  - **Cookies:**
    - `pa_at` holds the access token. `pa_rt` holds the refresh token (organisation users only). `pa_kind` is `org` or `vendor`.
    - All three are httpOnly, Secure (outside localhost), SameSite=Strict, Path=/.
  - **Sign-in route handlers** (`/bff/auth/*`) pass each sign-in step through to the API and set the cookies on success:
    - password, then `SECOND_FACTOR_REQUIRED` with its challenge token, then the code;
    - Firebase phone;
    - `ORGANISATION_CHOICE_REQUIRED` and the choice;
    - vendor Google.
    - Problem bodies go back to the browser unchanged, minus tokens.
  - **Proxy** `/bff/api/[...path]` → `API_BASE_URL/v1/...`:
    - adds `Authorization` from the cookie, and passes through If-Match, Idempotency-Key, ETag and the content type;
    - on 401 with a refresh cookie: one refresh, cookies rotated, one retry; if that fails, cookies are cleared and 401 is returned;
    - vendor cookies may only reach `/v1/vendor/...`, and organisation cookies never reach it;
    - mutating requests must carry a same-origin `Origin` header (CSRF), on top of SameSite=Strict.
  - **Server components** read through the same helper, server-side.
  - **Sign-out** calls `/v1/auth/sign-out` and clears the cookies, even if the API call fails.
- **Areas and menus:**
  - After sign-in: a vendor goes to `/vendor`, a user with any non-self grant goes to `/app`, anyone else goes to `/me`.
  - A menu item shows when its module is on (`/v1/me/modules`) and the user holds its operation code (`/v1/me` grants). Hiding is convenience only; the API still decides.
  - The slice-1 menu is Employees (C02, WRK-08 non-self). Other items arrive with their slices.
- **Employees (`/app/employees`):**
  - **List:** search (`q`), status and branch filters, cursor paging ("Load more"), sort by code or name.
  - **Detail:** personal, contact and emergency details, plus current assignments.
  - **Create (WRK-01):**
    - Branch, department, designation and similar values come from the API lists.
    - Sends an Idempotency-Key, created once per form open.
    - `VALIDATION_FAILED` errors appear on the fields they name.
    - `EMPLOYEE_LIMIT_REACHED` shows as a banner.
  - **Edit (WRK-02):**
    - Plain fields only, with If-Match from the ETag.
    - 412 shows "changed by someone else — reload". Dated fields are not editable here (EM-R6).
  - **Self (`/me`):** the employee's own record, read-only (WRK-08 Self).
- **States:** every page has loading, empty, error, forbidden (403) and not-found (404) states; forms add validation and conflict states.
- **Accessibility:** labelled inputs, keyboard reachable, visible focus, errors tied to fields (`aria-describedby`).

## Tests
- **Vitest:**
  - cookie handling and refresh-once in the proxy (fake fetch);
  - the vendor/org route guard and the CSRF origin check;
  - the menu builder (grants × modules);
  - mapping a problem body to field errors.
- **Playwright smoke**, against a local API and the throwaway Postgres on 55432: password sign-in, employee list, create, edit, sign-out. Seeded with a dev script; the test is skipped when `API_BASE_URL` is not set.
- `npm run lint`, `tsc --noEmit` and `next build` are clean.

## Not in this slice
- Firebase phone sign-in and vendor Google sign-in are wired, but need real Firebase and Google configuration to try out. Locally only password sign-in works.
- Other areas' screens, settings screens generated from the schema (TSD §12), web check-in, and the Tamil/Hindi text.
- Spreadsheet employee upload, transfers and assignment changes, and status changes (probation, notice, exit, rejoin).
- Hosting (D5: Firebase App Hosting or Cloud Run) is decided at deployment.

## Review focus
1. No token reaches page scripts. The proxy cannot be used to reach the vendor API with an organisation session, or the other way round. CSRF.
2. Refresh rotation under parallel requests: two 401s must not burn the refresh token twice.
3. No business rule in the web app; every rule comes from API answers.
4. Error, conflict and permission states.

## Review fixes
1. **Proxy guard:** `.` path segments, and segments containing `/` (sent as `%2F`), are refused. URL parsing would otherwise collapse them after the check and reach `/v1/vendor` or `/v1/auth` with an organisation session.
2. **Refresh failures:** only a refusal (400/401/422) ends the session. If the API is down or failing, the refresh cookie is kept, the failure is not cached, and the page answers 503 instead of signing the user out.
3. **Headers:** every page sends `X-Frame-Options: DENY`, `frame-ancestors 'none'`, `nosniff` and `Referrer-Policy: same-origin`. A full Content-Security-Policy waits for the sign-in origins at deployment.
4. **Employee list:** only the newest search's answer may change the table, so a slow earlier answer cannot overwrite a newer one.
5. **List and detail agree:** `GET /v1/employees` skips rows without a current master branch, which the detail view would answer 404.
6. **Menus:** menus, buttons and the /app landing count only `all` or `scoped` grants; `approve` and `self` grants do not open admin screens.
7. **Dev seed:** checks the database host by parsing the URL, not by matching a substring.
8. **Tests added:** organisation isolation, literal `_` in search, 422 for bad inputs, and empty lists for approve-only grants and for self grants without an employee record.

Not changed: after a revoked session, `api()` redirects to sign-in without clearing the cookies. This is cosmetic: the next sign-in replaces them.
