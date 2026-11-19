# Phase 5 Slice 11: Vendor Console and Admin Sign-in — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** The vendor can onboard a customer end to end:
1. Vendor staff sign in with their company Google accounts.
2. They create an organisation (plan, modules, employee limit, subscription, the seven standard roles with the matrix's default grants, and a first Organisation Admin).
3. They manage plans, modules, limits, vendor settings, subscription, suspension and vendor users.

Organisation admins sign in with email and password, plus an authenticator-app code when AC-08 is on.

**Decisions (owner, 2026-11-19):**
- **D-22:** vendor sign-in with Google accounts restricted to the vendor's domain (OQ-V01-17).
- **D-23:** the AC-08 second step is an authenticator-app code (TOTP, RFC 6238) (OQ-C03-06, method part).
- **D-24:** slice 11 as here; opening balances, support access, usage/health, sector-template maintenance and announcements move to slice 12.

**Architecture:**
- **Generated default grants:**
  - `docs/domain/gen_default_grants.py` turns PERMISSION_MATRIX.md into `domain-iam/src/defaults.rs`, covering both the system-role grants (P9) and the vendor-role grants (VA/VS/VR columns).
  - Cells: Y→all, S→scoped, Self→self, A→approve. `?` and `—` grant nothing. `Cfg(X)` → `setting_ref`.
  - Rerun the script whenever the matrix changes.
- **Vendor sign-in:**
  - `POST /v1/vendor/auth/google/sign-in` (contract amendment, D-22).
  - Verifies a Google ID token: Google's keys, issuer `accounts.google.com`, audience = `VENDOR_GOOGLE_CLIENT_ID`, `hd` = `VENDOR_GOOGLE_DOMAIN`, `email_verified`.
  - Finds the active `platform.vendor_user` and issues a 1-hour vendor token (`typ: vendor`). No refresh: sign in with Google again.
  - Organisation access tokens reject unknown claims, so a vendor or challenge token can never pass as one, and vice versa.
- **Vendor routes:**
  - Each request runs as `app_vendor`, which sees only the platform schema and the audit log. The vendor user's `active` is re-read per request.
  - The routes are mounted only when the two `VENDOR_GOOGLE_*` variables are set; otherwise they answer 404.
  - They are deployed as a third Cloud Run service (`payroll-vendor`) whose login role `vendor_login` holds `app_vendor` and `app_api`. The public API's `api_login` still cannot become `app_vendor`.
- **Organisation creation is one transaction:**
  1. Platform rows as `app_vendor`: organisation, plan, modules, limit, subscription, audit.
  2. Then `SET LOCAL ROLE app_api` with `app.set_org(new)`: the seven system roles with default grants, and the first admin with the Organisation Admin role and scope `all`.
  3. If the template's content holds `organisation_settings`, that becomes the first settings version.
- **Admin password sign-in:**
  - Email + password checked against the Argon2id hash across organisations (`iam.resolve_login`). Several organisations → ORGANISATION_CHOICE_REQUIRED, as for phone sign-in.
  - **AC-08 on:** 401 `SECOND_FACTOR_REQUIRED` with a 5-minute `challenge_token`.
    - **First sign-in:** the response also carries an `otpauth://` enrolment URI. The new secret travels encrypted (AES-256-GCM, `MFA_ENCRYPTION_KEY`) inside the challenge, and is stored only after the first correct code. That first code proves possession (trust on first use).
    - **Later sign-ins:** the stored secret is used.
  - `POST /v1/auth/second-factor/verify` checks the code (30-second steps, ±1 step) and opens the session. A wrong code → 400 OTP_INVALID.
  - **AC-08 off:** tokens directly.

## Rules
- **Vendor permissions (matrix VA/VS/VR):**
  - `full_admin` holds every operation here.
  - `read_only` views (PLT-10, PLT-19).
  - **Assumption, G-01 open:** `support` gets the same viewing rights as `read_only`.
  - A missing permission → 403. Organisation tokens → 401 on vendor routes.
- **Create organisation (PLT-01):**
  - The template and plan must exist.
  - Modules may only be optional modules M01–M04 that exist in V1 (V1: M02). Common modules C01–C16 are always on, so listing them → 422.
  - Every required part must be present (AC-V01-01.2).
  - **First admin:** an email (stored) and a **mobile** (required). The account signs in by phone OTP until a password-setting invitation exists, because the contract has no password for the first admin.
  - Audited with the template, plan, modules and limit.
- **Dated rows** (plan, limit, module entitlement):
  - A change from a date (default today) closes the open row at that date and starts a new one with the reason. A date at or before the open row's start → 422; history rows never change.
  - Switching a module on or off when it is already in that state → 200 with no change.
  - Switching off a common module → 422.
- **Vendor settings and subscription:** the current row is updated, with old and new values in the audit (the history for these kinds is the audit trail).
- **Suspend / reactivate (PLT-06):** sets the state, reason and time; already in that state → 200. Organisation writes are already refused while suspended (VC-R4).
- **History (PLT-19):** the dated rows (plan, module, limit) plus audit entries (vendor settings, subscription, state), oldest first.
- **Plans (PLT-04):** create (code unique, 409 ALREADY_EXISTS) and change (If-Match).
- **Vendor users (PLT-12):**
  - Create (email unique), change role (same role → 200), deactivate (already → 200).
  - A full admin cannot change or deactivate themselves, and the last active full admin cannot be removed.

## Not in this slice (slice 12)
- Opening balances (C15).
- Support access grants (VC-03, SM-SUPPORT).
- Usage and health views (J10).
- Sector-template maintenance; announcements.
- Copying a template's policy sets and lists into the organisation (OQ-V01-10).
- A first-admin password invitation.
- TOTP code-replay protection within a step: needs a column (Gate 3 amendment); for now a code stays usable during its 30–90 s window.

## Review focus
1. No organisation user can reach a vendor route, and no vendor token can reach an organisation route (and vice versa for challenge tokens).
2. Organisation creation is atomic and the roles switch correctly; the new organisation is isolated from the others.
3. The TOTP and secret handling: encryption, the trust-on-first-use window, timing, and that nothing secret is logged or returned beyond the enrolment URI.
4. The Google ID token checks (issuer, audience, `hd`, `email_verified`, expiry).
5. The dated-history invariants and the vendor-role permissions.
