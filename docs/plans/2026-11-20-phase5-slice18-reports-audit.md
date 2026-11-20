# Phase 5 Slice 18: Reports, jobs and audit views — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR and the Auditor run the BRD reports as spreadsheets (R1–R10, R13, R25–R27). Each run is a job the client polls for a short-lived download link. The audit trail and the settings history can be read on screen. The pending Gate 3 items ship in the same migration.

**Decisions (user, 2026-11-20):**
- **D-38:** migration 0010 approved.
- **D-39:** xlsx and CSV.
- **D-40:** dashboards and upload links follow in slice 18b; vendor views in slice 19.

**Spec:** `docs/api/openapi/reports.yaml` (report runs, `/v1/jobs/{id}`, `/v1/audit/*`); BRD §14, §16; PERMISSION_MATRIX RPT-01..06, ADT-01..02. **Migration 0010** (Gate 3, D-38).

## Rules
- **Reports:**
  - Grants: R1–R5 RPT-01, R6–R9 RPT-02, R10 RPT-03, R13 RPT-04, R25–R26 RPT-05, R27 RPT-06. A report outside its route → 404.
  - The employees are those the caller sees for that grant, then the filters: period, branch, department, employees. Dates come from `period_id`, or from `from` + `to` at most a year apart. R13 always needs a period.
  - The xlsx default writes every cell as a string, never a formula. CSV follows RFC 4180 and prefixes cells starting with `= + - @` (unless numeric) to stop formula injection.
  - R13 is read from the period's totals: frozen at lock, provisional before.
- **Report contents:**
  - **R1:** day rows with status, first in / last out (shift zone), net hours, late / early minutes, flags.
  - **R2:** day counts per status, plus marks.
  - **R3:** marks, minutes and the latest lock's penalties.
  - **R4:** regularisations.
  - **R5:** flagged punches.
  - **R6:** leave balance at the end of the range, and days taken in it.
  - **R7:** absent days with informed / unpaid / uninformed counts.
  - **R8:** permissions and on-duty.
  - **R9:** comp-off earned, taken and expired in the range, and the balance at its end.
  - **R10:** employee list with branch, department, category and type.
  - **R25:** confirmed opening balances.
  - **R26:** settings and policy changes.
  - **R27:** the audit trail.
- **Jobs:**
  - A run is stored as `app.job`, its file under `{org}/reports/{job}.{format}` (TB-02).
  - The job is born finished; the report runs inside the request (shortcut).
  - `GET /v1/jobs/{id}` shows only the caller's own jobs (others → 404) and issues a fresh 15-minute link, audited. The idempotent answer never stores a link.
- **Audit views:**
  - `GET /v1/audit/entries` (ADT-01): filters by date, actor, kind, target, action and support access only; newest first or oldest first; the newest 500. Each view is itself audited.
  - `GET /v1/audit/settings-changes` (ADT-02): policy and settings changes, optionally those mentioning one BRD setting id.
- **Gate 3 items (0010):**
  - Authenticator codes are single-use: the last accepted step is kept, and a code of that step or an earlier one is refused.
  - The relay uses the new outbox index, and deletes events dispatched over 30 days ago and events never consumed after 90 days.
  - Vendor replays use the `change_ref` index.
- **Build:** the `db` crate gets a build script, so a new migration always rebuilds the embedded set.

## Review fixes
- **Audit scope:** the audit trail (R27, `GET /v1/audit/entries`), the settings history (R26, `GET /v1/audit/settings-changes`) carry no branch or department, so only a holder of the grant for the whole organisation reads them; a branch-scoped Auditor gets 403 instead of every branch's entries.
- **Indexes fitted to their queries (0010, not yet applied outside tests):** the vendor replay index leads with `change_ref` (the lookup has no organisation); retention gets `(org_id, dispatched_at)` for dispatched events.
- **Retention reaches only this relay's events:** dispatched events after 30 days; its own given-up events after 90; undispatched events of other types are never deleted (they wait for their consumers).
- **Organisation dates:** R5 uses each punch's own zone; R26, R27 and the audit-view date filters use the organisation's date (India, as `org_today`); timestamps print in that zone.
- **No silent cuts:** a report over 200,000 rows is refused (422) instead of truncated; xlsx cells over Excel's limit are cut with a marker (the CSV keeps all).
- **CSV guard:** cells starting with tab or carriage return are prefixed too.
- **Links:** polling a report job re-checks the report's grant before issuing a link.
- **Smaller:** R3 shows minute penalties; R8 survives an empty place; R13 joins totals by map; `actor_kind` validated; audit pages are 200 (the spec's maximum).
- **Test:** the single-use code test reuses the exact code, so a 30-second boundary cannot make it flaky.

## Not in this slice
- **Next slices:** dashboards and upload links (18b); vendor views (19).
- **Report gaps:**
  - Device status in R5 (no device registry rows yet).
  - Sandwich days in R7.
  - Headcount totals in R10: the list is given, and totals are left to the spreadsheet.
  - MODULE_DISABLED (module switches are not enforced anywhere yet).
- **Limits:**
  - Paging of audit views.
  - Reports above 200,000 rows.
  - Queued jobs.
- **Scope over time:** who the caller sees is decided by today's assignments, not those over the report's range (an employee who moved branch reports with the new one).
- **Report rights for other roles (G-33, G-34):** test grants give HR everything; the default grants follow the permission matrix.

## Review focus
1. Scope and tenant isolation in every report query, the jobs view and the audit views.
2. Report correctness against the BRD (columns, ranges, current results only, locked periods unchanged).
3. File safety (formula injection, paths), links never stored, migration 0010 (RLS, grants, down/up), single-use codes.
