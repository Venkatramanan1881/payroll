# Phase 5 Slice 7: Organisation Setup, Background Jobs, Deployment Kit — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Organisations configure themselves through the API instead of test fixtures (branches, time zones, geo-fences, IP allow lists, lists, contractors, organisation settings). The two timed jobs that slices 4–6 depend on run: J1 nightly recalculation (missed-punch cut-off, DS-10 absence charge) and J3 approval timeouts (RG-07, LM-13). The repository carries what Cloud Run deployment needs.

**Architecture:**
- New crate `domain-org` (setup) and `domain-attendance::jobs` (J1, J3).
- Job endpoints `POST /internal/jobs/{name}` are protected by a shared job secret (`JOB_SECRET`, from Secret Manager). Cloud Scheduler sends it as a bearer token.
- Jobs list active organisations through the vendor role (`app_vendor` sees `platform.organisation`), then work per organisation as `app_api` under row-level security, exactly as requests do.
- The jobs run as a separate Cloud Run service with its own login role (`jobs_login`, member of `app_api` and `app_vendor`). The internet-facing API's role (`api_login`) is a member of `app_api` only, and that service has no `JOB_SECRET`.
- J1 works one short transaction per employee, using each employee's local date (master branch zone); J3 locks and re-checks each request and runs it in a savepoint.
- Each run writes one `system_job` audit entry per organisation. A `job_runs` table would need a migration, which needs a Gate 3 amendment: deferred.
- No new migration in this slice.

**Spec:** `docs/api/openapi/organisation.yaml` (organisation, branches, time zones, geo-fences, IP allow list, lists, contractors, organisation settings); TSD §10 (J1, J3), §12 deployment; BRD OR-01..04, EM-01..08, PC-03, PC-05, PC-R14, RG-07, LM-13; PERMISSION_MATRIX ORG-01..07.

## Rules
- **Settings keys follow the contract** (`OrganisationSettings`): `employee_code_mode` (EM-01), `allowed_branches_mode` (EM-06), `non_allowed_branch_punch` (EM-07), and so on. The readers written in slices 1 and 3 move from `"EM-01"` style keys to these. Access settings keep their own contract keys in the same row (`delegation_enabled` for AC-03).
- **Settings versions** are append-only.
  - A new version needs a complete, valid document, a reason and `effective_from`.
  - The same `effective_from` again → 409 VERSION_FROZEN.
  - A date in any locked period → 409 PERIOD_LOCKED.
  - AC-04 "with Organisation Admin approval" is not built: versions save directly. The settings-change approval flow comes with the access-settings slice.
- **Branches (ORG-01):**
  - Code unique within the organisation.
  - A branch is created with its first time zone.
  - Changing the time zone ends the current range and starts a new one from `valid_from` (never back-dated into a locked period).
  - Branches are visible within scope (ORG-07).
- **Geo-fences (ORG-02):**
  - Created switched off.
  - "Test" measures a given point against the fence and records `tested_at` / `tested_by`.
  - Switching on does not require a test: PC-R14 says a fence *can* be tested first, and the approved contract does not refuse an untested one. (Review fix: the first build refused it.)
- **IP allow list (ORG-03):** replaced as a whole, with a reason; each entry must be a valid CIDR.
- **Lists (ORG-04) and contractors (ORG-05):** create, rename, retire (never delete).
- **J1 nightly recalculation:**
  - For each active organisation and each employee, recalculate the open days of the last 7 days, today excluded.
  - This applies the missed-punch cut-off (D-08) and the DS-10 absence charge after their waiting time.
  - Days with unchanged inputs write nothing.
  - Locked days are skipped.
- **J3 approval timeouts:**
  - Pending regularisation requests time out per RG-07 `{days, action}`; leave requests per their type version's LM-13 `timeout_days`/`timeout_action`.
  - The days are counted from the last step, in calendar days (G-F is open).
  - `escalate`:
    - marks the request escalated;
    - routes it to the HR pool;
    - writes an `escalated` step;
    - runs once per level.
  - `auto_approve` / `auto_reject`: decided by the system with basis `auto_on_timeout`.
  - An auto-approved leave is still balance-checked; with too little balance it is escalated instead.

## Tasks
1. Organisation settings: typed validation, get / list / create version; move the readers to contract keys.
2. Organisation, branches, time zones, geo-fences (test, switch on and off), IP allow list.
3. Lists and contractors.
4. Jobs: `/internal/jobs/nightly-recalculation`, `/internal/jobs/approval-timeouts`; job secret check; audit per organisation.
5. Deployment kit: `backend/Dockerfile`, `docs/architecture/DEPLOYMENT.md` (Cloud Run, Secret Manager, Cloud Scheduler, migrations as owner, API login role), `docs/database/supabase_roles.sql` (login role, membership of `app_api` and `app_vendor`).
6. Full suite, clippy, review, fixes, merge, push.

## Review focus
1. No setup endpoint reaches another organisation's rows; scoped HR sees only their branches.
2. A geo-fence starts off; moving it switches it off and clears its test.
3. A settings version in a locked period, or on an existing date, is refused.
4. J1 is idempotent: a second run writes nothing new. J3 never escalates twice and never auto-approves without balance.
5. The job endpoints refuse a missing or wrong secret, comparing it in constant time.

## Not in this slice
Leave accruals and year end (J5); the other jobs (J2, J4, J6–J12); AC-04 settings-change approvals and the access-settings endpoints; holiday calendars per branch (already in scheduling); a `job_runs` table; device gateway.
