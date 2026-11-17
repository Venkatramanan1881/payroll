//! Regression tests for the slice-2 final-review findings (one or more per finding).

mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Locks `employee`'s attendance for [from, to) the way a period lock does.
async fn lock(pool: &PgPool, org: Uuid, employee: Uuid, from: &str, to: &str) {
    let (cycle, period) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO period.attendance_cycle (org_id, id, owner_kind, owner_id, valid) VALUES ($1, $2, 'organisation', $1, daterange('2025-01-01', NULL))").bind(org).bind(cycle).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO period.attendance_period (org_id, id, cycle_id, period_start, period_end) VALUES ($1, $2, $3, $4::date, ($5::date - 1))").bind(org).bind(period).bind(cycle).bind(from).bind(to).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO period.lock_coverage (org_id, employee_id, dates, period_id) VALUES ($1, $2, daterange($3::date, $4::date), $5)").bind(org).bind(employee).bind(from).bind(to).bind(period).execute(pool).await.unwrap();
}

async fn emp(app: &axum::Router, s: &common::SeedOrg) -> Uuid {
    common::create_employee(app, s, "Asha").await["id"].as_str().unwrap().parse().unwrap()
}

async fn shift(app: &axum::Router, s: &common::SeedOrg, code: &str, from: &str, def: Value) -> Value {
    let (st, b, _) = common::call(app, s, "POST", "/v1/shifts", Some(json!({ "code": code, "name": code, "effective_from": from, "definition": def })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    b
}

fn day() -> Value {
    json!({ "shift_type": "fixed", "start_time": "09:00", "end_time": "18:00" })
}

async fn post(app: &axum::Router, s: &common::SeedOrg, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let (st, b, _) = common::call(app, s, "POST", uri, body, None).await;
    (st, b)
}

// Critical 1: a version published for a future date (state 'scheduled') is frozen like an in-force one.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn scheduled_shift_version_is_frozen(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let v = shift(&app, &s, "GEN", "2099-01-01", day()).await["versions"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(post(&app, &s, &format!("/v1/shift-versions/{v}/publish"), None).await.1["state"], "scheduled");
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/shift-versions/{v}"), None, None).await;
    let (st, b, _) = common::call(&app, &s, "PATCH", &format!("/v1/shift-versions/{v}"), Some(json!({ "end_time": "17:00:00" })), etag.as_deref()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")), "{b}");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn scheduled_policy_version_is_frozen(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let (_, set) = post(&app, &s, "/v1/policy-sets", Some(json!({ "name": "P", "effective_from": "2099-01-01" }))).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(post(&app, &s, &format!("/v1/policy-versions/{v}/publish"), None).await.1["state"], "scheduled");
    let (_, _, etag) = common::call(&app, &s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    let (st, b, _) = common::call(&app, &s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/LA"), Some(json!({ "schema_version": 1, "settings": { "LA-03": 5 } })), etag.as_deref()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")), "PUT: {b}");
    let (st, b) = post(&app, &s, &format!("/v1/policy-versions/{v}/rule-groups/LA/switch"), Some(json!({ "enabled": false }))).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")), "switch: {b}");
    let (st, b, _) = common::call(&app, &s, "PATCH", &format!("/v1/policy-versions/{v}"), Some(json!({ "reason": "x" })), etag.as_deref()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("VERSION_FROZEN")), "PATCH: {b}");
}

// Important 2: back-dating a policy version over a locked period is refused, not only an exactly-locked start date.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn backdated_policy_publish_over_lock_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let e = emp(&app, &s).await;
    lock(&pool, s.org_id, e, "2026-01-01", "2026-02-01").await;
    let (_, set) = post(&app, &s, "/v1/policy-sets", Some(json!({ "name": "P", "effective_from": "2025-12-15" }))).await;
    let (st, b) = post(&app, &s, &format!("/v1/policy-versions/{}/publish", set["versions"][0]["id"].as_str().unwrap()), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("PERIOD_LOCKED")), "{b}");
}

// Important 3: other back-dated writes that change locked days are refused.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn backdated_writes_over_lock_409(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let e = emp(&app, &s).await;
    // A shift published before the lock exists, for the schedule test below.
    let gen = shift(&app, &s, "GEN", "2025-06-01", day()).await;
    post(&app, &s, &format!("/v1/shift-versions/{}/publish", gen["versions"][0]["id"].as_str().unwrap()), None).await;
    let (_, set) = post(&app, &s, "/v1/policy-sets", Some(json!({ "name": "P", "effective_from": "2025-06-01" }))).await;
    lock(&pool, s.org_id, e, "2026-01-01", "2026-02-01").await;
    let locked = |r: (StatusCode, Value)| assert_eq!((r.0, r.1["code"].clone()), (StatusCode::CONFLICT, json!("PERIOD_LOCKED")), "{}", r.1);

    locked(post(&app, &s, "/v1/employee-schedules", Some(json!({ "employee_id": e, "source": "fixed_shift", "shift_id": gen["id"], "valid_from": "2026-01-15", "reason": "x" }))).await);
    let later = shift(&app, &s, "EVE", "2026-01-10", day()).await;
    locked(post(&app, &s, &format!("/v1/shift-versions/{}/publish", later["versions"][0]["id"].as_str().unwrap()), None).await);
    locked(post(&app, &s, "/v1/policy-assignments", Some(json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-10", "reason": "x" }))).await);
    let (_, cal) = post(&app, &s, "/v1/holiday-calendars", Some(json!({ "name": "TN" }))).await;
    locked(post(&app, &s, &format!("/v1/holiday-calendars/{}/holidays", cal["id"].as_str().unwrap()), Some(json!({ "holiday_date": "2026-01-15", "name": "Pongal", "kind": "festival", "reason": "x" }))).await);
    // Writes that only touch open dates still work.
    let (st, b) = post(&app, &s, "/v1/policy-assignments", Some(json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-03-01", "reason": "x" }))).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    // Ending an assignment that covers the locked month, on a date inside it, is refused.
    let old: Uuid = sqlx::query_scalar("INSERT INTO policy.policy_assignment (org_id, policy_set_id, level, valid, reason) VALUES ($1, $2, 'organisation', daterange('2025-06-01', '2026-02-15'), 'pre-lock') RETURNING id")
        .bind(s.org_id).bind(set["id"].as_str().unwrap().parse::<Uuid>().unwrap()).fetch_one(&pool).await.unwrap();
    locked(post(&app, &s, &format!("/v1/policy-assignments/{old}/end"), Some(json!({ "valid_to": "2026-01-10", "reason": "x" }))).await);
}

// Important 4: an alternating pattern also honours its weekly days.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alternating_with_weekly_days(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let gen = shift(&app, &s, "GEN", "2026-01-01", day()).await;
    post(&app, &s, &format!("/v1/shift-versions/{}/publish", gen["versions"][0]["id"].as_str().unwrap()), None).await;
    let (st, pat) = post(&app, &s, "/v1/weekly-off-patterns", Some(json!({ "name": "Sun + 2/4 Sat", "kind": "alternating", "weekly_days": [7], "alt_day": 6, "alt_weeks": [2, 4] }))).await;
    assert_eq!(st, StatusCode::CREATED, "{pat}");
    let e = emp(&app, &s).await;
    post(&app, &s, "/v1/employee-schedules", Some(json!({ "employee_id": e, "source": "fixed_shift", "shift_id": gen["id"], "weekly_off_pattern_id": pat["id"], "valid_from": "2026-01-01", "reason": "x" }))).await;
    let mut tx = db::begin_org(&pool, s.org_id).await.unwrap();
    let off = |d: &str| d.parse::<NaiveDate>().unwrap();
    assert!(domain_sched::schedule::expected_day(&mut tx, e, off("2026-11-15"), s.branch_id).await.unwrap().weekly_off, "Sunday");
    assert!(domain_sched::schedule::expected_day(&mut tx, e, off("2026-11-14"), s.branch_id).await.unwrap().weekly_off, "2nd Saturday");
    assert!(!domain_sched::schedule::expected_day(&mut tx, e, off("2026-11-07"), s.branch_id).await.unwrap().weekly_off, "1st Saturday");
}

// Important 5: SH-06 = shift_end_date is refused until the resolver supports it.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn night_end_date_refused_422(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool);
    let def = json!({ "shift_type": "night", "start_time": "22:00", "end_time": "06:00", "is_night_shift": true, "night_attendance_date": "shift_end_date" });
    let (st, _, _) = common::call(&app, &s, "POST", "/v1/shifts", Some(json!({ "code": "N", "name": "N", "effective_from": "2026-01-01", "definition": def })), None).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
}

// Important 6: holiday-calendar writes respect the caller's branch scope.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn calendar_write_outside_scope_404(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')").bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    let app = common::app(pool.clone());
    let (_, mine) = post(&app, &s, "/v1/holiday-calendars", Some(json!({ "name": "Mine" }))).await;
    let (_, theirs) = post(&app, &s, "/v1/holiday-calendars", Some(json!({ "name": "Theirs" }))).await;
    sqlx::query("UPDATE org.branch SET holiday_calendar_id = $2 WHERE id = $1").bind(s.branch_id).bind(mine["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&pool).await.unwrap();
    sqlx::query("UPDATE org.branch SET holiday_calendar_id = $2 WHERE id = $1").bind(other).bind(theirs["id"].as_str().unwrap().parse::<Uuid>().unwrap()).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)").bind(s.org_id).bind(s.hr_user_id).bind(s.branch_id).execute(&pool).await.unwrap();
    let holiday = json!({ "holiday_date": "2026-03-10", "name": "Local", "kind": "festival", "reason": "x" });
    assert_eq!(post(&app, &s, &format!("/v1/holiday-calendars/{}/holidays", mine["id"].as_str().unwrap()), Some(holiday.clone())).await.0, StatusCode::CREATED);
    assert_eq!(post(&app, &s, &format!("/v1/holiday-calendars/{}/holidays", theirs["id"].as_str().unwrap()), Some(holiday)).await.0, StatusCode::NOT_FOUND);
}

// Important 7: same-organisation employee outside the HR Manager's branch scope → roster write 404, nothing written.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn roster_same_org_out_of_scope_404(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let e = emp(&app, &s).await;
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')").bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(s.hr_user_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)").bind(s.org_id).bind(s.hr_user_id).bind(other).execute(&pool).await.unwrap();
    let (st, _) = post(&app, &s, "/v1/roster-entries/set", Some(json!({ "entries": [ { "employee_id": e, "roster_date": "2026-03-03", "is_weekly_off": true } ] }))).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM sched.roster_entry").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}
