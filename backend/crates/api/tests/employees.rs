mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn create_req(token: &str, key: Uuid, body: &Value) -> Request<Body> {
    Request::post("/v1/employees")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .header("idempotency-key", key.to_string())
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn new_employee(branch: Uuid, name: &str) -> Value {
    json!({ "full_name": name, "joining_date": "2026-11-01", "employment_status": "active", "master_branch_id": branch })
}

async fn create(app: &axum::Router, s: &common::SeedOrg, body: &Value) -> (StatusCode, Value) {
    let (res, b) = common::send(app, create_req(&s.hr_token, Uuid::new_v4(), body)).await;
    (res.status(), b)
}

async fn set_limit(pool: &PgPool, org: Uuid, max: i32, on_reach: &str) {
    let vendor = Uuid::new_v4();
    sqlx::query("INSERT INTO platform.vendor_user (id, email, full_name, vendor_role) VALUES ($1, $2, 'V', 'full_admin')")
        .bind(vendor).bind(format!("{vendor}@test.invalid")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO platform.employee_limit (org_id, max_active, on_reach, valid, set_by) VALUES ($1, $2, $3, daterange('2000-01-01', NULL), $4)")
        .bind(org).bind(max).bind(on_reach).bind(vendor).execute(pool).await.unwrap();
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_201_with_auto_code(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (st, first) = create(&app, &s, &new_employee(s.branch_id, "Asha")).await;
    assert_eq!(st, StatusCode::CREATED, "{first}");
    assert_eq!(first["employee_code"], "HQ-0001");
    assert_eq!(first["current"]["master_branch_id"], s.branch_id.to_string());
    assert_eq!(first["employment_status"], "active");
    let (_, second) = create(&app, &s, &new_employee(s.branch_id, "Bala")).await;
    assert_eq!(second["employee_code"], "HQ-0002");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_code_rules_follow_em01(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let mut with_code = new_employee(s.branch_id, "Asha");
    with_code["employee_code"] = json!("E-1");
    let (st, _) = create(&app, &s, &with_code).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "code is refused when EM-01 = automatic");

    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2000-01-01', '{\"EM-01\":\"manual\"}', 1)")
        .bind(s.org_id).execute(&pool).await.unwrap();
    let (st, _) = create(&app, &s, &new_employee(s.branch_id, "Bala")).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "code is required when EM-01 = manual");
    let (st, body) = create(&app, &s, &with_code).await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(body["employee_code"], "E-1");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_same_idempotency_key_once(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let key = Uuid::new_v4();
    let body = new_employee(s.branch_id, "Asha");
    let (r1, b1) = common::send(&app, create_req(&s.hr_token, key, &body)).await;
    let (r2, b2) = common::send(&app, create_req(&s.hr_token, key, &body)).await;
    assert_eq!((r1.status(), r2.status()), (StatusCode::CREATED, StatusCode::CREATED));
    assert_eq!(b1, b2);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workforce.employee").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_same_key_other_body_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let key = Uuid::new_v4();
    common::send(&app, create_req(&s.hr_token, key, &new_employee(s.branch_id, "Asha"))).await;
    let (res, body) = common::send(&app, create_req(&s.hr_token, key, &new_employee(s.branch_id, "Other"))).await;
    assert_eq!(res.status(), StatusCode::CONFLICT);
    assert_eq!(body["code"], "IDEMPOTENCY_CONFLICT");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_without_idempotency_key_428(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let req = Request::post("/v1/employees")
        .header("authorization", format!("Bearer {}", s.hr_token))
        .header("content-type", "application/json")
        .body(Body::from(new_employee(s.branch_id, "Asha").to_string()))
        .unwrap();
    let (res, body) = common::send(&app, req).await;
    assert_eq!(res.status(), StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(body["code"], "PRECONDITION_REQUIRED");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn limit_block_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    set_limit(&pool, s.org_id, 1, "block").await;
    let app = common::app(pool);
    assert_eq!(create(&app, &s, &new_employee(s.branch_id, "Asha")).await.0, StatusCode::CREATED);
    let (st, body) = create(&app, &s, &new_employee(s.branch_id, "Bala")).await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(body["code"], "EMPLOYEE_LIMIT_REACHED");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn limit_allow_and_alert_creates_and_alerts(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    set_limit(&pool, s.org_id, 1, "allow_and_alert").await;
    let app = common::app(pool.clone());
    create(&app, &s, &new_employee(s.branch_id, "Asha")).await;
    assert_eq!(create(&app, &s, &new_employee(s.branch_id, "Bala")).await.0, StatusCode::CREATED);
    let alerts: i64 = sqlx::query_scalar("SELECT count(*) FROM app.outbox WHERE event_type = 'EmployeeLimitExceeded'").fetch_one(&pool).await.unwrap();
    assert_eq!(alerts, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn limit_race_never_exceeds(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    set_limit(&pool, s.org_id, 1, "block").await;
    let app = common::app(pool.clone());
    let (asha, bala) = (new_employee(s.branch_id, "Asha"), new_employee(s.branch_id, "Bala"));
    let (a, b) = tokio::join!(create(&app, &s, &asha), create(&app, &s, &bala));
    let mut statuses = [a.0, b.0];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workforce.employee").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_branch_out_of_scope_403(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')")
        .bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)")
        .bind(s.org_id).bind(s.hr_user_id).bind(s.branch_id).execute(&pool).await.unwrap();
    let app = common::app(pool);
    assert_eq!(create(&app, &s, &new_employee(other, "Asha")).await.0, StatusCode::FORBIDDEN);
    assert_eq!(create(&app, &s, &new_employee(s.branch_id, "Bala")).await.0, StatusCode::CREATED);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_unknown_branch_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    assert_eq!(create(&app, &s, &new_employee(Uuid::new_v4(), "Asha")).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_writes_audit_and_outbox(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, body) = create(&app, &s, &new_employee(s.branch_id, "Asha")).await;
    let id: Uuid = body["id"].as_str().unwrap().parse().unwrap();
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE action = 'employee.create' AND target_id = $1 AND actor_id = $2")
        .bind(id).bind(s.hr_user_id).fetch_one(&pool).await.unwrap();
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM app.outbox WHERE event_type = 'EmployeeCreated' AND aggregate_id = $1")
        .bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!((audits, events), (1, 1));
}
