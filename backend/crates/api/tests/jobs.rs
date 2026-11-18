//! Timed jobs J1 and J3 (slice 7).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn run(app: &axum::Router, name: &str, secret: Option<&[u8]>) -> (StatusCode, Value) {
    let mut b = axum::http::Request::post(format!("/internal/jobs/{name}"));
    if let Some(s) = secret {
        b = b.header("authorization", format!("Bearer {}", String::from_utf8_lossy(s)));
    }
    let (res, body) = common::send(app, b.body(axum::body::Body::empty()).unwrap()).await;
    (res.status(), body)
}

async fn org_policy(app: &axum::Router, s: &common::SeedOrg, groups: &[(&str, Value)]) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    for (g, doc) in groups {
        let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
        let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/{g}"), Some(json!({ "schema_version": 1, "settings": doc })), etag.as_deref()).await;
        assert_eq!(st, StatusCode::OK, "{b}");
    }
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

/// Review focus 5.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn job_secret_is_required(pool: PgPool) {
    let app = common::app(pool);
    assert_eq!(run(&app, "nightly-recalculation", None).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(run(&app, "nightly-recalculation", Some(b"wrong-secret")).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(run(&app, "no-such-job", Some(common::JOB_SECRET)).await.0, StatusCode::NOT_FOUND);
}

/// J1 charges a past absence (DS-10) and a second run writes nothing (review focus 4).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn nightly_recalculation_charges_absences_once(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[("DS", json!({ "DS-10": { "action": "loss_of_pay" } }))]).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (st, r) = run(&app, "nightly-recalculation", Some(common::JOB_SECRET)).await;
    assert_eq!((st, r["organisations"].as_i64()), (StatusCode::OK, Some(1)), "{r}");
    assert!(r["results"][0]["summary"]["results_written"].as_i64().unwrap() >= 1, "{r}");
    let yesterday = (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(1);
    let (_, d) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/attendance/employees/{}/days/{yesterday}", w.employee_id), None, &[]).await;
    assert_eq!(d["current_result"]["day_effects"][0]["effect_kind"], "absence_lop", "{d}");
    let (_, again) = run(&app, "nightly-recalculation", Some(common::JOB_SECRET)).await;
    assert_eq!(again["results"][0]["summary"]["results_written"], 0, "unchanged inputs write nothing: {again}");
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE actor_kind = 'system_job'").fetch_one(&pool).await.unwrap();
    assert_eq!(audits, 2);
}

/// J3 escalates an old regularisation once; auto-approves an old leave with balance and escalates one without.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approval_timeouts_escalate_once_and_never_overdraw(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[("RG", json!({ "RG-07": { "days": 1, "action": "escalate" } }))]).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let d = (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(3);
    // Requests submitted three days ago (inserted directly: the audit log cannot be back-dated).
    let reg = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO attendance.regularisation_request (org_id, id, employee_id, attendance_date, request_type, proposed_in_at, proposed_out_at, reason,
           requested_by_user_id, created_at) VALUES ($1, $2, $3, $4, 'whole_day', $5, $6, 'x', $7, now() - interval '3 days')",
    )
    .bind(s.org_id).bind(reg).bind(w.employee_id).bind(d)
    .bind(d.and_hms_opt(3, 30, 0).unwrap().and_utc()).bind(d.and_hms_opt(12, 30, 0).unwrap().and_utc()).bind(w.user_id)
    .execute(&pool).await.unwrap();

    let version = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 1, "timeout_action": "auto_approve", "during_notice": "allow", "combine_allowed": true });
    let (st, t, _) = common::call(&app, &s, "POST", "/v1/leave/types", Some(json!({ "code": "CL", "name": "Casual", "version": version })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    let cl: Uuid = t["id"].as_str().unwrap().parse().unwrap();
    let (st, _, _) = common::call(&app, &s, "POST", &format!("/v1/leave/employees/{}/ledger-adjustments", w.employee_id),
        Some(json!({ "leave_type_id": cl, "quantity": "1.00", "entry_date": "2026-01-01", "reason": "opening" })), None).await;
    assert_eq!(st, StatusCode::CREATED);
    let mut leaves = vec![];
    // The first (older) request takes the only day of balance; the second would overdraw.
    for (day, age) in [(d, 4), (d - Duration::days(1), 3)] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO leave.leave_request (org_id, id, employee_id, leave_type_id, from_date, to_date, state, requested_by, submitted_at)
                     VALUES ($1, $2, $3, $4, $5, $5, 'pending_l1', $6, now() - make_interval(days => $7))")
            .bind(s.org_id).bind(id).bind(w.employee_id).bind(cl).bind(day).bind(w.user_id).bind(age).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO leave.leave_request_day (org_id, leave_request_id, leave_date, fraction) VALUES ($1, $2, $3, 1.0)")
            .bind(s.org_id).bind(id).bind(day).execute(&pool).await.unwrap();
        leaves.push(id);
    }

    let (st, r) = run(&app, "approval-timeouts", Some(common::JOB_SECRET)).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let sum = &r["results"][0]["summary"];
    assert_eq!((sum["regularisation"]["escalated"].as_i64(), sum["leave"]["auto_approved"].as_i64(), sum["leave"]["escalated"].as_i64()), (Some(1), Some(1), Some(1)), "{r}");
    let (state, escalated, approver): (String, bool, Option<Uuid>) =
        sqlx::query_as("SELECT state, escalated, current_approver_user_id FROM attendance.regularisation_request WHERE id = $1").bind(reg).fetch_one(&pool).await.unwrap();
    assert_eq!((state.as_str(), escalated, approver), ("pending_l1", true, None), "escalated to the HR pool");
    let states: Vec<String> = sqlx::query_scalar("SELECT state FROM leave.leave_request ORDER BY from_date DESC").fetch_all(&pool).await.unwrap();
    assert_eq!(states, ["approved", "pending_l1"], "the second leave would overdraw: escalated, not approved");
    let balance: String = sqlx::query_scalar("SELECT to_char(sum(quantity), 'FM990.00') FROM leave.leave_ledger_entry").fetch_one(&pool).await.unwrap();
    assert_eq!(balance, "0.00");

    let (_, again) = run(&app, "approval-timeouts", Some(common::JOB_SECRET)).await;
    let sum = &again["results"][0]["summary"];
    assert_eq!((sum["regularisation"]["escalated"].as_i64(), sum["leave"]["escalated"].as_i64()), (Some(0), Some(0)), "never twice at one level: {again}");
}
