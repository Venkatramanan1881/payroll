mod common;

use axum::http::StatusCode;
use domain_iam::tokens::{parse_refresh, verify_access};
use serde_json::json;
use sqlx::PgPool;

const MOBILE: &str = "+919800000001";

async fn sign_in(app: &axum::Router, mobile: &str) -> (StatusCode, serde_json::Value) {
    let body = json!({ "firebase_id_token": format!("phone:{mobile}") });
    let (res, body) = common::send(app, common::post_json("/v1/auth/firebase/sign-in", body)).await;
    (res.status(), body)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signin_single_org_returns_tokens(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let user = common::seed_mobile_user(&pool, s.org_id, MOBILE).await;
    let (status, body) = sign_in(&common::app(pool.clone()), MOBILE).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["expires_in"], 900);
    assert_eq!(body["token_type"], "Bearer");
    let claims = verify_access(common::JWT_SECRET, body["access_token"].as_str().unwrap(), common::now()).unwrap();
    assert_eq!((claims.sub, claims.org), (user, s.org_id));
    let (org, hash) = parse_refresh(body["refresh_token"].as_str().unwrap()).unwrap();
    assert_eq!(org, s.org_id);
    let sessions: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM iam.session WHERE user_id = $1 AND refresh_token_hash = $2 AND revoked_at IS NULL",
    )
    .bind(user).bind(hex(&hash)).fetch_one(&pool).await.unwrap();
    assert_eq!(sessions, 1);
    let audits: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit.audit_entry WHERE org_id = $1 AND action = 'auth.sign_in' AND actor_id = $2",
    )
    .bind(s.org_id).bind(user).fetch_one(&pool).await.unwrap();
    assert_eq!(audits, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signin_unknown_number_401_same_body_as_bad_token(pool: PgPool) {
    common::seed_org(&pool).await;
    let app = common::app(pool);
    let (s1, mut unknown) = sign_in(&app, "+910000000000").await;
    let (res, mut bad) = common::send(&app, common::post_json("/v1/auth/firebase/sign-in", json!({ "firebase_id_token": "garbage" }))).await;
    assert_eq!((s1, res.status()), (StatusCode::UNAUTHORIZED, StatusCode::UNAUTHORIZED));
    unknown.as_object_mut().unwrap().remove("correlation_id");
    bad.as_object_mut().unwrap().remove("correlation_id");
    assert_eq!(unknown, bad);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn signin_two_orgs_409_then_choice_signs_in(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    let user_a = common::seed_mobile_user(&pool, a.org_id, MOBILE).await;
    let user_b = common::seed_mobile_user(&pool, b.org_id, MOBILE).await;
    let app = common::app(pool);
    let (status, body) = sign_in(&app, MOBILE).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], "ORGANISATION_CHOICE_REQUIRED");
    let cands = body["candidates"].as_array().unwrap();
    assert_eq!(cands.len(), 2);
    let text = body.to_string();
    assert!(!text.contains(&a.org_id.to_string()) && !text.contains(&b.org_id.to_string()), "org ids must stay opaque");
    let mut chosen = Vec::new();
    for c in cands {
        let choice = json!({ "choice_token": body["choice_token"], "candidate_ref": c["candidate_ref"] });
        let (res, tokens) = common::send(&app, common::post_json("/v1/auth/organisation-choice", choice)).await;
        assert_eq!(res.status(), StatusCode::OK, "{tokens}");
        let claims = verify_access(common::JWT_SECRET, tokens["access_token"].as_str().unwrap(), common::now()).unwrap();
        chosen.push((claims.org, claims.sub));
    }
    chosen.sort();
    let mut expected = vec![(a.org_id, user_a), (b.org_id, user_b)];
    expected.sort();
    assert_eq!(chosen, expected);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn choice_ref_cannot_pick_other_org(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    common::seed_mobile_user(&pool, a.org_id, MOBILE).await;
    common::seed_mobile_user(&pool, b.org_id, MOBILE).await;
    let app = common::app(pool);
    let (_, body) = sign_in(&app, MOBILE).await;
    let tampered = format!("{}x", body["choice_token"].as_str().unwrap());
    for (choice_token, candidate_ref) in [
        (body["choice_token"].clone(), json!("7")),
        (json!(tampered), body["candidates"][0]["candidate_ref"].clone()),
    ] {
        let choice = json!({ "choice_token": choice_token, "candidate_ref": candidate_ref });
        let (res, _) = common::send(&app, common::post_json("/v1/auth/organisation-choice", choice)).await;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    }
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn refresh_rotates(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    common::seed_mobile_user(&pool, s.org_id, MOBILE).await;
    let app = common::app(pool);
    let (_, first) = sign_in(&app, MOBILE).await;
    let (res, second) = common::send(&app, common::post_json("/v1/auth/refresh", json!({ "refresh_token": first["refresh_token"] }))).await;
    assert_eq!(res.status(), StatusCode::OK, "{second}");
    assert_ne!(second["refresh_token"], first["refresh_token"]);
    let (res, _) = common::send(&app, common::post_json("/v1/auth/refresh", json!({ "refresh_token": first["refresh_token"] }))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED, "old refresh token must be dead after rotation");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sign_out_revokes(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    common::seed_mobile_user(&pool, s.org_id, MOBILE).await;
    let app = common::app(pool);
    let (_, tokens) = sign_in(&app, MOBILE).await;
    let req = axum::http::Request::post("/v1/auth/sign-out")
        .header("authorization", format!("Bearer {}", tokens["access_token"].as_str().unwrap()))
        .body(axum::body::Body::empty())
        .unwrap();
    let (res, _) = common::send(&app, req).await;
    assert_eq!(res.status(), StatusCode::NO_CONTENT);
    let (res, _) = common::send(&app, common::post_json("/v1/auth/refresh", json!({ "refresh_token": tokens["refresh_token"] }))).await;
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}
