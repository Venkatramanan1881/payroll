# Phase 6 Web Slice 10: Reports, dashboards, audit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** the people who run the organisation see its state and keep a record of it:
- a dashboard for the caller's role;
- the widgets each role's dashboard shows;
- report runs with their spreadsheets;
- the audit trail and the settings history.

**Spec:**
- `docs/api/openapi/reports.yaml`: report runs, jobs, dashboards, widget choices, audit entries, settings changes.
- PERMISSION_MATRIX RPT-01..06, DSH-01..03, ADT-01..02.

**Decision (owner, 2026-11-21):** web slice 10 follows the 11-slice outline.

## What the API offers today
- **Built:** every operation above.
- **Two limits:**
  - **Report runs** write the spreadsheet to the storage bucket (`EXPORT_BUCKET`). Without one, as locally, the run is refused, as the payroll export was in slice 7.
  - **Default dashboard filters (DSH-03)** are held by no role (G-35), so they are left out.

## Backend (dev only)
- `dev_seed` adds an Auditor, `auditor@dev.invalid`, scope "all". By default only the Auditor reads the audit trail and the settings history (G-34 open for the Organisation Admin and HR).

## Screens
- **Dashboard (`/app/dashboard`, DSH-01, C16):**
  - It becomes the first menu item, so `/app` lands on it.
  - It shows the widgets of the caller's role, each with its time, and a branch filter.
  - Widgets are drawn generically: counts as figures, lists as small tables. They carry no money.
  - For DSH-02 holders: which widgets each role shows, picked from the role's offered list, with If-Match.
- **Reports (`/app/reports`, C13):**
  - **Sub-navigation:** Reports · Audit trail · Settings history.
  - **Reports:**
    - only the reports the caller's grants allow (R1–R5 RPT-01, R6–R9 RPT-02, R10 RPT-03, R13 RPT-04, R25/R26 RPT-05, R27 RPT-06);
    - filters: from and to, or a period (R13 always uses a period), one branch, one department, and the file type;
    - **running one** returns a job. The screen follows it until it finishes, and each download asks for a fresh link (links expire and are never kept).
  - **Audit trail (ADT-01):**
    - the newest entries, filtered by dates, action, target type, actor kind and "under vendor support access";
    - each entry's before and after on request.
  - **Settings history (ADT-02):** settings and policy changes, filtered by dates and setting (for example PL-06).

## Tests
- **Playwright (fresh seed):**
  1. HR lands on the dashboard and sees its widgets, then opens reports and sees only HR's reports.
  2. The Organisation Admin changes the widgets of the HR Manager role.
  3. The Auditor reads the audit trail, opens an entry, and reads the settings history.
- **Not exercised locally:** report runs and downloads, which need the bucket.

## Not in this slice
- Default dashboard filters (DSH-03, G-35).
- The employee's dashboard on self-service: the self-service pages already show the same facts.
- Paging beyond the API's newest 200 audit entries.

## Built along the way
- **Dashboard tables hide ids:** they show no raw id columns, because ids are for links, not reading. This came from checking a screenshot.
- **Shared sub-navigation:** a `SubNav` component, now used by the organisation, access and reports sections. It marks the current page (`aria-current`).

## Review fixes
1. **A key per distinct run:** a retry of the same run reuses its Idempotency-Key. Any other report or filter gets a new one, so a lost answer no longer blocks later runs with IDEMPOTENCY_CONFLICT.
2. **Audit entries say who and what:** each entry now shows the actor's and the target's ids. The trail holds no names.
3. **Links follow the grants:** the reports section links to the audit trail and settings history only for ADT-01 and ADT-02 holders. HR sees neither, and the browser test checks this.
4. **Periods are read regardless:** R13's period list is no longer limited to PER-08 holders, since the API decides who reads periods. A refusal is shown under the field.
5. **Job polling:**
   - a polled job keeps no download link, because each download asks for a fresh one;
   - a passing network or server failure is retried instead of leaving the job stuck.
6. **The widget renderer** copes with `null` data and `null` rows.
7. **Widget choices:**
   - saving waits for the choices to finish reloading;
   - a 412 (someone saved first) reads them again.
8. **Accessibility:**
   - "Saved" and the run state are in live regions that stay mounted, so they are announced;
   - widget titles are headings.
9. **Landing page:** only DSH-01 makes the dashboard the landing page. A role with DSH-02 alone would have landed on a refusal.

**Not done:**
- **R26/R27 and the audit pages need organisation-wide scope.** A branch-scoped Auditor sees them and is refused, because `/v1/me` carries no scope. This is the same limitation as slices 8 and 9.
- **Untranslated data labels:** labels inside widget data (for example "waiting on you") come from the API's field names.
- **Report runs and downloads stay untested locally,** because they need the bucket.
