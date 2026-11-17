#![allow(dead_code)] // each test file uses a different subset

use axum::body::Body;
use axum::http::{Request, Response};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &[u8] = b"test-secret-not-for-production";

/// A request with a bearer token and optional JSON body / extra headers.
pub fn req(method: &str, uri: &str, token: &str, body: Option<serde_json::Value>, headers: &[(&str, String)]) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(uri).header("authorization", format!("Bearer {token}"));
    for (k, v) in headers {
        b = b.header(*k, v);
    }
    match body {
        Some(json) => b.header("content-type", "application/json").body(Body::from(json.to_string())).unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

/// Creates an employee in the seed organisation's HQ branch through the API; returns its JSON.
pub async fn create_employee(app: &axum::Router, s: &SeedOrg, name: &str) -> serde_json::Value {
    let body = serde_json::json!({ "full_name": name, "joining_date": "2026-01-01", "employment_status": "active", "master_branch_id": s.branch_id });
    let (res, json) = send(app, req("POST", "/v1/employees", &s.hr_token, Some(body), &[("idempotency-key", Uuid::new_v4().to_string())])).await;
    assert_eq!(res.status(), axum::http::StatusCode::CREATED, "{json}");
    json
}

/// Test verifier: a token "phone:<E.164>" verifies to that number; anything else is rejected.
pub struct FakeVerifier;

#[async_trait::async_trait]
impl domain_iam::firebase::PhoneTokenVerifier for FakeVerifier {
    async fn verify(&self, id_token: &str) -> Result<String, domain_iam::tokens::TokenError> {
        id_token.strip_prefix("phone:").map(str::to_string).ok_or(domain_iam::tokens::TokenError)
    }
}

pub fn state(pool: PgPool) -> api::AppState {
    api::AppState { pool, jwt_secret: JWT_SECRET.to_vec(), phone_verifier: std::sync::Arc::new(FakeVerifier) }
}

pub fn app(pool: PgPool) -> axum::Router {
    api::router(state(pool))
}

pub fn post_json(uri: &str, body: serde_json::Value) -> Request<Body> {
    Request::post(uri).header("content-type", "application/json").body(Body::from(body.to_string())).unwrap()
}

/// An employee-style user who signs in with a mobile number (Firebase).
pub async fn seed_mobile_user(pool: &PgPool, org_id: Uuid, mobile: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, mobile) VALUES ($1, $2, 'mobile_otp', $3)")
        .bind(org_id).bind(id).bind(mobile).execute(pool).await.unwrap();
    id
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

/// One organisation, one branch "HQ", and an HR Manager with the slice-1/2 grants (WRK, SCH, POL) scoped to all branches.
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
    for op in ["WRK-01", "WRK-02", "WRK-03", "WRK-08", "SCH-01", "SCH-02", "SCH-03", "SCH-04", "SCH-05", "POL-01", "POL-02", "POL-04", "POL-06", "POL-07", "TCP-11", "CMP-01"] {
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
    domain_iam::tokens::issue_access(JWT_SECRET, user_id, org_id, None, now())
}

pub fn bearer(token: &str) -> axum::http::HeaderMap {
    let mut h = axum::http::HeaderMap::new();
    h.insert("authorization", format!("Bearer {token}").parse().unwrap());
    h
}

/// POST/PUT/PATCH helper for HR with a fresh Idempotency-Key (and optional If-Match).
pub async fn call(app: &axum::Router, s: &SeedOrg, method: &str, uri: &str, body: Option<serde_json::Value>, if_match: Option<&str>) -> (axum::http::StatusCode, serde_json::Value, Option<String>) {
    let mut headers = vec![("idempotency-key", Uuid::new_v4().to_string())];
    if let Some(t) = if_match {
        headers.push(("if-match", t.to_string()));
    }
    let (res, json) = send(app, req(method, uri, &s.hr_token, body, &headers)).await;
    let etag = res.headers().get("etag").map(|v| v.to_str().unwrap().to_string());
    (res.status(), json, etag)
}
