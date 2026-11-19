# Phase 5 Slice 10: Users, Roles and Access (C03) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** An organisation manages its own access through the API, so test fixtures are no longer the only way:
- user accounts, with their roles and data scope;
- roles and their grants;
- approver routing and delegations;
- the access settings.

Each request reads grants and scope afresh, so every change takes effect on the user's next request.

**Architecture:**
- New module `domain-iam::admin`; routes in `api/src/routes/iam.rs`.
- Access settings (AC-03..06, AC-08) live in the organisation-settings versions under their own keys (already carried forward by every version). A change writes a new version from `effective_from`, keeping every other key.
- Passwords are hashed with Argon2id (new dependency `argon2`); the password itself is never stored.
- No new migration.

**Spec:** `docs/api/openapi/identity.yaml` (`/v1/me*`, `/v1/users*`, `/v1/roles*`, `/v1/approver-rules*`, `/v1/delegations*`, `/v1/access-settings*`); BRD UM-R1..R4, AC-01..AC-08; C03 user stories US-C03-01..10; PERMISSION_MATRIX IAM-01..11, P9, P10; schema 0002 (`iam.*`).

## Rules
- **Who (G-07, G-08, G-10 open; the matrix's Y cells):** these are organisation-wide operations; a branch- or department-scoped holder gets 403.
  - Users and their access: IAM-01, IAM-02.
  - Grants: IAM-03; custom roles: IAM-04.
  - Approver rules: IAM-05.
  - Access settings: IAM-06; employee visibility: IAM-07.
  - Delegations: IAM-09.
  - Own language: IAM-11 (self).
  - Reading users needs IAM-01, reading roles IAM-03.
- **Users:**
  - `mobile_otp` needs a mobile number; `password` needs an email and an initial password (8+ characters), stored as an Argon2id hash.
  - Roles must be the organisation's own. An employee link must exist.
  - Scopes: exactly one `all`, or branches only, or departments only. Mixing → 422 (OQ-C03-04).
  - Deactivate sets the account inactive and revokes every session. Sign-in and existing tokens are already refused per request (AC-C03-03.7). Reactivate restores the account. Both are idempotent and need a reason.
  - Change-access (If-Match) replaces roles and scopes; old and new values are audited (AC-C03-03.6).
  - Nobody deactivates or changes their own access (it prevents locking yourself out).
- **Roles:**
  - System and custom roles are listed. Custom roles need AC-06 on, else 403 (AC-C03-02.2); they start with no grants (P9); names are unique.
  - Only custom roles can be renamed (OQ-C03-01).
  - Change-grants (If-Match, reason) replaces the grants. Codes must be operations of the matrix's organisation sections (§3.2–§3.17); PLT- codes and the fixed denials ADT-03 and ADT-04 → 422 (P10). Old and new values are audited (AC-C03-01.6).
  - Grants of a switched-off optional module (comp-off CMP- without M02) are left out of reads (AC-C03-01.5, MD-03).
- **Approver rules:** set per request type, at one or two levels.
  - `named_user` needs an active user of the organisation.
  - The level a request type does not set returns to its default (level 1 reporting manager, level 2 HR); set rules are listed.
  - BR-07 stays enforced per request (routing already sends a requester's own request to the HR pool).
- **Delegations:**
  - AC-03 off → 403 (AC-C03-08.3).
  - Both users active, delegate ≠ approver, `valid_from` < `valid_to`; an overlapping window of one approver → 409 CONFLICTING_SETTINGS.
  - Ending shortens the window to now; already ended → 200.
- **Access settings:**
  - Defaults: AC-03 on (the BRD gives none; matches the code); AC-04 `hr_only` (BRD); AC-06 on (BRD); AC-08 on (BRD); AC-05 items all on (OQ-C03-13 open: employees see their own data).
  - A change needs a reason and `effective_from`. It writes a new organisation-settings version from that date, carrying everything else.
    - The same date as an existing version → 409 VERSION_FROZEN.
    - A later version already exists → 409 ILLEGAL_TRANSITION (it would undo the change).
    - No organisation settings yet → 422.
  - **AC-04 = `hr_with_org_admin_approval` is refused (422) until the settings-change approval workflow (SM-SETCHG, `/v1/settings-changes`) is built.** So every change here applies at once (200), never 202.
- **Me:**
  - `GET /v1/me`: own account, roles, scopes, effective grants (strongest per operation) and the offered languages (OR-03).
  - `PATCH /v1/me/language`: If-Match; only an offered language.
  - `GET /v1/me/modules`: today's entitlements (common modules always on) and the plan's `mobile_app`.

## Not in this slice
- Password sign-in and the AC-08 second step (UM-R3; OQ-C03-06 decides who signs in with a password).
- Vendor support access (VC-03).
- The AC-04 settings-change approval workflow.
- Seeding the seven system roles with the matrix defaults: they come with organisation creation in the vendor console (slice 11).

## Review focus
1. No path lets a user grant themselves more, or act on another organisation's users, roles or delegations.
2. Deactivation cuts access immediately; sessions are revoked.
3. Grant validation (P10, PLT-), scope validation (OQ-C03-04), AC-06 and AC-03 switches.
4. Access-settings versions are append-only, never lose other keys, and do not silently revert.
5. Passwords are only stored as Argon2id hashes and never logged or returned.
