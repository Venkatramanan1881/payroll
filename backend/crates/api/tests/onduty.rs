//! On-duty (slice 14).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn org_policy(app: &axum::Router, s: &common::SeedOrg, od: Option<Value>) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    if let Some(doc) = od {
        let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
        let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/OD"), Some(json!({ "schema_version": 1, "settings": doc })), etag.as_deref()).await;
        assert_eq!(st, StatusCode::OK, "{b}");
    }
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

async fn manual(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, times: &[String]) {
    let entries: Vec<Value> = times.iter().map(|t| json!({ "employee_id": w.employee_id, "event_at": t })).collect();
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, r) = common::as_user(app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
}

async fn day(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, date: &str) -> Value {
    let (_, d) = common::as_user(app, &s.hr_token, "GET", &format!("/v1/attendance/employees/{}/days/{date}", w.employee_id), None, &[]).await;
    d["current_result"].clone()
}

fn ask(kind: &str, from: &str, to: &str) -> Value {
    json!({ "duration_kind": kind, "from_date": from, "to_date": to, "purpose": "client visit", "place": "Chennai" })
}

/// PM-R5 / PM-R6: refusals, a full day on duty, hours covering the shift start, and cancellation restoring the day.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn on_duty_counts_as_worked(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, None).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await; // 09:00-18:00 India
    let d = (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(1);
    let (ds, d2) = (d.to_string(), (d - Duration::days(1)).to_string());
    manual(&app, &s, &w, &[format!("{d}T04:30:00Z"), format!("{d}T12:30:00Z")]).await; // 10:00-18:00
    assert_eq!(day(&app, &s, &w, &ds).await["late_minutes"], 60);
    let submit = |body: Value| {
        let app = app.clone();
        let tok = w.token.clone();
        async move { common::as_user(&app, &tok, "POST", "/v1/on-duty/requests", Some(body), &key()).await }
    };

    // Refusals create nothing.
    assert_eq!(submit(ask("full_day", &d2, &ds)).await.0, StatusCode::UNPROCESSABLE_ENTITY, "to_date = from_date unless several_days");
    let mut no_place = ask("full_day", &ds, &ds);
    no_place["place"] = json!(" ");
    assert_eq!(submit(no_place).await.0, StatusCode::UNPROCESSABLE_ENTITY, "OD-06 place");
    assert_eq!(submit(ask("hours", &ds, &ds)).await.0, StatusCode::UNPROCESSABLE_ENTITY, "hours need times");
    let old = (d - Duration::days(10)).to_string();
    let (st, b) = submit(ask("full_day", &old, &old)).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("OUTSIDE_REQUEST_WINDOW")), "OD-04 two days after: {b}");
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/on-duty/requests", None, &[]).await.1["items"].as_array().map(Vec::len), Some(0));

    // Hours at the shift start: no late mark, the time counts as worked.
    let mut hours = ask("hours", &ds, &ds);
    hours["from_time"] = json!("09:00");
    hours["to_time"] = json!("10:00");
    let (st, r) = submit(hours).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::CREATED, Some("pending_l1")), "{r}");
    let (st, b) = submit(ask("full_day", &ds, &ds)).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("OVERLAPPING_REQUEST")), "{b}");
    let id = r["id"].as_str().unwrap();
    assert_eq!(common::as_user(&app, &w.token, "POST", &format!("/v1/on-duty/requests/{id}/approve"), None, &key()).await.0, StatusCode::FORBIDDEN, "BR-07");
    let (st, ok) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{id}/approve"), None, &key()).await;
    assert_eq!((st, ok["state"].as_str()), (StatusCode::OK, Some("approved")), "{ok}");
    let r = day(&app, &s, &w, &ds).await;
    assert_eq!((r["late_minutes"].as_i64(), r["day_status"].as_str()), (Some(0), Some("present")), "{r}");

    // Cancellation (PRM-11) brings the late mark back.
    let (st, c) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{id}/cancel"), Some(json!({ "reason": "trip called off" })), &key()).await;
    assert_eq!((st, c["state"].as_str()), (StatusCode::OK, Some("cancelled")), "{c}");
    assert_eq!(day(&app, &s, &w, &ds).await["late_minutes"], 60);
    assert_eq!(common::as_user(&app, &w.token, "POST", &format!("/v1/on-duty/requests/{id}/cancel"), Some(json!({ "reason": "x" })), &key()).await.0,
               StatusCode::FORBIDDEN, "PRM-11 has no default holder for employees (G-27)");

    // A full day away with no punches is on duty, not absent.
    let (st, r) = submit(ask("full_day", &d2, &d2)).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{}/approve", r["id"].as_str().unwrap()), None, &key()).await;
    assert_eq!(st, StatusCode::OK);
    let r = day(&app, &s, &w, &d2).await;
    assert_eq!((r["day_status"].as_str(), r["day_fraction_present"].as_str()), (Some("on_duty"), Some("1.00")), "{r}");
    assert!(r["explanation"].to_string().contains("PM-R6"), "{r}");

    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/on-duty/requests/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}

/// OD-01 off, OD-02 kinds, OD-05 two levels with a reason to reject, and options not built yet refused.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn settings_and_two_levels(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, set, _) = common::call(&app, &s, "POST", "/v1/policy-sets", Some(json!({ "name": "Probe", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    let (st, _, _) = common::call(&app, &s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/OD"), Some(json!({ "schema_version": 1, "settings": { "OD-08": true } })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "OD-08 waits for overtime");

    org_policy(&app, &s, Some(json!({ "OD-02": { "several_days": false }, "OD-05": { "levels": "manager_then_hr" } }))).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let d = ((Utc::now() + Duration::minutes(330)).date_naive() + Duration::days(3)).to_string();
    let d2 = ((Utc::now() + Duration::minutes(330)).date_naive() + Duration::days(4)).to_string();
    let (st, _) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask("several_days", &d, &d2)), &key()).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "OD-02 several days switched off");
    let (st, r) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask("half_day", &d, &d)), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let id = r["id"].as_str().unwrap();
    let (st, l1) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{id}/approve"), None, &key()).await;
    assert_eq!((st, l1["state"].as_str()), (StatusCode::OK, Some("pending_l2")), "{l1}");
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{id}/approve"), None, &key()).await;
    assert_eq!(st, StatusCode::CONFLICT, "the same person never decides both levels");
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{id}/reject"), None, &key()).await;
    assert_eq!(st, StatusCode::CONFLICT, "nor rejects at level 2 after approving level 1");
}

/// OD-01 off refuses every application.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn switched_off(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, Some(json!({ "OD-01": false }))).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let d = (Utc::now() + Duration::minutes(330)).date_naive().to_string();
    let (st, b) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask("full_day", &d, &d)), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("RULE_GROUP_DISABLED")), "{b}");
}
