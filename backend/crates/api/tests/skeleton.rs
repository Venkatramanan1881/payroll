mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::IntoResponse;
use http_body_util::BodyExt;
use sqlx::PgPool;

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn health_ok(pool: PgPool) {
    let app = common::app(pool);
    let (res, body) = common::send(&app, Request::get("/health").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(body, serde_json::json!({ "status": "ok" }));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unknown_route_is_problem_json_not_found(pool: PgPool) {
    let app = common::app(pool);
    let (res, body) = common::send(&app, Request::get("/v1/nope").body(Body::empty()).unwrap()).await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(res.headers()["content-type"], "application/problem+json");
    assert_eq!(body["code"], "NOT_FOUND");
    assert_eq!(body["status"], 404);
}

#[tokio::test]
async fn internal_error_hides_its_message() {
    let res = api::ApiError::Internal(anyhow::anyhow!("secret db detail")).into_response();
    assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let text = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(!text.contains("secret db detail"), "leaked: {text}");
    assert!(text.contains("\"code\":\"INTERNAL\""));
}
