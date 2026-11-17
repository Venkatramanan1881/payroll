//! Policy assignments and the D-03 policy resolver.

mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use domain_policy::assignment::{resolve_policy, ResolvedPolicy};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// A policy set with one version, published (effective 2026-01-01). Returns (set_id, version_id).
async fn published_set(app: &axum::Router, s: &common::SeedOrg, name: &str) -> (Uuid, Uuid) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": name, "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    (set["id"].as_str().unwrap().parse().unwrap(), v.parse().unwrap())
}

async fn assign(app: &axum::Router, s: &common::SeedOrg, set: Uuid, level: &str, target: Option<(&str, Uuid)>) -> (StatusCode, Value) {
    let mut body = json!({ "policy_set_id": set, "level": level, "valid_from": "2026-01-01", "reason": "setup" });
    if let Some((field, id)) = target {
        body[field] = json!(id);
    }
    let (st, b, _) = common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await;
    (st, b)
}

async fn resolve(pool: &PgPool, s: &common::SeedOrg, emp: Uuid, date: &str) -> Option<ResolvedPolicy> {
    let mut tx = db::begin_org(pool, s.org_id).await.unwrap();
    resolve_policy(&mut tx, emp, date.parse::<NaiveDate>().unwrap(), s.branch_id).await.unwrap()
}

async fn employee_with_department(app: &axum::Router, pool: &PgPool, s: &common::SeedOrg) -> (Uuid, Uuid) {
    let emp: Uuid = common::create_employee(app, s, "Asha").await["id"].as_str().unwrap().parse().unwrap();
    let dept = Uuid::new_v4();
    sqlx::query("INSERT INTO org.list_entry (org_id, id, kind, name) VALUES ($1, $2, 'department', 'Production')").bind(s.org_id).bind(dept).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, valid, list_entry_id) VALUES ($1, $2, 'department', daterange('2026-01-01', NULL), $3)")
        .bind(s.org_id).bind(emp).bind(dept).execute(pool).await.unwrap();
    (emp, dept)
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn org_level_applies_when_nothing_more_specific(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (set, version) = published_set(&app, &s, "Org default").await;
    let (st, b) = assign(&app, &s, set, "organisation", None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let emp: Uuid = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(resolve(&pool, &s, emp, "2026-03-02").await, Some(ResolvedPolicy { policy_set_id: set, version_id: Some(version), level: "organisation".into() }));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn department_beats_branch_d03(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (branch_set, _) = published_set(&app, &s, "Branch").await;
    let (dept_set, _) = published_set(&app, &s, "Department").await;
    let (emp, dept) = employee_with_department(&app, &pool, &s).await;
    assign(&app, &s, branch_set, "branch", Some(("branch_id", s.branch_id))).await;
    assign(&app, &s, dept_set, "department", Some(("list_entry_id", dept))).await;
    let r = resolve(&pool, &s, emp, "2026-03-02").await.unwrap();
    assert_eq!((r.policy_set_id, r.level.as_str()), (dept_set, "department"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn employee_beats_everything(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (org_set, _) = published_set(&app, &s, "Org").await;
    let (dept_set, _) = published_set(&app, &s, "Dept").await;
    let (emp_set, _) = published_set(&app, &s, "Personal").await;
    let (emp, dept) = employee_with_department(&app, &pool, &s).await;
    assign(&app, &s, org_set, "organisation", None).await;
    assign(&app, &s, dept_set, "department", Some(("list_entry_id", dept))).await;
    assign(&app, &s, emp_set, "employee", Some(("employee_id", emp))).await;
    assert_eq!(resolve(&pool, &s, emp, "2026-03-02").await.unwrap().policy_set_id, emp_set);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn shift_level_uses_rostered_shift(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (dept_set, _) = published_set(&app, &s, "Dept").await;
    let (night_set, _) = published_set(&app, &s, "Night workers").await;
    let (emp, dept) = employee_with_department(&app, &pool, &s).await;
    let (_, sh, _) = common::call(&app, &s, "POST", "/v1/shifts", Some(json!({ "code": "NGT", "name": "Night", "effective_from": "2026-01-01",
        "definition": { "shift_type": "night", "start_time": "22:00", "end_time": "06:00", "is_night_shift": true } })), None).await;
    let shift: Uuid = sh["id"].as_str().unwrap().parse().unwrap();
    common::call(&app, &s, "POST", &format!("/v1/shift-versions/{}/publish", sh["versions"][0]["id"].as_str().unwrap()), None, None).await;
    common::call(&app, &s, "POST", "/v1/roster-entries/set", Some(json!({ "entries": [ { "employee_id": emp, "roster_date": "2026-03-03", "is_weekly_off": false, "shift_id": shift } ] })), None).await;
    assign(&app, &s, dept_set, "department", Some(("list_entry_id", dept))).await;
    assign(&app, &s, night_set, "shift", Some(("shift_id", shift))).await;
    assert_eq!(resolve(&pool, &s, emp, "2026-03-03").await.unwrap().policy_set_id, night_set, "rostered on the night shift");
    assert_eq!(resolve(&pool, &s, emp, "2026-03-04").await.unwrap().policy_set_id, dept_set, "not rostered: department applies");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn version_by_effective_date_not_state(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (set, v1) = published_set(&app, &s, "Org").await;
    assign(&app, &s, set, "organisation", None).await;
    // A second version published for a future date is stored as 'scheduled'; once its date arrives it applies anyway.
    let (_, v2, _) = common::call(&app, &s, "POST", &format!("/v1/policy-sets/{set}/versions"), Some(json!({ "effective_from": "2099-01-01" })), None).await;
    let v2: Uuid = v2["id"].as_str().unwrap().parse().unwrap();
    common::call(&app, &s, "POST", &format!("/v1/policy-versions/{v2}/publish"), None, None).await;
    let emp: Uuid = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(resolve(&pool, &s, emp, "2026-03-02").await.unwrap().version_id, Some(v1));
    assert_eq!(resolve(&pool, &s, emp, "2099-02-01").await.unwrap().version_id, Some(v2), "scheduled state, but in force by date");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn no_assignment_none(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let emp: Uuid = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(resolve(&pool, &s, emp, "2026-03-02").await, None);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn overlapping_assignment_same_target_422_and_end(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (a, _) = published_set(&app, &s, "A").await;
    let (b, _) = published_set(&app, &s, "B").await;
    let (_, first) = assign(&app, &s, a, "organisation", None).await;
    assert_eq!(assign(&app, &s, b, "organisation", None).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, ended, _) = common::call(&app, &s, "POST", &format!("/v1/policy-assignments/{}/end", first["id"].as_str().unwrap()),
        Some(json!({ "valid_to": "2026-06-01", "reason": "replaced" })), None).await;
    assert_eq!((st, ended["valid_to"].as_str()), (StatusCode::OK, Some("2026-06-01")), "{ended}");
    let (_, list) = common::send(&app, common::req("GET", &format!("/v1/policy-assignments?policy_set_id={a}"), &s.hr_token, None, &[])).await;
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
}
