#![allow(dead_code)] // each test file uses a different subset

use axum::body::Body;
use axum::http::{Request, Response};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &[u8] = b"test-secret-not-for-production";

pub fn app(pool: PgPool) -> axum::Router {
    api::router(api::AppState { pool, jwt_secret: JWT_SECRET.to_vec() })
}

pub async fn send(app: &axum::Router, req: Request<Body>) -> (Response<Body>, serde_json::Value) {
    let res = app.clone().oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    let bytes = body.collect().await.unwrap().to_bytes();
    let json = if bytes.is_empty() { serde_json::Value::Null } else { serde_json::from_slice(&bytes).unwrap() };
    (Response::from_parts(parts, Body::empty()), json)
}

pub struct SeedOrg {
    pub org_id: Uuid,
    pub branch_id: Uuid,
    pub hr_user_id: Uuid,
    pub hr_token: String,
}

/// One organisation, one branch "HQ", and an HR Manager with WRK-01/02/03/08 scoped to all branches.
/// Inserted as the superuser (the test pool), the way vendor set-up would.
pub async fn seed_org(pool: &PgPool) -> SeedOrg {
    let org_id = Uuid::new_v4();
    let branch_id = Uuid::new_v4();
    let hr_user_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    sqlx::query("INSERT INTO platform.organisation (id, legal_name, display_name) VALUES ($1, 'Org', 'Org')")
        .bind(org_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'HQ', 'Head office')")
        .bind(org_id).bind(branch_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'hr@test.invalid', 'x')")
        .bind(org_id).bind(hr_user_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name, system_code) VALUES ($1, $2, 'HR Manager', 'hr_manager')")
        .bind(org_id).bind(role_id).execute(pool).await.unwrap();
    for op in ["WRK-01", "WRK-02", "WRK-03", "WRK-08"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'scoped')")
            .bind(org_id).bind(role_id).bind(op).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)")
        .bind(org_id).bind(hr_user_id).bind(role_id).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')")
        .bind(org_id).bind(hr_user_id).execute(pool).await.unwrap();
    let hr_token = token(hr_user_id, org_id);
    SeedOrg { org_id, branch_id, hr_user_id, hr_token }
}

pub fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

pub fn token(user_id: Uuid, org_id: Uuid) -> String {
    domain_iam::tokens::issue_access(JWT_SECRET, user_id, org_id, now())
}

pub fn bearer(token: &str) -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert("authorization", format!("Bearer {token}").parse().unwrap());
    h
}
