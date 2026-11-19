# Phase 5 Slice 12: Opening Balances and Vendor Support Access — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:**
- A customer starting part-way through a year enters opening balances (leave per type; late and early counters), confirms them once, and later changes them only with a reason (C15).
- The vendor can request time-limited support access to an organisation; the Organisation Admin approves, rejects or ends it and sees the action log (VC-R6, VC-03).

**Scope note:** slice 11's split continues. Usage and health (J10), sector-template maintenance and announcements move to slice 13 with reports.

**Spec:**
- Contracts: `docs/api/openapi/opening-balances.yaml`; `identity.yaml` (`/v1/support-access-grants*`); `vendor.yaml` (`/v1/vendor/organisations/{id}/support-access-requests`, `/v1/vendor/support-access-grants`).
- BRD OB-R1..R5, OB-01, OB-02; VC-R6, VC-03.
- STATE_MACHINES SM-OB, SM-SUPPORT.
- PERMISSION_MATRIX OBL-01..04, PLT-13..18.
- Schema: 0005 (`openbal.*` with the confirmed-entry guard trigger); 0001 (`platform.support_access_grant`); 0006 (`organisation.support_access_needs_approval`).
- No new migration.

## Rules — opening balances (`domain-attendance::openbal`)
- **Settings:** OB-01 (`hr_only` default / `hr_with_org_admin_approval`) and OB-02 (allowed with a reason, default / not allowed) live under `opening_balances` in the organisation-settings versions, carried forward like the access and period settings. No contract route sets them yet, so the BRD defaults apply until one does.
- **Entries:**
  - The as-of date is mandatory and the employee must be within the caller's scope; otherwise it is a row error (422, `errors[].field = entries[i]`).
  - `leave` needs a leave type of the organisation.
  - `late_counter` and `early_counter` take a count.
  - `comp_off` and `permission_used` → 422 until their modules exist (slice 13): there is nowhere to post them yet.
  - An as-of date inside the employee's locked attendance → PERIOD_LOCKED.
  - One live entry per employee, kind and type in a batch (database index → 409 ALREADY_EXISTS).
- **Draft:** add, edit (If-Match) and remove entries; the batch is draft only, else ILLEGAL_TRANSITION.
- **Submit (B4):** only when OB-01 needs Organisation Admin approval, else ILLEGAL_TRANSITION. Draft → pending_approval, with an approval step `opening_balance`.
- **Confirm:**
  - **B3:** draft → confirmed by HR when OB-01 = `hr_only`. When approval is needed, confirming a draft → ILLEGAL_TRANSITION.
  - **B5:** pending_approval → confirmed by an OBL-02 approver who neither submitted nor created the batch (BR-07 → 403).
  - **Posting:** one `opening` leave-ledger entry per leave entry (quantity = value, entry date = as-of date, cause = the entry). It is idempotent by the ledger's unique key. A zero value posts nothing. Counter entries need no posting: the counter rebuild reads confirmed entries.
  - Confirming twice → 200 with no second posting.
- **Reject (B6, GAP-SM-13):** pending_approval → draft with a reason (decided here: the batch returns for correction). Same BR-07 rule.
- **Change after confirm (B7, OB-02):**
  - A reason is required.
  - A new entry in the same batch, and the old one gets `superseded_by_entry_id`.
  - Leave: a `reversal` of the old opening entry and an `opening` entry for the new one.
  - OB-02 not allowed → RULE_GROUP_DISABLED.
- **Scope:**
  - List, get and entries (OBL-04) show only employees in scope.
  - Confirm, submit and reject need every entry in the caller's scope (else 403).

## Rules — support access
- **Request (PLT-13, vendor):**
  - A reason is required.
  - **Window length (OQ-V01-13, decided here):** `requested_window_minutes` (default 120, at most 480).
  - VC-03 (`support_access_needs_approval`, default on) → `requested`; off → `active` at once, from now. Every request is new.
- **Organisation side:**
  - **List:** PLT-14 for approvers, or PLT-18 for the log readers.
  - **Approve (PLT-14):** requested → active, window from now to `window_end` or the requested length.
  - **Reject:** requested → rejected (reason).
  - **End (PLT-15):** active → ended; no role holds PLT-15 by default (G-03).
  - Repeats → 200; other states → ILLEGAL_TRANSITION.
  - An organisation user never approves for a vendor user (approvers are organisation users only).
- **Expiry (S5):** an active grant past `window_end` is shown and treated as `expired` (checked on every read).
- **Action log (PLT-18):** audit entries carrying the grant id. Data access under a grant (a grant-bound token, PLT-16) is G-02 / OQ-V01-12: not built, so a grant grants no data access yet.
- **Matrix:** PLT-14, PLT-15 and PLT-18 are organisation-side operations. They become grantable to organisation roles and the generator emits their organisation cells (Organisation Admin: PLT-14 approve under Cfg(VC-03), PLT-18; Auditor: PLT-18 scoped).

## Review fixes
- **OB-01 rights are separate:** entering, draft confirmation and submission need an entering grant (all/scoped). A pending batch is decided only by an approve-type holder, so a second HR Manager can no longer bypass the Organisation Admin.
- **Scope:** only a full or approve grant over scope `all` sees the whole organisation. A self-only reader sees their own balances and only the batches holding them.
- **BR-07 counts everyone who entered, edited, removed or submitted** (entries' `updated_by` plus the audit), not only the creator and the current submitter. A rejecting approver may still approve the corrected batch.
- **Locked periods:** confirmation re-checks them for every live entry (PERIOD_LOCKED), and the PERIOD_LOCKED code is used everywhere for a locked as-of date.
- **Late and early counters are refused for now:** no counter rebuild reads them yet, so accepting them would silently drop them.
- **Corrections:** a change's reversal is dated on the old as-of date; adding entries needs If-Match on the batch; batch-wide checks run under the batch row lock.
- **Support access:**
  - The requested window length is kept (on the request's audit entry) and used when approval gives no `window_end`.
  - Requests replay by key.
  - A deactivated vendor user's active grants end with them.
- **Known gap:** organisations created before this slice lack the Organisation Admin's default PLT-14/PLT-18 grants. Only test organisations exist, so nothing to backfill.

## Not in this slice
- Late and early counter opening balances (with the period-counter rebuild).
- The spreadsheet upload (needs `/v1/files/upload-links`).
- Comp-off and permission opening balances (slice 13).
- OB-01 and OB-02 setting routes.
- The grant-bound support token (G-02).
- Usage and health; templates; announcements.

## Review focus
1. Confirmed balances never change in place (the trigger plus the code); posting is idempotent; changes reverse and re-post.
2. BR-07 on approval; OB-01 state rules; scope on every route.
3. Support grants: state machine, expiry, the VC-03 switch, cross-organisation isolation of grants, vendor versus organisation actors.
