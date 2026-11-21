//! Self-service reads (web slice 4): the month calendar and AC-05 employee visibility.

mod common;

use axum::http::StatusCode;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn yesterday() -> NaiveDate {
    (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(1)
}

async fn setup(pool: &PgPool) -> (common::SeedOrg, axum::Router, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    let (_, set, _) = common::call(&app, &s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    common::call(&app, &s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(&app, &s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let d = yesterday();
    let entries: Vec<Value> = ["03:50", "12:00"].iter().map(|t| json!({ "employee_id": w.employee_id, "event_at": format!("{d}T{t}:00Z") })).collect();
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    (s, app, w)
}

async fn get(app: &axum::Router, token: &str, uri: &str) -> (StatusCode, Value) {
    common::as_user(app, token, "GET", uri, None, &[]).await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calendar_lists_every_date_of_the_month(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let d = yesterday();
    let month = format!("{:04}-{:02}", d.year(), d.month());
    let (st, cal) = get(&app, &w.token, &format!("/v1/attendance/employees/{}/calendar?month={month}", w.employee_id)).await;
    assert_eq!(st, StatusCode::OK, "{cal}");
    let days = cal["days"].as_array().unwrap();
    let next = if d.month() == 12 { NaiveDate::from_ymd_opt(d.year() + 1, 1, 1) } else { NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1) }.unwrap();
    assert_eq!(days.len() as i64, (next - NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap()).num_days());
    let mine = days.iter().find(|x| x["attendance_date"] == json!(d)).unwrap();
    assert!(mine["day_status"].is_string() && mine["processing_state"].is_string(), "{mine}");
    assert_eq!(get(&app, &w.token, &format!("/v1/attendance/employees/{}/calendar?month=2026-13", w.employee_id)).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let other = common::create_employee(&app, &s, "Bala").await;
    assert_eq!(get(&app, &w.token, &format!("/v1/attendance/employees/{}/calendar?month={month}", other["id"].as_str().unwrap())).await.0, StatusCode::NOT_FOUND);
}

/// AC-05 switched off: the employee's own reads omit what is hidden; HR still sees everything.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn employee_visibility_hides_from_employees_only(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let d = yesterday();
    let off = json!({ "employee_visibility": { "own_punches": false, "day_status": false, "counters": false, "permission_balance": true, "deduction_reason": false } });
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2000-01-01', $2, 1)")
        .bind(s.org_id).bind(off).execute(&pool).await.unwrap();
    let day_uri = format!("/v1/attendance/employees/{}/days/{d}", w.employee_id);
    let (st, own) = get(&app, &w.token, &day_uri).await;
    assert_eq!(st, StatusCode::OK, "{own}");
    let r = &own["current_result"];
    for hidden in ["sessions", "punches_used", "net_minutes", "flags", "day_status", "day_effects", "explanation", "late_mark", "late_minutes"] {
        assert!(r.get(hidden).is_none(), "{hidden} is omitted: {r}");
    }
    assert!(r["engine_version"].is_string() && r["produced_at"].is_string(), "the rest stays");
    let (_, hr) = get(&app, &s.hr_token, &day_uri).await;
    assert!(hr["current_result"]["sessions"].is_array() && hr["current_result"]["day_status"].is_string(), "HR sees all");
    let month = format!("{:04}-{:02}", d.year(), d.month());
    let (_, cal) = get(&app, &w.token, &format!("/v1/attendance/employees/{}/calendar?month={month}", w.employee_id)).await;
    assert!(cal["days"].as_array().unwrap().iter().all(|x| x.get("day_status").is_none() && x.get("late_mark").is_none() && x.get("has_exception").is_none()));
    let punch = hr["current_result"]["punches_used"][0]["raw_punch_id"].as_str().unwrap().to_string();
    assert_eq!(get(&app, &w.token, &format!("/v1/punches/{punch}")).await.0, StatusCode::FORBIDDEN, "nor one punch by id");
    assert_eq!(get(&app, &s.hr_token, &format!("/v1/punches/{punch}")).await.0, StatusCode::OK);
    assert_eq!(get(&app, &w.token, &format!("/v1/punches?employee_id={}", w.employee_id)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&app, &w.token, &format!("/v1/attendance/employees/{}/counters?date={d}", w.employee_id)).await.0, StatusCode::FORBIDDEN);
    assert_eq!(get(&app, &s.hr_token, &format!("/v1/punches?employee_id={}", w.employee_id)).await.0, StatusCode::OK);
}
