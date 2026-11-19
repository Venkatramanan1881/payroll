//! Vendor support access (slice 12).

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn vendor(pool: &PgPool, app: &axum::Router, email: &str, role: &str) -> String {
    sqlx::query("INSERT INTO platform.vendor_user (email, full_name, vendor_role) VALUES ($1, 'Staff', $2)").bind(email).bind(role).execute(pool).await.unwrap();
    let (res, b) = common::send(app, common::post_json("/v1/vendor/auth/google/sign-in", json!({ "id_token": format!("vendor:{email}") }))).await;
    assert_eq!(res.status(), StatusCode::OK, "{b}");
    b["access_token"].as_str().unwrap().to_string()
}

/// An Organisation Admin with the matrix's support-access grants (PLT-14 approve, PLT-18).
async fn org_admin(pool: &PgPool, org: Uuid) -> (Uuid, String) {
    let (user, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', $3, 'x')").bind(org).bind(user).bind(format!("{user}@t.invalid")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'OA')").bind(org).bind(role).execute(pool).await.unwrap();
    for (op, kind) in [("PLT-14", "approve"), ("PLT-18", "all")] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, $4)").bind(org).bind(role).bind(op).bind(kind).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(user).execute(pool).await.unwrap();
    (role, common::token(user, org))
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_approve_end_and_expiry(pool: PgPool) {
    let app = common::app(pool.clone());
    let s = common::seed_org(&pool).await;
    let ops = vendor(&pool, &app, "ops@probeyond.in", "full_admin").await;
    let viewer = vendor(&pool, &app, "viewer@probeyond.in", "read_only").await;
    let (oa_role, oa) = org_admin(&pool, s.org_id).await;
    let request = format!("/v1/vendor/organisations/{}/support-access-requests", s.org_id);
    assert_eq!(common::as_user(&app, &viewer, "POST", &request, Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::FORBIDDEN, "read_only cannot request");

    let k = key();
    let (st, g) = common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "ticket 42", "requested_window_minutes": 60 })), &k).await;
    assert_eq!((st, g["state"].as_str()), (StatusCode::CREATED, Some("requested")), "VC-03 on by default: {g}");
    let (_, again) = common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "ticket 42", "requested_window_minutes": 60 })), &k).await;
    assert_eq!(again["id"], g["id"], "a retried request is the same grant");
    let id = g["id"].as_str().unwrap().to_string();
    let (_, mine) = common::as_user(&app, &ops, "GET", "/v1/vendor/support-access-grants", None, &[]).await;
    assert_eq!(mine["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/support-access-grants", None, &[]).await.0, StatusCode::FORBIDDEN, "HR holds neither PLT-14 nor PLT-18");
    let (_, listed) = common::as_user(&app, &oa, "GET", "/v1/support-access-grants?state=requested", None, &[]).await;
    assert_eq!(listed["items"][0]["id"].as_str(), Some(&*id), "{listed}");

    let other = common::seed_org(&pool).await;
    let (_, other_oa) = org_admin(&pool, other.org_id).await;
    assert_eq!(common::as_user(&app, &other_oa, "POST", &format!("/v1/support-access-grants/{id}/approve"), None, &key()).await.0, StatusCode::NOT_FOUND, "TB-01");

    let end_at = chrono::Utc::now() + chrono::Duration::minutes(45);
    let (st, active) = common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{id}/approve"), Some(json!({ "window_end": end_at, "comment": "ok" })), &key()).await;
    assert_eq!((st, active["state"].as_str()), (StatusCode::OK, Some("active")), "{active}");
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{id}/approve"), None, &key()).await.0, StatusCode::OK, "approve twice: same answer");
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{id}/reject"), Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::CONFLICT);
    let end = format!("/v1/support-access-grants/{id}/end");
    assert_eq!(common::as_user(&app, &oa, "POST", &end, Some(json!({ "reason": "done" })), &key()).await.0, StatusCode::FORBIDDEN, "PLT-15 is held by nobody by default (G-03)");
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'PLT-15', 'all')").bind(s.org_id).bind(oa_role).execute(&pool).await.unwrap();
    let (st, ended) = common::as_user(&app, &oa, "POST", &end, Some(json!({ "reason": "done" })), &key()).await;
    assert_eq!((st, ended["state"].as_str()), (StatusCode::OK, Some("ended")));
    let (st, log) = common::as_user(&app, &oa, "GET", &format!("/v1/support-access-grants/{id}/actions"), None, &[]).await;
    assert_eq!((st, log["items"].as_array().map(Vec::len)), (StatusCode::OK, Some(0)), "no data access under a grant yet (G-02)");

    // A rejected request, then VC-03 off: active at once; past its window it reads as expired.
    let (_, second) = common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "ticket 43" })), &key()).await;
    let (_, rej) = common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{}/reject", second["id"].as_str().unwrap()), Some(json!({ "reason": "not now" })), &key()).await;
    assert_eq!(rej["state"], "rejected");
    // Approved without a window_end: the requested length applies (30 minutes, not the 2-hour default).
    let (_, short) = common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "quick look", "requested_window_minutes": 30 })), &key()).await;
    let (_, ok) = common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{}/approve", short["id"].as_str().unwrap()), None, &key()).await;
    let start: chrono::DateTime<chrono::Utc> = ok["window_start"].as_str().unwrap().parse().unwrap();
    let end: chrono::DateTime<chrono::Utc> = ok["window_end"].as_str().unwrap().parse().unwrap();
    assert_eq!((end - start).num_minutes(), 30, "{ok}");
    sqlx::query("UPDATE platform.organisation SET support_access_needs_approval = false WHERE id = $1").bind(s.org_id).execute(&pool).await.unwrap();
    let (_, direct) = common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "urgent" })), &key()).await;
    assert_eq!(direct["state"], "active", "{direct}");
    sqlx::query("UPDATE platform.support_access_grant SET window_start = now() - interval '3 hours', window_end = now() - interval '1 hour' WHERE id = $1")
        .bind(direct["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&pool).await.unwrap();
    let (_, expired) = common::as_user(&app, &oa, "GET", "/v1/support-access-grants?state=expired", None, &[]).await;
    assert_eq!(expired["items"].as_array().map(Vec::len), Some(1), "{expired}");
    let expired_id = expired["items"][0]["id"].as_str().unwrap();
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/support-access-grants/{expired_id}/end"), Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::CONFLICT, "an expired grant is not ended");
    assert_eq!(common::as_user(&app, &ops, "POST", &request, Some(json!({ "reason": "x", "requested_window_minutes": 9999 })), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
}
