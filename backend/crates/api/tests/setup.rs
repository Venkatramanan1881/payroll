//! Organisation setup endpoints (slice 7).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn settings() -> Value {
    json!({ "financial_year": { "kind": "april_to_march" }, "week_start": "monday", "languages": ["en"], "currency": "INR",
            "employee_code_mode": "manual", "mandatory_fields": [], "contact_self_edit": "off", "probation_months": 6,
            "rejoin_code": "new_code", "allowed_branches_mode": "master_only", "non_allowed_branch_punch": "record_and_flag",
            "other_branch_costing": "branch_worked" })
}

async fn post(app: &axum::Router, s: &common::SeedOrg, uri: &str, body: Value) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", uri, Some(body), None).await;
    (st, b)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn settings_versions_are_typed_append_only_and_drive_the_readers(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    // An earlier version holding an access setting (AC-03), as the access-settings slice will write it.
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2025-01-01', '{\"delegation_enabled\": false}', 1)")
        .bind(s.org_id).execute(&pool).await.unwrap();
    let body = json!({ "effective_from": "2026-01-01", "reason": "initial setup", "schema_version": 1, "settings": settings() });
    let (st, v) = post(&app, &s, "/v1/organisation-settings/versions", body.clone()).await;
    assert_eq!(st, StatusCode::CREATED, "{v}");
    assert_eq!((v["settings"]["employee_code_mode"].as_str(), &v["warnings"]), (Some("manual"), &json!([])));
    assert!(v["settings"].get("delegation_enabled").is_none(), "access settings are not part of OrganisationSettings");
    let carried: Option<bool> = sqlx::query_scalar("SELECT (settings->>'delegation_enabled')::boolean FROM org.organisation_settings_version WHERE effective_from = '2026-01-01'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(carried, Some(false), "access settings carried forward");

    let (st, b) = post(&app, &s, "/v1/organisation-settings/versions", body).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")));
    let mut bad = settings();
    bad["week_start"] = json!("someday");
    let (st, _) = post(&app, &s, "/v1/organisation-settings/versions", json!({ "effective_from": "2026-02-01", "reason": "x", "schema_version": 1, "settings": bad })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, cur) = common::as_user(&app, &s.hr_token, "GET", "/v1/organisation-settings?on=2026-03-01", None, &[]).await;
    assert_eq!(cur["effective_from"], "2026-01-01");

    // EM-01 = manual now drives employee creation (the reader uses the contract key).
    let emp = json!({ "full_name": "Asha", "joining_date": "2026-03-01", "employment_status": "active", "master_branch_id": s.branch_id });
    let (st, b) = post(&app, &s, "/v1/employees", emp).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "employee_code is required under manual: {b}");
}

/// Review focus 1: scoped HR sees only their branches; nobody reaches another organisation's branch.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn branches_scope_and_isolation(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (st, b) = post(&app, &s, "/v1/branches", json!({ "name": "Chennai", "code": "CHN", "tz": "Asia/Kolkata", "time_zone_valid_from": "2026-01-01" })).await;
    assert_eq!((st, b["tz"].as_str()), (StatusCode::CREATED, Some("Asia/Kolkata")), "{b}");
    let chn = b["id"].as_str().unwrap().to_string();
    assert_eq!(post(&app, &s, "/v1/branches", json!({ "name": "Dup", "code": "CHN", "tz": "Asia/Kolkata", "time_zone_valid_from": "2026-01-01" })).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(post(&app, &s, "/v1/branches", json!({ "name": "X", "code": "X", "tz": "Mars/Base", "time_zone_valid_from": "2026-01-01" })).await.0, StatusCode::UNPROCESSABLE_ENTITY);

    let (st, _, _) = common::call(&app, &s, "PATCH", &format!("/v1/branches/{chn}"), Some(json!({ "name": "Chennai Central" })), None).await;
    assert_eq!(st, StatusCode::PRECONDITION_REQUIRED);
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/branches/{chn}"), None, None).await;
    let (st, b, _) = common::call(&app, &s, "PATCH", &format!("/v1/branches/{chn}"), Some(json!({ "name": "Chennai Central" })), etag.as_deref()).await;
    assert_eq!((st, b["name"].as_str()), (StatusCode::OK, Some("Chennai Central")), "{b}");

    // An HR user scoped to HQ only.
    let user = Uuid::new_v4();
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'hq@test.invalid', 'x')").bind(s.org_id).bind(user).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'HQ HR')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    for op in ["ORG-07", "ORG-01", "ORG-02"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'scoped')").bind(s.org_id).bind(role).bind(op).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(user).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)").bind(s.org_id).bind(user).bind(s.branch_id).execute(&pool).await.unwrap();
    let hq = common::token(user, s.org_id);
    let (_, list) = common::as_user(&app, &hq, "GET", "/v1/branches", None, &[]).await;
    assert_eq!(list["items"].as_array().unwrap().len(), 1, "{list}");
    assert_eq!(common::as_user(&app, &hq, "GET", &format!("/v1/branches/{chn}"), None, &[]).await.0, StatusCode::NOT_FOUND);
    let (st, _) = common::as_user(&app, &hq, "POST", "/v1/branches", Some(json!({ "name": "Y", "code": "Y", "tz": "Asia/Kolkata", "time_zone_valid_from": "2026-01-01" })), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "a branch-scoped user cannot create branches");

    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/branches/{chn}"), None, &[]).await.0, StatusCode::NOT_FOUND);
}

/// Review focus 2: a fence is switched on only after a test; moving it switches it off again.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn geo_fence_test_before_switch_on(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (st, f) = post(&app, &s, &format!("/v1/branches/{}/geo-fences", s.branch_id), json!({ "name": "Gate", "latitude": 12.9716, "longitude": 77.5946, "radius_m": 200 })).await;
    assert_eq!((st, f["enabled"].as_bool()), (StatusCode::CREATED, Some(false)), "{f}");
    let id = f["id"].as_str().unwrap().to_string();
    let (st, b) = post(&app, &s, &format!("/v1/geo-fences/{id}/switch-on"), json!({})).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "{b}");
    let (st, t) = post(&app, &s, &format!("/v1/geo-fences/{id}/test"), json!({ "latitude": 12.9716 + 100.0 / 111_195.0, "longitude": 77.5946 })).await;
    assert_eq!((st, t["inside"].as_bool()), (StatusCode::OK, Some(true)), "{t}");
    assert!((t["distance_m"].as_f64().unwrap() - 100.0).abs() < 1.0);
    let (st, on) = post(&app, &s, &format!("/v1/geo-fences/{id}/switch-on"), json!({})).await;
    assert_eq!((st, on["enabled"].as_bool()), (StatusCode::OK, Some(true)));
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/branches/{}/geo-fences", s.branch_id), None, &[]).await;
    let etag = format!("W/\"{}\"", list["items"][0]["updated_at"].as_str().unwrap());
    let (st, moved, _) = common::call(&app, &s, "PATCH", &format!("/v1/geo-fences/{id}"), Some(json!({ "latitude": 13.0, "longitude": 77.6, "radius_m": 150 })), Some(&etag)).await;
    assert_eq!((st, moved["enabled"].as_bool(), &moved["tested_at"]), (StatusCode::OK, Some(false), &Value::Null), "{moved}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn ip_allow_list_lists_and_contractors(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let uri = format!("/v1/branches/{}/ip-allow-list", s.branch_id);
    let (_, _, etag) = common::call(&app, &s, "GET", &uri, None, None).await;
    let (st, _, _) = common::call(&app, &s, "POST", &format!("{uri}/change"), Some(json!({ "networks": ["not-an-ip"], "reason": "office" })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, b, _) = common::call(&app, &s, "POST", &format!("{uri}/change"), Some(json!({ "networks": ["10.0.0.0/8", "2001:db8::/32"], "reason": "office" })), etag.as_deref()).await;
    assert_eq!((st, b["networks"].as_array().map(|n| n.len())), (StatusCode::OK, Some(2)), "{b}");
    let (st, _, _) = common::call(&app, &s, "POST", &format!("{uri}/change"), Some(json!({ "networks": [], "reason": "x" })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::PRECONDITION_FAILED, "the old ETag is stale after a change");

    let (st, d) = post(&app, &s, "/v1/lists/department", json!({ "name": "Stores", "code": "ST" })).await;
    assert_eq!(st, StatusCode::CREATED, "{d}");
    assert_eq!(post(&app, &s, "/v1/lists/department", json!({ "name": "Stores" })).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(post(&app, &s, "/v1/lists/planet", json!({ "name": "Mars" })).await.0, StatusCode::NOT_FOUND);
    let id = d["id"].as_str().unwrap();
    let (st, r) = post(&app, &s, &format!("/v1/lists/department/{id}/retire"), json!({})).await;
    assert!(st == StatusCode::OK && r["retired_at"].is_string(), "{r}");
    let (_, active) = common::as_user(&app, &s.hr_token, "GET", "/v1/lists/department", None, &[]).await;
    assert_eq!(active["items"], json!([]));

    let (st, c) = post(&app, &s, "/v1/contractors", json!({ "name": "Acme Labour", "contact": "9876543210" })).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", "/v1/contractors", None, &[]).await;
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    let (_, o) = common::as_user(&app, &s.hr_token, "GET", "/v1/organisation", None, &[]).await;
    assert_eq!(o["state"], "active");
}
