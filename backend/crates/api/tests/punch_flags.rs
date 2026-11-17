mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const LAT: f64 = 12.9716;
const LON: f64 = 77.5946;

async fn setup(pool: &PgPool) -> (common::SeedOrg, axum::Router, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    common::fence(pool, s.org_id, s.branch_id, LAT, LON, 200).await;
    (s, app, w)
}

/// The worker punches 450 m from the fence centre; returns the outside_fence flag id.
async fn flagged_punch(app: &axum::Router, w: &common::Worker) -> String {
    let b = json!({ "idempotency_key": Uuid::new_v4(), "source": "app", "client_time": chrono::Utc::now(), "captured_tz": "Asia/Kolkata",
                    "location": { "latitude": LAT + 450.0 / 111_195.0, "longitude": LON, "accuracy_m": 10 } });
    let (st, r) = common::as_user(app, &w.token, "POST", "/v1/punches", Some(b), &[]).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    r["punch"]["flags"][0]["id"].as_str().unwrap().to_string()
}

async fn decide(app: &axum::Router, token: &str, flag: &str, action: &str, body: Option<Value>) -> (StatusCode, Value) {
    common::as_user(app, token, "POST", &format!("/v1/punch-flags/{flag}/{action}"), body, &[("idempotency-key", Uuid::new_v4().to_string())]).await
}

async fn grant(pool: &PgPool, org: Uuid, user: Uuid, op: &str, kind: &str) {
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, $3)").bind(org).bind(role).bind(format!("r-{role}")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, $4)").bind(org).bind(role).bind(op).bind(kind).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hr_lists_and_accepts_once(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let flag = flagged_punch(&app, &w).await;
    let (st, list) = common::as_user(&app, &s.hr_token, "GET", "/v1/punch-flags", None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{list}");
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert_eq!(list["items"][0]["reason"], "outside_fence");
    assert_eq!(list["items"][0]["punch"]["location"]["fence_distance_m"], 450, "HR holds TCP-08");

    let (st, r) = decide(&app, &s.hr_token, &flag, "accept", None).await;
    assert_eq!(st, StatusCode::OK, "{r}");
    assert_eq!((r["state"].as_str(), r["decision"]["decision"].as_str()), (Some("accepted"), Some("accepted")));
    let (st, again) = decide(&app, &s.hr_token, &flag, "accept", Some(json!({ "comment": "again" }))).await;
    assert_eq!((st, again["state"].as_str()), (StatusCode::OK, Some("accepted")));
    let (st, r) = decide(&app, &s.hr_token, &flag, "reject", Some(json!({ "reason": "changed my mind" }))).await;
    assert_eq!((st, r["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")));

    let decisions: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.punch_flag_decision").fetch_one(&pool).await.unwrap();
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM app.outbox WHERE event_type = 'PunchFlagDecided'").fetch_one(&pool).await.unwrap();
    assert_eq!((decisions, events), (1, 1));
    let (_, open) = common::as_user(&app, &s.hr_token, "GET", "/v1/punch-flags", None, &[]).await;
    assert_eq!(open["items"], json!([]), "default list shows raised flags only");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reject_needs_a_reason_and_key(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let flag = flagged_punch(&app, &w).await;
    let (st, _) = decide(&app, &s.hr_token, &flag, "reject", Some(json!({ "reason": "  " }))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/punch-flags/{flag}/reject"), Some(json!({ "reason": "x" })), &[]).await;
    assert_eq!(st, StatusCode::PRECONDITION_REQUIRED, "Idempotency-Key is required");
    let (st, r) = decide(&app, &s.hr_token, &flag, "reject", Some(json!({ "reason": "not at site" }))).await;
    assert_eq!((st, r["state"].as_str(), r["decision"]["reason"].as_str()), (StatusCode::OK, Some("rejected"), Some("not at site")));
    let punches: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(punches, 1, "a rejected punch stays as evidence");
}

/// Review focus 3 (BR-07).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn nobody_decides_their_own_punch(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    grant(&pool, s.org_id, w.user_id, "TCP-06", "scoped").await;
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(s.org_id).bind(w.user_id).execute(&pool).await.unwrap();
    let flag = flagged_punch(&app, &w).await;
    let (st, r) = decide(&app, &w.token, &flag, "accept", None).await;
    assert_eq!((st, r["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "{r}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_decides_only_for_their_team(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let other = common::seed_worker(&pool, &app, &s, "Ravi").await;
    common::day_shift(&app, &s, other.employee_id).await;
    let boss = common::seed_worker(&pool, &app, &s, "Meena").await;
    grant(&pool, s.org_id, boss.user_id, "TCP-06", "approve").await;
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, manager_employee_id, valid) VALUES ($1, $2, 'reporting_manager', $3, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(w.employee_id).bind(boss.employee_id).execute(&pool).await.unwrap();
    let mine = flagged_punch(&app, &w).await;
    let not_mine = flagged_punch(&app, &other).await;
    let (_, list) = common::as_user(&app, &boss.token, "GET", "/v1/punch-flags", None, &[]).await;
    let ids: Vec<&str> = list["items"].as_array().unwrap().iter().map(|f| f["id"].as_str().unwrap()).collect();
    assert_eq!(ids, [mine.as_str()]);
    assert_eq!(list["items"][0]["punch"]["location"], Value::Null, "the manager has no TCP-08");
    assert_eq!(decide(&app, &boss.token, &not_mine, "accept", None).await.0, StatusCode::NOT_FOUND);
    assert_eq!(decide(&app, &boss.token, &mine, "accept", None).await.0, StatusCode::OK);
}
