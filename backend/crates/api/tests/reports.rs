//! Reports, jobs and audit views (slice 18).

mod common;

use axum::http::StatusCode;
use chrono::{Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

fn today() -> NaiveDate {
    (Utc::now() + Duration::minutes(330)).date_naive()
}

/// The stored file behind a finished job.
fn file(store: &common::MemoryStore, job: &Value) -> Vec<u8> {
    let files = store.0.lock().unwrap();
    let id = job["id"].as_str().unwrap();
    files.iter().find(|(k, _)| k.contains(id)).map(|(_, v)| v.clone()).expect("the report file is stored")
}

/// Every report runs as a finished job with its file; R1 lists the punched days; scope, grants and filters hold.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn reports_run_as_jobs_with_files(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let store = Arc::new(common::MemoryStore::default());
    let app = api::router(common::state_with_store(pool.clone(), store.clone()));
    let (_, set, _) = common::call(&app, &s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    common::call(&app, &s, "POST", &format!("/v1/policy-versions/{}/publish", set["versions"][0]["id"].as_str().unwrap()), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(&app, &s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
    let w = common::seed_worker(&pool, &app, &s, "Asha, \"the boss\"").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (d1, d2) = (today() - Duration::days(3), today() - Duration::days(2));
    let mut entries = vec![];
    for d in [d1, d2] {
        entries.push(json!({ "employee_id": w.employee_id, "event_at": d.and_hms_opt(3, 30, 0).unwrap().and_utc() }));
        entries.push(json!({ "employee_id": w.employee_id, "event_at": d.and_hms_opt(12, 30, 0).unwrap().and_utc() }));
    }
    let (st, r) = common::as_user(&app, &s.hr_token, "POST", "/v1/manual-attendance", Some(json!({ "branch_id": s.branch_id, "reason": "device down", "entries": entries })), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{r}");

    let range = json!({ "from": today() - Duration::days(30), "to": today(), "format": "csv" });
    let run = |uri: &str, body: Value| {
        let (app, tok, uri) = (app.clone(), s.hr_token.clone(), uri.to_string());
        async move { common::as_user(&app, &tok, "POST", &uri, Some(body), &key()).await }
    };
    // R1: one line per punched day, the name quoted as CSV needs.
    let (st, job) = run("/v1/reports/attendance/R1/runs", range.clone()).await;
    assert_eq!((st, job["state"].as_str(), job["kind"].as_str()), (StatusCode::ACCEPTED, Some("succeeded"), Some("report:R1")), "{job}");
    let csv = String::from_utf8(file(&store, &job)).unwrap();
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].starts_with("date,employee_code,name,status"), "{csv}");
    for d in [d1, d2] {
        let line = lines.iter().find(|l| l.starts_with(&format!("{d},"))).unwrap_or_else(|| panic!("{d} missing: {csv}"));
        assert!(line.contains("\"Asha, \"\"the boss\"\"\",present,09:00,18:00,"), "{line}");
    }

    // Every other report runs (shapes are checked by the SQL itself running).
    for (uri, report) in [("attendance/R2", "R2"), ("attendance/R3", "R3"), ("attendance/R4", "R4"), ("attendance/R5", "R5"), ("leave/R6", "R6"), ("leave/R7", "R7"),
                          ("leave/R8", "R8"), ("leave/R9", "R9"), ("people", "R10"), ("control/R25", "R25"), ("control/R26", "R26"), ("audit-trail", "R27")] {
        let (st, job) = run(&format!("/v1/reports/{uri}/runs"), range.clone()).await;
        assert_eq!((st, job["kind"].as_str()), (StatusCode::ACCEPTED, Some(&*format!("report:{report}"))), "{report}: {job}");
    }
    let (_, people) = run("/v1/reports/people/runs", range.clone()).await;
    assert!(!String::from_utf8(file(&store, &people)).unwrap().contains(&w.employee_id.to_string()), "codes, never ids");

    // xlsx is the default format.
    let (st, x) = run("/v1/reports/attendance/R2/runs", json!({ "from": d1, "to": d2 })).await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(&file(&store, &x)[..2], b"PK", "an xlsx file is a zip");

    // Filters and refusals.
    assert_eq!(run("/v1/reports/attendance/R1/runs", json!({ "from": d2, "to": d1 })).await.0, StatusCode::UNPROCESSABLE_ENTITY, "from after to");
    assert_eq!(run("/v1/reports/payroll-input-summary/runs", range.clone()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "R13 needs a period");
    assert_eq!(run("/v1/reports/attendance/R9/runs", range.clone()).await.0, StatusCode::NOT_FOUND, "R9 is a leave report");
    let (_, none) = run("/v1/reports/attendance/R1/runs", json!({ "from": d1, "to": d2, "format": "csv", "employee_ids": [Uuid::new_v4()] })).await;
    assert_eq!(String::from_utf8(file(&store, &none)).unwrap().lines().count(), 1, "an unknown employee filters everything out");
    assert_eq!(common::as_user(&app, &w.token, "POST", "/v1/reports/attendance/R1/runs", Some(range.clone()), &key()).await.0, StatusCode::FORBIDDEN, "RPT-01");

    // Jobs: own only; a fresh link on each poll.
    let (st, polled) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/jobs/{}", job["id"].as_str().unwrap()), None, &[]).await;
    assert_eq!((st, polled["state"].as_str()), (StatusCode::OK, Some("succeeded")));
    assert!(polled["download_url"].as_str().unwrap().contains("/reports/"), "{polled}");
    assert_eq!(common::as_user(&app, &w.token, "GET", &format!("/v1/jobs/{}", job["id"].as_str().unwrap()), None, &[]).await.0, StatusCode::NOT_FOUND);
    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/jobs/{}", job["id"].as_str().unwrap()), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
    let (_, theirs) = common::as_user(&app, &other.hr_token, "POST", "/v1/reports/attendance/R1/runs", Some(json!({ "from": d1, "to": d2, "format": "csv", "employee_ids": [w.employee_id] })), &key()).await;
    assert_eq!(String::from_utf8(file(&store, &theirs)).unwrap().lines().count(), 1, "TB-01: another organisation's employee is not reported");
}

/// The audit views are read-only, filtered, and viewing the trail is itself audited.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn audit_views(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (_, set, _) = common::call(&app, &s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    let doc = json!({ "schema_version": 1, "settings": { "LA-03": 15 } });
    assert_eq!(common::call(&app, &s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/LA"), Some(doc), etag.as_deref()).await.0, StatusCode::OK);

    let (st, all) = common::as_user(&app, &s.hr_token, "GET", "/v1/audit/entries", None, &[]).await;
    assert_eq!(st, StatusCode::OK, "{all}");
    assert!(!all["items"].as_array().unwrap().is_empty());
    let (_, one) = common::as_user(&app, &s.hr_token, "GET", "/v1/audit/entries?action=policy_rule_group.put", None, &[]).await;
    assert_eq!(one["items"].as_array().map(Vec::len), Some(1), "{one}");
    let viewed: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE action = 'audit.viewed'").fetch_one(&pool).await.unwrap();
    assert_eq!(viewed, 2, "each view is audited");
    let (_, changes) = common::as_user(&app, &s.hr_token, "GET", "/v1/audit/settings-changes?setting_ref=LA-03", None, &[]).await;
    assert_eq!(changes["items"].as_array().map(Vec::len), Some(1), "{changes}");
    let (_, none) = common::as_user(&app, &s.hr_token, "GET", "/v1/audit/settings-changes?setting_ref=PL-06", None, &[]).await;
    assert_eq!(none["items"].as_array().map(Vec::len), Some(0));
    assert_eq!(common::as_user(&app, &s.hr_token, "GET", "/v1/audit/entries?sort=up", None, &[]).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/audit/entries", None, &[]).await.0, StatusCode::FORBIDDEN, "ADT-01");
    // An Auditor scoped to one branch reads no audit trail: entries carry no branch (it would see every branch).
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'branch auditor')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    for op in ["ADT-01", "ADT-02", "RPT-06"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'scoped')").bind(s.org_id).bind(role).bind(op).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(w.user_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)").bind(s.org_id).bind(w.user_id).bind(s.branch_id).execute(&pool).await.unwrap();
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/audit/entries", None, &[]).await.0, StatusCode::FORBIDDEN, "branch-scoped");
    assert_eq!(common::as_user(&app, &w.token, "GET", "/v1/audit/settings-changes", None, &[]).await.0, StatusCode::FORBIDDEN);
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    assert_eq!(common::as_user(&app, &w.token, "POST", "/v1/reports/audit-trail/runs", Some(json!({ "from": today, "to": today })), &key()).await.0, StatusCode::FORBIDDEN);

    let other = common::seed_org(&pool).await;
    let (_, theirs) = common::as_user(&app, &other.hr_token, "GET", "/v1/audit/entries?action=policy_rule_group.put", None, &[]).await;
    assert_eq!(theirs["items"].as_array().map(Vec::len), Some(0), "TB-01");
}
