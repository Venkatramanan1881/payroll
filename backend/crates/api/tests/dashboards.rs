//! Dashboards, widget choices, default filters and upload links (slice 18b).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

fn codes(d: &Value) -> Vec<String> {
    d["widgets"].as_array().unwrap().iter().map(|w| w["code"].as_str().unwrap().to_string()).collect()
}

/// DB-R1: each role lands on its widgets, within scope; DB-01 choices narrow them; DB-02 default filters apply.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn dashboards_choices_and_default_filters(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    sqlx::query("UPDATE iam.role SET system_code = 'employee' WHERE id IN (SELECT role_id FROM iam.user_role WHERE user_id = $1)").bind(w.user_id).execute(&pool).await.unwrap();

    let (st, hr) = common::as_user(&app, &s.hr_token, "GET", "/v1/dashboards/me", None, &[]).await;
    assert_eq!((st, hr["role"].as_str()), (StatusCode::OK, Some("hr_manager")), "{hr}");
    assert_eq!(codes(&hr), ["today_counts_by_branch", "missed_punches", "pending_approvals", "devices_offline", "joiners_leavers"]);
    let branches = &hr["widgets"][0]["data"]["branches"];
    assert_eq!(branches[0]["employees"], 1, "the worker in the seeded branch: {hr}");

    let (st, mine) = common::as_user(&app, &w.token, "GET", "/v1/dashboards/me", None, &[]).await;
    assert_eq!((st, mine["role"].as_str()), (StatusCode::OK, Some("employee")), "{mine}");
    assert_eq!(codes(&mine), ["today_punch_status", "month_attendance", "leave_balance", "permission_balance", "comp_off_balance", "my_pending_requests"]);
    assert_eq!(mine["widgets"][4]["data"]["balance"], "0.00", "{mine}");
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/dashboards/me?role=hr_manager", None, &[]).await.0, StatusCode::UNPROCESSABLE_ENTITY, "not their role");

    // DB-01: only offered widgets; If-Match required; the role then shows just its choice.
    let (st, ch, etag) = common::call(&app, &s, "GET", "/v1/dashboards/widget-choices", None, None).await;
    assert_eq!((st, etag.is_some()), (StatusCode::OK, true), "{ch}");
    let hr_role = ch["items"].as_array().unwrap().iter().find(|i| i["role_name"] == "HR Manager").unwrap()["role_id"].as_str().unwrap().to_string();
    let uri = format!("/v1/dashboards/widget-choices/{hr_role}");
    assert_eq!(common::call(&app, &s, "PUT", &uri, Some(json!({ "widgets": ["pending_approvals"] })), None).await.0, StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(common::call(&app, &s, "PUT", &uri, Some(json!({ "widgets": ["leave_balance"] })), etag.as_deref()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "not offered to HR");
    assert_eq!(common::call(&app, &s, "PUT", &uri, Some(json!({ "widgets": ["pending_approvals", "pending_approvals"] })), etag.as_deref()).await.0,
               StatusCode::UNPROCESSABLE_ENTITY, "each widget once");
    let (st, b, _) = common::call(&app, &s, "PUT", &uri, Some(json!({ "widgets": ["pending_approvals", "joiners_leavers"] })), etag.as_deref()).await;
    assert_eq!((st, b["default_applied"].as_bool()), (StatusCode::OK, Some(false)), "{b}");
    assert_eq!(common::call(&app, &s, "PUT", &uri, Some(json!({ "widgets": ["pending_approvals"] })), etag.as_deref()).await.0, StatusCode::PRECONDITION_FAILED, "stale ETag");
    let (_, hr) = common::as_user(&app, &s.hr_token, "GET", "/v1/dashboards/me", None, &[]).await;
    assert_eq!(codes(&hr), ["pending_approvals", "joiners_leavers"]);
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/dashboards/widget-choices", None, &[]).await.0, StatusCode::FORBIDDEN, "DSH-02");

    // DB-02: a branch outside the target user's scope is refused; within it, the default applies to their dashboard.
    let wf = format!("/v1/dashboards/default-filters/{}", w.user_id);
    let (_, f, etag) = common::call(&app, &s, "GET", &wf, None, None).await;
    assert_eq!(f["branch_id"], Value::Null);
    assert_eq!(common::call(&app, &s, "PUT", &wf, Some(json!({ "branch_id": s.branch_id })), etag.as_deref()).await.0, StatusCode::UNPROCESSABLE_ENTITY,
               "an employee's own scope holds no branch");
    let hf = format!("/v1/dashboards/default-filters/{}", s.hr_user_id);
    let (_, _, etag) = common::call(&app, &s, "GET", &hf, None, None).await;
    let (st, b, _) = common::call(&app, &s, "PUT", &hf, Some(json!({ "branch_id": s.branch_id, "period": "current" })), etag.as_deref()).await;
    assert_eq!((st, b["period"].as_str()), (StatusCode::OK, Some("current")), "{b}");
    let (_, hr) = common::as_user(&app, &s.hr_token, "GET", "/v1/dashboards/me", None, &[]).await;
    assert_eq!(hr["filters"]["branch_id"], json!(s.branch_id));
    let (st, _, _) = common::call(&app, &s, "PUT", &hf, Some(json!({ "period": "next" })), None).await;
    assert_eq!(st, StatusCode::PRECONDITION_REQUIRED);

    // Every role's widgets run (the HR user also holds the other built-in roles for this).
    for (code, first) in [("payroll_officer", "periods_awaiting_lock"), ("manager", "team_attendance_today"), ("supervisor", "who_is_in_now")] {
        let role = Uuid::new_v4();
        sqlx::query("INSERT INTO iam.role (org_id, id, name, system_code) VALUES ($1, $2, $3, $3)").bind(s.org_id).bind(role).bind(code).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(s.hr_user_id).bind(role).execute(&pool).await.unwrap();
        let (st, d) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/dashboards/me?role={code}"), None, &[]).await;
        assert_eq!((st, d["widgets"][0]["code"].as_str()), (StatusCode::OK, Some(first)), "{code}: {d}");
        assert!(d["widgets"].as_array().unwrap().iter().all(|w| w["data"].is_object()), "{d}");
    }
    // A branch the caller has no employee in shows nothing, never more.
    let (_, none) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/dashboards/me?role=manager&branch_id={}", Uuid::new_v4()), None, &[]).await;
    assert_eq!(none["widgets"][0]["data"]["employees"], 0, "{none}");

    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &wf, None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}

/// TB-02 / DC-06: signed upload links under the organisation's path; types and sizes refused; names cannot escape.
/// VC-R4: a suspended organisation gets no upload link but still runs reports.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_links_and_suspension(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let ask = |name: &str, ct: &str, size: i64| json!({ "purpose": "request_attachment", "file_name": name, "content_type": ct, "size_bytes": size });
    let k = key();
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("../../../etc/passwd.pdf", "application/pdf", 1000)), &k).await;
    assert_eq!((st, b["method"].as_str()), (StatusCode::CREATED, Some("PUT")), "{b}");
    let path = b["file"]["path"].as_str().unwrap().to_string();
    assert!(path.starts_with(&format!("{}/uploads/request_attachment/", s.org_id)) && !path.contains(".."), "{path}");
    // The link binds type, size and "not there yet": an uploaded file can never be replaced through a new link.
    assert_eq!((b["headers"]["x-goog-if-generation-match"].as_str(), b["headers"]["x-goog-content-length-range"].as_str()), (Some("0"), Some("1,1000")), "{b}");
    assert!(b["upload_url"].as_str().unwrap().contains("signed=content-type;x-goog-content-length-range;x-goog-if-generation-match"), "{b}");
    // A retry replays the same path with a fresh link; the key with another body is a conflict.
    let (st, again) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("../../../etc/passwd.pdf", "application/pdf", 1000)), &k).await;
    assert_eq!((st, again["file"]["path"].as_str()), (StatusCode::CREATED, Some(path.as_str())), "{again}");
    assert!(again["upload_url"].is_string());
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("other.pdf", "application/pdf", 1000)), &k).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("IDEMPOTENCY_CONFLICT")), "{b}");
    let (_, fresh) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("../../../etc/passwd.pdf", "application/pdf", 1000)), &key()).await;
    assert_ne!(fresh["file"]["path"], json!(path), "a new key never reaches an existing object");
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("x.exe", "application/x-msdownload", 10)), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some("FILE_TYPE_NOT_ALLOWED")), "{b}");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("big.pdf", "application/pdf", 6 * 1024 * 1024)), &key()).await.0,
               StatusCode::UNPROCESSABLE_ENTITY, "DC-06: 5 MB");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("a.pdf", "application/pdf", 10)), &[]).await.0, StatusCode::PRECONDITION_REQUIRED);

    sqlx::query("UPDATE platform.organisation SET state = 'suspended' WHERE id = $1").bind(s.org_id).execute(&pool).await.unwrap();
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/files/upload-links", Some(ask("a.pdf", "application/pdf", 10)), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("ORG_SUSPENDED")), "{b}");
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/reports/people/runs", Some(json!({ "from": today, "to": today, "format": "csv" })), &key()).await;
    assert_eq!(st, StatusCode::ACCEPTED, "reports run while suspended (VC-R4): {b}");
}
