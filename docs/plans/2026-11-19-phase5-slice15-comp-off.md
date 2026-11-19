# Phase 5 Slice 15: Compensatory off (M02) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Work on a weekly off or holiday earns a comp-off credit (CO-R1). Credits are approved on the shared approval flow unless HW-02 is off. They show as a balance and are taken like leave (CO-R2), and they lapse at expiry (V1).

**Decisions (user, 2026-11-19):**
- **D-25:** comp-off is taken as leave of the reserved type code `COMP_OFF`, earliest-expiring credit first.
- **D-26:** expired credits lapse; HW-06 = pay is refused until V2.
- **D-27:** expiry is counted in calendar days from the worked date.
- **D-28:** HW-01 = employee chooses (and overtime) is refused for now.

**Spec:** `docs/api/openapi/comp-off.yaml`; BRD CO-R1..CO-R4, HW-01..HW-06, OD-10; SM-COMPOFF; PERMISSION_MATRIX CMP-01..CMP-07; schema 0004/0006 (`leave.comp_off_credit`, `leave.comp_off_ledger_entry`). No new migration.

## Rules
- **Settings (HW rule group, `domain-policy/src/hw.rs`):**
  - HW-01: `comp_off` or `ignore`. `overtime` (V3) and `employee_chooses` (D-28) → 422.
  - HW-02: on/off.
  - HW-03: on → 422 (no prior-approval request type, OQ-M02-02).
  - HW-04: `{half_day_hours, full_day_hours}`, default 4 and 8.
  - HW-05: days, default 60.
  - HW-06: `lapse` only.
- **Earning (C1/C3/C5):** after each day result of a day already over:
  - Status `weekly_off` or `holiday` with HW-01 = comp_off earns by net minutes: at least HW-04 full → 1.00; at least half → 0.50; less → nothing. Net, not gross: OQ-M02-03 assumption.
  - An approved full-day on-duty on such a day earns 1.00 when OD-10 is on.
  - One credit per worked date.
  - HW-02 on → `pending_approval`, routed by AC-02 (`comp_off`, level 1). Off → `credited` at once with a credit entry made by the system.
  - A pending credit follows recalculations: its fraction is updated, or it becomes `not_credited` when the day no longer earns. A credited one is never changed by them (OQ-M02-04).
- **Decide (CMP-05, one level):**
  - Approve → `credited`, expiry = worked date + HW-05, one credit ledger entry.
  - Reject needs a reason → `not_credited`.
  - BR-07; scope 404.
  - Repeats: approving a credited credit → 200 with no second entry; rejecting a not_credited credit → 200; the opposite decision → ILLEGAL_TRANSITION.
- **Taking (CO-R2, C6–C8):** a leave request of type `COMP_OFF`.
  - Submit refuses more than the usable balance: credits not expired by the leave's first day → LIMIT_EXCEEDED.
  - Approval debits the credits earliest-expiring first (`partially_used` / `used`).
  - Cancelling approved leave reverses the debits; a credit that lapsed meanwhile stays lapsed (GAP-SM-15).
  - Leave adjustments and leave opening balances on `COMP_OFF` are refused.
- **Expiry (J6, C9):** `POST /internal/jobs/comp-off-expiry`. Credits past `expires_on` lapse with an expiry entry for what is left.
- **Choice (CMP-04):** `RULE_GROUP_DISABLED` (D-28).
- **Balance and ledger (CMP-07):** the balance is the sum of the ledger, shown with open credits and pending days.
- **Slice 14 gap closed:** leave is refused on a day covered by an active full-day or several-day on-duty.

## Review fixes
- **Rejection is final:** a claim an approver rejected is never raised again by recalculations (only the system's own withdrawal lets a day earn again).
- **Timeouts:** J3 auto-approval of `COMP_OFF` leave debits comp-off credits and escalates when they fall short (it never touches a leave account).
- **Serialisation:** one employee's comp-off writes (earning, decisions, debits, reversals, expiry) take a per-employee transaction lock, so two approvals can never overdraw a credit and a cancel racing the expiry run never leaves days behind a lapsed credit. A debit checks the whole amount before posting anything.
- **Night shifts:** a day earns only once its window has closed, not at midnight.
- **Earned before taken:** a credit is usable only for leave after its worked date.
- **Repeat decisions:** a closed credit answers only to whoever decided it (others get NOT_FOUND), as leave and on-duty do.
- **Dates:** an approved credit's ledger entry carries the organisation's date.

## Not in this slice
- **Waiting on open questions:**
  - Comp-off opening balances (OQ-C15-02).
  - HW-03 prior approval (OQ-M02-02).
  - Corrections after credit (OQ-M02-04).
  - Approval timeouts for credits (OQ-M02-05).
- **Not built yet:**
  - The R9 register and comp-off shown inside the leave balances list.
  - Module switches (MODULE_DISABLED, MD-03), which are not enforced anywhere yet.
- **Usability by date:** judged on the leave's first day, not per day.
- **Pending comp-off leave** is not counted at submit (as for paid leave); approval re-checks the balance.
- **No `explanation`** on credits, and no list paging or branch/department filters (as other lists).

## Review focus
1. Ledger integrity:
   - one credit entry per credit;
   - debits never exceed what is left;
   - reversals are once only;
   - the expiry entry equals what is left;
   - states always follow the ledger.
2. Earning: only days already over; pending credits follow recalculations; credited ones never change; on-duty with OD-10.
3. Leave integration: no leave ledger posting for `COMP_OFF`; LIMIT_EXCEEDED at submit and at approval.
