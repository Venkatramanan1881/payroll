//! Leave (slice 6). Shift 09:00-18:00 India time = 03:30Z-12:30Z; middle of the shift 13:30 = 08:00Z.

mod common;

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn days_ago(n: i64) -> NaiveDate {
    (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(n)
}

fn version(extra: Value) -> Value {
    let mut v = json!({
        "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true,
        "min_notice_days": 0, "approval_levels": "manager", "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true,
    });
    for (k, x) in extra.as_object().unwrap() {
        v[k] = x.clone();
    }
    v
}

async fn post(app: &axum::Router, token: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    common::as_user(app, token, "POST", uri, body, &[("idempotency-key", Uuid::new_v4().to_string())]).await
}

async fn leave_type(app: &axum::Router, s: &common::SeedOrg, code: &str, extra: Value) -> String {
    let (st, b) = post(app, &s.hr_token, "/v1/leave/types", Some(json!({ "code": code, "name": code, "version": version(extra) }))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    b["id"].as_str().unwrap().to_string()
}

/// An organisation policy from 2026-01-01 with optional rule-group documents.
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

async fn grant(pool: &PgPool, org: Uuid, user: Uuid, op: &str, kind: &str) {
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, $3)").bind(org).bind(role).bind(format!("r-{role}")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, $4)").bind(org).bind(role).bind(op).bind(kind).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
}

/// Worker with a day shift and a manager (LVE-04 approve) they report to.
async fn people(pool: &PgPool, app: &axum::Router, s: &common::SeedOrg) -> (common::Worker, common::Worker) {
    let w = common::seed_worker(pool, app, s, "Asha").await;
    common::day_shift(app, s, w.employee_id).await;
    let boss = common::seed_worker(pool, app, s, "Meena").await;
    grant(pool, s.org_id, boss.user_id, "LVE-04", "approve").await;
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, manager_employee_id, valid) VALUES ($1, $2, 'reporting_manager', $3, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(w.employee_id).bind(boss.employee_id).execute(pool).await.unwrap();
    (w, boss)
}

async fn credit(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, t: &str, qty: &str) {
    let body = json!({ "leave_type_id": t, "quantity": qty, "entry_date": "2026-01-01", "reason": "opening balance" });
    let (st, b) = post(app, &s.hr_token, &format!("/v1/leave/employees/{}/ledger-adjustments", w.employee_id), Some(body)).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
}

async fn balance(app: &axum::Router, w: &common::Worker, t: &str) -> String {
    let (st, b) = common::as_user(app, &w.token, "GET", &format!("/v1/leave/employees/{}/balances", w.employee_id), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b["balances"].as_array().unwrap().iter().find(|x| x["leave_type_id"] == t).unwrap()["balance"].as_str().unwrap().to_string()
}

async fn day(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, d: NaiveDate) -> Value {
    let (st, b) = common::as_user(app, &s.hr_token, "GET", &format!("/v1/attendance/employees/{}/days/{d}", w.employee_id), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b["current_result"].clone()
}

/// Review focus 1: approving twice debits once; cancelling approved leave reverses once.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approve_debits_once_and_cancel_reverses(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[]).await;
    let (w, boss) = people(&pool, &app, &s).await;
    let cl = leave_type(&app, &s, "CL", json!({})).await;
    credit(&app, &s, &w, &cl, "5.00").await;
    let d = days_ago(1);
    let (st, r) = post(&app, &w.token, "/v1/leave/requests", Some(json!({ "leave_type_id": cl, "from_date": d, "to_date": d, "reason": "fever" }))).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::CREATED, Some("pending_l1")), "{r}");
    assert_eq!(r["current_approver_user_id"], json!(boss.user_id));
    let id = r["id"].as_str().unwrap();
    let (st, r) = post(&app, &boss.token, &format!("/v1/leave/requests/{id}/approve"), None).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("approved")), "{r}");
    assert_eq!(post(&app, &boss.token, &format!("/v1/leave/requests/{id}/approve"), None).await.0, StatusCode::OK);
    assert_eq!(balance(&app, &w, &cl).await, "4.00");
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "leave");

    let (st, r) = post(&app, &w.token, &format!("/v1/leave/requests/{id}/cancel"), Some(json!({ "reason": "came to work after all" }))).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("cancelled")), "{r}");
    assert_eq!(post(&app, &w.token, &format!("/v1/leave/requests/{id}/cancel"), Some(json!({ "reason": "again" }))).await.0, StatusCode::OK);
    assert_eq!(balance(&app, &w, &cl).await, "5.00");
    let kinds: Vec<String> = sqlx::query_scalar("SELECT entry_kind FROM leave.leave_ledger_entry ORDER BY created_at").fetch_all(&pool).await.unwrap();
    assert_eq!(kinds, ["adjustment", "debit", "reversal"]);
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "absent", "cancelled leave: the day is recalculated");
}

/// Review focus 3 (E12): first-half leave, in 20 minutes after the second half starts.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e12_half_day_leave_with_late_second_half(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[]).await;
    let (w, boss) = people(&pool, &app, &s).await;
    let cl = leave_type(&app, &s, "CL", json!({})).await;
    credit(&app, &s, &w, &cl, "2.00").await;
    let d = days_ago(1);
    // In 13:50 India time (08:20Z): 20 minutes after the second half starts at the middle of the shift (13:30).
    let body = json!({ "branch_id": s.branch_id, "reason": "x", "entries": [
        { "employee_id": w.employee_id, "event_at": format!("{d}T08:20:00Z") }, { "employee_id": w.employee_id, "event_at": format!("{d}T12:30:00Z") } ] });
    assert_eq!(post(&app, &s.hr_token, "/v1/manual-attendance", Some(body)).await.0, StatusCode::CREATED);
    let (_, r) = post(&app, &w.token, "/v1/leave/requests", Some(json!({ "leave_type_id": cl, "from_date": d, "to_date": d, "from_part": "first_half", "to_part": "first_half" }))).await;
    let id = r["id"].as_str().unwrap();
    assert_eq!(r["days"], json!([{ "leave_date": d, "fraction": "0.50", "half": "first_half" }]));
    post(&app, &boss.token, &format!("/v1/leave/requests/{id}/approve"), None).await;
    let result = day(&app, &s, &w, d).await;
    assert_eq!((result["day_status"].as_str(), result["late_minutes"].as_i64(), result["late_mark"].as_bool()), (Some("half_day_leave"), Some(20), Some(true)), "{result}");
    assert_eq!(balance(&app, &w, &cl).await, "1.50");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submission_refusals(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[]).await;
    let (w, _boss) = people(&pool, &app, &s).await;
    let cl = leave_type(&app, &s, "CL", json!({ "half_day_allowed": false })).await;
    let only_staff = leave_type(&app, &s, "SL", json!({ "eligibility": { "category_ids": [Uuid::new_v4()] } })).await;
    credit(&app, &s, &w, &cl, "1.00").await;
    let req = |t: &str, from: NaiveDate, to: NaiveDate, part: &str| json!({ "leave_type_id": t, "from_date": from, "to_date": to, "from_part": part, "to_part": part });
    let d = days_ago(1);
    let cases = [
        (req(&cl, d, d, "first_half"), StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED"),
        (req(&cl, days_ago(6), days_ago(6), "full"), StatusCode::CONFLICT, "OUTSIDE_REQUEST_WINDOW"),
        (req(&only_staff, d, d, "full"), StatusCode::CONFLICT, "NOT_ELIGIBLE"),
        (req(&cl, d - Duration::days(1), d, "full"), StatusCode::CONFLICT, "LIMIT_EXCEEDED"),
    ];
    for (body, status, code) in cases {
        let (st, b) = post(&app, &w.token, "/v1/leave/requests", Some(body.clone())).await;
        assert_eq!((st, b["code"].as_str()), (status, Some(code)), "{body} -> {b}");
    }
    assert_eq!(post(&app, &w.token, "/v1/leave/requests", Some(req(&cl, d, d, "full"))).await.0, StatusCode::CREATED);
    let (st, b) = post(&app, &w.token, "/v1/leave/requests", Some(req(&cl, d, d, "full"))).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("OVERLAPPING_REQUEST")), "{b}");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM leave.leave_request").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1, "refused submissions create nothing");
}

/// Review focus 5: whoever submitted on behalf cannot approve it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn on_behalf_submitter_cannot_approve(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[]).await;
    let (w, boss) = people(&pool, &app, &s).await;
    let cl = leave_type(&app, &s, "CL", json!({ "approval_levels": "manager_then_hr" })).await;
    credit(&app, &s, &w, &cl, "3.00").await;
    let d = days_ago(1);
    let (st, r) = post(&app, &s.hr_token, "/v1/leave/requests/on-behalf", Some(json!({ "employee_id": w.employee_id, "leave_type_id": cl, "from_date": d, "to_date": d }))).await;
    assert_eq!((st, r["requested_by"].clone()), (StatusCode::CREATED, json!(s.hr_user_id)), "{r}");
    let id = r["id"].as_str().unwrap();
    let (_, r) = post(&app, &boss.token, &format!("/v1/leave/requests/{id}/approve"), None).await;
    assert_eq!(r["state"], "pending_l2", "{r}");
    let (st, b) = post(&app, &s.hr_token, &format!("/v1/leave/requests/{id}/approve"), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "{b}");
}

/// Review focus 4 and DS-10: an absence is charged (loss of pay, or LV-04 auto-debit) and undone when leave covers it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn absence_auto_debit_and_its_reversal(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let cl = leave_type(&app, &s, "CL", json!({})).await;
    org_policy(&app, &s, &[("DS", json!({ "DS-10": { "action": "auto_debit" } })), ("LV", json!({ "LV-04": [cl] }))]).await;
    let (w, boss) = people(&pool, &app, &s).await;
    credit(&app, &s, &w, &cl, "2.00").await;
    let d = days_ago(1);
    let recalc = json!({ "from": d, "to": d, "reason": "nightly", "employee_ids": [w.employee_id] });
    assert_eq!(post(&app, &s.hr_token, "/v1/attendance/recalculations", Some(recalc)).await.0, StatusCode::ACCEPTED);
    let r = day(&app, &s, &w, d).await;
    assert!(r["flags"].as_array().unwrap().iter().any(|f| f == "absence_auto_debited"), "{r}");
    assert_eq!(balance(&app, &w, &cl).await, "1.00");

    let (_, req) = post(&app, &w.token, "/v1/leave/requests", Some(json!({ "leave_type_id": cl, "from_date": d, "to_date": d }))).await;
    let id = req["id"].as_str().unwrap();
    assert_eq!(post(&app, &boss.token, &format!("/v1/leave/requests/{id}/approve"), None).await.0, StatusCode::OK);
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "leave");
    assert_eq!(balance(&app, &w, &cl).await, "1.00", "auto-debit reversed, leave debited: 2 - 1 + 1 - 1");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn absence_without_balance_is_loss_of_pay(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[("DS", json!({ "DS-10": { "action": "loss_of_pay" } }))]).await;
    let (w, _boss) = people(&pool, &app, &s).await;
    let d = days_ago(1);
    let recalc = json!({ "from": d, "to": d, "reason": "nightly", "employee_ids": [w.employee_id] });
    post(&app, &s.hr_token, "/v1/attendance/recalculations", Some(recalc)).await;
    let r = day(&app, &s, &w, d).await;
    assert_eq!(r["day_effects"][0]["effect_kind"], "absence_lop", "{r}");
    assert_eq!(r["day_effects"][0]["days"], "1.00");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn bulk_decisions_report_each_item(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, &[]).await;
    let (w, boss) = people(&pool, &app, &s).await;
    let other = common::seed_worker(&pool, &app, &s, "Ravi").await;
    common::day_shift(&app, &s, other.employee_id).await;
    let cl = leave_type(&app, &s, "CL", json!({})).await;
    credit(&app, &s, &w, &cl, "3.00").await;
    credit(&app, &s, &other, &cl, "3.00").await;
    let d = days_ago(1);
    let (_, a) = post(&app, &w.token, "/v1/leave/requests", Some(json!({ "leave_type_id": cl, "from_date": d, "to_date": d }))).await;
    let (_, b) = post(&app, &other.token, "/v1/leave/requests", Some(json!({ "leave_type_id": cl, "from_date": d, "to_date": d }))).await;
    let (st, res) = post(&app, &boss.token, "/v1/leave/requests/bulk-decisions", Some(json!({ "decision": "approve", "request_ids": [a["id"], b["id"]] }))).await;
    assert_eq!(st, StatusCode::OK, "{res}");
    assert_eq!((res["items"][0]["outcome"].as_str(), res["items"][0]["state"].as_str()), (Some("done"), Some("approved")), "{res}");
    assert_eq!((res["items"][1]["outcome"].as_str(), res["items"][1]["error"]["code"].as_str()), (Some("refused"), Some("NOT_FOUND")), "not routed to this manager");
}
