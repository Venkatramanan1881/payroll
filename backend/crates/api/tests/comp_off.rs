//! Comp-off (slice 15).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

fn today() -> NaiveDate {
    (Utc::now() + Duration::minutes(330)).date_naive()
}

async fn org_policy(app: &axum::Router, s: &common::SeedOrg, hw: Option<Value>) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    let (st, _, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/HW"), Some(json!({ "schema_version": 1, "settings": { "HW-06": "pay" } })), etag.as_deref()).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "HW-06 pay is V2 (D-26)");
    if let Some(doc) = hw {
        let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/HW"), Some(json!({ "schema_version": 1, "settings": doc })), etag.as_deref()).await;
        assert_eq!(st, StatusCode::OK, "{b}");
    }
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

/// A weekly off on `d`, worked from 08:00 India for `hours`.
async fn work_off_day(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, d: NaiveDate, hours: i64) {
    let (st, b, _) = common::call(app, s, "POST", "/v1/roster-entries/set", Some(json!({ "entries": [ { "employee_id": w.employee_id, "roster_date": d, "is_weekly_off": true } ] })), None).await;
    assert!(st.is_success(), "{b}");
    let start = d.and_hms_opt(2, 30, 0).unwrap().and_utc();
    let entries = json!([{ "employee_id": w.employee_id, "event_at": start }, { "employee_id": w.employee_id, "event_at": start + Duration::hours(hours) }]);
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, r) = common::as_user(app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
}

async fn credits(app: &axum::Router, w: &common::Worker) -> Vec<Value> {
    let (_, b) = common::as_user(app, &w.token, "GET", "/v1/comp-off/credits", None, &[]).await;
    b["items"].as_array().cloned().unwrap_or_default()
}

async fn balance(app: &axum::Router, w: &common::Worker) -> Value {
    let (st, b) = common::as_user(app, &w.token, "GET", &format!("/v1/comp-off/employees/{}/balance", w.employee_id), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b
}

async fn comp_off_leave_type(app: &axum::Router, s: &common::SeedOrg) -> String {
    let v = json!({
        "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true,
        "min_notice_days": 0, "approval_levels": "manager", "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true,
    });
    let (st, b) = common::as_user(app, &s.hr_token, "POST", "/v1/leave/types", Some(json!({ "code": "COMP_OFF", "name": "Comp-off", "version": v })), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    b["id"].as_str().unwrap().to_string()
}

/// CO-R1/HW-02 earn → pending → approved (credited, expiry from the worked date); CO-R2 taken as COMP_OFF leave and
/// given back when the leave is cancelled; BR-07.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn earn_approve_take_and_cancel(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, None).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let off = today() - Duration::days(3);
    work_off_day(&app, &s, &w, off, 10).await;

    let c = credits(&app, &w).await;
    assert_eq!(c.len(), 1, "{c:?}");
    assert_eq!((c[0]["state"].as_str(), c[0]["fraction"].as_str(), c[0]["worked_date"].as_str()), (Some("pending_approval"), Some("1.00"), Some(off.to_string().as_str())));
    assert_eq!(balance(&app, &w).await["balance"], "0.00", "not in balance until approved (AC-M02-03.1)");
    let id = c[0]["id"].as_str().unwrap();
    assert_eq!(common::as_user(&app, &w.token, "POST", &format!("/v1/comp-off/credits/{id}/approve"), None, &key()).await.0, StatusCode::FORBIDDEN, "BR-07");
    let (st, r) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/comp-off/credits/{id}/reject"), None, &key()).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "reject needs a reason: {r}");
    let (st, r) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/comp-off/credits/{id}/approve"), None, &key()).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("credited")), "{r}");
    assert_eq!(r["expires_on"], json!((off + Duration::days(60)).to_string()), "D-27");
    let (st, again) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/comp-off/credits/{id}/approve"), None, &key()).await;
    assert_eq!((st, again["remaining"].as_str()), (StatusCode::OK, Some("1.00")), "no second entry");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/comp-off/credits/{id}/reject"), Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::CONFLICT);
    assert_eq!(balance(&app, &w).await["balance"], "1.00");

    // A rejected claim is final: recalculating the day never raises it again (SM-COMPOFF).
    let off2 = today() - Duration::days(6);
    work_off_day(&app, &s, &w, off2, 10).await;
    let rid = credits(&app, &w).await.iter().find(|c| c["worked_date"] == json!(off2.to_string())).unwrap()["id"].as_str().unwrap().to_string();
    let (st, r) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/comp-off/credits/{rid}/reject"), Some(json!({ "reason": "not authorised" })), &key()).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("not_credited")), "{r}");
    let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
    sqlx::query("UPDATE attendance.attendance_day SET current_result_id = NULL WHERE employee_id = $1 AND attendance_date = $2").bind(w.employee_id).bind(off2).execute(&mut **tx).await.ok();
    domain_attendance::recalculate(&mut tx, w.employee_id, off2, "recalculation", None).await.unwrap();
    tx.commit().await.unwrap();
    let again: Vec<Value> = credits(&app, &w).await.into_iter().filter(|c| c["worked_date"] == json!(off2.to_string())).collect();
    assert_eq!(again.len(), 1, "{again:?}");

    // Taken as leave of the reserved type: more than the balance is refused; approval debits; cancelling gives it back.
    let t = comp_off_leave_type(&app, &s).await;
    let ask = |from: NaiveDate, to: NaiveDate| json!({ "leave_type_id": t, "from_date": from, "to_date": to, "from_part": "full", "to_part": "full", "reason": "rest" });
    let later = today() + Duration::days(5);
    let (st, b) = common::as_user(&app, &w.token, "POST", "/v1/leave/requests", Some(ask(later, later + Duration::days(1))), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("LIMIT_EXCEEDED")), "{b}");
    let (st, lr) = common::as_user(&app, &w.token, "POST", "/v1/leave/requests", Some(ask(later, later)), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{lr}");
    let lid = lr["id"].as_str().unwrap();
    let (st, ok) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/leave/requests/{lid}/approve"), None, &key()).await;
    assert_eq!((st, ok["state"].as_str()), (StatusCode::OK, Some("approved")), "{ok}");
    let b = balance(&app, &w).await;
    assert_eq!((b["balance"].as_str(), b["open_credits"].as_array().map(Vec::len)), (Some("0.00"), Some(0)), "{b}");
    assert_eq!(credits(&app, &w).await[0]["state"], "used");
    let (st, c) = common::as_user(&app, &w.token, "POST", &format!("/v1/leave/requests/{lid}/cancel"), Some(json!({ "reason": "plans changed" })), &key()).await;
    assert_eq!(st, StatusCode::OK, "{c}");
    assert_eq!(balance(&app, &w).await["balance"], "1.00");
    assert_eq!(credits(&app, &w).await[0]["state"], "credited");
    let (_, l) = common::as_user(&app, &w.token, "GET", &format!("/v1/comp-off/employees/{}/ledger", w.employee_id), None, &[]).await;
    let kinds: Vec<&str> = l["items"].as_array().unwrap().iter().filter_map(|e| e["entry_kind"].as_str()).collect();
    assert_eq!(kinds.len(), 3, "credit, debit, reversal: {l}");

    // A day already covered by full-day on-duty takes no leave.
    let away = today() + Duration::days(7);
    let od = json!({ "duration_kind": "full_day", "from_date": away, "to_date": away, "purpose": "audit", "place": "Madurai" });
    assert_eq!(common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(od), &key()).await.0, StatusCode::CREATED);
    let (st, b) = common::as_user(&app, &w.token, "POST", "/v1/leave/requests", Some(ask(away, away)), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("OVERLAPPING_REQUEST")), "{b}");

    // A leave adjustment cannot create comp-off out of thin air.
    let body = json!({ "leave_type_id": t, "quantity": "2.00", "entry_date": "2026-01-01", "reason": "x" });
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/leave/employees/{}/ledger-adjustments", w.employee_id), Some(body), &key()).await.0,
               StatusCode::UNPROCESSABLE_ENTITY);
    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/comp-off/credits/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}

/// HW-02 off credits at once; HW-04 half and nothing below; J6 lapses at expiry (D-26); no choice is offered (D-28).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn half_day_system_credit_and_lapse(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, Some(json!({ "HW-02": false, "HW-05": 1, "HW-06": "lapse" }))).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    work_off_day(&app, &s, &w, today() - Duration::days(3), 6).await;
    work_off_day(&app, &s, &w, today() - Duration::days(4), 2).await;

    let c = credits(&app, &w).await;
    assert_eq!(c.len(), 1, "two hours earn nothing (AC-M02-02.3): {c:?}");
    assert_eq!((c[0]["state"].as_str(), c[0]["fraction"].as_str()), (Some("credited"), Some("0.50")), "credited by the system (AC-M02-03.4)");
    assert_eq!(balance(&app, &w).await["balance"], "0.50");

    let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
    let r = domain_attendance::comp_off::expire_due(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(r["lapsed"], 1);
    assert_eq!(balance(&app, &w).await["balance"], "0.00");
    assert_eq!(credits(&app, &w).await[0]["state"], "expired_lapsed");

    // OD-10: an approved full-day on-duty on a weekly off earns a full comp-off.
    let d = today() - Duration::days(2);
    let (st, b, _) = common::call(&app, &s, "POST", "/v1/roster-entries/set", Some(json!({ "entries": [ { "employee_id": w.employee_id, "roster_date": d, "is_weekly_off": true } ] })), None).await;
    assert!(st.is_success(), "{b}");
    let od = json!({ "duration_kind": "full_day", "from_date": d, "to_date": d, "purpose": "audit", "place": "Madurai" });
    let (st, r) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(od), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/on-duty/requests/{}/approve", r["id"].as_str().unwrap()), None, &key()).await;
    assert_eq!(st, StatusCode::OK);
    let c = credits(&app, &w).await;
    assert_eq!((c[0]["worked_date"].as_str(), c[0]["fraction"].as_str()), (Some(d.to_string().as_str()), Some("1.00")), "{c:?}");

    let (st, b) = common::as_user(&app, &w.token, "POST", "/v1/comp-off/choices", Some(json!({ "worked_date": today() - Duration::days(3), "choice": "pay" })), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("RULE_GROUP_DISABLED")), "{b}");
}

/// J3: an automatic approval of comp-off leave debits the credits, and escalates when they fall short.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn timeout_approval_debits_comp_off(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, Some(json!({ "HW-02": false }))).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    work_off_day(&app, &s, &w, today() - Duration::days(5), 10).await;
    assert_eq!(balance(&app, &w).await["balance"], "1.00");
    let v = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": false, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 1, "timeout_action": "auto_approve", "during_notice": "allow", "combine_allowed": true });
    let (st, t) = common::as_user(&app, &s.hr_token, "POST", "/v1/leave/types", Some(json!({ "code": "COMP_OFF", "name": "Comp-off", "version": v })), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    let t: Uuid = t["id"].as_str().unwrap().parse().unwrap();
    let mut leaves = vec![];
    for (day, age) in [(today() - Duration::days(2), 4), (today() - Duration::days(1), 3)] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO leave.leave_request (org_id, id, employee_id, leave_type_id, from_date, to_date, state, requested_by, submitted_at)
                     VALUES ($1, $2, $3, $4, $5, $5, 'pending_l1', $6, now() - make_interval(days => $7))")
            .bind(s.org_id).bind(id).bind(w.employee_id).bind(t).bind(day).bind(w.user_id).bind(age).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO leave.leave_request_day (org_id, leave_request_id, leave_date, fraction) VALUES ($1, $2, $3, 1.0)")
            .bind(s.org_id).bind(id).bind(day).execute(&pool).await.unwrap();
        leaves.push(id);
    }
    let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
    let r = domain_attendance::jobs::approval_timeouts(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!((r["leave"]["auto_approved"].as_i64(), r["leave"]["escalated"].as_i64()), (Some(1), Some(1)), "{r}");
    assert_eq!(balance(&app, &w).await["balance"], "0.00", "the unpaid type still debits comp-off");
    let states: Vec<String> = sqlx::query_scalar("SELECT state FROM leave.leave_request WHERE id = ANY($1) ORDER BY from_date").bind(&leaves).fetch_all(&pool).await.unwrap();
    assert_eq!(states, vec!["approved".to_string(), "pending_l1".to_string()]);
}
