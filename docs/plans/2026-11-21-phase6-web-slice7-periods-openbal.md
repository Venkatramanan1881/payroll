# Phase 6 Web Slice 7: Periods, lock, adjustments, exports, opening balances — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR closes attendance periods on the web:
- cycles and period settings;
- what blocks a lock;
- the lock itself;
- unlock requests and decisions;
- frozen or provisional totals;
- adjustments carried into a period;
- payroll input exports and their short-lived download links.

It also enters and confirms opening balances.

**Spec:**
- `docs/api/openapi/period.yaml`: cycles, settings, periods, checklist, lock, unlock requests, totals, adjustments, exports, download link.
- `opening-balances.yaml`: batches, entries, submit, confirm, reject, change.
- PERMISSION_MATRIX PER-01..08, OBL-01..04.

**Decision (owner, 2026-11-21):** web slice 7 follows the 11-slice outline.

## Backend (additive)
- **Names:** a route helper `with_names` adds `employee_code` / `employee_name` to every item that names an employee. It applies to period totals, adjustments, checklist items and opening-balance entries. The items were already filtered to the caller's scope; this is display only. Recorded in the contracts; tests assert the names.
- **`dev_seed`:**
  - adds a monthly organisation cycle from January and runs J7 in-process, so past periods are "awaiting lock";
  - grants HR the dev-only PER-04 / PER-06 / PER-07 (G-30, G-31 open: no default role holds them).

## Screens
- **Periods (`/app/periods`, PER-08):**
  - Periods, newest first.
  - **For organisation-wide PER-01:**
    - cycles: list and add (organisation or a branch; calendar or custom start; cut-off);
    - period settings: PL-01..07 with a reason, If-Match.
- **A period (`/app/periods/{id}`):**
  - Before locking: checklist counts and items (PER-02).
  - Lock with an optional comment, after a confirmation (PER-03; awaiting lock or unlocked).
  - Unlock requests: request with a reason (PER-04, locked periods), approve / reject (PER-05).
  - Totals per employee (frozen or provisional, as the API gives them).
  - Adjustments into the period.
  - Payroll input exports: produce (PER-06, locked periods) and download through a fresh signed link per click (PER-07).
  - State history.
- **Opening balances (`/app/opening-balances`):**
  - **Batches:** list them, and start one (OBL-01).
  - **A batch:**
    - entries, each named, with their type;
    - while a draft, add, change and remove entries (If-Match, plus an Idempotency-Key on add);
    - confirm directly, or submit for the Organisation Admin (OB-01 decides which, and the API refuses the other);
    - an approver confirms or rejects a submitted batch;
    - a confirmed entry changes by reversal with a reason (OBL-03).
- **Menus:** Periods (C12) and Opening balances (C15).

## Tests
- **Playwright (fresh seed):**
  1. HR allows unlocks (settings), then reviews and locks October.
  2. HR requests an unlock.
  3. HR starts an opening-balance batch, adds and changes an entry for Asha, and confirms it.

  The shift test now publishes from next month, since a version may not start inside a locked period.
- **Not exercised locally:** the payroll export needs the Cloud Storage bucket.

## Not in this slice
- Spreadsheet uploads of opening balances (upload links), the PayrollInput spreadsheet layout (OQ-C12-11), and a period-per-branch picker.

## Review fixes
1. **The Organisation Admin can decide again** (a backend bug that predates this slice):
   - `approve_unlock` and `reject_unlock` used `require_org_wide`, which refuses `approve` grants, the default kind of the Organisation Admin's PER-05. New `require_org_wide_decider` accepts an approve-type grant over the whole organisation.
   - **Decide-only readers:** the PER-05 decider may read the periods and unlock requests it decides; the OBL-02 decider may read batches and entries (`read_scope`).
   - **Tests:** they now use the default `approve` grant, check that the decider can read but cannot see the totals (PER-08), and that the OBL-02 approver can list batches.
2. **No stale cards:** the period page's cards are keyed on the period's state, so totals, adjustments and exports read again after a lock, unlock or decision.
3. **Downloads open:** the download tab opens within the click and is then pointed at the fresh signed link, so browsers do not block it as a popup. The link is still never kept.
4. **Controls shown as the API checks them:**
   - **Period page:** decide-unlock accepts all, scoped or approve; lock, unlock and checklist accept all or scoped (the API also requires organisation-wide); exports are listed only with PER-07; totals and adjustments need PER-08.
   - **Opening balances:** confirming or rejecting a submitted batch is shown only to an approve-type OBL-02 holder (as `batch_command`); new batch and entry edits need OBL-01; changing a confirmed entry needs OBL-03.
5. **Settings save waits for settings:** saving is disabled until a settings version exists (no ETag to change against).
6. **Smaller fixes:** the checklist has a header row, the per-row Save and Remove buttons name the employee for screen readers, and the batch page shows a loading state.
