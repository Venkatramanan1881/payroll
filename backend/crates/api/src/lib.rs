mod error;
mod extract;
mod idempotency;
mod routes;

pub use error::ApiError;
pub use extract::{authenticate, unix_now, Auth};

use axum::{routing::{get, post}, Json, Router};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Vec<u8>,
    pub phone_verifier: Arc<dyn domain_iam::firebase::PhoneTokenVerifier>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))
        .route("/v1/auth/firebase/sign-in", post(routes::auth::firebase_sign_in))
        .route("/v1/auth/organisation-choice", post(routes::auth::choose_organisation))
        .route("/v1/auth/refresh", post(routes::auth::refresh))
        .route("/v1/auth/sign-out", post(routes::auth::sign_out))
        .route("/v1/shifts", post(routes::sched::create_shift).get(routes::sched::list_shifts))
        .route("/v1/shifts/{id}", get(routes::sched::get_shift))
        .route("/v1/shifts/{id}/versions", post(routes::sched::create_shift_version))
        .route("/v1/shift-versions/{id}", get(routes::sched::get_shift_version).patch(routes::sched::update_shift_version))
        .route("/v1/shift-versions/{id}/publish", post(routes::sched::publish_shift_version))
        .route("/v1/employees", post(routes::employees::create))
        .route("/v1/employees/{employee_id}", get(routes::employees::get).patch(routes::employees::patch))
        .route("/v1/employees/{employee_id}/assignments", get(routes::employees::list_assignments))
        .route("/v1/employees/{employee_id}/change-assignment", post(routes::employees::change_assignment))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
}
