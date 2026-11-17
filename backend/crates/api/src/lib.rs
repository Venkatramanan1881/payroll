mod error;
mod extract;
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
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
}
