# Phase 5 Slice 14: On-duty (C09, part 2) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Employees apply for on-duty (official work away from the workplace) for hours, a half day, a full day or several days. Approvers decide on the shared approval flow. Approved on-duty counts as worked with no late or early mark for the time it covers (PM-R5, PM-R6), and never uses the permission limit (OD-12).

**Scope (D-24, re-split):** slice 14 on-duty; slice 15 compensatory off; slice 16 late/early counters and period penalties; slice 17 reports, notifications, dashboards and the remaining vendor views.

**Spec:** `docs/api/openapi/permissions.yaml` (`/v1/on-duty/requests*`); BRD PM-R5, PM-R6, OD-01..OD-12; SM-REQ; PERMISSION_MATRIX PRM-08..PRM-11; schema 0004 (`perm.on_duty_request`). No new migration.

## Rules
- **Settings (OD rule group, `domain-policy/src/od.rs`):** parsed when the policy is saved. Options whose behaviour is not built yet are **refused (422), never stored and ignored:**
  - OD-03 other than `none` (limit outcome, OQ-C09-12);
  - OD-05 `timeout_action` other than `escalate` (no on-duty timer runs);
  - OD-06 mandatory proof (no file attachments yet);
  - OD-07 other than `not_required` (OQ-C09-13);
  - OD-08 true (overtime is V3);
  - non-empty OD-11 eligibility;
  - OD-12 (fixed) and unknown keys.
- **Submit (PRM-08, own):** each refusal creates nothing.
  - OD-01 off → RULE_GROUP_DISABLED.
  - OD-02 kind off; purpose/place missing (OD-06); `to_date = from_date` unless several days (at most 61 days); times exactly for `hours`; a `proof` file → 422.
  - OD-04 window on the employee's own date (before only, or up to N days after) → OUTSIDE_REQUEST_WINDOW.
  - Locked day → PERIOD_LOCKED.
  - Full-day leave on a day, or any leave for a full/several-day on-duty, or another active on-duty on those dates → OVERLAPPING_REQUEST.
  - Routed by AC-02 (`on_duty`, level 1).
- **Decide (PRM-09 level 1, PRM-10 level 2 when OD-05 = manager_then_hr):** as permissions — BR-07, delegation, scope 404, the same person never at both levels, reject needs a reason, a locked day refuses final approval. Final approval recalculates every covered day.
- **Cancel (PRM-11):** G-27 is open, so no role holds it by default; whoever is granted it may cancel within scope with a reason. An approved on-duty's days are recalculated from punches (AC-C09-11.6).
- **Effect on the day (engine version 4):**
  - Full day / several days: the day is `on_duty`, fraction 1.00, punches kept as evidence but excluded (PM-R6).
  - Hours: like a permission that counts as worked — an edge of the shift moves, and the uncovered part is credited.
  - Half day: covers the half without kept punches (first kept punch before mid-shift → second half, else first half); the split point is the shift's fixed half-day split if set, else mid-shift. Which half is not a stored column (OQ-C09-12).

## Not in this slice
- Proof files, OD-07 punch evidence, OD-03 limits, OD-09 daily allowance (V3), OD-10 comp-off (slice 15), OD-11 eligibility, the on-duty timer (J3), the change route (G-27).
- Leave submission does not yet refuse a day already covered by approved on-duty (the day shows on-duty; leave balance still charged).

## Review focus
1. Every refusal creates nothing; overlap and locks are serialised per employee (advisory lock).
2. The engine change: full-day short-circuit, hours/half-day edge moves and credit, fingerprints (ENGINE_VERSION 4), and no effect from pending, cancelled or rejected requests.
3. BR-07, scope, two-level separation, locked days.
