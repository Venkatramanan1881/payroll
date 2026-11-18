# Phase 5 Slice 8: Attendance Periods and Lock — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Attendance periods exist and move through their states. HR sees the pre-lock checklist and locks a period, which freezes every day and the period totals (PL-R2, PL-R3, BR-06). An unlock goes through Organisation Admin approval (PL-06, BR-07). Period totals (layer 5) are visible, provisional before lock and frozen at lock.

**Architecture:**
- New module `domain-attendance::period` (beside the approval flow and `recalculate`, which unlock needs): cycles, period generation, state housekeeping, settings, checklist, totals, lock, unlock.
- Unlock requests use the shared approval flow (`domain-attendance::approval`, request type `period_unlock`, `app.approval_step`).
- The lock materialises `period.lock_coverage` per employee. The existing triggers then refuse any new day result in the period (BR-06), and `recalculate` already skips locked days.
- New job `POST /internal/jobs/period-housekeeping` (TSD J7): generates periods up to today and moves Open → Past cut-off → Awaiting lock by the organisation's date.
- No new migration.

**Spec:** `docs/api/openapi/period.yaml`; BRD PL-R1..R3, PL-01..07; STATE_MACHINES SM-PERIOD, SM-DAY A7/A8; C12 user stories US-C12-01..05; DECISIONS D-10 (HR locks in V1); PERMISSION_MATRIX PER-01..05, PER-08; schema 0005 (`period.*`), 0006 (`period.unlock_request`).

## Rules
- **Cycles (PER-01, AC-C12-01):**
  - Monthly only in V1; other frequencies → 422.
  - Calendar month, or a custom start day (e.g. 26th → 25th). A start day beyond a month's length uses that month's last day.
  - Optional cut-off day (PL-02); the period's cut-off date is that day inside the period, clamped to the period.
  - **Owner:** `organisation` only. `branch` and `policy_set` → 422 until DP-01 is answered: their lock coverage depends on that answer.
  - Cycles of one owner never overlap (database exclusion → 422).
  - Creating a cycle generates its periods from `valid_from` through the period containing today (P1).
- **Housekeeping job (J7):** per organisation, generates any missing periods up to today, then:
  - Open → Past cut-off when the cut-off date has passed;
  - Open / Past cut-off → Awaiting lock when the period end has passed.

  Each change writes a state-history row with no user (system). It is idempotent: a second run changes nothing.
- **Settings PL-03..07 (PER-01):**
  - Stored under the `period` key of the organisation-settings versions and carried forward by every new organisation-settings version, as the access settings already are.
  - BRD defaults when unset: assumed_present, manual, block, never, 3.
  - PATCH (If-Match, reason) writes a new organisation-settings version from today, keeping every other key. A version already starting today → 409 VERSION_FROZEN (versions are append-only, DP-10).
  - Automatic lock (PL-04 = automatic) is stored but not acted on: that job waits on OQ-C12-08.
- **Checklist (PER-02, AC-C12-03):** a live view for employees in the caller's scope. For a locked period, the snapshot stored at lock. Items:
  - `pending_request`: leave or regularisation pending at level 1 or 2 with a date in the period;
  - `flagged_day`: a day whose processing state is `exception`, or a punch flag still `raised` on a punch in the period;
  - `unresolved_missed_punch`: a current day result carrying the `missed_punch` flag.

  Each (kind, employee, date, request) appears once.
- **Lock (PER-03, D-10 HR; AC-C12-04):**
  - Only from Awaiting lock or Unlocked; Open / Past cut-off → 409 ILLEGAL_TRANSITION.
  - Locking a locked period → 200 with the lock record and no second effect (P6).
  - PL-05 = block with a non-empty checklist → 409 PENDING_ITEMS_BLOCK_LOCK with the items.
  - **Effects, in one transaction:**
    - checklist snapshot stored;
    - totals frozen (one `period_totals` row per employee);
    - `lock_coverage` for every employee employed in the period;
    - every attendance day in the period → `locked`;
    - state history and audit (user, time);
    - outbox `PeriodLocked`.
  - Lock order (OQ-C12-09) is not enforced.
- **Unlock (PER-04 request, PER-05 approve/reject; AC-C12-05):**
  - PL-06 = never → 409 UNLOCK_NOT_ALLOWED, also re-checked at approval.
  - A reason is required; the period must be Locked; one pending request per period.
  - Approval is single-level, to holders of PER-05 (Organisation Admin); the requester can never approve (BR-07 → 403).
  - **Approve:**
    - lock coverage removed;
    - each day re-evaluated (a skipped recalculation restores the processing state);
    - period → Unlocked, with history recording both requester and approver;
    - frozen totals stay as history (append-only), and re-locking freezes a new row.
  - Approve on approved → 200 with no second effect; reject on rejected → 200; reject on approved → 409.
- **Totals (PER-08):**
  - `basis: provisional` (computed live) before lock; `frozen` (the latest frozen row) after.
  - Built only from the current day results and their day effects (BR-08 references = result ids). The fields are listed under the assumption below.

  **Assumption to confirm:** the BRD names "payable days" without a formula, so I'm using the common Indian-payroll reading:
  - `deduction_lines` (each line carries its result ids):
    - `lop`: `absence_lop` effects, plus `hours_shortfall` and `missed_punch` effects counted in days;
    - `uninformed_absence_extra_cut`: `uninformed_absence_cut` effects;
    - `late_early_penalty`: `late_slab` / `early_slab` effects counted in days.
  - `deduction_days` = sum of all lines.
  - `payable_days` = calendar days employed in the period − deduction days, never below 0.
  - `late_minutes` / `early_minutes` = sum over the current results.
  - `branch_days` = sum of `day_fraction_present` per costed branch (EM-08).
  - `overtime_identified_minutes` = null (D-09); `pay_hold` = false (LM-31 not yet built).
  - PL-05 allow at lock: a pending day counts as a full `lop` day (absent) or carries no deduction (present).

## Steps
1. Plan (this document).
2. `domain-attendance::period`: cycles, generation, housekeeping (TDD through API tests).
3. Settings, checklist, totals.
4. Lock and unlock, including the processing-state restore in `recalculate`.
5. Routes (`routes/period.rs`), job wiring, error codes PENDING_ITEMS_BLOCK_LOCK / UNLOCK_NOT_ALLOWED.
6. Full suite, clippy, review, fixes, merge, push.

## Review focus
1. After lock, no input changes a day or total in the period (BR-06), and the lock is idempotent.
2. PL-05 block refuses with the items; allow treats pending days as configured.
3. Unlock: PL-06 never refused; requester can never approve (BR-07); approval re-evaluates days.
4. Checklist and totals respect branch scope; nothing crosses organisations.
5. Housekeeping is idempotent and dates are the organisation's (India) dates.

## Not in this slice (next: slice 9)
- Period adjustments after lock (PL-R4, E15, SM-ADJ) and the recalculation path that measures them.
- The payroll input export (MD-06) with its file and signed download link. This needs a storage decision (Supabase Storage bucket and signed URLs).
- The automatic lock job (OQ-C12-08).
- NT-09 / NT-10 notifications.
- Cycles owned by a branch or policy set (DP-01).
