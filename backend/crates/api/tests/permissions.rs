//! Permissions (slice 13).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn org_policy(app: &axum::Router, s: &common::SeedOrg) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

fn version(approval: &str) -> Value {
    json!({ "effective_from": "2026-01-01", "reason": "setup", "duration_unit": "block_30", "min_minutes": 30, "max_minutes": 120,
            "limit_basis": "count", "limit_count": 2, "limit_period": "calendar_month", "max_per_day": 1, "allowed_position": "any",
            "paid": true, "counts_as_worked": true, "counts_toward_overtime": false, "apply_window": "after_within_days", "after_event_days": 2,
            "min_notice_minutes": 0, "approval": approval, "timeout_action": "escalate", "over_limit": "block", "over_stay": "late_early_minutes",
            "same_day_half_leave_allowed": false, "adjacent_off_allowed": true, "mid_period_joiner": "full", "eligibility": {},
            "blackout_enabled": false, "blackout_dates": [], "punch_requirement": {}, "converted_uses_limit": true })
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

async fn usage(app: &axum::Router, w: &common::Worker, date: &str) -> Value {
    let (_, u) = common::as_user(app, &w.token, "GET", &format!("/v1/permissions/employees/{}/usage?on_date={date}", w.employee_id), None, &[]).await;
    u["items"][0].clone()
}

/// PM-R1..R4 and PM-R6: refused applications, approval posting usage, no late mark inside an approved permission at
/// the shift start, and cancellation reversing both.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn permission_covers_the_shift_start(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await; // 09:00-18:00 India
    let d = (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(1);
    let ds = d.to_string();
    manual(&app, &s, &w, &[format!("{d}T04:30:00Z"), format!("{d}T12:30:00Z")]).await; // in at 10:00, out at 18:00
    assert_eq!(day(&app, &s, &w, &ds).await["late_minutes"], 60, "an hour late without a permission");

    let mut bad = version("manager");
    bad["over_limit"] = json!("hr_approval_unpaid");
    assert_eq!(common::call(&app, &s, "POST", "/v1/permissions/types", Some(json!({ "code": "X", "name": "X", "version": bad })), None).await.0,
               StatusCode::UNPROCESSABLE_ENTITY, "options not built yet are refused, not ignored");
    let (st, t, _) = common::call(&app, &s, "POST", "/v1/permissions/types", Some(json!({ "code": "PER", "name": "Personal", "version": version("manager") })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    let ptype = t["id"].as_str().unwrap().to_string();

    let ask = |start: &str, end: &str| json!({ "permission_type_id": ptype, "permission_date": ds, "start_time": start, "end_time": end, "reason": "bank" });
    let submit = |body: Value| {
        let app = app.clone();
        let tok = w.token.clone();
        async move { common::as_user(&app, &tok, "POST", "/v1/permissions/requests", Some(body), &key()).await }
    };
    assert_eq!(submit(ask("09:00", "09:45")).await.0, StatusCode::UNPROCESSABLE_ENTITY, "PM-03 blocks of 30");
    assert_eq!(submit(ask("08:00", "09:00")).await.0, StatusCode::UNPROCESSABLE_ENTITY, "PM-10 inside the shift");
    let (st, r) = submit(ask("09:00", "10:00")).await;
    assert_eq!((st, r["state"].as_str(), r["minutes"].as_i64()), (StatusCode::CREATED, Some("pending_l1"), Some(60)), "{r}");
    let (st, b) = submit(ask("12:00", "12:30")).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("LIMIT_EXCEEDED")), "PM-09 one a day: {b}");
    let id = r["id"].as_str().unwrap();

    let (st, ok) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/permissions/requests/{id}/approve"), None, &key()).await;
    assert_eq!((st, ok["state"].as_str()), (StatusCode::OK, Some("approved")), "{ok}");
    let u = usage(&app, &w, &ds).await;
    assert_eq!((u["used_count"].as_i64(), u["used_minutes"].as_i64(), u["remaining_count"].as_i64()), (Some(1), Some(60), Some(1)), "{u}");
    let r = day(&app, &s, &w, &ds).await;
    assert_eq!((r["late_minutes"].as_i64(), r["day_status"].as_str()), (Some(0), Some("present")), "no late mark inside the permission: {r}");
    assert!(r["explanation"].to_string().contains("PM-12"), "{r}");

    // Cancellation: the employee only before the start; HR after it with a reason. Usage and the late mark come back.
    let cancel = format!("/v1/permissions/requests/{id}/cancel");
    assert_eq!(common::as_user(&app, &w.token, "POST", &cancel, Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::CONFLICT, "after the start: HR only");
    let (st, c) = common::as_user(&app, &s.hr_token, "POST", &cancel, Some(json!({ "reason": "entered by mistake" })), &key()).await;
    assert_eq!((st, c["state"].as_str()), (StatusCode::OK, Some("cancelled")), "{c}");
    assert_eq!(usage(&app, &w, &ds).await["used_count"], 0);
    assert_eq!(day(&app, &s, &w, &ds).await["late_minutes"], 60);

    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/permissions/requests/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}

/// PM-16 auto within limit; the monthly count limit; BR-07; versions are append-only.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn limits_auto_approval_and_versions(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (_, t, _) = common::call(&app, &s, "POST", "/v1/permissions/types", Some(json!({ "code": "AUTO", "name": "Auto", "version": version("auto_within_limit") })), None).await;
    let ptype = t["id"].as_str().unwrap().to_string();
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let mut states = vec![];
    for back in [2, 1, 0] {
        let date = today - Duration::days(back);
        let body = json!({ "permission_type_id": ptype, "permission_date": date, "start_time": "13:00", "end_time": "13:30", "reason": "errand" });
        let (st, r) = common::as_user(&app, &w.token, "POST", "/v1/permissions/requests", Some(body), &key()).await;
        states.push((st, r["state"].as_str().map(str::to_string).or(r["code"].as_str().map(str::to_string))));
    }
    let same_month = (today - Duration::days(2)).format("%m").to_string() == today.format("%m").to_string();
    if same_month {
        assert_eq!(states, vec![(StatusCode::CREATED, Some("approved".into())), (StatusCode::CREATED, Some("approved".into())), (StatusCode::CONFLICT, Some("LIMIT_EXCEEDED".into()))],
                   "two a month, approved at once (PM-16); the third is over the limit");
    }

    let mut v2 = version("manager");
    v2["effective_from"] = json!("2026-01-01");
    let (st, b, _) = common::call(&app, &s, "POST", &format!("/v1/permissions/types/{ptype}/versions"), Some(v2.clone()), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")), "{b}");
    v2["effective_from"] = json!("2026-02-01");
    assert_eq!(common::call(&app, &s, "POST", &format!("/v1/permissions/types/{ptype}/versions"), Some(v2), None).await.0, StatusCode::CREATED);
    let (_, full, _) = common::call(&app, &s, "GET", &format!("/v1/permissions/types/{ptype}"), None, None).await;
    assert_eq!(full["versions"].as_array().map(Vec::len), Some(2), "{full}");

    // BR-07: an HR user who is also the employee never approves their own permission.
    let hr_emp = common::create_employee(&app, &s, "Ravi").await["id"].as_str().unwrap().parse::<Uuid>().unwrap();
    sqlx::query("UPDATE iam.user_account SET employee_id = $1 WHERE id = $2").bind(hr_emp).bind(s.hr_user_id).execute(&pool).await.unwrap();
    common::day_shift(&app, &s, hr_emp).await;
    let hr_role: Uuid = sqlx::query_scalar("SELECT role_id FROM iam.user_role WHERE user_id = $1").bind(s.hr_user_id).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'PRM-02', 'self')").bind(s.org_id).bind(hr_role).execute(&pool).await.unwrap();
    let (_, t2, _) = common::call(&app, &s, "POST", "/v1/permissions/types", Some(json!({ "code": "MGR", "name": "Managed", "version": version("manager") })), None).await;
    let body = json!({ "permission_type_id": t2["id"], "permission_date": today, "start_time": "17:00", "end_time": "18:00", "reason": "doctor" });
    let (st, own) = common::as_user(&app, &s.hr_token, "POST", "/v1/permissions/requests", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{own}");
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/permissions/requests/{}/approve", own["id"].as_str().unwrap()), None, &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")));
}
