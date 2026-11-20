# Phase 6 Web Slice 3: Leave and approvals — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR and managers handle leave on the web:
- see requests;
- decide those waiting for them;
- cancel where allowed;
- apply on an employee's behalf;
- see an employee's balances.

**Spec:**
- `docs/api/openapi/leave.yaml`: types, balances, the request list / get / approve / reject / cancel, and on-behalf.
- PERMISSION_MATRIX LVE-01..08.

**Decision (owner, 2026-11-20):** web slice 3 is leave plus approvals.

## Screens
- **Leave (`/app/leave`):**
  - **"Waiting for me":** requests routed to the caller or to someone who delegated to them.
  - **"All":** LVE-08 scope; filters for state and leave type.
  - **Rows:** employee, type, dates and parts, days, state.
  - Menu item "Leave" (C08): LVE-08 all/scoped, or LVE-04/05 approve.
- **Request (`/app/leave/{id}`):**
  - **Shows:** dates and parts, each day's fraction, reason, state, and approval steps.
  - **Approve / reject:** shown while pending (shared Decide control).
  - **Cancel with a reason (LVE-06):** offered while pending or approved, only to users who hold LVE-06.
- **Apply on behalf (`/app/leave/new`, LVE-03):**
  - **Fields:** an employee search (`/v1/employees?q=`), the leave types the API lists, from/to dates, parts (full, first half, second half) and reason. One Idempotency-Key per form open.
  - The chosen employee's balances (`/v1/leave/employees/{id}/balances`) are shown as the API gives them.
- **Employee page:** a "Leave balances" card for users with LVE-08 all/scoped.

**Open questions:**
- **No default role holds these grants:** LVE-03 (G-25), LVE-06 (G-24) and LVE-07 (G-26). The screens appear only for users given them.
- **Dev seed:** grants them to HR in a separate "Dev leave extras" role, local only.

## Backend (additive)
- Leave requests carry `employee_code`, `employee_name` and `leave_type_name`.
- **`GET /v1/leave/requests?awaiting_me=true`:** only pending requests routed to the caller, directly or through a delegation that is allowed. Recorded in leave.yaml.
- **`dev_seed`:**
  - leave types CL and SL;
  - 6 days of CL for Asha (ledger adjustment);
  - the dev-only grants above;
  - a pending CL request by Asha.

## Tests
- **Backend:** names and `awaiting_me` (the approver sees it, others do not, and it disappears once decided).
- **Playwright (fresh seed):**
  - HR approves Asha's pending CL request from "Waiting for me";
  - HR applies a half-day SL on behalf of Asha and sees it;
  - HR cancels it with a reason;
  - balances show on Asha's employee page.
- **Vitest:** the menu item for approve-only users.

## Not in this slice
- Leave types and versions (settings slice), ledger and manual adjustments, bulk decisions, absconding cases, the cross-type `/v1/approvals/pending` (not built in the API), and employee self-apply (self-service slice).

## Review fixes
1. **"Waiting for me" matches the decision itself:**
   - `awaiting_me` now lists exactly the pending requests `can_decide` accepts: routed to the caller, through an allowed delegation, or unrouted (no manager, or the manager was the requester) within the caller's LVE-04 / LVE-05 scope.
   - `can_decide` also applies decide()'s rule that the first-level approver never decides the second level.
   - Test: `unrouted_requests_wait_for_scoped_deciders`.
2. **Approve / Reject / Cancel follow the API's answer:**
   - `GET /v1/leave/requests/{id}` returns `can_decide` and `can_cancel`, from the same checks as the commands; the page shows the buttons from those.
   - Requesters can now cancel their own requests, which the hard-coded LVE-06 test had hidden.
3. **The list shows the API's day count:** each request carries `total_days`, so the list no longer adds day fractions in the browser.
4. **On-behalf picker:**
   - It uses a choice only while it is still listed, so it can never submit for someone a later search dropped.
   - The search waits 300 ms after typing stops, and search errors are shown.
   - Retired leave types are left out.
   - The page checks LVE-03 on the server.
5. **Error messages use the translations:** the alert looks every API error code up in the translation file instead of a fixed list, so codes added in later slices (LIMIT_EXCEEDED, SELF_APPROVAL_FORBIDDEN, ...) show their message.

Not changed: the dev seed already refuses non-local databases (checked by host).
