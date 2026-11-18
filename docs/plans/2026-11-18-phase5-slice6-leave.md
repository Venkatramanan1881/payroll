# Phase 5 Slice 6: Leave — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Organisation leave types with append-only versions; balances as a ledger; leave requests on the shared approval flow; approved leave shown on the day (full and half day, E12); absent days without leave become loss of pay or an automatic leave debit (DS-10, D-07/D-07b, LV-04).

**Architecture:**
- `domain-attendance::approval`: shared routing (AC-02 by request type), the delegation check (AC-03, honouring the AC-03 switch) and `app.approval_step` rows (BR-07 enforced by the table too). Leave uses it; regularisation now routes through it as well. Regularisation still reads its steps from the audit log: moving it to `app.approval_step` is a follow-up.
- `domain-attendance::leave`: types, versions, balances, ledger, requests.
- **Engine:** the day takes an optional leave cover. A full day is `leave`. A half day moves the shift start or end to the split point (SH-11), measures late or early against it (E12, LA-20 default yes) and gives `half_day_leave`. A working day with no punches and no leave is charged under DS-10 once its waiting days have passed.

**Spec:** `docs/api/openapi/leave.yaml` (types, versions, balances, ledger, adjustment, requests, on-behalf, approve, reject, bulk, cancel); BRD LM-01..15, LM-28, LV-01, LV-04, DS-10, E12; DECISIONS D-06, D-07, D-07b; SM-REQ; PERMISSION_MATRIX LVE-01..08; schema 0004 (`leave.*`, `app.approval_step`).

## Rules
- **Types (LVE-01):** create a type with its first version; publish later versions (append-only; the same `effective_from` again → 409 VERSION_FROZEN; reason required). The version in force on a date = latest `effective_from` ≤ date.
- **Balance = sum of ledger entries (LM-R4).** HR posts `adjustment` entries with a reason (LVE-07); a locked `entry_date` → 409 PERIOD_LOCKED. Unpaid types track no balance: no balance check and no debit.
- **Submit (LVE-02 own, LVE-03 on behalf):** the version in force on `from_date`. Checks, each refusing with nothing created:
  - eligibility, LM-01 (category, employee type, after probation) → 409 NOT_ELIGIBLE;
  - half day with LM-08 off, or a missing LM-12 document → 422;
  - notice LM-09, after-the-event window LM-11 → 409 OUTSIDE_REQUEST_WINDOW;
  - max consecutive LM-10, balance LM-07 (including the allowed negative) → 409 LIMIT_EXCEEDED;
  - overlap with an active request, or another type adjacent when LM-14 = block → 409 OVERLAPPING_REQUEST;
  - any locked date → 409 PERIOD_LOCKED.

  Days on a weekly off or holiday are not counted (sandwich LM-16 is off by default and comes later). A multi-day request may start on a second half and end on a first half.
- **Approve and reject:** LVE-04 at level 1, LVE-05 at level 2 (LM-13 `manager_then_hr`). Routing, delegation and 404/403/BR-07 behave as in slice 5. Final approval re-checks the balance, posts one `debit` (idempotent by the request) and recalculates each day. Bulk decisions run each item on its own.
- **Cancel (LVE-06, G-24 open):** the requester, or LVE-06 within scope. Pending → cancelled. Approved → cancelled, plus a `reversal` of the debit and recalculation. A reason is required.
- **DS-10 (typed now):** `{action: pending | loss_of_pay | auto_debit, pending_days}`, default pending for 3 days. When the waiting time has passed:
  - `pending` and `loss_of_pay` → `absence_lop` 1 day;
  - `auto_debit` → the first LV-04 leave type with enough balance gets an `absence_auto_debit` entry. The entry is caused by that day's result and reversed if the day stops being an absence. With no balance anywhere → loss of pay.
- **LV-01:** punches on a full leave day are ignored and the day is flagged `punched_on_leave` (`auto_cancel` is read as flag for now).

## Tasks
1. Engine: DS-10 and LV settings, leave cover (full and half), absence charge. Unit tests including E12.
2. Approval module (routing, delegation, steps); regularisation switched to it.
3. Leave types, versions, balances, ledger, adjustment.
4. Requests: submit, on-behalf, approve, reject, bulk, cancel; debit and reversal; recalculation.
5. Day overlay and DS-10 auto-debit in recalculation. API tests: full day, E12 half day, absence to loss of pay and to auto-debit, cancel → reversal.
6. Full suite, clippy, review, fixes, merge, push.

## Review focus
1. Approving twice debits once; cancelling approved leave reverses once.
2. The balance can never go below the LM-07 allowance through two approvals at the same time (account row locked).
3. E12: first-half leave, in 14:20 against a 14:00 second half → 20 minutes late, half-day leave, late mark.
4. An auto-debit is undone when a punch or leave later covers the day.
5. BR-07: whoever submitted on behalf cannot approve it.

## Not in this slice
Accruals, year end and proration jobs (LM-02..06 run with the jobs slice); sandwich LM-16..20; uninformed absence and excess absence LM-21..27; absconding LM-29..32; LM-15 notice period (needs the exit flow); the generic approvals inbox; LV-03 work from home; penalties recovered from leave (LA-12/EC-11).
