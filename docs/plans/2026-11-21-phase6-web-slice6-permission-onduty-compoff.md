# Phase 6 Web Slice 6: Permission, on-duty and comp-off — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The remaining request types on the web:
- **HR and managers** see and decide permission, on-duty and comp-off requests (with a "waiting for me" inbox like leave).
- **Employees** apply for permission and on-duty, see their own requests, cancel where allowed, see their comp-off balance and credits, and choose pay or comp-off for a worked off-day.

**Spec:**
- `docs/api/openapi/permissions.yaml`: permission types, permission requests, usage, and on-duty requests.
- `comp-off.yaml`: credits, choices, balance.
- PERMISSION_MATRIX PRM-02..13, CMP-04..07.

**Decision (owner, 2026-11-21):** web slice 6 is permission, on-duty and comp-off (slice order from the 11-slice outline).

## Backend (additive, the slice-3 lesson applied)
- **Shared two-level decide check:** `approval::can_decide_two_level` answers whether `decide` would accept the caller now:
  - never one's own (BR-07);
  - the level's grant with routing or scope;
  - never the first-level approver at level 2.

  Leave, permission and on-duty all use it; leave's own copy is replaced. Comp-off (single level, CMP-05) has `comp_off::can_decide`.
- **"Waiting for me":** `approval::awaiting_candidates` lists pending requests routed to the caller, to someone who delegated to them, or to nobody. Each type keeps only those its `can_decide` accepts. Exposed as `?awaiting_me=true` on the permission, on-duty and comp-off lists.
- **Detail flags:**
  - permission and on-duty requests carry `can_decide` and `can_cancel`, from the same checks as the commands;
  - comp-off credits carry `can_decide`.
- **Names:** all three carry `employee_code` / `employee_name`; permission requests also carry `permission_type_name`. Recorded in the contracts.
- **Tests:** one per type: inbox membership, the flags for the employee and for HR, and the request leaving the inbox once decided.

## Screens
- **Requests (`/app/requests`):**
  - Tabs per type the user can work with (permission: PRM-03/04/07/13; on-duty: PRM-09/10/11; comp-off: CMP-05/07), each with "Waiting for me" and "All".
  - Menu item "Requests" (C09 or M02).
- **A request (`/app/requests/{kind}/{id}`, and `/me/requests/{kind}/{id}` for employees):**
  - Its details and approval steps.
  - Approve / reject when `can_decide`; cancel with a reason when `can_cancel`.
- **My requests (`/me/requests`):**
  - **Permissions:** usage this period (PRM-13), apply (type, date, from–to, reason), my requests.
  - **On-duty:** apply (kind; dates; times only for "hours", as OD-02 requires; purpose; place), my requests.
  - **Comp-off:** balance and credits. The "pay or comp-off" choice waits until the API accepts choices (D-28).

## Tests
- **Vitest:** the menu item for approve-only grants.
- **Playwright (fresh seed):**
  1. Asha applies for a permission and an on-duty request.
  2. Meena (HR) approves the permission from "Waiting for me" and rejects the on-duty request with a reason.
  3. Asha sees both outcomes in her requests.

  The dev seed adds a permission type.

## Not in this slice
- **Not built yet:** permission types (settings slice), permission change and conversion suggestions (not built in the API), the comp-off ledger view, document and proof uploads.

## Review fixes
1. **Whoever may decide a request may open it:** GET of a permission, on-duty request or comp-off credit also admits a caller whose `can_decide` is true, as leave already did. A delegate (AC-03), or an approver reached through the unrouted pool, used to see the request in "Waiting for me" but get NOT_FOUND on opening it.
2. **Cancel follows PM-29:**
   - New `permission::can_cancel`, next to `cancel()`: the employee may cancel only before the start time; HR (PRM-07 in scope) at any time.
   - The route's `can_cancel` uses it, so a past permission no longer shows the employee a Cancel button that always fails.
   - The test now asserts that the employee may not cancel yesterday's permission, and that HR may.
3. **On-duty times only for "hours":** the form sends times only for "hours", as the API requires (half day with times was always refused).
4. **No comp-off choice form yet:** the "pay or comp-off" form is gone while the API refuses every choice (D-28). The credits list has an empty state, and a failure to load permission types is shown.
5. **Accessible filters:** the requests board's type and "waiting / all" switches are toggle buttons (`aria-pressed`), not tabs without panels.
