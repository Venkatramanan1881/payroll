# Phase 5 Slice 1: Isolation, Sign-in, Employee Master — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A running Rust API where an employee signs in with Firebase, and an HR Manager creates, reads, edits and re-assigns employees, with every request confined to the caller's organisation and scope.

**Architecture:** One Cargo workspace at `backend/` (ADR-001): `db` (migrations, organisation-scoped transactions), `domain-iam`, `domain-workforce` (use cases, no HTTP), `api` (Axum transport). Every request runs in one transaction that first does `SET LOCAL ROLE app_api` and `app.set_org(<org from token>)`, so RLS (migrations 0001–0008) enforces isolation even if application code is wrong. Crates for other contexts (`core`, `gateway`, `jobs`) are created by the slice that first needs them.

**Tech Stack:** Rust stable, Axum 0.8, Tokio, SQLx 0.8 (postgres, uuid, chrono, migrate, runtime-tokio, tls-rustls), serde, jsonwebtoken, sha2, rand, thiserror. Tests: `#[sqlx::test]` against a throwaway local PostgreSQL 16.

**Spec:** `docs/api/openapi/identity.yaml` (`identityFirebaseSignIn`, `identityChooseOrganisation`, `identityRefreshToken`, `identitySignOut`), `docs/api/openapi/workforce.yaml` (`workforceCreateEmployee`, `workforceGetEmployee`, `workforceUpdateEmployee`, `workforceListEmployeeAssignments`, `workforceChangeAssignment`), `docs/api/API_CONVENTIONS.md`, `docs/api/ERROR_CATALOG.md`, `docs/database/DATA_MODEL.md`, `docs/domain/PERMISSION_MATRIX.md` (WRK-01/02/03/08). All paths are under `.claude/skills/payroll-saas-engineering/`.

## Global Constraints

- Organisation always comes from the verified token; a body or parameter `org_id` is rejected (TB-01, API_CONVENTIONS §1).
- Every write and its `audit.audit_entry` row commit in the same transaction (TSD §3.3).
- Errors are `application/problem+json` with a stable `code` from ERROR_CATALOG.md; a resource outside scope or organisation is `404 NOT_FOUND`.
- `POST` commands require `Idempotency-Key`; same key + same body returns the stored response, different body → `409 IDEMPOTENCY_CONFLICT`.
- `PATCH` requires `If-Match`; stale → `412 PRECONDITION_FAILED`, missing → `428 PRECONDITION_REQUIRED`.
- Access token life 900 s; refresh tokens stored only as SHA-256 hashes (TSD §8).
- No secrets in the repo: `DATABASE_URL`, `APP_JWT_SECRET`, `FIREBASE_PROJECT_ID` come from the environment.
- Never run anything against Supabase or any non-local database in this slice.

## Review Focus

1. **A token from org A used on an org B employee id** → 404, never 200 or 403 (no existence leak). Test in Task 6.
2. **HR Manager limited to branch X reading or creating an employee in branch Y** → 404 / 403 FORBIDDEN-scope. Test in Task 6.
3. **The same `POST /v1/employees` retried with the same `Idempotency-Key`** → one employee, identical response. Test in Task 5.
4. **Two creates racing at the employee limit with VC-01 = block** → never more than the limit. Test in Task 5.
5. **A Firebase token for a number on two organisations** → 409 with candidates, and the opaque choice reference cannot be replayed for a third organisation. Test in Task 4.

---

## File Structure

```
backend/
  Cargo.toml                       workspace
  README.md                        how to run tests (throwaway PG on :55432)
  crates/db/                       migrations/ (moved from docs/database/migrations), src/lib.rs (OrgTx, pool)
  crates/domain-iam/               src/{lib,tokens,firebase,signin,authz}.rs
  crates/domain-workforce/         src/{lib,employee,assignment}.rs
  crates/api/                      src/{main,app,error,idempotency,extract}.rs, src/routes/{auth,employees}.rs
  crates/api/tests/                {auth,employees,isolation}.rs + common/mod.rs (seed helpers)
```

Migrations **move** (not copy) into `backend/crates/db/migrations/` as MIGRATIONS.md §1 planned; `docs/database/MIGRATIONS.md` and `DATA_MODEL.md` paths are updated in Task 1.

---

### Task 1: Workspace, migrations and organisation-scoped transaction

**Files:**
- Create: `backend/Cargo.toml`, `backend/README.md`, `backend/crates/db/{Cargo.toml,src/lib.rs}`
- Move: `docs/database/migrations/*` → `backend/crates/db/migrations/`
- Create migration `0009_idempotency.up.sql` / `.down.sql`: table `app.idempotency_key (org_id uuid, key uuid, request_hash bytea NOT NULL, response_status int, response_body jsonb, created_at timestamptz default now(), PRIMARY KEY (org_id, key))`, registered with `app.register_tenant_table`. **Gate 3 amendment: needs owner approval before merge.**
- Modify: `docs/database/MIGRATIONS.md` (location + 0009 row), `docs/database/DATA_MODEL.md` (§3 row)
- Test: `backend/crates/db/tests/org_tx.rs`

**Interfaces:**
- Produces: `pub static MIGRATOR: sqlx::migrate::Migrator` (`sqlx::migrate!("./migrations")`); `pub struct OrgTx<'a>(pub sqlx::Transaction<'a, Postgres>)`; `pub async fn begin_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgTx<'_>>` — runs `SET LOCAL ROLE app_api` then `SELECT app.set_org($1)`; `OrgTx` derefs to the transaction.

- [ ] **Step 1: Write the failing tests** in `org_tx.rs` using `#[sqlx::test(migrator = "db::MIGRATOR")]`:
  - `sees_only_own_org`: as superuser insert orgs A, B and one `org.branch` each; `begin_org(A)` → `SELECT count(*) FROM org.branch` = 1.
  - `unset_org_sees_nothing`: plain tx with `SET LOCAL ROLE app_api` and no `set_org` → count 0.
  - `cannot_write_other_org`: in `begin_org(B)` insert a branch with `org_id = A` → error code `42501`.
- [ ] **Step 2:** `cd backend && cargo test -p db` → FAIL (crate does not compile).
- [ ] **Step 3: Implement** `begin_org` and `MIGRATOR`; workspace members `crates/*`; SQLx features as in Tech Stack. README: start the throwaway cluster exactly as in Phase 3 (initdb into a temp dir, port 55432, trust auth) and `export DATABASE_URL=postgres://postgres@localhost:55432/postgres`.
- [ ] **Step 4:** `cargo test -p db` → 3 passed. Also run `docs/database/tests/tenant_isolation.sql` against a migrated database → "all checks passed".
- [ ] **Step 5: Commit** `feat(db): workspace, migrations in db crate, org-scoped transactions`.

---

### Task 2: API skeleton — problem+json errors and idempotency

**Files:**
- Create: `backend/crates/api/{Cargo.toml,src/main.rs,src/app.rs,src/error.rs,src/idempotency.rs}`
- Test: `backend/crates/api/tests/common/mod.rs`, `backend/crates/api/tests/skeleton.rs`

**Interfaces:**
- Consumes: `db::begin_org`, `db::MIGRATOR`.
- Produces:
  - `pub struct AppState { pub pool: PgPool, pub jwt_secret: Vec<u8>, pub phone_verifier: Arc<dyn domain_iam::firebase::PhoneTokenVerifier> }`
  - `pub fn router(state: AppState) -> axum::Router`
  - `pub enum ApiError { Unauthenticated, Forbidden, NotFound, OrgSuspended, ValidationFailed(Vec<FieldError>), Conflict(&'static str), PreconditionFailed, PreconditionRequired, IdempotencyConflict, EmployeeLimitReached, OrganisationChoiceRequired(Vec<OrgCandidate>), Internal(anyhow::Error) }` with `IntoResponse` → RFC 9457 body `{type,title,status,code,detail,correlation_id,errors}`; `Internal` never leaks its message.
  - `pub async fn with_idempotency<F, Fut>(tx: &mut OrgTx, key: Uuid, body: &[u8], run: F) -> Result<(StatusCode, serde_json::Value), ApiError>` — hash = SHA-256 of body; existing row with same hash → stored response; different hash → `IdempotencyConflict`.
  - Test helper `common::app() -> (axum::Router, PgPool)` and `common::seed_org(pool) -> SeedOrg { org_id, branch_id, hr_user_id, hr_token }` (HR role with grants WRK-01/02/03/08 = `scoped`, scope `all`).

- [ ] **Step 1: Write failing tests** in `skeleton.rs`: `GET /health` → 200 `{"status":"ok"}`; unknown route → 404 with `content-type: application/problem+json` and `"code":"NOT_FOUND"`; `ApiError::Internal` response body does not contain the inner message.
- [ ] **Step 2:** `cargo test -p api --test skeleton` → FAIL.
- [ ] **Step 3: Implement** error mapping (status per ERROR_CATALOG), router with `/health` and fallback, `main.rs` reading env (`DATABASE_URL`, `APP_JWT_SECRET`, `FIREBASE_PROJECT_ID`, `PORT` default 8080) and running migrations only when `RUN_MIGRATIONS=1` (never by default).
- [ ] **Step 4:** `cargo test -p api --test skeleton` → PASS.
- [ ] **Step 5: Commit** `feat(api): axum skeleton, problem+json errors, idempotency store`.

---

### Task 3: Tokens, authentication and authorization

**Files:**
- Create: `backend/crates/domain-iam/{Cargo.toml,src/lib.rs,src/tokens.rs,src/authz.rs}`, `backend/crates/api/src/extract.rs`
- Test: `backend/crates/domain-iam/src/tokens.rs` (unit), `backend/crates/api/tests/authz.rs`

**Interfaces:**
- Produces:
  - `pub fn issue_access(secret: &[u8], user_id: Uuid, org_id: Uuid, now: i64) -> String` (HS256, `exp = now + 900`, claims `sub`, `org`, `iat`, `exp`)
  - `pub fn verify_access(secret: &[u8], token: &str, now: i64) -> Result<AccessClaims, TokenError>`
  - Refresh token format `"{org_id}.{base64url(32 random bytes)}"`; `pub fn new_refresh(org_id: Uuid) -> (String, [u8; 32] /*sha256*/)`; `pub fn parse_refresh(token: &str) -> Option<(Uuid, [u8; 32])>` (the org prefix lets the server `begin_org` before the RLS-protected lookup).
  - `pub struct Principal { pub user_id: Uuid, pub org_id: Uuid, pub employee_id: Option<Uuid> }`
  - `pub async fn require(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Grant, AuthzError>` — union of the user's `iam.role_grant` rows for `op`; `Grant { kind: GrantKind /*All|Scoped|SelfOnly|Approve*/, scope: Scope }` where `Scope` is loaded from `iam.user_scope` (`All`, `Branches(Vec<Uuid>)`, `Departments(Vec<Uuid>)`).
  - `pub fn employee_in_scope(scope: &Scope, master_branch: Uuid, department: Option<Uuid>) -> bool`
  - Axum extractor `Auth(Principal, OrgTx)`: bearer token → `verify_access` → `begin_org` → user `active` and organisation `state = 'active'` (suspended + non-GET → `OrgSuspended`).

- [ ] **Step 1: Write failing tests:** unit `access_roundtrip`, `access_expired_rejected` (now = exp + 1), `access_wrong_secret_rejected`, `refresh_parse_rejects_garbage`; integration `no_token_401`, `inactive_user_401`, `suspended_org_post_403_ORG_SUSPENDED`, `missing_grant_403_FORBIDDEN`, `scope_branch_limits` (`employee_in_scope` true for listed branch, false otherwise).
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement** as above. Grants are read per request (no caching) so revocation is immediate.
- [ ] **Step 4:** `cargo test -p domain-iam && cargo test -p api --test authz` → PASS.
- [ ] **Step 5: Commit** `feat(iam): access/refresh tokens, auth extractor, grant and scope checks`.

---

### Task 4: Firebase sign-in, organisation choice, refresh, sign-out

**Files:**
- Create: `backend/crates/domain-iam/src/firebase.rs`, `src/signin.rs`; `backend/crates/api/src/routes/auth.rs`
- Test: unit tests in `firebase.rs`; `backend/crates/api/tests/auth.rs`

**Interfaces:**
- Produces:
  - `#[async_trait] pub trait PhoneTokenVerifier: Send + Sync { async fn verify(&self, id_token: &str) -> Result<String /*E.164 phone*/, TokenError>; }`
  - `pub struct GoogleVerifier { project_id: String, keys: RwLock<CachedKeys> }` — fetches `https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com`, caches for the response's `max-age`, verifies RS256 by `kid`, `aud == project_id`, `iss == "https://securetoken.google.com/{project_id}"`, `exp`/`iat`, non-empty `sub`, and reads `phone_number`.
  - Routes: `POST /v1/auth/firebase/sign-in`, `POST /v1/auth/organisation-choice`, `POST /v1/auth/refresh`, `POST /v1/auth/sign-out` (bodies and responses exactly as identity.yaml; `TokenPair { access_token, refresh_token, expires_in: 900, token_type: "Bearer" }`).
  - Organisation choice reference: HS256 JWT `{ sub: firebase phone, cands: [org_id...], exp: now+300 }`; the chosen org must be in `cands`.

- [ ] **Step 1: Write failing tests** (fake verifier in tests):
  - `GoogleVerifier` unit tests with a locally generated RSA key injected as the key set: valid token → phone; wrong `aud` → error; wrong `iss` → error; expired → error; unknown `kid` → error.
  - `signin_single_org_returns_tokens` (200, `expires_in == 900`, an `iam.session` row with the hash, an audit row `auth.sign_in`).
  - `signin_unknown_number_401_same_body_as_bad_token` (bodies equal except `correlation_id`).
  - `signin_two_orgs_409_then_choice_signs_in`; `choice_ref_cannot_pick_other_org` (409 ref + org C not in candidates → 401).
  - `refresh_rotates` (old refresh token rejected after use), `sign_out_revokes`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.** Candidate lookup uses `iam.resolve_login(mobile, NULL)`; organisation display names are read by opening `begin_org(candidate)` per candidate.
- [ ] **Step 4:** `cargo test -p domain-iam && cargo test -p api --test auth` → PASS.
- [ ] **Step 5: Commit** `feat(auth): firebase sign-in exchange, organisation choice, refresh rotation, sign-out`.

---

### Task 5: Create employee

**Files:**
- Create: `backend/crates/domain-workforce/{Cargo.toml,src/lib.rs,src/employee.rs}`, `backend/crates/api/src/routes/employees.rs`
- Test: `backend/crates/api/tests/employees.rs`

**Interfaces:**
- Consumes: `Auth`, `require(.., "WRK-01")`, `employee_in_scope`, `with_idempotency`.
- Produces: `pub async fn create_employee(tx: &mut OrgTx<'_>, actor: &Principal, input: EmployeeCreate, today: NaiveDate) -> Result<Employee, WorkforceError>`; `EmployeeCreate` / `Employee` match workforce.yaml field names.

Rules (spec values):
- EM-01 from the latest `org.organisation_settings_version` (`settings->>'EM-01'`, default `"automatic"`): automatic → code `"{branch.code}-{n:04}"` with `n` = next number for that prefix (retry once on unique violation); `employee_code` in body when automatic → 422; manual and missing → 422.
- VC-01: count employees with status ≠ `exited`; at `max_active` with `on_reach = 'block'` → 409 `EMPLOYEE_LIMIT_REACHED`; `allow_and_alert` → create and write outbox `EmployeeLimitExceeded`. Serialise the count with `SELECT ... FROM platform.organisation WHERE id = $org FOR UPDATE` (via the org's own row, visible to `app_api`).
- Inserts `workforce.employee`, `employment_status_history`, one `employee_assignment` per supplied kind with `valid = [joining_date,)`, `employee_allowed_branch` rows; audit `employee.create`; outbox `EmployeeCreated`.
- Master branch must be in the caller's scope, else 403 `FORBIDDEN`.

- [ ] **Step 1: Write failing tests:** `create_201_with_auto_code` (code `"HQ-0001"`, then `"HQ-0002"`); `create_manual_code_required_422`; `create_same_idempotency_key_once` (two POSTs → one row, identical bodies); `create_same_key_other_body_409`; `limit_block_409` (limit 1, second create → 409); `limit_race_never_exceeds` (two concurrent creates at limit 1 → exactly one 201); `create_branch_out_of_scope_403`; `create_writes_audit_and_outbox`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test -p api --test employees create_` and `limit_` → PASS.
- [ ] **Step 5: Commit** `feat(workforce): create employee with code generation, limit, scope, audit`.

---

### Task 6: Read, edit and re-assign employees

**Files:**
- Create: `backend/crates/domain-workforce/src/assignment.rs`
- Modify: `backend/crates/domain-workforce/src/employee.rs`, `backend/crates/api/src/routes/employees.rs`
- Test: `backend/crates/api/tests/employees.rs`, `backend/crates/api/tests/isolation.rs`

**Interfaces:**
- Produces: `get_employee(tx, id) -> Option<(Employee, ETag)>`; `update_employee(tx, actor, id, if_match: &str, patch: EmployeePatch) -> Result<(Employee, ETag), WorkforceError>`; `list_assignments(tx, id) -> Vec<Assignment>`; `change_assignment(tx, actor, id, kind, target_id, effective_from: NaiveDate, reason: Option<String>) -> Result<Assignment, WorkforceError>`.
- ETag = `W/"<updated_at as microseconds>"`.

Rules:
- Out of scope or other organisation → 404 (never 403).
- PATCH accepts only the fields in workforce.yaml `EmployeePatch`; unknown field → 422.
- `change_assignment`: ends the current range of that kind at `effective_from` (upper bound) and inserts `[effective_from,)`; `effective_from` before the current range start → 422; a date covered by `period.lock_coverage` for this employee → 409 `PERIOD_LOCKED` (OQ-C02-08 neutral: refuse, do not back-post). `reporting_manager` equal to the employee → 422. Audit `employee.assignment_change` with before/after; outbox `EmployeeAssignmentChanged`.

- [ ] **Step 1: Write failing tests:** `get_returns_etag`; `patch_stale_etag_412`; `patch_missing_if_match_428`; `patch_unknown_field_422`; `change_department_closes_old_range` (two rows, ranges adjacent, no overlap); `change_backdated_into_lock_409`; `manager_self_422`. In `isolation.rs`: `other_org_employee_404` for GET, PATCH, assignments and change-assignment; `hr_scoped_to_branch_x_gets_404_for_branch_y`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test --workspace` → all PASS; `cargo clippy --workspace -- -D warnings` → clean.
- [ ] **Step 5: Commit** `feat(workforce): get, patch with ETag, dated assignment changes; isolation tests`.

---

## Not in this slice (next slices)

Password + second-step sign-in and vendor sign-in (D-16); employee list, uploads, status transitions, transfer-master-branch, allowed-branch changes, custom fields, contact-change requests; organisation set-up endpoints (branches are seeded directly in tests until then); deployment (Cloud Run, Secret Manager).
