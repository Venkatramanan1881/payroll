//! Regression tests for the final-review findings (one per finding).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn get(app: &axum::Router, token: &str, id: &str) -> (StatusCode, String, Value) {
    let (res, body) = common::send(app, common::req("GET", &format!("/v1/employees/{id}"), token, None, &[])).await;
    let etag = res.headers().get("etag").map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
    (res.status(), etag, body)
}

async fn department(pool: &PgPool, org: Uuid) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO org.list_entry (org_id, id, kind, name) VALUES ($1, $2, 'department', $3)")
        .bind(org).bind(id).bind(id.to_string()).execute(pool).await.unwrap();
    id
}

fn change_req(s: &common::SeedOrg, id: &str, key: Uuid, etag: &str, body: &Value) -> axum::http::Request<axum::body::Body> {
    common::req("POST", &format!("/v1/employees/{id}/change-assignment"), &s.hr_token, Some(body.clone()),
        &[("idempotency-key", key.to_string()), ("if-match", etag.to_string())])
}

/// Important 1: an employee who joins in the future stays visible (current = assignments on the joining date).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn future_joiner_is_visible(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let body = json!({ "full_name": "Future", "joining_date": "2099-01-01", "employment_status": "on_probation", "master_branch_id": s.branch_id });
    let (res, created) = common::send(&app, common::req("POST", "/v1/employees", &s.hr_token, Some(body), &[("idempotency-key", Uuid::new_v4().to_string())])).await;
    assert_eq!(res.status(), StatusCode::CREATED);
    assert_eq!(created["current"]["master_branch_id"], s.branch_id.to_string());
    let (st, _, got) = get(&app, &s.hr_token, created["id"].as_str().unwrap()).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(got["current"]["master_branch_id"], s.branch_id.to_string());
}

/// Important 2: two PATCHes holding the same ETag — exactly one wins, the other gets 412.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn concurrent_patches_one_wins(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (_, etag, _) = get(&app, &s.hr_token, &id).await;
    // Hold the row so both requests are in flight against the same version before either can write.
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT 1 FROM workforce.employee WHERE id = $1::uuid FOR UPDATE").bind(&id).execute(&mut *blocker).await.unwrap();
    let spawn = |name: &'static str| {
        let (app, req) = (app.clone(), common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token, Some(json!({ "full_name": name })), &[("if-match", etag.clone())]));
        tokio::spawn(async move { common::send(&app, req).await.0.status() })
    };
    let (a, b) = (spawn("One"), spawn("Two"));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    blocker.commit().await.unwrap();
    let mut st = [a.await.unwrap(), b.await.unwrap()];
    st.sort();
    assert_eq!(st, [StatusCode::OK, StatusCode::PRECONDITION_FAILED]);
}

/// Important 3: the same Idempotency-Key with the same body on another employee must not replay the first answer.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn idempotency_key_bound_to_path(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let d = department(&pool, s.org_id).await;
    let app = common::app(pool);
    let a = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let b = common::create_employee(&app, &s, "Bala").await["id"].as_str().unwrap().to_string();
    let body = json!({ "kind": "department", "value_id": d, "valid_from": "2026-02-01", "reason": "x" });
    let key = Uuid::new_v4();
    let (_, ea, _) = get(&app, &s.hr_token, &a).await;
    assert_eq!(common::send(&app, change_req(&s, &a, key, &ea, &body)).await.0.status(), StatusCode::OK);
    let (_, eb, _) = get(&app, &s.hr_token, &b).await;
    let (res, problem) = common::send(&app, change_req(&s, &b, key, &eb, &body)).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert_eq!(problem["code"], "IDEMPOTENCY_CONFLICT");
}

/// Important 4: repeating an applied change (same value, same date) is a 200 no-op (workforce.yaml).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repeated_change_is_noop(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let d = department(&pool, s.org_id).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let body = json!({ "kind": "department", "value_id": d, "valid_from": "2026-02-01", "reason": "x" });
    let (_, e1, _) = get(&app, &s.hr_token, &id).await;
    let (_, first) = common::send(&app, change_req(&s, &id, Uuid::new_v4(), &e1, &body)).await;
    let (_, e2, _) = get(&app, &s.hr_token, &id).await;
    let (res, again) = common::send(&app, change_req(&s, &id, Uuid::new_v4(), &e2, &body)).await;
    assert_eq!(res.status(), StatusCode::OK, "{again}");
    assert_eq!(again["assignment"]["id"], first["assignment"]["id"]);
    let (_, e3, _) = get(&app, &s.hr_token, &id).await;
    assert_eq!(e2, e3, "a no-op must not change the employee");
}

/// Important 5: badly typed PATCH fields are 422, never 500.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn patch_bad_types_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    for body in [json!({ "date_of_birth": "abc" }), json!({ "email": 5 }), json!({ "address": { "x": 1 } }), json!({ "emergency_contact": "text" }), json!({ "full_name": "   " })] {
        let (_, etag, _) = get(&app, &s.hr_token, &id).await;
        let (res, _) = common::send(&app, common::req("PATCH", &format!("/v1/employees/{id}"), &s.hr_token, Some(body.clone()), &[("if-match", etag)])).await;
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    }
}

/// Minor 1, raised to Important: a suspended organisation's user can still sign out (revoke the session).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn suspended_org_can_sign_out(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    common::seed_mobile_user(&pool, s.org_id, "+919800000001").await;
    let app = common::app(pool.clone());
    let (_, tokens) = common::send(&app, common::post_json("/v1/auth/firebase/sign-in", json!({ "firebase_id_token": "phone:+919800000001" }))).await;
    sqlx::query("UPDATE platform.organisation SET state = 'suspended' WHERE id = $1").bind(s.org_id).execute(&pool).await.unwrap();
    let (res, _) = common::send(&app, common::req("POST", "/v1/auth/sign-out", tokens["access_token"].as_str().unwrap(), None, &[])).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
}
