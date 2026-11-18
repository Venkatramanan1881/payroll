mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

const LAT: f64 = 12.9716;
const LON: f64 = 77.5946;
const M_PER_DEG: f64 = 111_195.0;

fn body(source: &str, metres_north: Option<f64>) -> Value {
    json!({
        "idempotency_key": Uuid::new_v4(),
        "source": source,
        "client_time": chrono::Utc::now(),
        "captured_tz": "Asia/Kolkata",
        "location": metres_north.map(|m| json!({ "latitude": LAT + m / M_PER_DEG, "longitude": LON, "accuracy_m": 10 })),
    })
}

async fn punch(app: &axum::Router, w: &common::Worker, b: &Value) -> (StatusCode, Value) {
    common::as_user(app, &w.token, "POST", "/v1/punches", Some(b.clone()), &[]).await
}

/// Worker with a day shift and the HQ fence (radius 200 m).
async fn setup(pool: &PgPool) -> (common::SeedOrg, axum::Router, common::Worker) {
    let s = common::seed_org(pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    common::fence(pool, s.org_id, s.branch_id, LAT, LON, 200).await;
    (s, app, w)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e18_web_sixty_metres_accepted(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let (st, b) = punch(&app, &w, &body("web", Some(60.0))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["outcome"], "accepted");
    assert_eq!(b["punch"]["flags"], json!([]));
    assert_eq!(b["punch"]["location"], Value::Null, "the employee has no TCP-08");
    assert_eq!(b["punch"]["branch_id"], json!(s.branch_id));
    let (inside, dist): (Option<bool>, Option<i32>) = sqlx::query_as("SELECT inside_fence, fence_distance_m FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!((inside, dist), (Some(true), Some(60)));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e18_web_450_metres_flagged_not_counted(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let (st, b) = punch(&app, &w, &body("web", Some(450.0))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["outcome"], "flagged");
    assert_eq!(b["punch"]["flags"][0]["reason"], "outside_fence");
    assert_eq!(b["punch"]["flags"][0]["state"], "raised");
    assert_eq!(b["punch"]["flags"][0]["counts_before_decision"], false);
    let events: Vec<String> = sqlx::query_scalar("SELECT event_type FROM app.outbox WHERE event_type LIKE 'Punch%' ORDER BY event_type").fetch_all(&pool).await.unwrap();
    assert_eq!(events, ["PunchFlagged", "PunchRecorded"]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e18_web_location_refused_blocked_and_recorded(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let mut b = body("web", None);
    b["location_unavailable_reason"] = json!("permission_denied");
    let (st, r) = punch(&app, &w, &b).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{r}");
    assert_eq!((r["code"].as_str(), r["rule_ref"].as_str()), (Some("PUNCH_REFUSED"), Some("PC-13")));
    assert!(r["detail"].as_str().unwrap().contains("location"), "the employee is told why");
    let attempt: Uuid = r["attempt_id"].as_str().unwrap().parse().unwrap();
    let (rule, emp): (String, Uuid) = sqlx::query_as("SELECT rule_ref, employee_id FROM timecap.punch_attempt_refused WHERE id = $1").bind(attempt).fetch_one(&pool).await.unwrap();
    assert_eq!((rule.as_str(), emp), ("PC-13", w.employee_id));
    let punches: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(punches, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn retry_same_key_returns_200_stored_punch(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let b = body("app", Some(450.0));
    let (st1, first) = punch(&app, &w, &b).await;
    let (st2, again) = punch(&app, &w, &b).await;
    assert_eq!((st1, st2), (StatusCode::CREATED, StatusCode::OK));
    assert_eq!(first, again);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn e17_branch_not_allowed_flagged_and_costed_there(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'C', 'Branch C')").bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    common::fence(&pool, s.org_id, other, LAT + 0.2, LON, 200).await;
    let (st, b) = punch(&app, &w, &body("app", Some(0.2 * M_PER_DEG))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["punch"]["flags"][0]["reason"], "branch_not_allowed");
    assert_eq!(b["punch"]["flags"].as_array().unwrap().len(), 1);
    assert_eq!(b["punch"]["branch_id"], json!(other));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn other_organisations_fences_are_never_considered(pool: PgPool) {
    let (_s, app, w) = setup(&pool).await;
    let s2 = common::seed_org(&pool).await;
    common::fence(&pool, s2.org_id, s2.branch_id, LAT + 0.2, LON, 200).await;
    let (st, b) = punch(&app, &w, &body("app", Some(0.2 * M_PER_DEG))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    assert_eq!(b["punch"]["flags"][0]["reason"], "outside_fence");
    assert_eq!(b["punch"]["branch_id"], Value::Null);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn client_inside_claim_is_refused_and_hr_without_employee_forbidden(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let mut b = body("app", Some(10.0));
    b["inside_fence"] = json!(true);
    let (st, r) = punch(&app, &w, &b).await;
    assert_eq!((st, r["code"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some("VALIDATION_FAILED")));
    let (st, _) = common::as_user(&app, &s.hr_token, "POST", "/v1/punches", Some(body("app", Some(10.0))), &[]).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "HR has no TCP-03 in the seed");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hr_sees_location_with_tcp08(pool: PgPool) {
    let (s, app, w) = setup(&pool).await;
    let (_, b) = punch(&app, &w, &body("app", Some(60.0))).await;
    let id = b["punch"]["id"].as_str().unwrap();
    let (st, hr) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/punches/{id}"), None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{hr}");
    assert_eq!(hr["location"]["fence_distance_m"], 60);
    assert_eq!(hr["location"]["inside_fence"], true);
}
