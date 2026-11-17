//! Review Focus 1 and 2: another organisation's employee, or one outside the caller's branch scope, is always 404.

mod common;

/// What the owner gets for the four requests in `statuses`: reads succeed, writes fail only on the fake ETag.
const OWNER: [axum::http::StatusCode; 4] = [
    axum::http::StatusCode::OK,
    axum::http::StatusCode::PRECONDITION_FAILED,
    axum::http::StatusCode::OK,
    axum::http::StatusCode::PRECONDITION_FAILED,
];

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn statuses(app: &axum::Router, token: &str, id: &str) -> [StatusCode; 4] {
    let key = || ("idempotency-key", Uuid::new_v4().to_string());
    let tag = || ("if-match", "W/\"0\"".to_string());
    let change = json!({ "kind": "department", "value_id": Uuid::new_v4(), "valid_from": "2026-02-01", "reason": "x" });
    let reqs = [
        common::req("GET", &format!("/v1/employees/{id}"), token, None, &[]),
        common::req("PATCH", &format!("/v1/employees/{id}"), token, Some(json!({ "full_name": "X" })), &[tag()]),
        common::req("GET", &format!("/v1/employees/{id}/assignments"), token, None, &[]),
        common::req("POST", &format!("/v1/employees/{id}/change-assignment"), token, Some(change), &[key(), tag()]),
    ];
    let mut out = [StatusCode::OK; 4];
    for (i, r) in reqs.into_iter().enumerate() {
        out[i] = common::send(app, r).await.0.status();
    }
    out
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn other_org_employee_404(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    let app = common::app(pool);
    let id = common::create_employee(&app, &a, "Asha").await["id"].as_str().unwrap().to_string();
    assert_eq!(statuses(&app, &a.hr_token, &id).await, OWNER, "control: the routes exist and answer the owner");
    assert_eq!(statuses(&app, &b.hr_token, &id).await, [StatusCode::NOT_FOUND; 4]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hr_scoped_to_branch_x_gets_404_for_branch_y(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let id = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assert_eq!(statuses(&app, &s.hr_token, &id).await, OWNER, "control: in scope before narrowing");
    // Narrow the HR Manager to another branch only.
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')").bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)")
        .bind(s.org_id).bind(s.hr_user_id).bind(other).execute(&pool).await.unwrap();
    assert_eq!(statuses(&app, &s.hr_token, &id).await, [StatusCode::NOT_FOUND; 4]);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn employee_reads_own_record_only(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let me = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let other = common::create_employee(&app, &s, "Bala").await["id"].as_str().unwrap().to_string();
    // An employee user linked to "me", with the BRD §5 self-only view grant.
    let (user, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, mobile, employee_id) VALUES ($1, $2, 'mobile_otp', '+919800000009', $3)")
        .bind(s.org_id).bind(user).bind(me.parse::<Uuid>().unwrap()).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name, system_code) VALUES ($1, $2, 'Employee', 'employee')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'WRK-08', 'self')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(user).bind(role).execute(&pool).await.unwrap();
    let token = common::token(user, s.org_id);
    let get = |id: &str| common::req("GET", &format!("/v1/employees/{id}"), &token, None, &[]);
    assert_eq!(common::send(&app, get(&me)).await.0.status(), StatusCode::OK);
    assert_eq!(common::send(&app, get(&other)).await.0.status(), StatusCode::NOT_FOUND);
}
