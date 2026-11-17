mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn get(app: &axum::Router, token: &str, id: &str) -> (StatusCode, Option<String>, Value) {
    let (res, body) = common::send(app, common::req("GET", &format!("/v1/employees/{id}"), token, None, &[])).await;
    let etag = res.headers().get("etag").map(|v| v.to_str().unwrap().to_string());
    (res.status(), etag, body)
}

async fn department(pool: &PgPool, org: Uuid, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO org.list_entry (org_id, id, kind, name) VALUES ($1, $2, 'department', $3)")
        .bind(org).bind(id).bind(name).execute(pool).await.unwrap();
    id
}

fn change(value: Uuid, from: &str) -> Value {
    json!({ "kind": "department", "value_id": value, "valid_from": from, "reason": "reorganisation" })
}

async fn post_change(app: &axum::Router, s: &common::SeedOrg, id: &str, etag: &str, body: Value) -> (StatusCode, Value) {
    let headers = [("idempotency-key", Uuid::new_v4().to_string()), ("if-match", etag.to_string())];
    let (res, b) = common::send(app, common::req("POST", &format!("/v1/employees/{id}/change-assignment"), &s.hr_token, Some(body), &headers)).await;
    (res.status(), b)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_returns_employee_and_etag(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let e = common::create_employee(&app, &s, "Asha").await;
    let (st, etag, body) = get(&app, &s.hr_token, e["id"].as_str().unwrap()).await;
    assert_eq!(st, StatusCode::OK);
    assert!(etag.is_some_and(|t| t.starts_with("W/\"")));
    assert_eq!(body, e);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_updates_with_current_etag(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    let (res, body) = common::send(&app, common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token,
        Some(json!({ "full_name": "Asha K", "mobile": null })), &[("if-match", etag.clone().unwrap())])).await;
    assert_eq!(res.status(), StatusCode::OK, "{body}");
    assert_eq!(body["full_name"], "Asha K");
    assert_ne!(res.headers().get("etag").unwrap().to_str().unwrap(), etag.unwrap());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_stale_etag_412(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    let patch = |name: &str, tag: String| common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token, Some(json!({ "full_name": name })), &[("if-match", tag)]);
    assert_eq!(common::send(&app, patch("One", etag.clone().unwrap())).await.0.status(), StatusCode::OK);
    let (res, body) = common::send(&app, patch("Two", etag.unwrap())).await;
    assert_eq!(res.status(), StatusCode::PRECONDITION_FAILED);
    assert_eq!(body["code"], "PRECONDITION_FAILED");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_missing_if_match_428(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (res, _) = common::send(&app, common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token, Some(json!({ "full_name": "X" })), &[])).await;
    assert_eq!(res.status(), StatusCode::PRECONDITION_REQUIRED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_unknown_or_empty_body_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    for body in [json!({ "salary": 1 }), json!({}), json!({ "full_name": null }), json!({ "custom_fields": {} })] {
        let (_, etag, _) = get(&app, &s.hr_token, &id).await;
        let (res, _) = common::send(&app, common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token, Some(body.clone()), &[("if-match", etag.unwrap())])).await;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn change_department_closes_old_range(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let (d1, d2) = (department(&pool, s.org_id, "Sales").await, department(&pool, s.org_id, "Stores").await);
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    let (st, b) = post_change(&app, &s, &id, &etag.unwrap(), change(d1, "2026-02-01")).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert_eq!(b["adjustment_posted"], false);
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    assert_eq!(post_change(&app, &s, &id, &etag.unwrap(), change(d2, "2026-03-01")).await.0, StatusCode::OK);
    let (_, list) = common::send(&app, common::req("GET", &format!("/v1/employees/{id}/assignments?kind=department"), &s.hr_token, None, &[])).await;
    let items = list["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!((items[0]["valid_from"].as_str(), items[0]["valid_to"].as_str()), (Some("2026-02-01"), Some("2026-03-01")));
    assert_eq!((items[1]["valid_from"].as_str(), items[1]["valid_to"].clone()), (Some("2026-03-01"), Value::Null));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn change_with_stale_etag_412(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let d1 = department(&pool, s.org_id, "Sales").await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    assert_eq!(post_change(&app, &s, &id, etag.as_ref().unwrap(), change(d1, "2026-02-01")).await.0, StatusCode::OK);
    assert_eq!(post_change(&app, &s, &id, etag.as_ref().unwrap(), change(d1, "2026-04-01")).await.0, StatusCode::PRECONDITION_FAILED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn change_backdated_into_lock_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let d1 = department(&pool, s.org_id, "Sales").await;
    let app = common::app(pool.clone());
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let emp: Uuid = id.parse().unwrap();
    let (cycle, period) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO period.attendance_cycle (org_id, id, owner_kind, owner_id, valid) VALUES ($1, $2, 'organisation', $1, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.attendance_period (org_id, id, cycle_id, period_start, period_end) VALUES ($1, $2, $3, '2026-01-01', '2026-01-31')")
        .bind(s.org_id).bind(period).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.lock_coverage (org_id, employee_id, dates, period_id) VALUES ($1, $2, daterange('2026-01-01', '2026-02-01'), $3)")
        .bind(s.org_id).bind(emp).bind(period).execute(&pool).await.unwrap();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    let (st, body) = post_change(&app, &s, &id, &etag.unwrap(), change(d1, "2026-01-15")).await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(body["code"], "PERIOD_LOCKED");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_self_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    let body = json!({ "kind": "reporting_manager", "value_id": id, "valid_from": "2026-02-01", "reason": "x" });
    assert_eq!(post_change(&app, &s, &id, &etag.unwrap(), body).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}
