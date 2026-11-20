//! Notifications (slice 17).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn dispatch(pool: &PgPool, org: Uuid) -> Value {
    let mut tx = db::begin_org(pool, org).await.unwrap();
    let r = domain_attendance::notify::dispatch(&mut tx, org).await.unwrap();
    tx.commit().await.unwrap();
    r
}

async fn inbox(app: &axum::Router, token: &str) -> Value {
    let (st, b) = common::as_user(app, token, "GET", "/v1/notifications/inbox", None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    b
}

async fn patch_rules(app: &axum::Router, s: &common::SeedOrg, changes: Value) -> (StatusCode, Value) {
    let (_, _, etag) = common::call(app, s, "GET", "/v1/notifications/rules", None, None).await;
    let (st, b, _) = common::call(app, s, "PATCH", "/v1/notifications/rules", Some(json!({ "reason": "setup", "changes": changes })), etag.as_deref()).await;
    (st, b)
}

/// NT-05 end to end: the approver hears a request is waiting, the requester hears the decision; rules switch them;
/// each event is sent once; NT-06 digests once a day.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn request_lifecycle_notifies_through_the_rules(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, set, _) = common::call(&app, &s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    common::call(&app, &s, "POST", &format!("/v1/policy-versions/{}/publish", set["versions"][0]["id"].as_str().unwrap()), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(&app, &s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let boss = common::seed_worker(&pool, &app, &s, "Meena").await;
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'mgr')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'PRM-09', 'approve')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(boss.user_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, manager_employee_id, valid) VALUES ($1, $2, 'reporting_manager', $3, daterange('2026-01-01', NULL))")
        .bind(s.org_id).bind(w.employee_id).bind(boss.employee_id).execute(&pool).await.unwrap();

    // Rules: in-app on by default; other channels cannot be switched on yet (D-34, D-35); If-Match required.
    let (st, r, etag) = common::call(&app, &s, "GET", "/v1/notifications/rules", None, None).await;
    assert_eq!((st, etag.is_some()), (StatusCode::OK, true));
    let on: Vec<&Value> = r["items"].as_array().unwrap().iter().filter(|i| i["enabled"] == true).collect();
    assert!(on.iter().all(|i| i["channel"] == "in_app") && on.len() == 5, "{r}");
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/notifications/rules", None, &[]).await.0, StatusCode::FORBIDDEN, "NTF-01");
    let (st, _) = patch_rules(&app, &s, json!([{ "event_code": "NT-05", "channel": "email", "recipient_kind": "employee", "enabled": true }])).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "email waits for a provider");
    let (st, _, _) = common::call(&app, &s, "PATCH", "/v1/notifications/rules", Some(json!({ "reason": "x", "changes": [] })), None).await;
    assert_eq!(st, StatusCode::PRECONDITION_REQUIRED);

    let day = |n: i64| ((Utc::now() + Duration::minutes(330)).date_naive() + Duration::days(n)).to_string();
    let ask = |d: String| json!({ "duration_kind": "full_day", "from_date": d, "to_date": d, "purpose": "client visit", "place": "Chennai" });
    let (st, first) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask(day(3))), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{first}");
    assert_eq!(dispatch(&pool, s.org_id).await["sent"], 1);
    let b = inbox(&app, &boss.token).await;
    assert_eq!((b["unread_count"].as_i64(), b["items"][0]["event_code"].as_str(), b["items"][0]["link"]["kind"].as_str()), (Some(1), Some("NT-05"), Some("on_duty")), "{b}");
    assert_eq!(inbox(&app, &w.token).await["unread_count"], 0, "no notice of one's own submission");

    let id = first["id"].as_str().unwrap();
    let (st, _) = common::as_user(&app, &boss.token, "POST", &format!("/v1/on-duty/requests/{id}/reject"), Some(json!({ "reason": "not needed" })), &key()).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(dispatch(&pool, s.org_id).await["sent"], 1);
    let mine = inbox(&app, &w.token).await;
    assert!(mine["items"][0]["body"].as_str().unwrap().contains("rejected"), "{mine}");
    assert_eq!(dispatch(&pool, s.org_id).await["events"], 0, "each event once");

    // Read: own ids only; another user's id is ignored.
    let nid = mine["items"][0]["id"].as_str().unwrap();
    assert_eq!(common::as_user(&app, &boss.token, "POST", "/v1/notifications/inbox/mark-read", Some(json!({ "ids": [nid] })), &[]).await.0, StatusCode::NO_CONTENT);
    assert_eq!(inbox(&app, &w.token).await["unread_count"], 1, "not the boss's to read");
    assert_eq!(common::as_user(&app, &w.token, "POST", "/v1/notifications/inbox/mark-read", Some(json!({ "all": true })), &[]).await.0, StatusCode::NO_CONTENT);
    assert_eq!(inbox(&app, &w.token).await["unread_count"], 0);

    // Switched off: the requester hears nothing of the next decision.
    let (st, b) = patch_rules(&app, &s, json!([{ "event_code": "NT-05", "channel": "in_app", "recipient_kind": "employee", "enabled": false }])).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    let (_, second) = common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask(day(5))), &key()).await;
    dispatch(&pool, s.org_id).await;
    common::as_user(&app, &boss.token, "POST", &format!("/v1/on-duty/requests/{}/reject", second["id"].as_str().unwrap()), Some(json!({ "reason": "no" })), &key()).await;
    dispatch(&pool, s.org_id).await;
    assert_eq!(inbox(&app, &w.token).await["items"].as_array().map(Vec::len), Some(1), "still only the first decision");

    // NT-06: one digest per approver per day.
    common::as_user(&app, &w.token, "POST", "/v1/on-duty/requests", Some(ask(day(7))), &key()).await;
    for expected in [1, 0] {
        let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
        let r = domain_attendance::notify::digest(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
        assert_eq!(r["sent"], expected, "{r}");
    }
    let other = common::seed_org(&pool).await;
    assert_eq!(inbox(&app, &other.hr_token).await["items"].as_array().map(Vec::len), Some(0), "TB-01");
}

/// Push devices: own only; the same token moves to whoever registers it last.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn push_devices_register_move_and_unregister(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let body = json!({ "token": "fcm-token-1", "platform": "android", "app_version": "1.0.0" });
    let (st, d) = common::as_user(&app, &w.token, "POST", "/v1/notifications/push-devices", Some(body.clone()), &[]).await;
    assert_eq!((st, d["platform"].as_str()), (StatusCode::OK, Some("android")), "{d}");
    assert_eq!(common::as_user(&app, &w.token, "POST", "/v1/notifications/push-devices", Some(json!({ "token": "t", "platform": "windows" })), &[]).await.0,
               StatusCode::UNPROCESSABLE_ENTITY);
    let (st, moved) = common::as_user(&app, &s.hr_token, "POST", "/v1/notifications/push-devices", Some(body), &[]).await;
    assert_eq!(st, StatusCode::OK);
    let live: Vec<Uuid> = sqlx::query_scalar("SELECT user_id FROM notify.push_device WHERE revoked_at IS NULL").fetch_all(&pool).await.unwrap();
    assert_eq!(live, vec![s.hr_user_id], "one live row per token");
    assert_eq!(common::as_user(&app, &w.token, "DELETE", &format!("/v1/notifications/push-devices/{}", moved["id"].as_str().unwrap()), None, &[]).await.0,
               StatusCode::NO_CONTENT, "not theirs: 204, nothing changed");
    let still: i64 = sqlx::query_scalar("SELECT count(*) FROM notify.push_device WHERE revoked_at IS NULL").fetch_one(&pool).await.unwrap();
    assert_eq!(still, 1);
    assert_eq!(common::as_user(&app, &s.hr_token, "DELETE", &format!("/v1/notifications/push-devices/{}", moved["id"].as_str().unwrap()), None, &[]).await.0, StatusCode::NO_CONTENT);
    let still: i64 = sqlx::query_scalar("SELECT count(*) FROM notify.push_device WHERE revoked_at IS NULL").fetch_one(&pool).await.unwrap();
    assert_eq!(still, 0);
}
