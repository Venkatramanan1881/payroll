# Phase 5 Slice 13: Permissions (C09, part 1) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Employees apply for short permissions (hours away within a shift) under per-type rules. Approvers decide them on the shared approval flow. Usage is tracked against the period's limit (PM-R4), and an approved permission counts as worked time with no late or early mark inside it (PM-R6, PM-12).

**Scope split (continuing slices 11–12):**
- **Slice 13:** permissions only.
- **Slice 14:** on-duty (it needs the OD rule-group settings parsed), compensatory off, and the late/early counters, which unblock their opening balances.
- **Slice 15:** reports, notifications, dashboards and the remaining vendor views.

**Spec:** `docs/api/openapi/permissions.yaml` (types, versions, requests, approve/reject/cancel, usage); BRD PM-R1..R4, PM-R6, PM-01..PM-30; SM-REQ; PERMISSION_MATRIX PRM-01..07, PRM-13; schema 0004 (`perm.*`). No new migration.

## Rules
- **Types (PRM-01):**
  - A type is created with its first version; versions are append-only (the same `effective_from` → VERSION_FROZEN; reason required).
  - Options whose behaviour is not built yet are **refused (422), not stored and ignored:**
    - PM-18 other than `block` (OQ-C09-05);
    - PM-19 other than `late_early_minutes` (OQ-C09-08);
    - PM-23 carry forward and PM-24 proration (OQ-C09-02);
    - non-empty PM-25 eligibility.
- **Submit (PRM-02, own):** each refusal creates no request.
  - Reason (PM-28).
  - Block size (PM-03) and minimum/maximum (PM-04).
  - Blackout (PM-26).
  - Position inside the shift (PM-10).
  - Application window and notice (PM-14 / PM-15) → OUTSIDE_REQUEST_WINDOW.
  - Locked day → PERIOD_LOCKED.
  - Same-day half leave (PM-20) and overlap with another active permission → OVERLAPPING_REQUEST.
  - Per-day and per-week caps (PM-09; pending requests count) and the period limit (PM-05..PM-08, used = the usage ledger, opening balances included) → LIMIT_EXCEEDED.
  - **PM-16:** `auto_within_limit` → approved at once (usage posted, day recalculated); else pending_l1 routed by AC-02 (level 2 for `manager_then_hr`).
- **Decide (PRM-03 / PRM-04):** as leave:
  - BR-07, delegation, scope 404, the same person never at both levels, reject needs a reason.
  - Final approval re-checks the limit and the lock, posts one usage entry (idempotent by the request) and recalculates the day.
- **Cancel (PM-29):** the employee before the start time (PRM-06); HR within scope after it, with a reason (PRM-07). An approved permission gets a usage reversal and a recalculation; a locked day → PERIOD_LOCKED.
- **Usage (PRM-13):** per type for the period containing `on_date`: used, limit, remaining.
- **Limit periods (PM-08):** week (Monday-based, OQ-C09-03), calendar month, quarter, year; `payroll_period` = the attendance period, else the calendar month (OQ-C09-01).
- **Effect on the day (engine version 3):**
  - **At the start of the shift:** the expected start moves to the permission's end, so no late mark falls inside it.
  - **At the end:** the expected end moves to its start, so no early mark.
  - **Credit:** minutes that count as worked (PM-12) are credited to net minutes (`DayInput.credit_min`, explained as PM-12).

## Not in this slice
- **Not built yet:**
  - the PM-29 change route (OQ-C09-10);
  - late/early conversion (LA-15 / EC-13, GAP-SM-11);
  - J3 timeouts for permissions;
  - documents (file links);
  - PM-21 adjacent off and PM-22 consecutive days;
  - PM-27 punch matching;
  - the PM-17 timer.
- **Stored but not enforced (known):** PM-27 punch requirements and PM-13 overtime (overtime is V3).

## Review focus
1. Every refusal creates nothing. Usage posts once and reverses once. The limit is re-checked at decision.
2. The engine change: edge moves, credit, fingerprints (ENGINE_VERSION 3), and no effect from cancelled or rejected requests.
3. BR-07, scope, PM-29 actor rules, locked days.
