# Phase 5 Slice 4: Daily Attendance Results — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn each employee's punches into one explained, append-only result per day: sessions, worked minutes, late and early minutes and marks, day status, and the day effects (deductions in days or minutes, never money). Recalculate whenever a punch, a flag decision or a manual entry changes a day.

**Architecture:**
- New crate **`engine`**: a pure function `compute_day(&DayInput) -> DayOutput` with no database or network calls (TSD §5.2, BR-02). The API, later jobs and the tests all call it.
- New crate **`domain-attendance`**: gathers the inputs (expected day, shift version, policy version, punches and their flag states), runs the engine, and stores the result. It skips the write when the input fingerprint has not changed (SM-DAY A11) and never writes a current result for a locked date (BR-06).
- Results are append-only (BRX-03). `attendance_day.current_result_id` points at the newest one; a new result supersedes it.

**Spec:** BRD v0.8 §9.6–9.10 (PR, BK, LA, EC, EA, MP, DS), BR-02/03/04/06/08/09, worked examples E1, E2, E6, E7, E8, E13; DECISIONS D-04, D-05c, D-07, D-08, D-09, D-17; `docs/api/openapi/attendance.yaml` (`attendanceGetDay`, `attendanceListDaily`, `attendanceRequestRecalculation`); migrations 0005, 0007, 0008 (`attendance.*`, `missed_punch_cutoff`).

## Rules in this slice

| Group | Evaluated now | Later |
|---|---|---|
| PR | PR-01 paired / first-in-last-out (direction-tagged falls back to paired until devices send tags), PR-02/03 repeat window, PR-04 minimum gap, PR-05 maximum session, PR-06 too many punches | — |
| BK | BK-01 none / fixed auto-deduct, BK-02 paid/unpaid, BK-03 duration | window, flexible, caps, excess (BK-04..09) |
| LA | LA-01 on/off, LA-02 measured from, LA-03 grace, LA-05 slabs (no action, late mark, deduct minutes, quarter/half/full day), LA-23 | counters and period penalties (LA-04, 06..12), make-up, permission conversion, hold check-in, waivers, flexible shifts |
| EC | EC-01, EC-02, EC-04, EC-15; BR-09 (a missing check-out is never early) | counters, prior approval, permission conversion |
| EA | EA-01 shift start / punch time, EA-02 cap (reported only) | approval variant |
| MP | MP-01, MP-02 before the cut-off; MP-03 at the cut-off (D-08) | reminders (MP-04), auto check-out (MP-05) |
| DS | DS-01 shift / hours / hybrid, DS-02, DS-03, DS-04, DS-05 half day / deduct minutes, DS-06, DS-07 | DS-05 make up within the week, DS-09, DS-10 debits (needs Leave) |

Settings are typed and validated on `PUT .../rule-groups/{PR,BK,LA,EC,EA,MP,DS}` like PC. Keys not evaluated yet are still accepted and stored, and the PUT response lists them in `warnings` so the user knows they have no effect yet.

**BR-04 (no double penalty):** a late arrival or early leaving is also an hours shortfall. Under hybrid status, the larger of (slab deductions) and (hours deduction) applies once; under shift-based status only slabs apply; under hours-based status only the hours rule applies (E8).

**Absent working day:** status `absent`, processing state `absence_pending` (DS-10 default "pending for 3 days"); the loss-of-pay or leave debit waits for the Leave slice.

**Worked on a weekly off or holiday:** status stays `weekly_off` / `holiday`, flag `worked_on_off_day`, minutes recorded (comp-off later). No schedule at all: status from punches, flag `no_shift`.

## Rulings (to confirm)
- **Recalculation runs inline** in the same transaction as the punch, flag decision or manual entry (that date, and the day before for night shifts). TSD §5.2 queues it through Cloud Tasks; the same function moves behind the queue in the deployment slice.
- **The missed-punch cut-off (D-08)** is applied whenever a day is recalculated after it. The nightly job that recalculates open days at the cut-off comes with the deployment slice (Cloud Scheduler).
- **Overlapping punch windows (OQ-C05-04):** each day's window is half-open and clipped by its neighbours, so no punch counts on two days. Two timed shifts split their overlap halfway between the first one's end and the next one's start; a timed shift keeps its whole window against an off day.
- **Zone:** shift times, cut-off and `schedule_tz` use the zone of the branch punched at (D-04), else the master branch.
- **Open shifts and shifts without set times** are hours-only days against their required minutes (DS-01 b), with no late or early.
- **No result for a future date**, and none for a day with no schedule and no punches (nothing was expected).
- **Slab ranges are raw minutes late** (default 11–120 with grace 10); LA-02 only changes the minutes reported.
- **A day with no completed pair gets no late or early slab** on top of being absent (BR-04). MP half day sets the day to a half even from nothing; "absent if none" belongs to pairs-only (MP-03).
- **A stored rule-group document that fails the newer typed checks** keeps that group's defaults instead of blocking punches (shortcut: no visible warning yet).
- **A day with no policy version in force** gets no result (`day_result.policy_version_id` is required); the day shows `processing_state = exception`.

## Tasks
1. **Typed settings** for PR, BK, LA, EC, EA, MP, DS with BRD defaults and `warnings` for not-yet-evaluated keys. Unit tests per group; API test for a 422 and a warning.
2. **Engine: punches to sessions** (window selection SH-07, exclusion of rejected and not-yet-counting flagged punches per D-05c, PR-02..06, breaks). Unit tests.
3. **Engine: day outcome** (late/early, slabs, MP, DS, BR-04, BR-09, EA, explanation steps, fingerprint). Unit tests for E1, E2, E6, E7, E8, E13 and the cut-off.
4. **Store and recalculate** (`domain-attendance::recalculate`): append-only result + punches used + effects, supersede, fingerprint skip, lock skip, outbox `DayResultChanged`; called from punch, sync, flag decision and manual attendance.
5. **Reads and HR recalculation:** `GET /v1/attendance/employees/{id}/days/{date}`, `GET /v1/attendance/daily` (ATT-01, scope), `POST /v1/attendance/recalculations` (ATT-06, range ≤ 31 days, 202).
6. Full suite, clippy, final review, merge, push.

## Review focus
1. E6: in 11:30 → half day, late slab 0.5 day, **no** late mark (LA-23 = no).
2. E13: in 09:00, no out → before the cut-off flagged `missed_punch`, state `exception`, no early mark (BR-09); after the cut-off MP-03 pairs only → absent.
3. A punch whose flag is raised and does not count (outside_fence) is excluded until accepted; accepting it recalculates the day.
4. Recalculating with unchanged inputs writes nothing; a locked date never gets a new current result.
5. A night shift (22:00–06:00) is one day, dated by its start (SH-06 default).
