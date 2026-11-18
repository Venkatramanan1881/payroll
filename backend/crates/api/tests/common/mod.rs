#![allow(dead_code)] // each test file uses a different subset

use axum::body::Body;
use axum::http::{Request, Response};
use http_body_util::BodyExt;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

pub const JWT_SECRET: &[u8] = b"test-secret-not-for-production";
pub const JOB_SECRET: &[u8] = b"test-job-secret-not-for-production-0123456789";

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
    api::AppState { pool, jwt_secret: JWT_SECRET.to_vec(), phone_verifier: std::sync::Arc::new(FakeVerifier), job_secret: Some(JOB_SECRET.to_vec()) }
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
    for op in ["WRK-01", "WRK-02", "WRK-03", "WRK-08", "SCH-01", "SCH-02", "SCH-03", "SCH-04", "SCH-05", "POL-01", "POL-02", "POL-04", "POL-06", "POL-07", "TCP-11", "CMP-01", "TCP-04", "TCP-06", "TCP-07", "TCP-08", "TCP-09", "ATT-01", "ATT-06", "ATT-03", "ATT-04", "ATT-05", "LVE-01", "LVE-03", "LVE-04", "LVE-05", "LVE-06", "LVE-07", "LVE-08", "ORG-01", "ORG-02", "ORG-03", "ORG-04", "ORG-05", "ORG-06", "ORG-07"] {
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

/// An employee of the seed organisation who signs in to the app: TCP-03 (punch), TCP-07 and TCP-09 on own records.
pub struct Worker {
    pub employee_id: Uuid,
    pub user_id: Uuid,
    pub token: String,
}

pub async fn seed_worker(pool: &PgPool, app: &axum::Router, s: &SeedOrg, name: &str) -> Worker {
    let employee_id: Uuid = create_employee(app, s, name).await["id"].as_str().unwrap().parse().unwrap();
    let user_id = Uuid::new_v4();
    let role_id = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.user_account (org_id, id, employee_id, login_kind, mobile) VALUES ($1, $2, $3, 'mobile_otp', $4)")
        .bind(s.org_id).bind(user_id).bind(employee_id).bind(format!("+91{}", 6_000_000_000u64 + (user_id.as_u128() % 1_000_000_000) as u64))
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, $3)").bind(s.org_id).bind(role_id).bind(format!("Employee {role_id}")).execute(pool).await.unwrap();
    for op in ["TCP-03", "TCP-07", "TCP-09", "ATT-01", "ATT-02", "LVE-02", "LVE-08"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'self')")
            .bind(s.org_id).bind(role_id).bind(op).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(user_id).bind(role_id).execute(pool).await.unwrap();
    Worker { employee_id, user_id, token: token(user_id, s.org_id) }
}

/// An enabled geo-fence for a branch (PC-03).
pub async fn fence(pool: &PgPool, org: Uuid, branch: Uuid, lat: f64, lon: f64, radius_m: i32) {
    sqlx::query("INSERT INTO org.geo_fence (org_id, branch_id, latitude, longitude, radius_m, enabled) VALUES ($1, $2, $3, $4, $5, true)")
        .bind(org).bind(branch).bind(lat).bind(lon).bind(radius_m).execute(pool).await.unwrap();
}

/// A published 09:00-18:00 shift assigned to the employee from 2026-01-01, so no day is an off day.
pub async fn day_shift(app: &axum::Router, s: &SeedOrg, employee: Uuid) {
    let code = format!("G{}", &Uuid::new_v4().simple().to_string()[..6]);
    let def = serde_json::json!({ "shift_type": "fixed", "start_time": "09:00", "end_time": "18:00" });
    let (st, b, _) = call(app, s, "POST", "/v1/shifts", Some(serde_json::json!({ "code": code, "name": code, "effective_from": "2026-01-01", "definition": def })), None).await;
    assert_eq!(st, axum::http::StatusCode::CREATED, "{b}");
    let v = b["versions"][0]["id"].as_str().unwrap().to_string();
    call(app, s, "POST", &format!("/v1/shift-versions/{v}/publish"), None, None).await;
    let body = serde_json::json!({ "employee_id": employee, "source": "fixed_shift", "shift_id": b["id"], "valid_from": "2026-01-01", "reason": "joining" });
    let (st, b, _) = call(app, s, "POST", "/v1/employee-schedules", Some(body), None).await;
    assert_eq!(st, axum::http::StatusCode::CREATED, "{b}");
}

/// A request as another user (worker) with optional extra headers.
pub async fn as_user(app: &axum::Router, token: &str, method: &str, uri: &str, body: Option<serde_json::Value>, headers: &[(&str, String)]) -> (axum::http::StatusCode, serde_json::Value) {
    let (res, json) = send(app, req(method, uri, token, body, headers)).await;
    (res.status(), json)
}
