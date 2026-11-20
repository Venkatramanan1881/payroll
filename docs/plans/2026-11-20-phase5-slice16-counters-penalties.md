# Phase 5 Slice 16: Period counters and penalties — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Count each employee's late and early marks and minutes over the attendance period (PeriodCounterSet) and apply the period-end penalties (BR-10, LA-06..LA-12, EC-05..EC-11). Also accept the remaining opening balances: late and early counters, and permission used.

**Decisions (user, 2026-11-20):**
- **D-29:** recovery comes from the listed leave codes, else loss of pay.
- **D-30:** when both a count and a minutes penalty apply, only the larger applies.
- **D-31:** penalties are a preview until the lock, then recorded at the lock.
- **D-32:** counters reset every payroll period only.
- **D-33:** opening counters are whole marks.

**Spec:** `docs/api/openapi/attendance.yaml` (`GET /v1/attendance/employees/{id}/counters`, ATT-13); BRD LA-06..LA-12, LA-18, EC-05..EC-11, EC-17, BR-04, BR-10, E5; SM-PERIOD P3/P5; schema 0005 (`attendance.period_counter`, `attendance.period_penalty`, `leave.leave_ledger_entry` `penalty_debit`, `perm.permission_usage_entry` origin `opening`). No new migration.

## Rules
- **Settings (`engine/src/penalty.rs`):** parsed with the LA/EC groups but kept out of the day fingerprint, so days are not recalculated.
  - LA-06 / EC-05: on/off.
  - LA-07 / EC-06: free marks.
  - LA-08 / EC-07: every Nth.
  - LA-09 / EC-08: quarter, half or full day; `fixed_amount` → 422 (OQ-C06-18).
  - LA-10: on/off. LA-11, and EC-09 as an object: `{threshold_minutes, mode: actual_excess | per_block, block_minutes, size}`.
  - LA-12 / EC-11: `{leave_types: [codes], then: loss_of_pay | none}`; `permission_balance` and `fixed_fine` → 422.
  - LA-18 / EC-17.reset: `payroll_period` only. EC-10: `separate` only.
- **Counters:**
  - Taken from the current day results in the employee's part of the period: late and early marks and minutes.
  - Confirmed opening late and early counters dated in the period are added (D-33).
- **Penalties:**
  - Count penalty: `(marks − free) / every`, times the size.
  - Minutes penalty: `minutes − threshold`. With `actual_excess` the excess minutes are converted to days on the DS-02 full day, else 480 minutes. With `per_block` it is `(excess / block) × size`.
  - The larger of the two applies (D-30).
  - Settings come from the policy in force on the employee's last day in the period.
- **Recovery (D-29):** the first listed paid leave type (never `COMP_OFF`) whose balance covers the whole penalty; else loss of pay or report only.
  - Before the lock (D-31): a preview inside provisional totals; loss of pay shows as a `late_early_penalty` deduction line.
  - At lock:
    - earlier runs' leave debits for the period are reversed;
    - penalties are recorded (`period_penalty`, one run);
    - leave recoveries post a `penalty_debit`;
    - loss-of-pay penalties go into the frozen totals;
    - the counter set is kept (`period_counter`).
- **GET counters (ATT-13):** the period containing the date (else the calendar month). It shows the counter and the latest recorded run when locked, or the preview otherwise. Another organisation's employee → 404.
- **Opening balances:**
  - Late and early counters: whole marks, no type.
  - Permission used: whole count with `permission_type_id`. It is posted as an `opening` usage entry in the limit period of its date, and a change reverses and reposts it.
  - Comp-off openings stay refused (OQ-C15-02).

## Review fixes
- **BR-04 across day and period:** late or early minutes of a day already cut by a slab or an hours shortfall never feed the period minutes penalty.
- **Re-lock view:** GET counters shows only the penalties recorded by the latest lock (not an older run).
- **Stored documents:** a penalty key refused since it was stored drops only that group's penalties, never the day settings (grace, slabs).
- **Recovery balance:** read as at the period end; at lock the leave account is locked first; while a period is unlocked, the preview ignores its own earlier, not yet reversed penalty debits.
- **Employed part:** GET counters uses the employee's employed part of the period, as the totals and the lock do.
- **Bounds:** a penalty never exceeds the days employed in the period; opening counters and opening permission use are 0-366.
- **Cost:** settings are loaded once per policy version per lock or preview.
- **Rounding:** excess minutes convert to days rounded down and part blocks count as nothing (shortcut until OQ-C06-25).

## Not in this slice
- **Recovery and reset options:** permission-balance recovery and fixed amounts or fines (money, V2); combined counters (EC-10); other reset periods.
- **Not built yet:**
  - Grace-use counters (LA-04 / EC-03).
  - LA-13 compensation and LA-15 / EC-13 conversion.
  - LA-19 escalation alerts (slice 17 notifications).
  - Absence cuts LM-25 / LM-26, and their order against penalties (OQ-X-05).
- **Approximations:**
  - An opening permission count uses only the count limit (not minutes).
  - A few queries per employee remain at lock (policy, counters, recovery).
  - A penalty is taken whole from one leave type, never split across two.
  - The minutes-to-days conversion uses one day length, not each day's required minutes.
  - Preview penalties have no id (`provisional: true`).

## Review focus
1. Lock idempotence and re-lock: no double leave debits, and reversals happen once.
2. Penalty arithmetic and BR-04; settings refused when not built.
3. The opening permission-use ledger (post and reverse once); counter correctness against results and openings.
