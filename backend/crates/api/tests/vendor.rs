//! Vendor console and organisation onboarding (slice 11).

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn vendor_user(pool: &PgPool, email: &str, role: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO platform.vendor_user (email, full_name, vendor_role) VALUES ($1, 'Staff', $2) RETURNING id").bind(email).bind(role).fetch_one(pool).await.unwrap()
}

async fn sign_in(app: &axum::Router, email: &str) -> (StatusCode, Value) {
    let (res, body) = common::send(app, common::post_json("/v1/vendor/auth/google/sign-in", json!({ "id_token": format!("vendor:{email}") }))).await;
    (res.status(), body)
}

async fn token(app: &axum::Router, email: &str) -> String {
    let (st, b) = sign_in(app, email).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b["access_token"].as_str().unwrap().to_string()
}

fn org_body(template: Uuid, plan: &str) -> Value {
    json!({ "display_name": "Sri Bakery", "legal_name": "Sri Bakery Pvt Ltd", "sector_template_id": template, "plan_id": plan, "modules": ["M02"],
            "employee_limit": { "max_active": 50, "on_reach": "block" },
            "first_admin": { "email": "owner@sribakery.in", "mobile": "+919811100001" },
            "subscription": { "start_date": "2026-11-01", "renewal_date": "2027-10-31", "billing_status": "paid" } })
}

async fn etag(app: &axum::Router, token: &str, uri: &str) -> String {
    let (res, _) = common::send(app, common::req("GET", uri, token, None, &[])).await;
    res.headers().get("etag").unwrap().to_str().unwrap().to_string()
}

/// Sign-in, plans, organisation creation with its standard roles and first admin, replay by key, and permissions.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn onboarding_an_organisation(pool: PgPool) {
    let app = common::app(pool.clone());
    vendor_user(&pool, "ops@probeyond.in", "full_admin").await;
    vendor_user(&pool, "viewer@probeyond.in", "read_only").await;
    assert_eq!(sign_in(&app, "stranger@probeyond.in").await.0, StatusCode::UNAUTHORIZED, "only vendor users");
    let ops = token(&app, "ops@probeyond.in").await;
    let viewer = token(&app, "viewer@probeyond.in").await;

    // Token kinds never cross: an organisation token on a vendor route, a vendor token on an organisation route.
    let s = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/vendor/organisations", None, &[]).await.0, StatusCode::UNAUTHORIZED);
    assert_eq!(common::as_user(&app, &ops, "GET", "/v1/me", None, &[]).await.0, StatusCode::UNAUTHORIZED);

    let plan_body = json!({ "code": "STD", "name": "Standard", "modules": ["M02"], "employee_limit": 100, "mobile_app": true });
    let k = key();
    let (st, plan) = common::as_user(&app, &ops, "POST", "/v1/vendor/plans", Some(plan_body.clone()), &k).await;
    assert_eq!(st, StatusCode::CREATED, "{plan}");
    let (st, again) = common::as_user(&app, &ops, "POST", "/v1/vendor/plans", Some(plan_body.clone()), &k).await;
    assert_eq!((st, &again["id"]), (StatusCode::CREATED, &plan["id"]), "same key, same plan");
    assert_eq!(common::as_user(&app, &ops, "POST", "/v1/vendor/plans", Some(plan_body), &key()).await.0, StatusCode::CONFLICT, "code unique");
    assert_eq!(common::as_user(&app, &viewer, "POST", "/v1/vendor/plans", Some(json!({ "code": "X" })), &key()).await.0, StatusCode::FORBIDDEN, "read_only");

    let template: Uuid = sqlx::query_scalar("INSERT INTO platform.sector_template (code, name, content) VALUES ('bakery', 'Bakery', $1) RETURNING id")
        .bind(json!({ "organisation_settings": { "languages": ["en", "ta"], "week_start": "monday" } }))
        .fetch_one(&pool).await.unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let mut no_mobile = org_body(template, plan_id);
    no_mobile["first_admin"] = json!({ "email": "owner@sribakery.in" });
    assert_eq!(common::as_user(&app, &ops, "POST", "/v1/vendor/organisations", Some(no_mobile), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let mut common_module = org_body(template, plan_id);
    common_module["modules"] = json!(["C01"]);
    assert_eq!(common::as_user(&app, &ops, "POST", "/v1/vendor/organisations", Some(common_module), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);

    let k = key();
    let (st, org) = common::as_user(&app, &ops, "POST", "/v1/vendor/organisations", Some(org_body(template, plan_id)), &k).await;
    assert_eq!((st, org["plan_id"].as_str(), org["employee_limit"]["max_active"].as_i64(), org["modules"][0]["module_code"].as_str()),
               (StatusCode::CREATED, Some(plan_id), Some(50), Some("M02")), "{org}");
    let (_, replayed) = common::as_user(&app, &ops, "POST", "/v1/vendor/organisations", Some(org_body(template, plan_id)), &k).await;
    assert_eq!(replayed["id"], org["id"], "a retried create returns the same organisation");
    let org_id: Uuid = org["id"].as_str().unwrap().parse().unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM platform.organisation WHERE display_name = 'Sri Bakery'").fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);

    // The seven system roles with the matrix's grants, and the first admin.
    let roles: i64 = sqlx::query_scalar("SELECT count(*) FROM iam.role WHERE org_id = $1 AND system_code IS NOT NULL").bind(org_id).fetch_one(&pool).await.unwrap();
    let oa_grants: i64 = sqlx::query_scalar("SELECT count(*) FROM iam.role_grant g JOIN iam.role r ON r.id = g.role_id WHERE r.org_id = $1 AND r.system_code = 'org_admin'")
        .bind(org_id).fetch_one(&pool).await.unwrap();
    let expected = domain_iam::defaults::SYSTEM_ROLE_GRANTS.iter().filter(|g| g.0 == "org_admin").count() as i64;
    assert_eq!((roles, oa_grants), (7, expected));
    let langs: Value = sqlx::query_scalar("SELECT settings->'languages' FROM org.organisation_settings_version WHERE org_id = $1").bind(org_id).fetch_one(&pool).await.unwrap();
    assert_eq!(langs, json!(["en", "ta"]), "template settings copied");
    let (res, signed) = common::send(&app, common::post_json("/v1/auth/firebase/sign-in", json!({ "firebase_id_token": "phone:+919811100001" }))).await;
    assert_eq!(res.status(), StatusCode::OK, "{signed}");
    let admin = signed["access_token"].as_str().unwrap();
    let (st, users) = common::as_user(&app, admin, "GET", "/v1/users", None, &[]).await;
    assert_eq!((st, users["items"][0]["roles"][0]["system_code"].as_str()), (StatusCode::OK, Some("org_admin")), "{users}");
    assert_eq!(common::as_user(&app, admin, "GET", &format!("/v1/users/{}", s.hr_user_id), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01: another organisation's user");

    // Dated history: a change takes effect after the current row; a plan change is listed with the original.
    let uri = format!("/v1/vendor/organisations/{org_id}/change-plan");
    let mut h = key();
    h.push(("if-match", etag(&app, &ops, &format!("/v1/vendor/organisations/{org_id}")).await));
    let today = (chrono::Utc::now() + chrono::Duration::minutes(330)).date_naive();
    assert_eq!(common::as_user(&app, &ops, "POST", &uri, Some(json!({ "plan_id": plan_id, "reason": "same day" })), &h).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let mut h = key();
    h.push(("if-match", etag(&app, &ops, &format!("/v1/vendor/organisations/{org_id}")).await));
    let (st, changed) = common::as_user(&app, &ops, "POST", &uri, Some(json!({ "plan_id": plan_id, "valid_from": today + chrono::Duration::days(1), "reason": "renewal" })), &h).await;
    assert_eq!(st, StatusCode::OK, "{changed}");
    let (_, hist) = common::as_user(&app, &viewer, "GET", &format!("/v1/vendor/organisations/{org_id}/history?kind=plan"), None, &[]).await;
    assert_eq!(hist["items"].as_array().map(Vec::len), Some(2), "{hist}");

    // Modules: off once, off again changes nothing, common modules never switch.
    let off = format!("/v1/vendor/organisations/{org_id}/modules/M02/switch-off");
    let body = json!({ "valid_from": today + chrono::Duration::days(1), "reason": "not used" });
    let (st, m) = common::as_user(&app, &ops, "POST", &off, Some(body.clone()), &key()).await;
    assert_eq!((st, m["enabled"].as_bool()), (StatusCode::OK, Some(false)), "{m}");
    common::as_user(&app, &ops, "POST", &off, Some(body), &key()).await;
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM platform.module_entitlement WHERE org_id = $1").bind(org_id).fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 2);
    let c01 = format!("/v1/vendor/organisations/{org_id}/modules/C01/switch-off");
    assert_eq!(common::as_user(&app, &ops, "POST", &c01, Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);

    // Suspension: organisation users may read only (VC-R4); read_only vendors cannot suspend.
    let suspend = format!("/v1/vendor/organisations/{org_id}/suspend");
    assert_eq!(common::as_user(&app, &viewer, "POST", &suspend, Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::FORBIDDEN);
    let (_, sus) = common::as_user(&app, &ops, "POST", &suspend, Some(json!({ "reason": "unpaid" })), &key()).await;
    assert_eq!(sus["state"], "suspended");
    let (st, b) = common::as_user(&app, admin, "POST", "/v1/roles", Some(json!({ "name": "X" })), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("ORG_SUSPENDED")));
    assert_eq!(common::as_user(&app, admin, "GET", "/v1/users", None, &[]).await.0, StatusCode::OK, "reads still work");
    let (_, back) = common::as_user(&app, &ops, "POST", &format!("/v1/vendor/organisations/{org_id}/reactivate"), Some(json!({ "reason": "paid" })), &key()).await;
    assert_eq!(back["state"], "active");
}

/// Vendor users: no self-change, the last full admin stays, deactivation ends the console session at once.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn vendor_users(pool: PgPool) {
    let app = common::app(pool.clone());
    let ops_id = vendor_user(&pool, "ops@probeyond.in", "full_admin").await;
    let ops = token(&app, "ops@probeyond.in").await;
    let (st, u) = common::as_user(&app, &ops, "POST", "/v1/vendor/vendor-users", Some(json!({ "email": "help@probeyond.in", "full_name": "Help", "vendor_role": "support" })), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{u}");
    let help = token(&app, "help@probeyond.in").await;
    assert_eq!(common::as_user(&app, &help, "GET", "/v1/vendor/organisations", None, &[]).await.0, StatusCode::OK, "support views (G-01 assumption)");
    let mut h = key();
    h.push(("if-match", "W/\"x\"".to_string()));
    assert_eq!(common::as_user(&app, &ops, "POST", &format!("/v1/vendor/vendor-users/{ops_id}/change-role"), Some(json!({ "vendor_role": "read_only", "reason": "x" })), &h).await.0,
               StatusCode::PRECONDITION_FAILED);
    let off = format!("/v1/vendor/vendor-users/{}/deactivate", u["id"].as_str().unwrap());
    let (_, d) = common::as_user(&app, &ops, "POST", &off, Some(json!({ "reason": "left" })), &key()).await;
    assert_eq!(d["active"], false);
    assert_eq!(common::as_user(&app, &help, "GET", "/v1/vendor/organisations", None, &[]).await.0, StatusCode::UNAUTHORIZED, "deactivated: the token stops at once");
    assert_eq!(common::as_user(&app, &ops, "POST", &format!("/v1/vendor/vendor-users/{ops_id}/deactivate"), Some(json!({ "reason": "x" })), &key()).await.0, StatusCode::FORBIDDEN, "not yourself");
}
