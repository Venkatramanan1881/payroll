# Phase 5 Slice 19: Vendor console views — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The vendor sees each organisation's usage and health from a platform summary, never from client tables (VC-R3, VC-R8, P12). Vendor staff also maintain sector templates (VC-R7) and send announcements to chosen organisations (VC-R9). This is the last backend slice of Phase 5.

**Spec:** `docs/api/openapi/vendor.yaml` (`/v1/vendor/usage`, `/health`, `/sector-templates*`, `/announcements`); BRD VC-R3, VC-R7..R9; PERMISSION_MATRIX PLT-07, PLT-10, PLT-11; schema 0001 (`platform.organisation_usage`, `sector_template`, `announcement`, `announcement_target`). No new migration.

## Rules
- **J10 usage refresh** (`POST /internal/jobs/usage-refresh`, hourly):
  - Per organisation, counts are taken as the organisation (`app_api`, RLS): active employees, branches, devices online and total (not retired), last sign-in (latest session; whose is OQ-V01-07), and periods awaiting lock.
  - The counts are written to `platform.organisation_usage` as `app_vendor`.
- **Usage (PLT-10):**
  - Per organisation: the summary row, plus the employee limit in force today. `over_limit` filters.
  - `last_pay_run` is null (V2).
  - Before the first refresh: zeros, with the organisation's creation as `refreshed_at`.
- **Health (PLT-10):**
  - Devices online / total, and periods awaiting lock, from the same summary. `only_with_issues` (default true) keeps organisations with an offline device or a period waiting.
  - `device_sync_failures` and `failed_jobs` are empty: no platform source yet (OQ-V01-15, TB-03).
  - `failed_pay_runs` is null.
- **Sector templates:**
  - List and get (PLT-10); get includes the organisations created from the template.
  - Create (PLT-07): idempotent by key; a duplicate code → 409 ALREADY_EXISTS; content must be an object.
  - Change (PLT-07): If-Match; name and/or content; the code never changes; `version` increments; before and after are audited. Organisations already made from the template are not changed (OQ-V01-10).
- **Announcements:**
  - Send (PLT-11): idempotent by key; title 1–200, body 1–5000, 1–10000 distinct existing organisations. Stored with their targets and audited.
  - Who receives an announcement inside each organisation, and through which channel, is OQ-V01-16: no organisation user is notified yet.
  - List (PLT-10): newest 200.
- **Access:** organisation tokens get 401 on every vendor route; read-only vendor staff can view but not change.

## Review fixes
- **One meaning of "active":** J10 counts employees not exited, as the VC-01 limit check does (future joiners included), so the vendor sees the same count that blocks hiring.
- **Least privilege for J10:** the summary row is written as `app_jobs` under its row policy (only the transaction's own organisation), not as the all-tenant vendor role; `jobs_login` becomes a member of `app_jobs` (`supabase_roles.sql`, to be re-run).
- **Limit date:** the employee limit in force is looked up on the India date, as limits are written.
- **Over the limit includes at it:** under `block` an organisation at its limit already refuses hires.
- **Announcements:** a repeated organisation id is refused (422, uniqueItems); the audit keeps the chosen organisation ids.
- **Template ETag:** `updated_at` uses the clock at the write, so it only moves forward.
- **Query parameters:** usage and health take only their own filter.
- **Tests:** duplicate ids refused; an organisation at its limit appears under `over_limit=true`.

## Not in this slice
- **Open questions:**
  - Delivery of announcements to organisation users (OQ-V01-16).
  - Device sync failures and failed-job history (OQ-V01-15; a job-run table would need a migration).
  - The vendor dashboard widgets (DB-R2, OQ-C16-06).
- **Not built yet:** paging past 1000 organisations or 200 announcements.

## Review focus
1. P12: no vendor view reads client tables; J10's role switches stay within one organisation's transaction.
2. Template and announcement validation, idempotency, audit and If-Match.
3. The usage and health filters.
