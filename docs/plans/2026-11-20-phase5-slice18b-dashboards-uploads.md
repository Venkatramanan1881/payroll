# Phase 5 Slice 18b: Dashboards and upload links — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:**
- Every signed-in user lands on their role's dashboard (DB-R1..R8), with counts within their scope.
- The Organisation Admin chooses each role's widgets (DB-01), and default filters can be set per user (DB-02).
- Files are uploaded through short-lived signed links under the organisation's own folder (TB-02, DC-06).

**Spec:** `docs/api/openapi/reports.yaml` (`/v1/dashboards/*`, `/v1/files/upload-links`); BRD §9.16, DC-06, TB-02, VC-R4; PERMISSION_MATRIX DSH-01..03. Tables from migration 0010. Decision D-41 (assumed defaults).

## Rules
- **Widgets per built-in role** (`iam.role.system_code`):
  - **OA / HR:** today's counts by branch, missed punches (exception days in the last week), approvals waiting on the caller, offline devices, joiners and leavers this month.
  - **Payroll Officer:** periods awaiting lock, pre-lock issues (pending requests and exception days in those periods).
  - **Manager:** team attendance today, approvals waiting, team leave in the next 14 days, late and absence totals over 30 days (OQ-C16-05).
  - **Supervisor:** who is in now (an odd number of punches today), today's and tomorrow's roster.
  - **Employee:** today's punches and status, this month's attendance, leave balances, permission usage, comp-off balance, own pending requests.
  - The Auditor and custom roles: none (D-41).
- **GET /v1/dashboards/me (DSH-01):**
  - Shows the named role, else the broadest of the caller's roles; a role the caller lacks → 422.
  - Widgets are the role's DB-01 choice, else everything offered.
  - Scope is the DSH-01 grant; a branch filter (from the query, else the caller's DB-02 default) only narrows it.
- **Widget choices (DSH-02):**
  - GET lists every role with offered, chosen and `default_applied`, with an ETag for the set.
  - PUT needs If-Match. Only offered widgets, each once, at most 50. Changes are serialised per organisation and audited before and after.
- **Default filters (DSH-03):**
  - GET and PUT a user's branch and period (`current` or `previous`). PUT needs If-Match.
  - The branch must be inside the target user's scope: any branch for an organisation-wide scope, their listed branches otherwise, none for a self-only scope.
  - Audited. An unknown user → 404.
- **Upload links (TB-02, DC-06):**
  - Purposes: request attachments (PDF, JPEG or PNG, 5 MB); opening balances and imports (CSV or xlsx, 10 MB, D-41). Anything else → 422 `FILE_TYPE_NOT_ALLOWED`.
  - The object path is `{org}/uploads/{purpose}/{idempotency key}/{safe name}`. The name keeps only letters, digits, `.`, `-` and `_`, with no leading dots and no `..`. A retried key signs the same path again.
  - A V4 signed PUT link valid for 15 minutes. Audited.
- **Suspension (VC-R4) fix:** report runs are exports and run while an organisation is suspended. Other writes, upload links included, still get ORG_SUSPENDED.

## Review fixes
- **Uploads cannot overwrite:** the object name holds a server-made id; the signed link binds `content-type`, `x-goog-content-length-range` (1 to the declared size) and `x-goog-if-generation-match: 0`, returned in `headers` for the client to send. Cloud Storage then refuses another type, a larger file, or any write to an object that exists. The request is idempotent by key (a retry replays the same path with a fresh link; the key with another body is IDEMPOTENCY_CONFLICT); the stored answer holds no link.
- **Offline devices in scope:** only devices of branches whose employees the caller sees, retired devices left out.
- **Who is in:** punches not yet matched to an employee never count.
- **Pre-lock issues:** a request or day touching several waiting periods counts once.
- **DB-R3:** today's counts by branch include late and not-yet-in.
- **Contract:** a user with no roles gets `role: "none"`.
- **Suspension:** only paths `/v1/reports/…/runs` pass while suspended.
- **Tests:** every role's widgets run; a branch outside the caller's employees shows nothing; duplicate widgets refused; upload replay and conflict.

## Not in this slice
- **Open questions:**
  - Vendor dashboard widgets (slice 19).
  - Auditor and custom-role dashboards (OQ-C16-01, -02).
  - Trend shapes (OQ-C16-05).
- **Not built yet:**
  - Widgets hidden for switched-off modules (module switches are not enforced anywhere yet).
  - AC-05 hiding for employees.
  - Per-widget freshness.
- **Approximations:**
  - The DB-02 default period is stored but no widget reads a period yet (widgets show today or this month).
  - Scope follows assignments on the server's (UTC) date while widgets use the organisation's date.
  - DC-06 is an organisation setting; its defaults are used.
  - Using an uploaded file is checked by the command that takes it, which arrives with those commands (spreadsheet imports).

## Review focus
1. Scope: every widget counts only employees in the caller's scope; the branch filter can never widen it; one's own widgets read only one's own data.
2. Upload links: path safety, type and size checks, organisation prefix, no link stored.
3. DB-01 / DB-02 concurrency and validation; the suspension change only lets report runs through.
