mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;

async fn post(app: &axum::Router, s: &common::SeedOrg, uri: &str, body: Value) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", uri, Some(body), None).await;
    (st, b)
}

async fn get(app: &axum::Router, s: &common::SeedOrg, uri: &str) -> Value {
    common::send(app, common::req("GET", uri, &s.hr_token, None, &[])).await.1
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fixed_pattern_needs_days_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (st, _) = post(&app, &s, "/v1/weekly-off-patterns", json!({ "name": "Sundays", "kind": "fixed_days" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, b) = post(&app, &s, "/v1/weekly-off-patterns", json!({ "name": "Sundays", "kind": "fixed_days", "weekly_days": [7] })).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["weekly_days"], json!([7]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alternating_pattern_201(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (st, b) = post(&app, &s, "/v1/weekly-off-patterns", json!({ "name": "2nd and 4th Sat", "kind": "alternating", "alt_day": 6, "alt_weeks": [2, 4] })).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!((b["alt_day"].clone(), b["alt_weeks"].clone()), (json!(6), json!([2, 4])));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retired_pattern_not_listed_by_default(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, p) = post(&app, &s, "/v1/weekly-off-patterns", json!({ "name": "Sundays", "kind": "fixed_days", "weekly_days": [7] })).await;
    let (st, _, _) = common::call(&app, &s, "POST", &format!("/v1/weekly-off-patterns/{}/retire", p["id"].as_str().unwrap()), None, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(get(&app, &s, "/v1/weekly-off-patterns").await["items"].as_array().unwrap().len(), 0);
    assert_eq!(get(&app, &s, "/v1/weekly-off-patterns?include_retired=true").await["items"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_holiday_then_list(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (st, cal) = post(&app, &s, "/v1/holiday-calendars", json!({ "name": "Tamil Nadu 2026" })).await;
    assert_eq!(st, StatusCode::CREATED, "{cal}");
    let id = cal["id"].as_str().unwrap();
    let (st, h) = post(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays"), json!({ "holiday_date": "2026-01-15", "name": "Pongal", "kind": "festival", "reason": "state list" })).await;
    assert_eq!(st, StatusCode::CREATED, "{h}");
    post(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays"), json!({ "holiday_date": "2026-01-26", "name": "Republic Day", "kind": "national", "reason": "state list" })).await;
    let all = get(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays")).await;
    assert_eq!(all["items"].as_array().unwrap().len(), 2);
    let national = get(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays?kind=national")).await;
    assert_eq!(national["items"][0]["name"], "Republic Day");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn duplicate_holiday_same_date_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, cal) = post(&app, &s, "/v1/holiday-calendars", json!({ "name": "TN" })).await;
    let uri = format!("/v1/holiday-calendars/{}/holidays", cal["id"].as_str().unwrap());
    post(&app, &s, &uri, json!({ "holiday_date": "2026-01-15", "name": "Pongal", "kind": "festival", "reason": "x" })).await;
    let (st, _) = post(&app, &s, &uri, json!({ "holiday_date": "2026-01-15", "name": "Again", "kind": "festival", "reason": "x" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn removed_holiday_not_listed_and_audited(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, cal) = post(&app, &s, "/v1/holiday-calendars", json!({ "name": "TN" })).await;
    let id = cal["id"].as_str().unwrap();
    let (_, h) = post(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays"), json!({ "holiday_date": "2026-01-15", "name": "Pongal", "kind": "festival", "reason": "x" })).await;
    let (st, _, _) = common::call(&app, &s, "POST", &format!("/v1/holidays/{}/remove", h["id"].as_str().unwrap()), None, None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    assert_eq!(get(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays")).await["items"].as_array().unwrap().len(), 0);
    let audited: Option<Value> = sqlx::query_scalar("SELECT before_value FROM audit.audit_entry WHERE action = 'holiday.remove'").fetch_optional(&pool).await.unwrap();
    assert_eq!(audited.unwrap()["name"], "Pongal");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_holiday_and_rename_calendar(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, cal) = post(&app, &s, "/v1/holiday-calendars", json!({ "name": "TN" })).await;
    let id = cal["id"].as_str().unwrap();
    let (_, h) = post(&app, &s, &format!("/v1/holiday-calendars/{id}/holidays"), json!({ "holiday_date": "2026-01-15", "name": "Pongal", "kind": "festival", "reason": "x" })).await;
    let hid = h["id"].as_str().unwrap();
    let (st, upd, _) = common::call(&app, &s, "PATCH", &format!("/v1/holidays/{hid}"), Some(json!({ "holiday_date": "2026-01-14", "reason": "moved" })), Some(&format!("W/\"{}\"", h["updated_at"].as_str().unwrap()))).await;
    assert_eq!(st, StatusCode::OK, "{upd}");
    assert_eq!(upd["holiday_date"], "2026-01-14");
    let (st, renamed, _) = common::call(&app, &s, "PATCH", &format!("/v1/holiday-calendars/{id}"), Some(json!({ "name": "Tamil Nadu" })), Some(&format!("W/\"{}\"", cal["updated_at"].as_str().unwrap()))).await;
    assert_eq!((st, renamed["name"].as_str()), (StatusCode::OK, Some("Tamil Nadu")), "{renamed}");
}
