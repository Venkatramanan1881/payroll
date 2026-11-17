mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn new_set(app: &axum::Router, s: &common::SeedOrg, name: &str, from: &str) -> Value {
    let (st, b, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": name, "effective_from": from })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    b
}

async fn version(app: &axum::Router, s: &common::SeedOrg, id: &str) -> (Value, String) {
    let (st, b, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{id}"), None, None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    (b, etag.unwrap())
}

async fn put_group(app: &axum::Router, s: &common::SeedOrg, v: &str, group: &str, settings: Value) -> (StatusCode, Value) {
    let (_, etag) = version(app, s, v).await;
    let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/{group}"), Some(json!({ "schema_version": 1, "settings": settings })), Some(&etag)).await;
    (st, b)
}

async fn publish(app: &axum::Router, s: &common::SeedOrg, v: &str) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    (st, b)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_set_has_draft_v1(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let set = new_set(&app, &s, "Factory", "2026-01-01").await;
    assert_eq!(set["name"], "Factory");
    assert_eq!(set["versions"].as_array().unwrap().len(), 1);
    assert_eq!(set["versions"][0]["state"], "draft");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn put_rule_group_unknown_key_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let v = new_set(&app, &s, "Factory", "2026-01-01").await["versions"][0]["id"].as_str().unwrap().to_string();
    for bad in [json!({ "LA-99x": 1 }), json!({ "PC-01": true }), json!([1, 2])] {
        assert_eq!(put_group(&app, &s, &v, "LA", bad.clone()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    assert_eq!(put_group(&app, &s, &v, "XX", json!({})).await.0, StatusCode::UNPROCESSABLE_ENTITY, "unknown rule group");
    let (st, ok) = put_group(&app, &s, &v, "LA", json!({ "LA-03": 10, "LA-05": [{ "from": 11, "to": 120, "action": "late_mark" }] })).await;
    assert_eq!(st, StatusCode::OK, "{ok}");
    assert_eq!(ok["document"]["settings"]["LA-03"], 10);
    assert_eq!(ok["warnings"], json!([]));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn put_rule_group_then_switch_off(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let v = new_set(&app, &s, "Factory", "2026-01-01").await["versions"][0]["id"].as_str().unwrap().to_string();
    put_group(&app, &s, &v, "LA", json!({ "LA-03": 10 })).await;
    let (st, doc, _) = common::call(&app, &s, "POST", &format!("/v1/policy-versions/{v}/rule-groups/LA/switch"), Some(json!({ "enabled": false })), None).await;
    assert_eq!(st, StatusCode::OK, "{doc}");
    assert_eq!(doc["enabled"], false);
    assert_eq!(version(&app, &s, &v).await.0["rule_groups"][0]["enabled"], false);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn copy_from_version_copies_rule_groups(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let set = new_set(&app, &s, "Factory", "2026-01-01").await;
    let v1 = set["versions"][0]["id"].as_str().unwrap().to_string();
    put_group(&app, &s, &v1, "LA", json!({ "LA-03": 10 })).await;
    let (st, v2, _) = common::call(&app, &s, "POST", &format!("/v1/policy-sets/{}/versions", set["id"].as_str().unwrap()), Some(json!({ "effective_from": "2026-06-01", "copy_from_version_id": v1 })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{v2}");
    assert_eq!(v2["rule_groups"][0]["settings"]["LA-03"], 10);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_supersedes_previous(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let set = new_set(&app, &s, "Factory", "2026-01-01").await;
    let v1 = set["versions"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(publish(&app, &s, &v1).await.1["state"], "in_force");
    let (_, v2, _) = common::call(&app, &s, "POST", &format!("/v1/policy-sets/{}/versions", set["id"].as_str().unwrap()), Some(json!({ "effective_from": "2026-06-01", "copy_from_version_id": v1 })), None).await;
    let v2 = v2["id"].as_str().unwrap().to_string();
    assert_eq!(publish(&app, &s, &v2).await.1["state"], "in_force");
    assert_eq!(version(&app, &s, &v1).await.0["state"], "superseded");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_into_locked_period_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let emp: Uuid = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().parse().unwrap();
    let (cycle, period) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO period.attendance_cycle (org_id, id, owner_kind, owner_id, valid) VALUES ($1, $2, 'organisation', $1, daterange('2026-01-01', NULL))").bind(s.org_id).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.attendance_period (org_id, id, cycle_id, period_start, period_end) VALUES ($1, $2, $3, '2026-01-01', '2026-01-31')").bind(s.org_id).bind(period).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.lock_coverage (org_id, employee_id, dates, period_id) VALUES ($1, $2, daterange('2026-01-01', '2026-02-01'), $3)").bind(s.org_id).bind(emp).bind(period).execute(&pool).await.unwrap();
    let v = new_set(&app, &s, "Factory", "2026-01-15").await["versions"][0]["id"].as_str().unwrap().to_string();
    let (st, body) = publish(&app, &s, &v).await;
    assert_eq!(st, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "PERIOD_LOCKED");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn edit_published_rule_group_409_version_frozen(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let v = new_set(&app, &s, "Factory", "2026-01-01").await["versions"][0]["id"].as_str().unwrap().to_string();
    put_group(&app, &s, &v, "LA", json!({ "LA-03": 10 })).await;
    publish(&app, &s, &v).await;
    let (st, body) = put_group(&app, &s, &v, "LA", json!({ "LA-03": 5 })).await;
    assert_eq!(st, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["code"], "VERSION_FROZEN");
    let (st, body, _) = common::call(&app, &s, "POST", &format!("/v1/policy-versions/{v}/rule-groups/LA/switch"), Some(json!({ "enabled": false })), None).await;
    assert_eq!((st, body["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pc_rule_group_needs_tcp11(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let v = new_set(&app, &s, "Factory", "2026-01-01").await["versions"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(put_group(&app, &s, &v, "PC", json!({ "PC-07": 2 })).await.0, StatusCode::OK);
    sqlx::query("DELETE FROM iam.role_grant WHERE operation_code = 'TCP-11'").execute(&pool).await.unwrap();
    assert_eq!(put_group(&app, &s, &v, "PC", json!({ "PC-07": 4 })).await.0, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pc_rule_group_is_typed(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let v = new_set(&app, &s, "Factory", "2026-01-01").await["versions"][0]["id"].as_str().unwrap().to_string();
    for bad in [json!({ "PC-04": "maybe" }), json!({ "PC-07": "3" }), json!({ "PC-14": { "threshold_m": 0 } }), json!({ "PC-03": 200 })] {
        assert_eq!(put_group(&app, &s, &v, "PC", bad.clone()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    let (st, body) = put_group(&app, &s, &v, "PC", json!({ "PC-04": "block", "PC-14": { "threshold_m": 50, "action": "flag" } })).await;
    assert_eq!(st, StatusCode::OK, "{body}");
}
