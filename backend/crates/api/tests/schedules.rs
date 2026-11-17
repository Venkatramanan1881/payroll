mod common;

use axum::http::StatusCode;
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

/// A published shift; returns (shift_id, version_id).
async fn shift(app: &axum::Router, s: &common::SeedOrg, code: &str, start: &str, end: &str, night: bool) -> (String, String) {
    let def = if night {
        json!({ "shift_type": "night", "start_time": start, "end_time": end, "is_night_shift": true })
    } else {
        json!({ "shift_type": "fixed", "start_time": start, "end_time": end })
    };
    let (st, b, _) = common::call(app, s, "POST", "/v1/shifts", Some(json!({ "code": code, "name": code, "effective_from": "2026-01-01", "definition": def })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let v = b["versions"][0]["id"].as_str().unwrap().to_string();
    common::call(app, s, "POST", &format!("/v1/shift-versions/{v}/publish"), None, None).await;
    (b["id"].as_str().unwrap().to_string(), v)
}

async fn assign(app: &axum::Router, s: &common::SeedOrg, emp: &str, body: Value) -> (StatusCode, Value) {
    let mut b = body;
    b["employee_id"] = json!(emp);
    let (st, r, _) = common::call(app, s, "POST", "/v1/employee-schedules", Some(b), None).await;
    (st, r)
}

async fn expected(pool: &PgPool, s: &common::SeedOrg, emp: &str, date: &str, branch: Uuid) -> domain_sched::schedule::ExpectedDay {
    let mut tx = db::begin_org(pool, s.org_id).await.unwrap();
    domain_sched::schedule::expected_day(&mut tx, emp.parse().unwrap(), d(date), branch).await.unwrap()
}

async fn calendar_for(pool: &PgPool, org: Uuid, branch: Uuid, date: &str, name: &str) {
    let cal = Uuid::new_v4();
    sqlx::query("INSERT INTO sched.holiday_calendar (org_id, id, name) VALUES ($1, $2, $3)").bind(org).bind(cal).bind(format!("cal-{cal}")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO sched.holiday (org_id, calendar_id, holiday_date, name, kind) VALUES ($1, $2, $3::date, $4, 'festival')")
        .bind(org).bind(cal).bind(date).bind(name).execute(pool).await.unwrap();
    sqlx::query("UPDATE org.branch SET holiday_calendar_id = $2 WHERE id = $1").bind(branch).bind(cal).execute(pool).await.unwrap();
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn fixed_schedule_expected_shift(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (sid, vid) = shift(&app, &s, "GEN", "09:00", "18:00", false).await;
    let emp = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    let (st, b) = assign(&app, &s, &emp, json!({ "source": "fixed_shift", "shift_id": sid, "valid_from": "2026-01-01", "reason": "joining" })).await;
    assert_eq!(st, StatusCode::CREATED, "{b}");
    let day = expected(&pool, &s, &emp, "2026-03-02", s.branch_id).await;
    assert_eq!(day.shift_version_id, Some(vid.parse().unwrap()));
    assert!(!day.weekly_off && day.holiday.is_none());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn roster_overrides_fixed_on_that_date(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (gen, _) = shift(&app, &s, "GEN", "09:00", "18:00", false).await;
    let (eve, eve_v) = shift(&app, &s, "EVE", "14:00", "23:00", false).await;
    let emp = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assign(&app, &s, &emp, json!({ "source": "fixed_shift", "shift_id": gen, "valid_from": "2026-01-01", "reason": "x" })).await;
    let entries = json!({ "entries": [ { "employee_id": emp, "roster_date": "2026-03-03", "is_weekly_off": false, "shift_id": eve },
                                       { "employee_id": emp, "roster_date": "2026-03-04", "is_weekly_off": true } ] });
    let (st, b, _) = common::call(&app, &s, "POST", "/v1/roster-entries/set", Some(entries), None).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    assert_eq!(b["entries"].as_array().unwrap().len(), 2);
    assert_eq!(expected(&pool, &s, &emp, "2026-03-03", s.branch_id).await.shift_version_id, Some(eve_v.parse().unwrap()));
    assert!(expected(&pool, &s, &emp, "2026-03-04", s.branch_id).await.weekly_off);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn alternating_second_and_fourth_saturday_off(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (gen, _) = shift(&app, &s, "GEN", "09:00", "18:00", false).await;
    let (_, pat, _) = common::call(&app, &s, "POST", "/v1/weekly-off-patterns", Some(json!({ "name": "2/4 Sat", "kind": "alternating", "alt_day": 6, "alt_weeks": [2, 4] })), None).await;
    let emp = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assign(&app, &s, &emp, json!({ "source": "fixed_shift", "shift_id": gen, "weekly_off_pattern_id": pat["id"], "valid_from": "2026-01-01", "reason": "x" })).await;
    assert!(expected(&pool, &s, &emp, "2026-11-14", s.branch_id).await.weekly_off, "2nd Saturday of Nov 2026");
    assert!(!expected(&pool, &s, &emp, "2026-11-07", s.branch_id).await.weekly_off, "1st Saturday works");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn holiday_from_branch_calendar_d04(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'BR2', 'Branch 2')").bind(s.org_id).bind(other).execute(&pool).await.unwrap();
    calendar_for(&pool, s.org_id, s.branch_id, "2026-01-15", "Pongal").await;
    let app = common::app(pool.clone());
    let emp = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assert_eq!(expected(&pool, &s, &emp, "2026-01-15", s.branch_id).await.holiday.as_deref(), Some("Pongal"));
    assert_eq!(expected(&pool, &s, &emp, "2026-01-15", other).await.holiday, None, "punch branch has no such holiday");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn night_shift_and_holiday_same_attendance_date(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    calendar_for(&pool, s.org_id, s.branch_id, "2026-01-15", "Pongal").await;
    let app = common::app(pool.clone());
    let (night, night_v) = shift(&app, &s, "NGT", "22:00", "06:00", true).await;
    let emp = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assign(&app, &s, &emp, json!({ "source": "fixed_shift", "shift_id": night, "valid_from": "2026-01-01", "reason": "x" })).await;
    let day = expected(&pool, &s, &emp, "2026-01-15", s.branch_id).await;
    assert_eq!((day.shift_version_id, day.holiday.as_deref()), (Some(night_v.parse().unwrap()), Some("Pongal")));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn roster_entry_out_of_scope_404_nothing_written(pool: PgPool) {
    let a = common::seed_org(&pool).await;
    let b = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (gen, _) = shift(&app, &a, "GEN", "09:00", "18:00", false).await;
    let mine = common::create_employee(&app, &a, "Asha").await["id"].as_str().unwrap().to_string();
    let foreign = common::create_employee(&app, &b, "Bala").await["id"].as_str().unwrap().to_string();
    let entries = json!({ "entries": [ { "employee_id": mine, "roster_date": "2026-03-03", "is_weekly_off": false, "shift_id": gen },
                                       { "employee_id": foreign, "roster_date": "2026-03-03", "is_weekly_off": true } ] });
    let (st, _, _) = common::call(&app, &a, "POST", "/v1/roster-entries/set", Some(entries), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM sched.roster_entry").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0, "nothing written");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn my_roster_shows_only_self(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let (gen, _) = shift(&app, &s, "GEN", "09:00", "18:00", false).await;
    let me = common::create_employee(&app, &s, "Asha").await["id"].as_str().unwrap().to_string();
    assign(&app, &s, &me, json!({ "source": "fixed_shift", "shift_id": gen, "valid_from": "2026-01-01", "reason": "x" })).await;
    let user = common::seed_mobile_user(&pool, s.org_id, "+919800000001").await;
    sqlx::query("UPDATE iam.user_account SET employee_id = $2 WHERE id = $1").bind(user).bind(me.parse::<Uuid>().unwrap()).execute(&pool).await.unwrap();
    let role = Uuid::new_v4();
    sqlx::query("INSERT INTO iam.role (org_id, id, name, system_code) VALUES ($1, $2, 'Employee', 'employee')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'SCH-03', 'self')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(user).bind(role).execute(&pool).await.unwrap();
    let token = common::token(user, s.org_id);
    let (res, body) = common::send(&app, common::req("GET", "/v1/me/roster?from=2026-03-02&to=2026-03-03", &token, None, &[])).await;
    assert_eq!(res.status(), StatusCode::OK, "{body}");
    let days = body["items"].as_array().unwrap();
    assert_eq!(days.len(), 2);
    assert_eq!(days[0]["shift_code"], "GEN");
    assert_eq!(days[0]["start_time"], "09:00:00");
}
