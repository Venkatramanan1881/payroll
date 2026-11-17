# Phase 5 Slice 2: Shifts, Rosters, Holidays and Policy Sets — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** HR can define shifts, weekly offs, holiday calendars, employee schedules and rosters, and versioned policy sets with dated assignments; and the backend can answer, for any employee and date, "what was expected?" (ScheduleResolver) and "which policy version applies?" (PolicyResolver, decision D-03).

**Architecture:** Two new domain crates, `domain-sched` and `domain-policy`, beside slice 1's crates (ADR-001), with routes in `api`. Same patterns as slice 1: `Auth` extractor, `require(op)`, `idempotency::begin/finish` on POST commands, `If-Match` on PATCH, `db::audit` and outbox in the same transaction, problem+json errors. Both resolvers are plain async functions over an `OrgTx`, so slice 3 (punches) and the attendance engine call them directly.

**Tech Stack:** As slice 1 (Rust, Axum 0.8, SQLx 0.8, PostgreSQL 16). No new dependencies.

**Spec:** `docs/api/openapi/scheduling.yaml`, `docs/api/openapi/policy.yaml`, `docs/api/API_CONVENTIONS.md`, `docs/api/ERROR_CATALOG.md`, `docs/domain/DOMAIN_MODEL.md` §3.6–3.7 and §4, `docs/domain/STATE_MACHINES.md` SM-POLICYVER, `docs/domain/PERMISSION_MATRIX.md` (SCH-01..06, POL-01..07), `docs/requirements/DECISIONS.md` (D-03, D-04), migration `0003_timecap_sched_policy.up.sql`.

## Global Constraints

- Every slice-1 constraint still holds (organisation from token only, audit in the same transaction, problem+json codes, Idempotency-Key on POST commands, If-Match on PATCH, no Supabase).
- Policy precedence is fixed: **employee > shift > department > category > branch > organisation** (D-03).
- The holiday calendar for an expected day comes from the branch passed in: the punch's branch when there is one, otherwise the master branch (D-04).
- A version that is `in_force` or `superseded` is never edited: the DB trigger refuses it, and the API maps that to `409 VERSION_FROZEN` (G5).
- Resolvers pick versions **by effective date**, not by the stored `state` flag, so correctness never depends on a job flipping `scheduled` → `in_force` on time.
- Rule-group settings are validated **structurally** in this slice: the group code is one of the 15 in the DB domain, the body is a JSON object, and every key is a setting ID of that group (`^<GROUP>-[0-9]{2}$`). Typed per-setting validation arrives with the slice that first evaluates each group (PC in slice 3). Logged as a ruling.

## Review Focus

1. **A night shift (22:00–06:00) on the day a holiday starts** → the expected day for the shift's attendance date (SH-06 default: start date) reports both the shift and the holiday, never two days. Test in Task 3.
2. **An employee matched by department and branch policy assignments on the same date** → the department one wins (D-03). Test in Task 5.
3. **Publishing a policy version whose effective date falls in a locked period** → refused, not silently applied to frozen days. Test in Task 4.
4. **Editing a version after it has been published** → 409 VERSION_FROZEN, never a 500 from the DB trigger. Test in Tasks 1 and 4.
5. **A roster entry for an employee outside the HR Manager's scope, or in another organisation** → 404, and nothing is written. Test in Task 3.

---

## File Structure

```
backend/crates/domain-sched/src/{lib,shift,calendar,schedule}.rs     shifts + versions; weekly offs + holidays; employee schedule, roster, ScheduleResolver
backend/crates/domain-policy/src/{lib,version,assignment}.rs          policy sets, versions, rule groups, publish; assignments + PolicyResolver
backend/crates/api/src/routes/{sched,policy}.rs                        HTTP for both
backend/crates/api/tests/{shifts,calendars,schedules,policy,resolver}.rs
```

Test seed: `common::seed_org` gains grants SCH-01..05 and POL-01, 02, 04, 06, 07 (`scoped`) for the HR Manager; an employee user gets SCH-03 `self` where a test needs "my roster".

---

### Task 1: Shifts and shift versions

**Files:** `domain-sched/src/shift.rs`, `api/src/routes/sched.rs`, `api/tests/shifts.rs`

**Interfaces — Produces:** `create_shift(tx, actor, code, name, effective_from, def: ShiftDefinition) -> Result<(shift, version) Value>`; `create_shift_version`, `get_shift`, `get_shift_version`, `update_draft_version`, `publish_shift_version`. `ShiftDefinition` mirrors scheduling.yaml `ShiftDefinition` field names (columns of `sched.shift_version`). Routes: `POST/GET /v1/shifts`, `GET /v1/shifts/{id}`, `POST /v1/shifts/{id}/versions`, `GET/PATCH /v1/shift-versions/{id}`, `POST /v1/shift-versions/{id}/publish`.

Rules: a new shift starts with one `draft` version. Publish moves draft → `in_force` when `effective_from` ≤ today, else `scheduled`, and marks the previous `in_force` version `superseded` once the new one is in force. Publish is direct: AC-04's approval path is open (G-08), and BRD AC-04's default is "HR Manager only", so approve/reject routes are not built. Shift-type checks follow the DB CHECKs (`night` requires `is_night_shift`; `split` requires `segments`).

- [ ] **Step 1: Failing tests:** `create_shift_201_with_draft_version`; `publish_today_in_force_and_supersedes_previous`; `publish_future_is_scheduled`; `patch_published_version_409_VERSION_FROZEN`; `duplicate_shift_code_422`; `night_type_requires_night_flag_422`; `list_shifts_other_org_empty`.
- [ ] **Step 2:** `cargo test -p api --test shifts` → FAIL.
- [ ] **Step 3: Implement.** Map the freeze trigger's error (SQLSTATE raised by `app.guard_frozen_version`) to `ApiError::VersionFrozen` (409).
- [ ] **Step 4:** → PASS. **Step 5:** commit `feat(sched): shifts and versioned shift definitions`.

### Task 2: Weekly-off patterns and holiday calendars

**Files:** `domain-sched/src/calendar.rs`, `routes/sched.rs`, `api/tests/calendars.rs`

**Interfaces — Produces:** `create_weekly_off_pattern`, `list_weekly_off_patterns`, `retire_weekly_off_pattern`; `create_holiday_calendar`, `list_holiday_calendars`, `rename_holiday_calendar`; `add_holiday`, `list_holidays(calendar, year?)`, `update_holiday`, `remove_holiday`. Routes as scheduling.yaml (`/v1/weekly-off-patterns…`, `/v1/holiday-calendars…`, `/v1/holidays/{id}…`).

Rules: `kind` fixed → `weekly_days` required (0 = Sunday … 6 = Saturday, per the DB CHECK); alternating → `alt_day` + `alt_weeks` (week-of-month 1–5); from_roster → neither. Removing a holiday is a soft remove (the row is ended or flagged, per the 0003 columns), audited with the reason. Branch ↔ calendar linking uses the existing `org.branch.holiday_calendar_id` column, set directly in tests until branch endpoints exist.

- [ ] **Step 1: Failing tests:** `fixed_pattern_needs_days_422`; `alternating_pattern_201`; `retired_pattern_not_listed_by_default`; `add_holiday_then_list_by_year`; `duplicate_holiday_same_date_422`; `remove_holiday_requires_reason_422`.
- [ ] **Steps 2–5** as Task 1; commit `feat(sched): weekly-off patterns and holiday calendars`.

### Task 3: Employee schedules, roster and the ScheduleResolver

**Files:** `domain-sched/src/schedule.rs`, `routes/sched.rs`, `api/tests/schedules.rs`

**Interfaces — Produces:**
- `assign_schedule(tx, actor, employee, source: "fixed" | "roster", shift_id?, weekly_off_pattern_id?, valid_from, reason)`: dated, ends the current range (same pattern as slice 1 `change_assignment`).
- `set_roster_entries(tx, actor, entries: [{employee_id, date, shift_version_id | weekly_off: true}])`: upsert per (employee, date). Every employee must be in the caller's scope, else the whole call is 404 and nothing is written.
- `get_my_roster(tx, employee, from, to)`.
- `pub async fn expected_day(tx, employee: Uuid, date: NaiveDate, branch: Uuid) -> Result<ExpectedDay>` where `ExpectedDay { shift_version_id: Option<Uuid>, weekly_off: bool, holiday: Option<String> /* name */ }`. The shift comes from the roster entry for the date when the schedule source is roster; otherwise from the fixed shift's version in force on the date. Weekly off comes from the roster entry, or from the pattern (fixed days, or alternating: weekday == alt_day and week-of-month ∈ alt_weeks). Holiday comes from `branch`'s calendar.

Routes: `POST/GET /v1/employee-schedules`, `POST /v1/roster-entries/set`, `GET /v1/roster-entries`, `GET /v1/me/roster`.

- [ ] **Step 1: Failing tests:** `fixed_schedule_expected_shift`; `roster_overrides_fixed_on_that_date`; `alternating_second_and_fourth_saturday_off` (2026-11-14 off, 2026-11-07 working); `holiday_from_branch_calendar_d04` (same date, two branches with different calendars → only one reports the holiday); `night_shift_and_holiday_same_attendance_date` (Review Focus 1); `roster_entry_out_of_scope_404_nothing_written` (Review Focus 5); `my_roster_shows_only_self`.
- [ ] **Steps 2–5**; commit `feat(sched): employee schedules, roster and expected-day resolver`.

### Task 4: Policy sets, versions and rule groups

**Files:** `domain-policy/src/version.rs`, `api/src/routes/policy.rs`, `api/tests/policy.rs`

**Interfaces — Produces:** `create_policy_set(tx, actor, name, effective_from, copy_from_version_id?)` (creates set + draft v1, copying rule groups when asked); `create_draft_version`, `get_version` (with its rule groups), `update_draft_version(effective_from)`, `put_rule_group(version, group, schema_version, settings)`, `switch_rule_group(version, group, enabled)`, `publish_version(version, today)`. Routes as policy.yaml for these operations (POL-01, 02, 06, 07).

Rules: structural validation as in Global Constraints. Publish works as for shifts (draft → in_force / scheduled; previous in force → superseded). It is refused with `409 PERIOD_LOCKED` when `effective_from` falls inside any locked attendance period of the organisation (`period.attendance_period` state `locked` covering the date). This is the neutral choice for GAP-SM-16 (back-dating into locked periods). Editing a published version or its rule groups → 409 VERSION_FROZEN.

- [ ] **Step 1: Failing tests:** `create_set_has_draft_v1`; `copy_from_version_copies_rule_groups`; `put_rule_group_unknown_key_422` (`{"LA-99x": 1}` and `{"PC-01": …}` in the LA group); `put_rule_group_then_switch_off`; `publish_supersedes_previous`; `publish_into_locked_period_409` (Review Focus 3); `edit_published_rule_group_409_VERSION_FROZEN` (Review Focus 4).
- [ ] **Steps 2–5**; commit `feat(policy): policy sets, versions, rule groups, publish`.

### Task 5: Policy assignments and the PolicyResolver (D-03)

**Files:** `domain-policy/src/assignment.rs`, `routes/policy.rs`, `api/tests/resolver.rs`

**Interfaces:**
- **Consumes:** `domain_sched::schedule::expected_day` (for the shift level).
- **Produces:** `create_assignment(tx, actor, policy_set_id, level, target, valid_from, valid_to?, reason)` (level ↔ target column per the 0003 CHECK; overlap at the same level and target → 422 from the exclusion constraint); `list_assignments`; `end_assignment(id, on, reason)`; `pub async fn resolve_policy(tx, employee, date, branch) -> Result<Option<ResolvedPolicy>>` where `ResolvedPolicy { policy_set_id, version_id, level }`. Algorithm: collect the assignments valid on `date` that match the employee at each level (employee id; shift of `expected_day`; current department, category and master branch from `workforce.employee_assignment`; organisation). Take the most specific level in D-03 order, then that set's published version with the greatest `effective_from` ≤ `date`. None when nothing matches.

Routes: `POST/GET /v1/policy-assignments`, `POST /v1/policy-assignments/{id}/end` (POL-04, POL-07).

- [ ] **Step 1: Failing tests:** `org_level_applies_when_nothing_more_specific`; `department_beats_branch_d03` (Review Focus 2); `employee_beats_everything`; `shift_level_uses_rostered_shift`; `version_by_effective_date_not_state` (a version with effective_from ≤ date but stored as `scheduled` still resolves); `no_assignment_none`; `overlapping_assignment_same_target_422`.
- [ ] **Step 2:** → FAIL. **Step 3: Implement.** **Step 4:** `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` → clean. **Step 5:** commit `feat(policy): dated assignments and D-03 policy resolver`.

---

## Not in this slice

- **Later slices:** shift-version and policy-version approve/reject (AC-04 approval path, G-08); policy preview (G8, needs the attendance engine); exemptions (G7, used by the engine); roster upload, copy-forward and rotate; optional holidays; shift change/swap requests (approval workflow).
- **Slice 3: punch collection.** App/web punch with fence and accuracy checks from the PC rule group, offline sync, devices and enrolment, manual attendance, punch flags. It needs this slice's resolvers, plus branch geo-fences (seeded directly in tests until the organisation endpoints exist).
