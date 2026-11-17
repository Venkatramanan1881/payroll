mod error;

pub use error::ApiError;

use axum::{routing::get, Json, Router};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Vec<u8>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({ "status": "ok" })) }))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
}
