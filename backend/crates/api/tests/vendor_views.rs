//! Vendor console: usage, health, sector templates and announcements (slice 19).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn vendor_token(pool: &PgPool, app: &axum::Router, email: &str, role: &str) -> String {
    sqlx::query("INSERT INTO platform.vendor_user (email, full_name, vendor_role) VALUES ($1, 'Staff', $2)").bind(email).bind(role).execute(pool).await.unwrap();
    let (res, b) = common::send(app, common::post_json("/v1/vendor/auth/google/sign-in", json!({ "id_token": format!("vendor:{email}") }))).await;
    assert_eq!(res.status(), StatusCode::OK, "{b}");
    b["access_token"].as_str().unwrap().to_string()
}

async fn etag(app: &axum::Router, token: &str, uri: &str) -> String {
    let (res, _) = common::send(app, common::req("GET", uri, token, None, &[])).await;
    res.headers().get("etag").unwrap().to_str().unwrap().to_string()
}

/// VC-R7: templates are added and changed with history; versions count; read-only staff and organisation users cannot.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sector_templates(pool: PgPool) {
    let app = common::app(pool.clone());
    let ops = vendor_token(&pool, &app, "ops@probeyond.in", "full_admin").await;
    let viewer = vendor_token(&pool, &app, "viewer@probeyond.in", "read_only").await;
    let body = json!({ "code": "textile", "name": "Textile mill", "content": { "shift": "general" } });
    let k = key();
    let (st, t) = common::as_user(&app, &ops, "POST", "/v1/vendor/sector-templates", Some(body.clone()), &k).await;
    assert_eq!((st, t["version"].as_i64()), (StatusCode::CREATED, Some(1)), "{t}");
    let (st, again) = common::as_user(&app, &ops, "POST", "/v1/vendor/sector-templates", Some(body.clone()), &k).await;
    assert_eq!((st, again["id"].clone()), (StatusCode::CREATED, t["id"].clone()), "replayed by key");
    let (st, dup) = common::as_user(&app, &ops, "POST", "/v1/vendor/sector-templates", Some(body.clone()), &key()).await;
    assert_eq!((st, dup["code"].as_str()), (StatusCode::CONFLICT, Some("ALREADY_EXISTS")), "{dup}");
    assert_eq!(common::as_user(&app, &viewer, "POST", "/v1/vendor/sector-templates", Some(json!({ "code": "x", "name": "x", "content": {} })), &key()).await.0, StatusCode::FORBIDDEN);

    let uri = format!("/v1/vendor/sector-templates/{}", t["id"].as_str().unwrap());
    let (st, full) = common::as_user(&app, &viewer, "GET", &uri, None, &[]).await;
    assert_eq!((st, full["content"]["shift"].as_str(), full["organisations"].as_array().map(Vec::len)), (StatusCode::OK, Some("general"), Some(0)), "{full}");
    assert_eq!(common::as_user(&app, &ops, "PATCH", &uri, Some(json!({ "name": "Textiles" })), &[]).await.0, StatusCode::PRECONDITION_REQUIRED);
    let tag = etag(&app, &ops, &uri).await;
    let (st, changed) = common::as_user(&app, &ops, "PATCH", &uri, Some(json!({ "name": "Textiles" })), &[("if-match", tag.clone())]).await;
    assert_eq!((st, changed["version"].as_i64(), changed["name"].as_str()), (StatusCode::OK, Some(2), Some("Textiles")), "{changed}");
    assert_eq!(common::as_user(&app, &ops, "PATCH", &uri, Some(json!({ "name": "Again" })), &[("if-match", tag)]).await.0, StatusCode::PRECONDITION_FAILED);
    let tag = etag(&app, &ops, &uri).await;
    assert_eq!(common::as_user(&app, &ops, "PATCH", &uri, Some(json!({ "code": "y" })), &[("if-match", tag)]).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE action = 'vendor.sector_template_changed'").fetch_one(&pool).await.unwrap();
    assert_eq!(history, 1, "old and new values kept");

    let s = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/vendor/sector-templates", None, &[]).await.0, StatusCode::UNAUTHORIZED, "AC-V01-10.4");
}

/// VC-R9 announcements to chosen organisations; VC-R3 / VC-R8 usage and health from the J10 summary only.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn announcements_usage_and_health(pool: PgPool) {
    let app = common::app(pool.clone());
    let ops = vendor_token(&pool, &app, "ops@probeyond.in", "full_admin").await;
    let viewer = vendor_token(&pool, &app, "viewer@probeyond.in", "read_only").await;
    let s = common::seed_org(&pool).await;
    common::seed_worker(&pool, &app, &s, "Asha").await;

    let ask = |orgs: Value| json!({ "title": "Maintenance", "body": "Sunday 02:00-04:00 IST", "organisation_ids": orgs });
    let (st, a) = common::as_user(&app, &ops, "POST", "/v1/vendor/announcements", Some(ask(json!([s.org_id]))), &key()).await;
    assert_eq!((st, a["organisation_ids"].clone()), (StatusCode::CREATED, json!([s.org_id])), "{a}");
    assert_eq!(common::as_user(&app, &ops, "POST", "/v1/vendor/announcements", Some(ask(json!([Uuid::new_v4()]))), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(common::as_user(&app, &viewer, "POST", "/v1/vendor/announcements", Some(ask(json!([s.org_id]))), &key()).await.0, StatusCode::FORBIDDEN, "AC-V01-12.3");
    let (_, list) = common::as_user(&app, &viewer, "GET", "/v1/vendor/announcements", None, &[]).await;
    assert_eq!(list["items"].as_array().map(Vec::len), Some(1));

    // Before J10 the organisation shows zeros; after it, its counts (and never an employee's details).
    let row = |v: &Value| v["items"].as_array().unwrap().iter().find(|i| i["organisation_id"] == json!(s.org_id)).cloned().unwrap();
    let (_, u) = common::as_user(&app, &viewer, "GET", "/v1/vendor/usage", None, &[]).await;
    assert_eq!(row(&u)["active_employees"], 0);
    let req = axum::http::Request::post("/internal/jobs/usage-refresh")
        .header("authorization", format!("Bearer {}", String::from_utf8_lossy(common::JOB_SECRET)))
        .body(axum::body::Body::empty())
        .unwrap();
    let (res, b) = common::send(&app, req).await;
    assert_eq!(res.status(), StatusCode::OK, "{b}");
    let (st, u) = common::as_user(&app, &viewer, "GET", "/v1/vendor/usage", None, &[]).await;
    assert_eq!(st, StatusCode::OK);
    let mine = row(&u);
    assert_eq!((mine["active_employees"].as_i64(), mine["branches"].as_i64(), mine["last_pay_run"].clone()), (Some(1), Some(1), Value::Null), "{mine}");
    assert!(!u.to_string().contains("Asha"), "no employee data");
    let (_, h) = common::as_user(&app, &viewer, "GET", "/v1/vendor/health?only_with_issues=false", None, &[]).await;
    assert_eq!(row(&h)["devices_total"], 0, "{h}");
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/vendor/usage", None, &[]).await.0, StatusCode::UNAUTHORIZED);
}
