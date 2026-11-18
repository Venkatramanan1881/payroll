mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hr_records_manual_punches_once(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let a = common::seed_worker(&pool, &app, &s, "Asha").await;
    let b = common::seed_worker(&pool, &app, &s, "Ravi").await;
    let body = json!({ "branch_id": s.branch_id, "reason": "Device down all morning", "entries": [
        { "employee_id": a.employee_id, "event_at": "2026-03-02T03:30:00Z" },
        { "employee_id": b.employee_id, "event_at": "2026-03-02T03:35:00Z" },
    ]});
    let key = Uuid::new_v4().to_string();
    let (st, r) = common::as_user(&app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body.clone()), &[("idempotency-key", key.clone())]).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");
    let p = &r["punches"][0];
    assert_eq!((p["source"].as_str(), p["is_manual"].as_bool(), p["manual_reason"].as_str()), (Some("manual"), Some(true), Some("Device down all morning")));
    assert_eq!(p["manual_entered_by"], json!(s.hr_user_id));
    assert_eq!(p["direction_tag"], serde_json::Value::Null, "direction is never assigned (BRX-01)");
    let (st, again) = common::as_user(&app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body), &[("idempotency-key", key)]).await;
    assert_eq!((st, &again), (StatusCode::CREATED, &r));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch WHERE source = 'manual'").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 2);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manual_refuses_blank_reason_and_unknown_people_whole_request(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let a = common::seed_worker(&pool, &app, &s, "Asha").await;
    let entry = json!({ "employee_id": a.employee_id, "event_at": "2026-03-02T03:30:00Z" });
    for body in [
        json!({ "branch_id": s.branch_id, "reason": " ", "entries": [entry] }),
        json!({ "branch_id": s.branch_id, "reason": "x", "entries": [entry, { "employee_id": Uuid::new_v4(), "event_at": "2026-03-02T03:30:00Z" }] }),
        json!({ "branch_id": s.branch_id, "reason": "x", "entries": [{ "employee_id": a.employee_id, "event_at": "2025-03-02T03:30:00Z" }] }),
    ] {
        let (st, r) = common::as_user(&app, &s.hr_token, "POST", "/v1/manual-attendance", Some(body.clone()), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "{body} -> {r}");
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM timecap.raw_punch").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0, "nothing stored from a refused request");
    let (st, _) = common::as_user(&app, &a.token, "POST", "/v1/manual-attendance", Some(json!({ "branch_id": s.branch_id, "reason": "x", "entries": [entry] })), &[("idempotency-key", Uuid::new_v4().to_string())]).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "employees have no TCP-04");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn refused_attempts_visible_to_self_and_hr(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let a = common::seed_worker(&pool, &app, &s, "Asha").await;
    let b = common::seed_worker(&pool, &app, &s, "Ravi").await;
    for w in [&a, &b] {
        let body = json!({ "idempotency_key": Uuid::new_v4(), "source": "web", "client_time": chrono::Utc::now(), "captured_tz": "Asia/Kolkata" });
        let (st, _) = common::as_user(&app, &w.token, "POST", "/v1/punches", Some(body), &[]).await;
        assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    }
    let (_, own) = common::as_user(&app, &a.token, "GET", "/v1/punch-attempts", None, &[]).await;
    assert_eq!(own["items"].as_array().unwrap().len(), 1, "{own}");
    assert_eq!((own["items"][0]["employee_id"].clone(), own["items"][0]["rule_ref"].as_str()), (json!(a.employee_id), Some("PC-13")));
    let (_, all) = common::as_user(&app, &s.hr_token, "GET", "/v1/punch-attempts", None, &[]).await;
    assert_eq!(all["items"].as_array().unwrap().len(), 2);
    let (_, own_punches) = common::as_user(&app, &b.token, "GET", &format!("/v1/punches?employee_id={}", a.employee_id), None, &[]).await;
    assert_eq!(own_punches["items"], json!([]), "another employee's punches are outside self scope");
}
