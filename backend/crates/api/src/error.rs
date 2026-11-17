//! RFC 9457 problem responses with stable codes from docs/api/ERROR_CATALOG.md.

use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthenticated,
    Forbidden,
    OrgSuspended,
    NotFound,
    Internal(anyhow::Error),
}

impl ApiError {
    fn parts(&self) -> (StatusCode, &'static str, &'static str) {
        match self {
            ApiError::Unauthenticated => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED", "Sign-in required"),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "Not allowed"),
            ApiError::OrgSuspended => (StatusCode::FORBIDDEN, "ORG_SUSPENDED", "Organisation is suspended"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "NOT_FOUND", "Not found"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", "Internal error"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, title) = self.parts();
        if let ApiError::Internal(e) = &self {
            eprintln!("internal error: {e:#}"); // logged, never returned to the caller
        }
        let body = json!({
            "type": format!("https://errors.payroll.example/{code}"),
            "title": title,
            "status": status.as_u16(),
            "code": code,
            "correlation_id": uuid::Uuid::new_v4(),
        });
        (status, [(header::CONTENT_TYPE, "application/problem+json")], body.to_string()).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(e: sqlx::Error) -> Self {
        ApiError::Internal(e.into())
    }
}

impl From<domain_iam::authz::AuthzError> for ApiError {
    fn from(e: domain_iam::authz::AuthzError) -> Self {
        match e {
            domain_iam::authz::AuthzError::Forbidden => ApiError::Forbidden,
            domain_iam::authz::AuthzError::Db(e) => e.into(),
        }
    }
}
