//! Regularisation requests, approvals and HR corrections (slice 5). Shift 09:00-18:00 India time = 03:30Z-12:30Z.

mod common;

use axum::http::StatusCode;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Yesterday in India (inside the RG-04 window).
fn yesterday() -> NaiveDate {
    (Utc::now() + Duration::minutes(330)).date_naive() - Duration::days(1)
}

fn at(d: NaiveDate, hm: &str) -> DateTime<Utc> {
    format!("{d}T{hm}:00Z").parse().unwrap()
}

/// An organisation policy (from 2026-01-01) with an optional RG document.
async fn org_policy(app: &axum::Router, s: &common::SeedOrg, rg: Option<Value>) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    if let Some(doc) = rg {
        let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
        let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/RG"), Some(json!({ "schema_version": 1, "settings": doc })), etag.as_deref()).await;
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

/// Worker with a day shift, and a manager (ATT-03 approve) they report to.
async fn setup(pool: &PgPool, rg: Option<Value>) -> (common::SeedOrg, axum::Router, common::Worker, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, rg).await;
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let boss = common::seed_worker(pool, &app, &s, "Meena").await;
    grant(pool, s.org_id, boss.user_id, "ATT-03", "approve").await;
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, manager_employee_id, valid) VALUES ($1, $2, 'reporting_manager', $3, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(w.employee_id).bind(boss.employee_id).execute(pool).await.unwrap();
    (s, app, w, boss)
}

async fn manual(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, times: &[DateTime<Utc>]) {
    let entries: Vec<Value> = times.iter().map(|t| json!({ "employee_id": w.employee_id, "event_at": t })).collect();
    let body = json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries });
    let (st, r) = common::as_user(app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
}

async fn post(app: &axum::Router, token: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    common::as_user(app, token, "POST", uri, body, &[("idempotency-key", Uuid::new_v4().to_string())]).await
}

async fn submit(app: &axum::Router, w: &common::Worker, body: Value) -> (StatusCode, Value) {
    post(app, &w.token, "/v1/attendance/regularisation-requests", Some(body)).await
}

async fn day(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, d: NaiveDate) -> Value {
    let (st, b) = common::as_user(app, &s.hr_token, "GET", &format!("/v1/attendance/employees/{}/days/{d}", w.employee_id), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b["current_result"].clone()
}

/// Review focus 1 and 4 (E13).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e13_missed_check_out_approved_by_the_manager(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    manual(&app, &s, &w, &[at(d, "03:30")]).await;
    let (st, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "missed_punch", "proposed_out_at": at(d, "12:35"), "reason": "forgot to punch out" })).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::CREATED, Some("pending_l1")), "{r}");
    assert_eq!(r["approval_steps"][0]["action"], "submitted");
    let id = r["id"].as_str().unwrap().to_string();

    let (_, queue) = common::as_user(&app, &boss.token, "GET", "/v1/attendance/regularisation-requests/pending-approval", None, &[]).await;
    assert_eq!(queue["items"].as_array().unwrap().len(), 1, "{queue}");
    let (_, hr_queue) = common::as_user(&app, &s.hr_token, "GET", "/v1/attendance/regularisation-requests/pending-approval", None, &[]).await;
    assert_eq!(hr_queue["items"], json!([]), "routed to the manager, not HR");

    let (st, r) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), Some(json!({ "comment": "ok" }))).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("approved")), "{r}");
    assert_eq!(r["decided_by_user_id"], json!(boss.user_id));
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "present");
    let (_, again) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!(again["state"], "approved");
    let refs: Vec<Option<Uuid>> = sqlx::query_scalar("SELECT manual_approval_ref FROM timecap.raw_punch WHERE manual_approval_ref IS NOT NULL").fetch_all(&pool).await.unwrap();
    assert_eq!(refs, [Some(id.parse().unwrap())], "one manual punch, carrying the approval (BR-01), applied once");
    let (st, _) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/reject"), Some(json!({ "reason": "no" }))).await;
    assert_eq!(st, StatusCode::CONFLICT);
}

/// Review focus 2 (BR-07), including as the approver's delegate.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn nobody_decides_their_own_request(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    grant(&pool, s.org_id, w.user_id, "ATT-03", "approve").await;
    sqlx::query("INSERT INTO iam.delegation (org_id, approver_user_id, delegate_user_id, valid) VALUES ($1, $2, $3, tstzrange(now() - interval '1 day', NULL))")
        .bind(s.org_id).bind(boss.user_id).bind(w.user_id).execute(&pool).await.unwrap();
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "card forgotten" })).await;
    let id = r["id"].as_str().unwrap();
    let (st, b) = post(&app, &w.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "{b}");
    let (st, _) = post(&app, &s.hr_token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "routed to the manager: HR is not this request's approver");
}

/// Review focus 3: two levels (RG-05).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn two_levels_manager_then_hr(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, Some(json!({ "RG-05": "two" }))).await;
    let d = yesterday();
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "device down" })).await;
    let id = r["id"].as_str().unwrap();
    let (_, r) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!(r["state"], "pending_l2", "{r}");
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0, "nothing applied before the final level");
    let (st, r) = post(&app, &s.hr_token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("approved")), "{r}");
    let levels: Vec<i64> = r["approval_steps"].as_array().unwrap().iter().map(|s| s["level"].as_i64().unwrap()).collect();
    assert_eq!(levels, [1, 1, 2]);
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "present");
}

/// Review focus 5: a late-arrival correction removes the late mark; the original punch stays (BR-01).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn late_arrival_correction_replaces_the_late_in(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    manual(&app, &s, &w, &[at(d, "03:55"), at(d, "12:30")]).await;
    assert_eq!(day(&app, &s, &w, d).await["late_mark"], true);
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "late_arrival", "proposed_in_at": at(d, "03:30"), "reason": "gate queue" })).await;
    let id = r["id"].as_str().unwrap();
    let (st, r) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    let result = day(&app, &s, &w, d).await;
    assert_eq!((result["late_mark"].as_bool(), result["late_minutes"].as_i64()), (Some(false), Some(0)), "{result}");
    assert!(result["punches_used"].as_array().unwrap().iter().any(|u| u["excluded_reason"] == "replaced_by_regularisation"));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 3, "the late punch is kept as evidence");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn submission_rules_window_limit_type_and_cancel(pool: PgPool) {
    let (_s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    let whole = |d: NaiveDate| json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "x" });
    let (st, b) = submit(&app, &w, whole(d - Duration::days(10))).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("OUTSIDE_REQUEST_WINDOW")), "{b}");
    let (st, _) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "late_arrival", "reason": "x" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "late arrival needs the in time");
    let mut ids = vec![];
    for _ in 0..3 {
        let (st, b) = submit(&app, &w, whole(d)).await;
        assert_eq!(st, StatusCode::CREATED, "{b}");
        ids.push(b["id"].as_str().unwrap().to_string());
    }
    let (_, fourth) = submit(&app, &w, whole(d)).await;
    assert_eq!(fourth["state"], "pending_l2", "over RG-03 (3) with RG-08 = HR approval goes to HR: {fourth}");
    let (st, c) = post(&app, &w.token, &format!("/v1/attendance/regularisation-requests/{}/cancel", ids[0]), None).await;
    assert_eq!((st, c["state"].as_str()), (StatusCode::OK, Some("cancelled")));
    assert_eq!(post(&app, &w.token, &format!("/v1/attendance/regularisation-requests/{}/cancel", ids[0]), None).await.0, StatusCode::OK);
    assert_eq!(post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{}/approve", ids[0]), None).await.0, StatusCode::CONFLICT);
    let (st, _) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{}/reject", ids[1]), Some(json!({ "reason": " " }))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "a rejection needs a reason");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hr_correction_and_bulk_outage(pool: PgPool) {
    let (s, app, w, _boss) = setup(&pool, None).await;
    let d = yesterday();
    let body = json!({ "employee_id": w.employee_id, "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "device fault" });
    let (st, r) = post(&app, &s.hr_token, "/v1/attendance/hr-corrections", Some(body)).await;
    assert_eq!((st, r["state"].as_str(), r["origin"].as_str()), (StatusCode::CREATED, Some("approved"), Some("hr_direct")), "{r}");
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "present");

    let other = common::seed_worker(&pool, &app, &s, "Ravi").await;
    common::day_shift(&app, &s, other.employee_id).await;
    let body = json!({ "branch_id": s.branch_id, "attendance_date": d, "correction": "use_shift_times", "reason": "device offline all day" });
    let (st, job) = post(&app, &s.hr_token, "/v1/attendance/bulk-regularisations", Some(body)).await;
    assert_eq!((st, job["state"].as_str()), (StatusCode::ACCEPTED, Some("succeeded")), "{job}");
    assert_eq!(day(&app, &s, &other, d).await["day_status"], "present");
    let corrections: i64 = sqlx::query_scalar("SELECT count(*) FROM attendance.regularisation_request WHERE employee_id = $1").bind(w.employee_id).fetch_one(&pool).await.unwrap();
    assert_eq!(corrections, 1, "a day already complete is not corrected again");
    let (st, list) = common::as_user(&app, &w.token, "GET", "/v1/attendance/regularisation-requests", None, &[]).await;
    assert_eq!((st, list["items"].as_array().unwrap().len()), (StatusCode::OK, 1), "the employee sees their own request");
}

/// Review fix 1: approve/reject never reveals a request to someone who is not its decider.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decided_requests_answer_only_to_their_decider(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "x" })).await;
    let id = r["id"].as_str().unwrap();
    assert_eq!(post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await.0, StatusCode::OK);
    let stranger = common::seed_worker(&pool, &app, &s, "Ravi").await;
    for action in ["approve", "reject"] {
        let (st, b) = post(&app, &stranger.token, &format!("/v1/attendance/regularisation-requests/{id}/{action}"), Some(json!({ "reason": "x" }))).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
    }
    let (st, b) = common::as_user(&app, &stranger.token, "GET", &format!("/v1/attendance/regularisation-requests/{id}"), None, &[]).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "{b}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delegate_decides_and_is_recorded(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let deputy = common::seed_worker(&pool, &app, &s, "Deputy").await;
    grant(&pool, s.org_id, deputy.user_id, "ATT-03", "approve").await;
    sqlx::query("INSERT INTO iam.delegation (org_id, approver_user_id, delegate_user_id, valid) VALUES ($1, $2, $3, tstzrange(now() - interval '1 day', NULL))")
        .bind(s.org_id).bind(boss.user_id).bind(deputy.user_id).execute(&pool).await.unwrap();
    let d = yesterday();
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "x" })).await;
    let id = r["id"].as_str().unwrap();
    let (_, queue) = common::as_user(&app, &deputy.token, "GET", "/v1/attendance/regularisation-requests/pending-approval", None, &[]).await;
    assert_eq!(queue["items"].as_array().unwrap().len(), 1, "{queue}");
    let (st, r) = post(&app, &deputy.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("approved")), "{r}");
    assert_eq!(r["approval_steps"][1]["on_behalf_of_user_id"], json!(boss.user_id));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn auto_approve_block_limit_and_switched_off(pool: PgPool) {
    let (s, app, w, _boss) = setup(&pool, Some(json!({ "RG-06": ["whole_day"], "RG-03": 1, "RG-08": "block", "RG-02": { "wrong_punch": false } }))).await;
    let d = yesterday();
    let whole = json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "x" });
    let (st, r) = submit(&app, &w, whole.clone()).await;
    assert_eq!((st, r["state"].as_str(), r["decided_basis"].as_str()), (StatusCode::CREATED, Some("approved"), Some("auto_on_submit")), "{r}");
    let actions: Vec<&str> = r["approval_steps"].as_array().unwrap().iter().map(|s| s["action"].as_str().unwrap()).collect();
    assert_eq!(actions, ["submitted", "auto_approved"]);
    assert_eq!(day(&app, &s, &w, d).await["day_status"], "present");
    let (st, b) = submit(&app, &w, whole).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("LIMIT_EXCEEDED")), "{b}");
    let (st, b) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "wrong_punch", "target_raw_punch_id": Uuid::new_v4(), "reason": "x" })).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("RULE_GROUP_DISABLED")), "{b}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn wrong_punch_target_must_be_on_the_day_and_whole_day_replaces_strays(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    manual(&app, &s, &w, &[at(d - Duration::days(5), "03:30"), at(d, "03:40")]).await;
    let old: Uuid = sqlx::query_scalar("SELECT id FROM timecap.raw_punch ORDER BY event_at LIMIT 1").fetch_one(&pool).await.unwrap();
    let (st, _) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "wrong_punch", "target_raw_punch_id": old, "reason": "x" })).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "a punch from another day cannot be targeted");

    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "card" })).await;
    let id = r["id"].as_str().unwrap();
    post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    let result = day(&app, &s, &w, d).await;
    assert_eq!((result["day_status"].as_str(), result["late_minutes"].as_i64()), (Some("present"), Some(0)), "{result}");
    assert!(result["punches_used"].as_array().unwrap().iter().any(|u| u["excluded_reason"] == "replaced_by_whole_day_correction"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn one_person_cannot_approve_both_levels_and_over_limit_needs_att05(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s, Some(json!({ "RG-05": "two", "RG-03": 1 }))).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await; // no manager: level 1 goes to the HR pool
    common::day_shift(&app, &s, w.employee_id).await;
    let d = yesterday();
    let whole = json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "x" });
    let (_, r) = submit(&app, &w, whole.clone()).await;
    let id = r["id"].as_str().unwrap();
    let (_, r) = post(&app, &s.hr_token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!(r["state"], "pending_l2", "{r}");
    let (st, b) = post(&app, &s.hr_token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "{b}");

    let (_, over) = submit(&app, &w, whole).await;
    assert_eq!(over["state"], "pending_l2", "{over}");
    let checker = common::seed_worker(&pool, &app, &s, "Checker").await;
    grant(&pool, s.org_id, checker.user_id, "ATT-04", "scoped").await;
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(s.org_id).bind(checker.user_id).execute(&pool).await.unwrap();
    let oid = over["id"].as_str().unwrap();
    assert_eq!(post(&app, &checker.token, &format!("/v1/attendance/regularisation-requests/{oid}/approve"), None).await.0, StatusCode::NOT_FOUND, "over-limit is ATT-05's");
    assert_eq!(post(&app, &checker.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await.0, StatusCode::OK, "a normal second level is ATT-04's");
}

/// NT-05 for regularisation (its steps live in the audit log): the manager hears of the request, the employee of
/// the decision.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn regularisation_notifies(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, None).await;
    let d = yesterday();
    let (st, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "card forgotten" })).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let dispatch = || async {
        let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
        let r = domain_attendance::notify::dispatch(&mut tx, s.org_id).await.unwrap();
        tx.commit().await.unwrap();
        r
    };
    assert_eq!(dispatch().await["sent"], 1);
    let (_, b) = common::as_user(&app, &boss.token, "GET", "/v1/notifications/inbox", None, &[]).await;
    assert_eq!(b["items"][0]["link"]["kind"], "regularisation", "{b}");
    let (st, _) = post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{}/approve", r["id"].as_str().unwrap()), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(dispatch().await["sent"], 1);
    let (_, mine) = common::as_user(&app, &w.token, "GET", "/v1/notifications/inbox", None, &[]).await;
    assert!(mine["items"][0]["body"].as_str().unwrap().contains("regularisation request was approved"), "{mine}");
}

/// A level-1 approval of a two-level regularisation reads as waiting for level 2, even when level 2 has already
/// decided before the relay runs.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn regularisation_level_one_wording_survives_a_late_relay(pool: PgPool) {
    let (s, app, w, boss) = setup(&pool, Some(json!({ "RG-05": "two" }))).await;
    let d = yesterday();
    let (_, r) = submit(&app, &w, json!({ "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at(d, "03:30"), "proposed_out_at": at(d, "12:30"), "reason": "card forgotten" })).await;
    let id = r["id"].as_str().unwrap();
    assert_eq!(post(&app, &boss.token, &format!("/v1/attendance/regularisation-requests/{id}/approve"), None).await.0, StatusCode::OK);
    let (st, b) = post(&app, &s.hr_token, &format!("/v1/attendance/regularisation-requests/{id}/reject"), Some(json!({ "reason": "no evidence" }))).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
    domain_attendance::notify::dispatch(&mut tx, s.org_id).await.unwrap();
    tx.commit().await.unwrap();
    let (_, mine) = common::as_user(&app, &w.token, "GET", "/v1/notifications/inbox", None, &[]).await;
    let bodies: Vec<&str> = mine["items"].as_array().unwrap().iter().filter_map(|i| i["body"].as_str()).collect();
    assert!(bodies.iter().any(|b| b.contains("approved at level 1 and now waits for level 2")), "{bodies:?}");
    assert!(!bodies.iter().any(|b| b.ends_with("request was approved.")), "never reported as finally approved: {bodies:?}");
}
