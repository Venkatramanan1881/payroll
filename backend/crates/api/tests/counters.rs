//! Period counters and period-end penalties (slice 16).

mod common;

use axum::http::StatusCode;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> [(&'static str, String); 1] {
    [("idempotency-key", Uuid::new_v4().to_string())]
}

fn last_month_start() -> NaiveDate {
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let n = today.year() * 12 + today.month0() as i32 - 1;
    NaiveDate::from_ymd_opt(n.div_euclid(12), n.rem_euclid(12) as u32 + 1, 1).unwrap()
}

async fn policy(app: &axum::Router, s: &common::SeedOrg, la: Value) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    for bad in [json!({ "LA-09": "fixed_amount" }), json!({ "LA-18": "quarter" }), json!({ "LA-12": { "then": "fixed_fine" } })] {
        let (st, _, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/LA"), Some(json!({ "schema_version": 1, "settings": bad })), etag.as_deref()).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "options not built are refused");
    }
    let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/LA"), Some(json!({ "schema_version": 1, "settings": la })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

async fn setup(pool: &PgPool, app: &axum::Router, s: &common::SeedOrg) {
    let org = json!({ "financial_year": { "kind": "april_to_march" }, "week_start": "monday", "languages": ["en"], "currency": "INR",
            "employee_code_mode": "automatic", "mandatory_fields": [], "contact_self_edit": "off", "probation_months": 6,
            "rejoin_code": "new_code", "allowed_branches_mode": "master_only", "non_allowed_branch_punch": "record_and_flag",
            "other_branch_costing": "branch_worked" });
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(s.org_id).bind(org).execute(pool).await.unwrap();
    let body = json!({ "owner_kind": "organisation", "owner_id": s.org_id, "frequency": "monthly", "start_kind": "calendar", "cutoff_day": 1, "valid_from": last_month_start() });
    let (st, c) = common::as_user(app, &s.hr_token, "POST", "/v1/period/cycles", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    let req = axum::http::Request::post("/internal/jobs/period-housekeeping")
        .header("authorization", format!("Bearer {}", String::from_utf8_lossy(common::JOB_SECRET)))
        .body(axum::body::Body::empty())
        .unwrap();
    assert_eq!(common::send(app, req).await.0.status(), StatusCode::OK);
}

/// Four days of last month, in at 09:30 India (30 minutes late, a late mark each), out at 18:00.
async fn four_late_days(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker) {
    let mut entries = vec![];
    for i in 0..4 {
        let d = last_month_start() + Duration::days(i + 1);
        entries.push(json!({ "employee_id": w.employee_id, "event_at": d.and_hms_opt(4, 0, 0).unwrap().and_utc() }));
        entries.push(json!({ "employee_id": w.employee_id, "event_at": d.and_hms_opt(12, 30, 0).unwrap().and_utc() }));
    }
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, r) = common::as_user(app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
}

async fn counters(app: &axum::Router, token: &str, w: &common::Worker) -> Value {
    let (st, c) = common::as_user(app, token, "GET", &format!("/v1/attendance/employees/{}/counters?date={}", w.employee_id, last_month_start()), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    c
}

async fn period_id(app: &axum::Router, s: &common::SeedOrg) -> String {
    let (_, list) = common::as_user(app, &s.hr_token, "GET", "/v1/periods", None, &[]).await;
    list["items"].as_array().unwrap().iter().find(|p| p["period_start"] == json!(last_month_start())).unwrap()["id"].as_str().unwrap().to_string()
}

/// E5: four late marks, every 3rd = half a day; a preview recovered from CL before the lock, recorded and debited
/// from CL at the lock (D-29, D-31); the minutes penalty is smaller, so only the count one applies (D-30).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn count_penalty_recovered_from_leave_at_lock(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    policy(&app, &s, json!({ "LA-06": true, "LA-10": true, "LA-12": { "leave_types": ["CL"], "then": "loss_of_pay" } })).await;
    setup(&pool, &app, &s).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let v = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 30, "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true });
    let (st, cl, _) = common::call(&app, &s, "POST", "/v1/leave/types", Some(json!({ "code": "CL", "name": "Casual", "version": v })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{cl}");
    let body = json!({ "leave_type_id": cl["id"], "quantity": "1.00", "entry_date": "2026-01-01", "reason": "opening" });
    assert_eq!(common::call(&app, &s, "POST", &format!("/v1/leave/employees/{}/ledger-adjustments", w.employee_id), Some(body), None).await.0, StatusCode::CREATED);
    four_late_days(&app, &s, &w).await;

    let c = counters(&app, &w.token, &w).await;
    assert_eq!((c["late_marks"].as_i64(), c["late_minutes"].as_i64()), (Some(4), Some(120)), "{c}");
    let p = &c["penalties"];
    assert_eq!((p.as_array().map(Vec::len), p[0]["rule_ref"].as_str(), p[0]["days"].as_str(), p[0]["recovered_as"].as_str(), p[0]["provisional"].as_bool()),
               (Some(1), Some("LA-06"), Some("0.50"), Some("CL"), Some(true)), "{c}");

    let period = period_id(&app, &s).await;
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{period}/lock"), None, &key()).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    let c = counters(&app, &s.hr_token, &w).await;
    assert_eq!((c["penalties"][0]["provisional"].as_bool(), c["penalties"][0]["id"].is_string()), (Some(false), true), "recorded at the lock: {c}");
    let bal: String = sqlx::query_scalar("SELECT to_char(sum(quantity), 'FM990.00') FROM leave.leave_ledger_entry").fetch_one(&pool).await.unwrap();
    assert_eq!(bal, "0.50", "half a day debited from CL");
    let (_, t) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{period}/totals?employee_id={}", w.employee_id), None, &[]).await;
    assert!(!t.to_string().contains("late_early_penalty"), "a leave recovery is not a deduction line: {t}");
    let kept: i64 = sqlx::query_scalar("SELECT late_marks::int8 FROM attendance.period_counter WHERE employee_id = $1").bind(w.employee_id).fetch_one(&pool).await.unwrap();
    assert_eq!(kept, 4);
}

/// With no leave listed the penalty is loss of pay, a deduction line in the totals; opening late marks count.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn penalty_as_loss_of_pay_with_opening_marks(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    policy(&app, &s, json!({ "LA-06": true, "LA-08": 3, "LA-09": "quarter_day" })).await;
    setup(&pool, &app, &s).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    four_late_days(&app, &s, &w).await;
    let opening = json!({ "entries": [{ "employee_id": w.employee_id, "balance_kind": "late_counter", "value": "2.00", "as_of_date": last_month_start() }] });
    let (st, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(opening), None).await;
    assert_eq!(st, StatusCode::CREATED, "{batch}");
    let (st, b, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{}/confirm", batch["id"].as_str().unwrap()), None, None).await;
    assert_eq!(st, StatusCode::OK, "{b}");

    let c = counters(&app, &s.hr_token, &w).await;
    assert_eq!((c["late_marks"].as_i64(), c["opening_late_marks"].as_i64()), (Some(6), Some(2)), "{c}");
    assert_eq!((c["penalties"][0]["days"].as_str(), c["penalties"][0]["recovered_as"].as_str()), (Some("0.50"), Some("loss_of_pay")), "6 marks / 3 = 2 quarter days: {c}");
    let period = period_id(&app, &s).await;
    let (_, t) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{period}/totals?employee_id={}", w.employee_id), None, &[]).await;
    let line = t["items"][0]["deduction_lines"].as_array().unwrap().iter().find(|l| l["reason"] == "late_early_penalty").cloned();
    assert_eq!(line.map(|l| l["days"].clone()), Some(json!("0.50")), "{t}");
    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/attendance/employees/{}/counters?date={}", w.employee_id, last_month_start()), None, &[]).await.0,
               StatusCode::NOT_FOUND, "TB-01");
}

/// OB-R2: an opening count of permissions used counts against the limit period of its date.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn opening_permission_use_counts(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let v = json!({ "effective_from": "2026-01-01", "reason": "setup", "duration_unit": "block_30", "min_minutes": 30, "max_minutes": 120,
            "limit_basis": "count", "limit_count": 2, "limit_period": "calendar_month", "max_per_day": 1, "allowed_position": "any",
            "paid": true, "counts_as_worked": true, "counts_toward_overtime": false, "apply_window": "after_within_days", "after_event_days": 2,
            "min_notice_minutes": 0, "approval": "manager", "timeout_action": "escalate", "over_limit": "block", "over_stay": "late_early_minutes",
            "same_day_half_leave_allowed": false, "adjacent_off_allowed": true, "mid_period_joiner": "full", "eligibility": {},
            "blackout_enabled": false, "blackout_dates": [], "punch_requirement": {}, "converted_uses_limit": true });
    let (st, t, _) = common::call(&app, &s, "POST", "/v1/permissions/types", Some(json!({ "code": "PER", "name": "Personal", "version": v })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let row = |value: &str| json!({ "employee_id": w.employee_id, "balance_kind": "permission_used", "permission_type_id": t["id"], "value": value, "as_of_date": today });
    let (st, _, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [row("1.50")] })), None).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "a count is whole");
    let (st, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [row("2.00")] })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{batch}");
    let (st, _, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{}/confirm", batch["id"].as_str().unwrap()), None, None).await;
    assert_eq!(st, StatusCode::OK);
    let (_, u) = common::as_user(&app, &w.token, "GET", &format!("/v1/permissions/employees/{}/usage?on_date={today}", w.employee_id), None, &[]).await;
    assert_eq!((u["items"][0]["used_count"].as_i64(), u["items"][0]["remaining_count"].as_i64()), (Some(2), Some(0)), "{u}");
}
