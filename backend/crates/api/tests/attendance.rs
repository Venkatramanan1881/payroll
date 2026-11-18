//! Daily attendance results end to end (slice 4). Shift 09:00-18:00 India time = 03:30Z-12:30Z.

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const LAT: f64 = 12.9716;
const LON: f64 = 77.5946;

/// An organisation-level policy, published from 2026-01-01 (a result needs a policy version, BR-05).
async fn org_policy(app: &axum::Router, s: &common::SeedOrg) {
    let (st, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{set}");
    let v = set["versions"][0]["id"].as_str().unwrap();
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    let (st, b, _) = common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
}

async fn setup(pool: &PgPool, with_policy: bool) -> (common::SeedOrg, axum::Router, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    if with_policy {
        org_policy(&app, &s).await;
    }
    (s, app, w)
}

async fn manual(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, times: &[&str]) {
    let entries: Vec<Value> = times.iter().map(|t| json!({ "employee_id": w.employee_id, "event_at": t })).collect();
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, r) = common::as_user(app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
}

async fn day(app: &axum::Router, token: &str, w: &common::Worker, date: &str) -> (StatusCode, Value) {
    common::as_user(app, token, "GET", &format!("/v1/attendance/employees/{}/days/{date}", w.employee_id), None, &[]).await
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e2_late_mark_with_explanation(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    manual(&app, &s, &w, &["2026-03-02T03:55:00Z", "2026-03-02T12:30:00Z"]).await;
    let (st, d) = day(&app, &w.token, &w, "2026-03-02").await;
    assert_eq!(st, StatusCode::OK, "{d}");
    let r = &d["current_result"];
    assert_eq!((r["day_status"].as_str(), r["late_minutes"].as_i64(), r["late_mark"].as_bool()), (Some("present"), Some(25), Some(true)), "{r}");
    assert_eq!((r["net_minutes"].as_i64(), r["day_fraction_present"].as_str()), (Some(455), Some("1.00")));
    assert_eq!(r["overtime_identified_minutes"], Value::Null, "D-09");
    let dirs: Vec<&str> = r["punches_used"].as_array().unwrap().iter().map(|u| u["assigned_direction"].as_str().unwrap()).collect();
    assert_eq!(dirs, ["in", "out"]);
    assert!(r["explanation"].as_array().unwrap().iter().any(|s| s["rule_ref"] == "LA-05"));
    assert_eq!((d["processing_state"].as_str(), d["history"].as_array().unwrap().len()), (Some("provisional"), 0), "both punches in one request: the second recalculation found nothing new");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e6_very_late_half_day_effect_no_mark(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    manual(&app, &s, &w, &["2026-03-02T06:00:00Z", "2026-03-02T12:30:00Z"]).await;
    let (_, d) = day(&app, &s.hr_token, &w, "2026-03-02").await;
    let r = &d["current_result"];
    assert_eq!((r["day_status"].as_str(), r["late_mark"].as_bool()), (Some("half_day"), Some(false)), "{r}");
    assert_eq!(r["day_effects"], json!([{ "effect_kind": "late_slab", "days": "0.50", "minutes": null, "reason_code": "150_minutes", "rule_ref": "LA-05" }]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unchanged_inputs_write_nothing_and_locked_dates_never_move(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    manual(&app, &s, &w, &["2026-03-02T03:30:00Z", "2026-03-02T12:30:00Z"]).await;
    let count = || async { sqlx::query_scalar::<_, i64>("SELECT count(*) FROM attendance.day_result").fetch_one(&pool).await.unwrap() };
    let before = count().await;
    let body = json!({ "from": "2026-03-02", "to": "2026-03-02", "reason": "check" });
    let (st, job) = common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!((st, job["state"].as_str()), (StatusCode::ACCEPTED, Some("succeeded")), "{job}");
    assert_eq!(count().await, before, "same fingerprint: no new result");

    let (cycle, period) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO period.attendance_cycle (org_id, id, owner_kind, owner_id, valid) VALUES ($1, $2, 'organisation', $1, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.attendance_period (org_id, id, cycle_id, period_start, period_end) VALUES ($1, $2, $3, '2026-03-01', '2026-03-31')")
        .bind(s.org_id).bind(period).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.lock_coverage (org_id, period_id, employee_id, dates) VALUES ($1, $2, $3, '[2026-03-01,2026-04-01)')")
        .bind(s.org_id).bind(period).bind(w.employee_id).execute(&pool).await.unwrap();
    manual(&app, &s, &w, &["2026-03-02T14:00:00Z"]).await;
    assert_eq!(count().await, before, "BR-06: a locked date gets no new result");
}

/// Review focus 3: a raised flag that does not count keeps its punch out until accepted; accepting recalculates.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accepting_an_outside_fence_flag_recalculates_the_day(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    common::fence(&pool, s.org_id, s.branch_id, LAT, LON, 200).await;
    let item = |at: &str, metres: f64| {
        let t: DateTime<Utc> = at.parse().unwrap();
        json!({ "idempotency_key": Uuid::new_v4(), "client_time": t, "captured_tz": "Asia/Kolkata",
                "device_uptime_ms": 100_000_000_000i64 - (Utc::now() - t).num_milliseconds(),
                "location": { "latitude": LAT + metres / 111_195.0, "longitude": LON, "accuracy_m": 10 } })
    };
    let body = json!({ "sync_client_time": Utc::now(), "sync_uptime_ms": 100_000_000_000i64,
                       "punches": [item("2026-03-03T03:30:00Z", 450.0), item("2026-03-03T12:30:00Z", 10.0)] });
    let (st, r) = common::as_user(&app, &w.token, "POST", "/v1/punches/sync", Some(body), &[]).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let (_, d) = day(&app, &s.hr_token, &w, "2026-03-03").await;
    assert_eq!(d["current_result"]["day_status"], "absent", "the in punch does not count yet: {d}");
    assert!(d["current_result"]["punches_used"].as_array().unwrap().iter().any(|u| u["excluded_reason"] == "flag_outside_fence_raised"));

    let (_, flags) = common::as_user(&app, &s.hr_token, "GET", "/v1/punch-flags?reason=outside_fence", None, &[]).await;
    let flag = flags["items"][0]["id"].as_str().unwrap();
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/punch-flags/{flag}/accept"), None, &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::OK);
    let (_, d) = day(&app, &s.hr_token, &w, "2026-03-03").await;
    assert_eq!(d["current_result"]["day_status"], "present", "{d}");
    assert_eq!(d["current_result"]["trigger_kind"], "flag_decision");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn daily_view_scope_and_no_policy(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    manual(&app, &s, &w, &["2026-03-02T03:30:00Z", "2026-03-02T12:30:00Z"]).await;
    let (st, list) = common::as_user(&app, &s.hr_token, "GET", "/v1/attendance/daily?date=2026-03-02", None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{list}");
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert_eq!((list["items"][0]["day_status"].as_str(), list["items"][0]["first_in_at"].is_string()), (Some("present"), true));
    let other = common::seed_worker(&pool, &app, &s, "Ravi").await;
    assert_eq!(day(&app, &other.token, &w, "2026-03-02").await.0, StatusCode::NOT_FOUND, "another employee's day is outside self scope");

    let (s2, app2, w2) = setup(&pool, false).await;
    manual(&app2, &s2, &w2, &["2026-03-02T03:30:00Z"]).await;
    let (_, d) = day(&app2, &s2.hr_token, &w2, "2026-03-02").await;
    assert_eq!((d["processing_state"].as_str(), &d["current_result"]), (Some("exception"), &Value::Null), "no policy version: no result");
}

/// Review focus 5: a night shift (22:00-06:00) is one day, dated by its start; its 06:00 out never counts on the next day.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn night_shift_is_one_day_and_does_not_leak_into_the_next(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Night").await;
    org_policy(&app, &s).await;
    let def = json!({ "shift_type": "night", "start_time": "22:00", "end_time": "06:00", "is_night_shift": true });
    let (st, b, _) = common::call(&app, &s, "POST", "/v1/shifts", Some(json!({ "code": "N1", "name": "Night", "effective_from": "2026-01-01", "definition": def })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let v = b["versions"][0]["id"].as_str().unwrap();
    common::call(&app, &s, "POST", &format!("/v1/shift-versions/{v}/publish"), None, None).await;
    let body = json!({ "employee_id": w.employee_id, "source": "fixed_shift", "shift_id": b["id"], "valid_from": "2026-01-01", "reason": "joining" });
    assert_eq!(common::call(&app, &s, "POST", "/v1/employee-schedules", Some(body), None).await.0, StatusCode::CREATED);
    // Mon 22:00 IST in, Tue 06:00 IST out.
    manual(&app, &s, &w, &["2026-03-02T16:30:00Z", "2026-03-03T00:30:00Z"]).await;
    let (_, mon) = day(&app, &s.hr_token, &w, "2026-03-02").await;
    let r = &mon["current_result"];
    assert_eq!((r["day_status"].as_str(), r["net_minutes"].as_i64(), r["late_mark"].as_bool()), (Some("present"), Some(420), Some(false)), "{mon}");
    let (_, tue) = day(&app, &s.hr_token, &w, "2026-03-03").await;
    assert_eq!(tue["current_result"]["punches_used"], json!([]), "Tuesday's night has not been worked: {tue}");
    assert_eq!(tue["current_result"]["day_status"], "absent");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn future_days_get_no_result_and_daily_filters_apply(pool: PgPool) {
    let (s, app, w) = setup(&pool, true).await;
    let today = (Utc::now() + chrono::Duration::minutes(330)).date_naive(); // India date
    let body = json!({ "from": today, "to": today + chrono::Duration::days(5), "reason": "check", "branch_id": s.branch_id });
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    let future: i64 = sqlx::query_scalar("SELECT count(*) FROM attendance.attendance_day WHERE attendance_date > (now() AT TIME ZONE 'Asia/Kolkata')::date")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(future, 0, "no absence is written for days not yet lived");

    manual(&app, &s, &w, &["2026-03-02T03:30:00Z", "2026-03-02T12:30:00Z"]).await;
    let get = |q: String| { let app = app.clone(); let t = s.hr_token.clone(); async move { common::as_user(&app, &t, "GET", &format!("/v1/attendance/daily?{q}"), None, &[]).await } };
    let (_, rows) = get(format!("date=2026-03-02&branch_id={}&sort=-employee_code&processing_state=provisional", s.branch_id)).await;
    assert_eq!(rows["items"].as_array().unwrap().len(), 1, "{rows}");
    let (_, none) = get(format!("date=2026-03-02&branch_id={}", Uuid::new_v4())).await;
    assert_eq!(none["items"], json!([]));
    assert_eq!(get("date=2026-03-02&sort=name".into()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}
