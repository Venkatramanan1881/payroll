//! Users, roles and access (slice 10).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

/// An Organisation Admin with the C03 grants and organisation-wide scope; returns (user id, token).
async fn org_admin(pool: &PgPool, org: Uuid) -> (Uuid, String) {
    let (user, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', $3, 'x')")
        .bind(org).bind(user).bind(format!("oa-{user}@test.invalid")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name, system_code) VALUES ($1, $2, 'Organisation Admin', 'org_admin')").bind(org).bind(role).execute(pool).await.unwrap();
    for op in ["IAM-01", "IAM-02", "IAM-03", "IAM-04", "IAM-05", "IAM-06", "IAM-07", "IAM-09", "IAM-11", "ORG-07"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'all')").bind(org).bind(role).bind(op).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(user).execute(pool).await.unwrap();
    (user, common::token(user, org))
}

async fn hr_role(pool: &PgPool, s: &common::SeedOrg) -> Uuid {
    sqlx::query_scalar("SELECT role_id FROM iam.user_role WHERE user_id = $1").bind(s.hr_user_id).fetch_one(pool).await.unwrap()
}

async fn org_settings(pool: &PgPool, org: Uuid) {
    let s = json!({ "financial_year": { "kind": "april_to_march" }, "week_start": "monday", "languages": ["en", "ta"], "currency": "INR",
            "employee_code_mode": "automatic", "mandatory_fields": [], "contact_self_edit": "off", "probation_months": 6,
            "rejoin_code": "new_code", "allowed_branches_mode": "master_only", "non_allowed_branch_punch": "record_and_flag",
            "other_branch_costing": "branch_worked" });
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(org).bind(s).execute(pool).await.unwrap();
}

async fn etag(app: &axum::Router, token: &str, uri: &str) -> String {
    let (res, _) = common::send(app, common::req("GET", uri, token, None, &[])).await;
    res.headers().get("etag").unwrap().to_str().unwrap().to_string()
}

/// Users: creation rules, Argon2id passwords, access change with If-Match, deactivation cutting access at once.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn users_are_created_changed_and_cut_off(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (oa_id, oa) = org_admin(&pool, s.org_id).await;
    let hr = hr_role(&pool, &s).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/users", None, &[]).await.0, StatusCode::FORBIDDEN, "HR holds no IAM-01 (G-07)");

    let body = json!({ "login_kind": "mobile_otp", "mobile": "+919876500001", "role_ids": [hr], "scopes": [{ "scope_kind": "branch", "branch_id": s.branch_id }] });
    let (st, u) = common::as_user(&app, &oa, "POST", "/v1/users", Some(body.clone()), &key()).await;
    assert_eq!((st, u["roles"][0]["id"].as_str(), u["scopes"][0]["scope_kind"].as_str()), (StatusCode::CREATED, Some(&*hr.to_string()), Some("branch")), "{u}");
    let (st, b) = common::as_user(&app, &oa, "POST", "/v1/users", Some(body), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ALREADY_EXISTS")), "mobile is unique: {b}");
    let mixed = json!({ "login_kind": "mobile_otp", "mobile": "+919876500002", "role_ids": [hr],
                        "scopes": [{ "scope_kind": "branch", "branch_id": s.branch_id }, { "scope_kind": "all" }] });
    assert_eq!(common::as_user(&app, &oa, "POST", "/v1/users", Some(mixed), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "OQ-C03-04");
    let short = json!({ "login_kind": "password", "email": "a@test.invalid", "initial_password": "short", "role_ids": [hr], "scopes": [{ "scope_kind": "all" }] });
    assert_eq!(common::as_user(&app, &oa, "POST", "/v1/users", Some(short), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let pw = json!({ "login_kind": "password", "email": "admin2@test.invalid", "initial_password": "correct horse battery", "role_ids": [hr], "scopes": [{ "scope_kind": "all" }] });
    let (st, p) = common::as_user(&app, &oa, "POST", "/v1/users", Some(pw), &key()).await;
    assert!(st == StatusCode::CREATED && p.get("password_hash").is_none() && !p.to_string().contains("horse"), "{p}");
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM iam.user_account WHERE email = 'admin2@test.invalid'").fetch_one(&pool).await.unwrap();
    assert!(hash.starts_with("$argon2id$"), "stored as an Argon2id hash only");

    let id = u["id"].as_str().unwrap();
    let user: Uuid = id.parse().unwrap();
    let token = common::token(user, s.org_id);
    let (st, me) = common::as_user(&app, &token, "GET", "/v1/me", None, &[]).await;
    assert_eq!((st, me["offered_languages"].clone()), (StatusCode::OK, json!(["en"])), "no settings yet: English only");
    assert!(me["grants"].as_array().unwrap().iter().any(|g| g["operation_code"] == "LVE-01"), "{me}");

    let uri = format!("/v1/users/{id}/change-access");
    let change = json!({ "role_ids": [hr], "scopes": [{ "scope_kind": "all" }], "reason": "promoted" });
    assert_eq!(common::as_user(&app, &oa, "POST", &uri, Some(change.clone()), &key()).await.0, StatusCode::PRECONDITION_REQUIRED);
    let tag = etag(&app, &oa, &format!("/v1/users/{id}")).await;
    let mut h = key();
    h.push(("if-match", tag));
    let (st, changed) = common::as_user(&app, &oa, "POST", &uri, Some(change), &h).await;
    assert_eq!((st, changed["scopes"][0]["scope_kind"].as_str()), (StatusCode::OK, Some("all")), "{changed}");
    let audited: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE action = 'iam.user_access_changed' AND after_value->'before'->'scopes'->0->>'scope_kind' = 'branch'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(audited, 1, "old and new values audited");

    let sid: Uuid = sqlx::query_scalar("INSERT INTO iam.session (org_id, user_id, refresh_token_hash, expires_at) VALUES ($1, $2, 'h', now() + interval '1 day') RETURNING id")
        .bind(s.org_id).bind(user).fetch_one(&pool).await.unwrap();
    let session_token = domain_iam::tokens::issue_access(common::JWT_SECRET, user, s.org_id, Some(sid), common::now());
    assert_eq!(common::as_user(&app, &session_token, "GET", "/v1/me", None, &[]).await.0, StatusCode::OK);
    let off = format!("/v1/users/{id}/deactivate");
    assert_eq!(common::as_user(&app, &oa, "POST", &off, Some(json!({ "reason": " " })), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, d) = common::as_user(&app, &oa, "POST", &off, Some(json!({ "reason": "left" })), &key()).await;
    assert_eq!((st, d["active"].as_bool()), (StatusCode::OK, Some(false)));
    assert_eq!(common::as_user(&app, &token, "GET", "/v1/me", None, &[]).await.0, StatusCode::UNAUTHORIZED, "the existing token stops working (AC-C03-03.7)");
    let revoked: bool = sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM iam.session WHERE user_id = $1").bind(user).fetch_one(&pool).await.unwrap();
    assert!(revoked, "sessions revoked");
    assert_eq!(common::as_user(&app, &oa, "POST", &off, Some(json!({ "reason": "again" })), &key()).await.0, StatusCode::OK, "idempotent");
    let (_, back) = common::as_user(&app, &oa, "POST", &format!("/v1/users/{id}/reactivate"), Some(json!({ "reason": "rejoined" })), &key()).await;
    assert_eq!(back["active"], true);
    assert_eq!(common::as_user(&app, &session_token, "GET", "/v1/me", None, &[]).await.0, StatusCode::UNAUTHORIZED, "a revoked session stays revoked after reactivation");
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/users/{oa_id}/deactivate"), Some(json!({ "reason": "oops" })), &key()).await.0, StatusCode::FORBIDDEN, "no self-lockout");

    // IAM-01 without IAM-02 cannot create a user (that would assign roles and scope).
    let (half, half_role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'half@test.invalid', 'x')").bind(s.org_id).bind(half).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'Half')").bind(s.org_id).bind(half_role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'IAM-01', 'all')").bind(s.org_id).bind(half_role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(half).bind(half_role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(s.org_id).bind(half).execute(&pool).await.unwrap();
    let grab = json!({ "login_kind": "password", "email": "mine@test.invalid", "initial_password": "long enough pw", "role_ids": [hr], "scopes": [{ "scope_kind": "all" }] });
    assert_eq!(common::as_user(&app, &common::token(half, s.org_id), "POST", "/v1/users", Some(grab), &key()).await.0, StatusCode::FORBIDDEN, "IAM-02 needed");

    let other = common::seed_org(&pool).await;
    let (_, other_oa) = org_admin(&pool, other.org_id).await;
    assert_eq!(common::as_user(&app, &other_oa, "GET", &format!("/v1/users/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}

/// Roles: custom roles start empty; grants validated (P10, PLT-); a switched-off module's grants are hidden.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn roles_and_grants(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, oa) = org_admin(&pool, s.org_id).await;
    let (st, r) = common::as_user(&app, &oa, "POST", "/v1/roles", Some(json!({ "name": "Store Manager" })), &key()).await;
    assert_eq!((st, &r["grants"], &r["system_code"]), (StatusCode::CREATED, &json!([]), &Value::Null), "{r}");
    assert_eq!(common::as_user(&app, &oa, "POST", "/v1/roles", Some(json!({ "name": "Store Manager" })), &key()).await.0, StatusCode::CONFLICT, "unique names");
    let id = r["id"].as_str().unwrap();
    let uri = format!("/v1/roles/{id}/change-grants");
    for bad in ["ADT-03", "PLT-01", "XYZ-01"] {
        let mut h = key();
        h.push(("if-match", etag(&app, &oa, &format!("/v1/roles/{id}")).await));
        let body = json!({ "grants": [{ "operation_code": bad, "grant_kind": "all" }], "reason": "x" });
        assert_eq!(common::as_user(&app, &oa, "POST", &uri, Some(body), &h).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    let mut h = key();
    h.push(("if-match", etag(&app, &oa, &format!("/v1/roles/{id}")).await));
    let body = json!({ "grants": [{ "operation_code": "LVE-01", "grant_kind": "all" }, { "operation_code": "CMP-01", "grant_kind": "all" }], "reason": "store work" });
    let (st, g) = common::as_user(&app, &oa, "POST", &uri, Some(body), &h).await;
    assert_eq!((st, g["grants"].as_array().map(Vec::len)), (StatusCode::OK, Some(1)), "comp-off (M02) is off, so CMP-01 is not listed: {g}");

    // The role's grant applies to a user holding it on their next request.
    let body = json!({ "login_kind": "mobile_otp", "mobile": "+919876500009", "role_ids": [id], "scopes": [{ "scope_kind": "all" }] });
    let (_, u) = common::as_user(&app, &oa, "POST", "/v1/users", Some(body), &key()).await;
    let token = common::token(u["id"].as_str().unwrap().parse().unwrap(), s.org_id);
    let (_, me) = common::as_user(&app, &token, "GET", "/v1/me", None, &[]).await;
    assert_eq!(me["grants"], json!([{ "operation_code": "LVE-01", "grant_kind": "all", "setting_ref": null }]), "{me}");

    let hr = hr_role(&pool, &s).await;
    let h = vec![("if-match", etag(&app, &oa, &format!("/v1/roles/{id}")).await)];
    let (st, renamed) = common::as_user(&app, &oa, "PATCH", &format!("/v1/roles/{id}"), Some(json!({ "name": "Shop Manager" })), &h).await;
    assert_eq!((st, renamed["name"].as_str()), (StatusCode::OK, Some("Shop Manager")));
    sqlx::query("UPDATE iam.role SET system_code = 'auditor' WHERE id = $1").bind(hr).execute(&pool).await.unwrap();
    let h = vec![("if-match", etag(&app, &oa, &format!("/v1/roles/{hr}")).await)];
    assert_eq!(common::as_user(&app, &oa, "PATCH", &format!("/v1/roles/{hr}"), Some(json!({ "name": "X" })), &h).await.0, StatusCode::CONFLICT, "system roles keep their names");
    let mut h = key();
    h.push(("if-match", etag(&app, &oa, &format!("/v1/roles/{hr}")).await));
    let writes = json!({ "grants": [{ "operation_code": "WRK-01", "grant_kind": "all" }], "reason": "auditor writes" });
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/roles/{hr}/change-grants"), Some(writes), &h).await.0, StatusCode::CONFLICT, "system roles keep their grants (G-09)");
    let (_, modules) = common::as_user(&app, &oa, "GET", "/v1/me/modules", None, &[]).await;
    assert_eq!((modules["items"].as_array().map(Vec::len), modules["mobile_app"].as_bool()), (Some(20), Some(false)), "{modules}");
}

/// Access settings: versioned changes; AC-06 and AC-03 switches; delegations; approver rules; own language.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn access_settings_switches_and_delegations(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (oa_id, oa) = org_admin(&pool, s.org_id).await;
    org_settings(&pool, s.org_id).await;
    let (_, before) = common::as_user(&app, &oa, "GET", "/v1/access-settings", None, &[]).await;
    assert_eq!((before["delegation_enabled"].as_bool(), before["settings_change_mode"].as_str(), before["custom_roles_enabled"].as_bool()), (Some(true), Some("hr_only"), Some(true)));

    // Delegation while AC-03 is on; overlapping windows refused; ending shortens to now.
    let now = chrono::Utc::now();
    let d = json!({ "approver_user_id": oa_id, "delegate_user_id": s.hr_user_id, "valid_from": now - chrono::Duration::hours(1), "valid_to": now + chrono::Duration::days(3) });
    let (st, del) = common::as_user(&app, &oa, "POST", "/v1/delegations", Some(d.clone()), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{del}");
    let (st, b) = common::as_user(&app, &oa, "POST", "/v1/delegations", Some(d), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("CONFLICTING_SETTINGS")));
    let (st, ended) = common::as_user(&app, &oa, "POST", &format!("/v1/delegations/{}/end", del["id"].as_str().unwrap()), None, &key()).await;
    assert!(st == StatusCode::OK && ended["valid_to"].as_str().unwrap() < del["valid_to"].as_str().unwrap(), "{ended}");
    let later = json!({ "approver_user_id": oa_id, "delegate_user_id": s.hr_user_id, "valid_from": now + chrono::Duration::days(5), "valid_to": now + chrono::Duration::days(6) });
    let (_, future) = common::as_user(&app, &oa, "POST", "/v1/delegations", Some(later), &key()).await;
    let (st, gone) = common::as_user(&app, &oa, "POST", &format!("/v1/delegations/{}/end", future["id"].as_str().unwrap()), None, &key()).await;
    assert_eq!((st, &gone["valid_to"]), (StatusCode::OK, &gone["valid_from"]), "{gone}");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM iam.delegation WHERE lower(valid) > now()").fetch_one(&pool).await.unwrap();
    assert_eq!(left, 0, "a delegation that never started is removed");

    let today = (chrono::Utc::now() + chrono::Duration::minutes(330)).date_naive();
    let change = |body: Value| {
        let app = app.clone();
        let oa = oa.clone();
        async move {
            let mut h = key();
            h.push(("if-match", etag(&app, &oa, "/v1/access-settings").await));
            common::as_user(&app, &oa, "POST", "/v1/access-settings/change", Some(body), &h).await
        }
    };
    let (st, _) = change(json!({ "settings_change_mode": "hr_with_org_admin_approval", "effective_from": today, "reason": "x" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "AC-04 approval waits for the settings-change workflow");
    let (st, after) = change(json!({ "custom_roles_enabled": false, "delegation_enabled": false, "effective_from": today, "reason": "lock down" })).await;
    assert_eq!((st, after["custom_roles_enabled"].as_bool(), after["delegation_enabled"].as_bool()), (StatusCode::OK, Some(false), Some(false)), "{after}");
    let (st, _) = change(json!({ "admin_second_step": false, "effective_from": today, "reason": "again" })).await;
    assert_eq!(st, StatusCode::CONFLICT, "one version per date");
    let kept: Value = sqlx::query_scalar("SELECT settings->'languages' FROM org.organisation_settings_version ORDER BY effective_from DESC LIMIT 1").fetch_one(&pool).await.unwrap();
    assert_eq!(kept, json!(["en", "ta"]), "other keys carried forward");
    assert_eq!(common::as_user(&app, &oa, "POST", "/v1/roles", Some(json!({ "name": "Late role" })), &key()).await.0, StatusCode::FORBIDDEN, "AC-06 off");
    let d = json!({ "approver_user_id": oa_id, "delegate_user_id": s.hr_user_id, "valid_from": now, "valid_to": now + chrono::Duration::days(1) });
    assert_eq!(common::as_user(&app, &oa, "POST", "/v1/delegations", Some(d), &key()).await.0, StatusCode::FORBIDDEN, "AC-03 off");

    let mut h = key();
    h.push(("if-match", etag(&app, &oa, "/v1/access-settings").await));
    let vis = json!({ "employee_visibility": { "own_punches": true, "day_status": true, "counters": false, "permission_balance": true, "deduction_reason": true },
                      "effective_from": today + chrono::Duration::days(1), "reason": "counters later" });
    let (st, v) = common::as_user(&app, &oa, "POST", "/v1/access-settings/employee-visibility/change", Some(vis), &h).await;
    assert_eq!((st, v["employee_visibility"]["counters"].as_bool()), (StatusCode::OK, Some(true)), "takes effect tomorrow: {v}");

    // Approver rules: named_user needs a user; HR as the source takes none.
    let rules = |body: Value| {
        let app = app.clone();
        let oa = oa.clone();
        async move {
            let mut h = key();
            h.push(("if-match", etag(&app, &oa, "/v1/approver-rules").await));
            common::as_user(&app, &oa, "POST", "/v1/approver-rules/change", Some(body), &h).await
        }
    };
    let (st, _) = rules(json!({ "request_type": "leave", "levels": [{ "level": 1, "source": "hr", "named_user_id": oa_id }], "reason": "x" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, r) = rules(json!({ "request_type": "leave", "levels": [{ "level": 1, "source": "named_user", "named_user_id": s.hr_user_id }], "reason": "HR decides" })).await;
    assert_eq!((st, r["items"][0]["source"].as_str()), (StatusCode::OK, Some("named_user")), "{r}");

    // Own language: only the organisation's languages (OR-03).
    let h = vec![("if-match", etag(&app, &oa, "/v1/me").await)];
    assert_eq!(common::as_user(&app, &oa, "PATCH", "/v1/me/language", Some(json!({ "language": "hi" })), &h).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, me) = common::as_user(&app, &oa, "PATCH", "/v1/me/language", Some(json!({ "language": "ta" })), &h).await;
    assert_eq!((st, me["language"].as_str()), (StatusCode::OK, Some("ta")));
}
