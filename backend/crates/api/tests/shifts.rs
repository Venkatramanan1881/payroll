mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;

fn day_shift(code: &str, from: &str) -> Value {
    json!({ "code": code, "name": format!("{code} shift"), "effective_from": from,
            "definition": { "shift_type": "fixed", "start_time": "09:00", "end_time": "18:00", "required_minutes": 480 } })
}

async fn create(app: &axum::Router, s: &common::SeedOrg, body: Value) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", "/v1/shifts", Some(body), None).await;
    (st, b)
}

async fn publish(app: &axum::Router, s: &common::SeedOrg, version: &str) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", &format!("/v1/shift-versions/{version}/publish"), None, None).await;
    (st, b)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_shift_201_with_draft_version(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (st, shift) = create(&app, &s, day_shift("GEN", "2026-01-01")).await;
    assert_eq!(st, StatusCode::CREATED, "{shift}");
    assert_eq!(shift["code"], "GEN");
    let versions = shift["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0]["state"], "draft");
    assert_eq!(versions[0]["start_time"], "09:00:00");
    assert_eq!(versions[0]["missed_punch_cutoff"], "02:30:00");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_today_in_force_and_supersedes_previous(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, shift) = create(&app, &s, day_shift("GEN", "2026-01-01")).await;
    let v1 = shift["versions"][0]["id"].as_str().unwrap().to_string();
    let (st, p1) = publish(&app, &s, &v1).await;
    assert_eq!((st, p1["state"].as_str()), (StatusCode::OK, Some("in_force")), "{p1}");
    let id = shift["id"].as_str().unwrap();
    let (_, v2, _) = common::call(&app, &s, "POST", &format!("/v1/shifts/{id}/versions"), Some(json!({ "effective_from": "2026-06-01" })), None).await;
    let v2 = v2["id"].as_str().unwrap().to_string();
    assert_eq!(publish(&app, &s, &v2).await.1["state"], "in_force");
    let (_, old) = common::send(&app, common::req("GET", &format!("/v1/shift-versions/{v1}"), &s.hr_token, None, &[])).await;
    assert_eq!(old["state"], "superseded");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_future_is_scheduled(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, shift) = create(&app, &s, day_shift("GEN", "2099-01-01")).await;
    let (st, v) = publish(&app, &s, shift["versions"][0]["id"].as_str().unwrap()).await;
    assert_eq!((st, v["state"].as_str()), (StatusCode::OK, Some("scheduled")));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_published_version_409_version_frozen(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, shift) = create(&app, &s, day_shift("GEN", "2026-01-01")).await;
    let v = shift["versions"][0]["id"].as_str().unwrap().to_string();
    publish(&app, &s, &v).await;
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/shift-versions/{v}"), None, None).await;
    let (st, body, _) = common::call(&app, &s, "PATCH", &format!("/v1/shift-versions/{v}"), Some(json!({ "end_time": "17:00" })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "VERSION_FROZEN");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_draft_version_ok(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, shift) = create(&app, &s, day_shift("GEN", "2026-01-01")).await;
    let v = shift["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/shift-versions/{v}"), None, None).await;
    let (st, body, _) = common::call(&app, &s, "PATCH", &format!("/v1/shift-versions/{v}"), Some(json!({ "end_time": "17:00:00" })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::OK, "{body}");
    assert_eq!(body["end_time"], "17:00:00");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_shift_code_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    create(&app, &s, day_shift("GEN", "2026-01-01")).await;
    assert_eq!(create(&app, &s, day_shift("GEN", "2026-01-01")).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn night_type_requires_night_flag_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let body = json!({ "code": "N", "name": "Night", "effective_from": "2026-01-01",
                       "definition": { "shift_type": "night", "start_time": "22:00", "end_time": "06:00" } });
    assert_eq!(create(&app, &s, body).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_shifts_other_org_empty(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    let app = common::app(pool);
    create(&app, &a, day_shift("GEN", "2026-01-01")).await;
    let (_, mine) = common::send(&app, common::req("GET", "/v1/shifts", &a.hr_token, None, &[])).await;
    let (_, theirs) = common::send(&app, common::req("GET", "/v1/shifts", &b.hr_token, None, &[])).await;
    assert_eq!(mine["items"].as_array().unwrap().len(), 1);
    assert_eq!(theirs["items"].as_array().unwrap().len(), 0);
}
