# Phase 6 Web Slice 9: Users and access — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** the Organisation Admin and HR manage who can do what:
- users;
- roles and their grants;
- who approves each request type;
- delegations;
- access settings (AC-03, AC-04, AC-06, AC-08) and what employees see about themselves (AC-05);
- vendor support-access requests.

**Spec:**
- `docs/api/openapi/identity.yaml` (users, roles, approver rules, delegations, access settings, support-access grants);
- PERMISSION_MATRIX IAM-01..10, PLT-14, PLT-15, PLT-18.

**Decision (owner, 2026-11-21):** web slice 9 follows the 11-slice outline.

## What the API offers today
- **Built:** every operation above.
- **Two limits, shown as the API reports them:**
  - **AC-04 "with Organisation Admin approval"** is refused: the settings-change approvals are not built.
  - **The Organisation Admin's lookups:** they hold no ORG-07 by default (G-14), so the branch and department lists their scope picker offers come back empty. "All" still works.

## Backend (dev only)
- **`dev_seed`:**
  - adds an Organisation Admin, `admin@dev.invalid`, with password sign-in and no authenticator step (AC-08 off in the seed);
  - adds a dev-only role holding IAM-06, IAM-09 and PLT-15. No default role holds them (G-08, G-10, G-03 open);
  - adds one support-access request from the dev vendor.

## Screens (`/app/access`, one menu item, C03)
- **Sub-navigation:** Users · Roles · Approvers · Delegations · Settings · Support access.
- **Users (IAM-01):**
  - **List:** sign-in, roles, scope, active.
  - **New user:** mobile with OTP, or email with an initial password; roles; one scope (all, a branch or a department).
- **A user:**
  - change roles and scope with a reason (IAM-02, If-Match);
  - deactivate or reactivate with a reason (IAM-01).
- **Roles (IAM-03):**
  - **List**, and a new custom role (IAM-04, AC-06).
  - **A role:**
    - rename a custom role (IAM-04, If-Match);
    - grants: change kind or condition, remove, or add an operation code. Replaced as a whole with a reason (IAM-03, If-Match). The API checks every code.
- **Approvers (IAM-05):**
  - the rule per request type and level;
  - set level 1 and an optional level 2: reporting manager, named user, department head or HR. A reason is required, with If-Match on the rules.
- **Delegations (IAM-09):**
  - list, with users shown by sign-in;
  - delegate an approver's approvals to another user for a window;
  - end one now.
- **Settings:**
  - access settings (IAM-06);
  - employee self-visibility (IAM-07).
  - Each needs a date it applies from (today or later) and a reason, with If-Match.
- **Support access:**
  - requests, with approve or reject (PLT-14, approve-type);
  - end an active grant with a reason (PLT-15);
  - the vendor's actions under a grant (PLT-18).

## Tests
- **Playwright (fresh seed).** The file is `users-access.spec.ts`, so it runs last: a future-dated settings version would otherwise hold back the period-settings test.
  1. **As the Organisation Admin:**
     1. create a user;
     2. change their access;
     3. deactivate them;
     4. create a role and give it a grant;
     5. delegate HR's approvals to the admin, then end the delegation;
     6. approve the vendor's support request, then end it.
  2. **As HR:**
     1. set the leave approver to the reporting manager plus HR;
     2. hide deduction reasons from employees, from tomorrow.

## Not in this slice
- Approving settings changes (AC-04); it waits on the backend.
- Linking a new user to an employee record. The Organisation Admin cannot read employees by default (G-14).
- Several scopes per user (OQ-C03-04): one is offered.
- An operation catalogue for the grants editor: codes are typed and the API validates them.

## Review fixes
1. **Saved choices show again:** a user's branch or department scope, and a named-user approver, now show once their options load. The selects are controlled; with `defaultValue` they came up blank. The browser test saves a branch scope and reloads to check it.
2. **Delegation windows are India time,** entered as they are shown (`zonedToUtc`), and labelled so.
3. **Grants of a switched-off module survive a save** (a backend bug that predates this slice):
   - `change_grants` deleted every grant, including those hidden while their module is off (AC-C03-01.5). Saving a role then wiped its comp-off grants for good.
   - Hidden grants are now kept unless the request sends them again. `tests/iam.rs` checks this.
4. **Approver defaults are shown:** a level with no rule shows the API's default (level 1 reporting manager, level 2 HR), instead of "None" as if it were switched off.
5. **Named-user approvers need the user list:** that choice is offered only to callers who can read users (IAM-01). HR cannot (G-08), and the "Not done" item below covers it.
6. **The admin's settings page:**
   - The dev seed gives the Organisation Admin ORG-07 (G-14 open), so their settings page and scope picker work. The browser test now saves access settings as the admin.
   - The page says it shows the settings in force today; a later-dated change appears from its date.
7. **No lost input:** forms remount only when the data they edit changes, not on a rename, a deactivation or the other form's save.
8. **Roles are read where the API allows it:** under IAM-03 too, and a refusal is shown rather than a form quietly missing.
9. **Accessibility:** support-access buttons (approve, reject, end, show actions) and grant rows name their item. `Decide` takes an optional `label` for this.

**Not done:**
- **Controls follow all-or-scoped grants,** while the handlers require organisation-wide scope. `/v1/me` does not carry a grant's scope; this is the same limitation as slice 8.
- **HR choosing a named-user approver** needs a user list readable under IAM-05 (a contract change, G-08).
- **Browser tests assume a fresh seed,** as the earlier ones do. A rerun on the same database meets the earlier delegation and dated versions.
