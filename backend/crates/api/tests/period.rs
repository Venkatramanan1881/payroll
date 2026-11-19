//! Attendance periods, checklist, lock, unlock and totals (slice 8).

mod common;

use axum::http::StatusCode;
use chrono::{Datelike, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> [(&'static str, String); 1] {
    [("idempotency-key", Uuid::new_v4().to_string())]
}

/// The India date and the first day of the month `back` months before it.
fn month_start(back: i32) -> NaiveDate {
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let n = today.year() * 12 + today.month0() as i32 - back;
    NaiveDate::from_ymd_opt(n.div_euclid(12), n.rem_euclid(12) as u32 + 1, 1).unwrap()
}

async fn org_policy(app: &axum::Router, s: &common::SeedOrg) {
    policy_waiting(app, s, 0).await
}

/// DS-10: loss of pay at once (0), or absences left pending for `pending_days` waiting days.
async fn policy_waiting(app: &axum::Router, s: &common::SeedOrg, pending_days: i64) {
    policy_ds10(app, s, json!({ "action": if pending_days > 0 { "pending" } else { "loss_of_pay" }, "pending_days": pending_days })).await
}

/// An organisation policy from 2026-01-01 with this DS-10 document.
async fn policy_ds10(app: &axum::Router, s: &common::SeedOrg, ds10: Value) {
    let (_, set, _) = common::call(app, s, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" })), None).await;
    let v = set["versions"][0]["id"].as_str().unwrap().to_string();
    let (_, _, etag) = common::call(app, s, "GET", &format!("/v1/policy-versions/{v}"), None, None).await;
    let doc = json!({ "schema_version": 1, "settings": { "DS-10": ds10 } });
    let (st, b, _) = common::call(app, s, "PUT", &format!("/v1/policy-versions/{v}/rule-groups/DS"), Some(doc), etag.as_deref()).await;
    assert_eq!(st, StatusCode::OK, "{b}");
    common::call(app, s, "POST", &format!("/v1/policy-versions/{v}/publish"), None, None).await;
    let body = json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "setup" });
    assert_eq!(common::call(app, s, "POST", "/v1/policy-assignments", Some(body), None).await.0, StatusCode::CREATED);
}

/// An organisation-settings version from 2026-01-01 (period settings PATCH builds on it).
async fn org_settings(pool: &PgPool, org: Uuid) {
    let s = json!({ "financial_year": { "kind": "april_to_march" }, "week_start": "monday", "languages": ["en"], "currency": "INR",
            "employee_code_mode": "automatic", "mandatory_fields": [], "contact_self_edit": "off", "probation_months": 6,
            "rejoin_code": "new_code", "allowed_branches_mode": "master_only", "non_allowed_branch_punch": "record_and_flag",
            "other_branch_costing": "branch_worked" });
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(org).bind(s).execute(pool).await.unwrap();
}

async fn housekeeping(app: &axum::Router) -> Value {
    let req = axum::http::Request::post("/internal/jobs/period-housekeeping")
        .header("authorization", format!("Bearer {}", String::from_utf8_lossy(common::JOB_SECRET)))
        .body(axum::body::Body::empty())
        .unwrap();
    let (res, body) = common::send(app, req).await;
    assert_eq!(res.status(), StatusCode::OK, "{body}");
    body["results"][0]["summary"].clone()
}

/// A calendar-month cycle from two months back, housekept: returns (last month's period id, the current period id).
async fn periods(app: &axum::Router, s: &common::SeedOrg) -> (String, String) {
    let body = json!({ "owner_kind": "organisation", "owner_id": s.org_id, "frequency": "monthly", "start_kind": "calendar", "cutoff_day": 1, "valid_from": month_start(2) });
    let (st, c) = common::as_user(app, &s.hr_token, "POST", "/v1/period/cycles", Some(body), &key()).await;
    assert_eq!(st, StatusCode::CREATED, "{c}");
    housekeeping(app).await;
    let find = |list: &Value, start: NaiveDate| list["items"].as_array().unwrap().iter().find(|p| p["period_start"] == json!(start)).unwrap()["id"].as_str().unwrap().to_string();
    let (_, list) = common::as_user(app, &s.hr_token, "GET", "/v1/periods", None, &[]).await;
    (find(&list, month_start(1)), find(&list, month_start(0)))
}

async fn patch_settings(app: &axum::Router, s: &common::SeedOrg, body: Value) -> (StatusCode, Value) {
    let (_, _, etag) = common::call(app, s, "GET", "/v1/period/settings", None, None).await;
    let (st, b, _) = common::call(app, s, "PATCH", "/v1/period/settings", Some(body), etag.as_deref()).await;
    (st, b)
}

/// Review focus 5: periods are generated per calendar month and moved by the housekeeping job, idempotently.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cycles_generate_periods_and_housekeeping_moves_them(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    for (k, v) in [("frequency", json!("weekly")), ("owner_kind", json!("branch")), ("start_kind", json!("custom"))] {
        let mut body = json!({ "owner_kind": "organisation", "owner_id": s.org_id, "frequency": "monthly", "start_kind": "calendar", "valid_from": month_start(2) });
        body[k] = v;
        assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/period/cycles", Some(body), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "{k}");
    }
    let (last, current) = periods(&app, &s).await;
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", "/v1/periods", None, &[]).await;
    let states: Vec<&str> = list["items"].as_array().unwrap().iter().map(|p| p["state"].as_str().unwrap()).collect();
    let today = (Utc::now() + Duration::minutes(330)).date_naive();
    let now_state = if today.day() > 1 { "past_cut_off" } else { "open" };
    assert_eq!(states, [now_state, "awaiting_lock", "awaiting_lock"], "newest first: {list}");
    let (_, p) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}"), None, &[]).await;
    assert_eq!((p["period_end"].as_str(), p["state_history"].as_array().map(Vec::len)), (Some(&*(month_start(0) - Duration::days(1)).to_string()), Some(2)), "{p}");
    let again = housekeeping(&app).await;
    assert_eq!((again["periods_generated"].as_i64(), again["moved"]["awaiting_lock"].as_i64(), again["moved"]["past_cut_off"].as_i64()), (Some(0), Some(0), Some(0)), "{again}");
    let body = json!({ "owner_kind": "organisation", "owner_id": s.org_id, "frequency": "monthly", "start_kind": "calendar", "valid_from": month_start(0) });
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/period/cycles", Some(body), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY, "cycles never overlap");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{current}/lock"), None, &key()).await.0, StatusCode::CONFLICT, "not before the end");
    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/periods/{last}"), None, &[]).await.0, StatusCode::NOT_FOUND);
}

/// Review focus 1 and 2: PL-05 block refuses with the items; allow-as-absent locks, freezing days and totals.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn lock_blocks_on_pending_then_freezes_days_and_totals(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let first = month_start(1);
    // Three absent days with results (DS-10 loss of pay), and a pending regularisation on a fourth.
    let body = json!({ "from": first, "to": first + Duration::days(2), "employee_ids": [w.employee_id], "reason": "check" });
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await.0, StatusCode::ACCEPTED);
    let pending_day = first + Duration::days(5);
    sqlx::query(
        "INSERT INTO attendance.regularisation_request (org_id, employee_id, attendance_date, request_type, proposed_in_at, proposed_out_at, reason, requested_by_user_id)
         VALUES ($1, $2, $3, 'whole_day', $4, $5, 'x', $6)",
    )
    .bind(s.org_id).bind(w.employee_id).bind(pending_day)
    .bind(pending_day.and_hms_opt(3, 30, 0).unwrap().and_utc()).bind(pending_day.and_hms_opt(12, 30, 0).unwrap().and_utc()).bind(w.user_id)
    .execute(&pool).await.unwrap();

    let (_, provisional) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    let t = &provisional["items"][0];
    assert_eq!((t["basis"].as_str(), t["deduction_days"].as_str()), (Some("provisional"), Some("3.00")), "{provisional}");
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/checklist"), None, &[]).await;
    assert_eq!((list["counts"]["pending_request"].as_i64(), list["items"][0]["date"].as_str()), (Some(1), Some(&*pending_day.to_string())), "{list}");

    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await;
    assert_eq!((st, b["code"].as_str(), b["errors"].as_array().map(Vec::len)), (StatusCode::CONFLICT, Some("PENDING_ITEMS_BLOCK_LOCK"), Some(1)), "{b}");

    let (st, set) = patch_settings(&app, &s, json!({ "pending_at_lock": "allow_as_absent", "reason": "month end" })).await;
    assert_eq!((st, set["pending_at_lock"].as_str(), set["unlock"].as_str()), (StatusCode::OK, Some("allow_as_absent"), Some("never")), "{set}");
    let (_, org) = common::as_user(&app, &s.hr_token, "GET", "/v1/organisation-settings", None, &[]).await;
    assert!(org["settings"].get("period").is_none(), "period settings are not OrganisationSettings: {org}");

    let (st, locked) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), Some(json!({ "comment": "October" })), &key()).await;
    assert_eq!((st, locked["state"].as_str()), (StatusCode::OK, Some("locked")), "{locked}");
    let (st, twice) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await;
    assert_eq!((st, &twice["locked_at"]), (StatusCode::OK, &locked["locked_at"]), "P6: no second effect");

    let (_, frozen) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals?employee_id={}", w.employee_id), None, &[]).await;
    let t = &frozen["items"][0];
    let days = (month_start(0) - first).num_days();
    assert_eq!((t["basis"].as_str(), t["deduction_days"].as_str(), t["payable_days"].as_str()), (Some("frozen"), Some("4.00"), Some(&*format!("{}.00", days - 4))), "{frozen}");
    assert_eq!(t["deduction_lines"][0]["explanation_refs"].as_array().map(Vec::len), Some(4), "three results and the PL-05 day");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM period.period_totals").fetch_one(&pool).await.unwrap();
    assert_eq!(rows, 1);

    // BR-06: the days are locked and nothing new is written for them.
    let states: Vec<String> = sqlx::query_scalar("SELECT DISTINCT processing_state FROM attendance.attendance_day WHERE attendance_date < $1").bind(month_start(0)).fetch_all(&pool).await.unwrap();
    assert_eq!(states, ["locked"]);
    let body = json!({ "from": first, "to": first + Duration::days(2), "employee_ids": [w.employee_id], "reason": "again" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    let results: i64 = sqlx::query_scalar("SELECT count(*) FROM attendance.day_result").fetch_one(&pool).await.unwrap();
    assert_eq!(results, 3);
    let (_, snap) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/checklist"), None, &[]).await;
    assert_eq!((snap["is_snapshot"].as_bool(), snap["counts"]["pending_request"].as_i64()), (Some(true), Some(1)), "{snap}");

    // BR-06: nobody can be hired into the locked month afterwards.
    let late = json!({ "full_name": "Ravi", "joining_date": first + Duration::days(3), "employment_status": "active", "master_branch_id": s.branch_id });
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", "/v1/employees", Some(late), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("PERIOD_LOCKED")), "{b}");
    // AC-C12-04.9: another organisation's periods are untouched.
    let other = common::seed_org(&pool).await;
    let (_, theirs) = common::as_user(&app, &other.hr_token, "GET", "/v1/periods", None, &[]).await;
    assert_eq!(theirs["items"], json!([]));
}

/// Review fix: absences still waiting under DS-10, and every day of a multi-day pending leave, are pending items (PL-05).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn waiting_absences_and_multi_day_leave_are_pending(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    policy_waiting(&app, &s, 60).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let first = month_start(1);
    // The last two days of the month: always within the 60 waiting days.
    let end = month_start(0) - Duration::days(1);
    let body = json!({ "from": end - Duration::days(1), "to": end, "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    let version = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": false, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 30, "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true });
    let (st, t, _) = common::call(&app, &s, "POST", "/v1/leave/types", Some(json!({ "code": "UL", "name": "Unpaid", "version": version })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    let leave = Uuid::new_v4();
    let from = first + Duration::days(10);
    sqlx::query("INSERT INTO leave.leave_request (org_id, id, employee_id, leave_type_id, from_date, to_date, state, requested_by) VALUES ($1, $2, $3, $4, $5, $6, 'pending_l1', $7)")
        .bind(s.org_id).bind(leave).bind(w.employee_id).bind(t["id"].as_str().unwrap().parse::<Uuid>().unwrap()).bind(from).bind(from + Duration::days(2)).bind(w.user_id)
        .execute(&pool).await.unwrap();
    for n in 0..3 {
        sqlx::query("INSERT INTO leave.leave_request_day (org_id, leave_request_id, leave_date, fraction) VALUES ($1, $2, $3, 1.0)")
            .bind(s.org_id).bind(leave).bind(from + Duration::days(n)).execute(&pool).await.unwrap();
    }
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/checklist"), None, &[]).await;
    assert_eq!((list["counts"]["flagged_day"].as_i64(), list["counts"]["pending_request"].as_i64()), (Some(2), Some(1)), "{list}");
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await;
    assert_eq!((st, b["errors"].as_array().map(Vec::len)), (StatusCode::CONFLICT, Some(3)), "{b}");
    assert_eq!(patch_settings(&app, &s, json!({ "pending_at_lock": "allow_as_absent", "reason": "close" })).await.0, StatusCode::OK);
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);
    let (_, t) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    assert_eq!(t["items"][0]["deduction_days"], "5.00", "two waiting absences and three leave days: {t}");
}

/// PL-05 allow-as-present: a pending day carries no deduction. Period settings cannot change under a later version.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn pending_days_counted_as_present(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let first = month_start(1);
    let body = json!({ "from": first, "to": first + Duration::days(2), "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    sqlx::query(
        "INSERT INTO attendance.regularisation_request (org_id, employee_id, attendance_date, request_type, proposed_in_at, proposed_out_at, reason, requested_by_user_id)
         VALUES ($1, $2, $3, 'whole_day', $4, $5, 'x', $6)",
    )
    .bind(s.org_id).bind(w.employee_id).bind(first)
    .bind(first.and_hms_opt(3, 30, 0).unwrap().and_utc()).bind(first.and_hms_opt(12, 30, 0).unwrap().and_utc()).bind(w.user_id)
    .execute(&pool).await.unwrap();
    assert_eq!(patch_settings(&app, &s, json!({ "pending_at_lock": "allow_as_present", "reason": "close" })).await.0, StatusCode::OK);
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);
    let (_, t) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    assert_eq!(t["items"][0]["deduction_days"], "2.00", "the pending day is counted present: {t}");

    let other = common::seed_org(&pool).await;
    org_settings(&pool, other.org_id).await;
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) SELECT org_id, current_date + 30, settings, 1 FROM org.organisation_settings_version WHERE org_id = $1")
        .bind(other.org_id).execute(&pool).await.unwrap();
    let (_, _, etag) = common::call(&app, &other, "GET", "/v1/period/settings", None, None).await;
    let (st, b, _) = common::call(&app, &other, "PATCH", "/v1/period/settings", Some(json!({ "unlock": "with_org_admin_approval", "reason": "x" })), etag.as_deref()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "a scheduled version would undo it: {b}");
}

/// Review focus 3: PL-06 never refuses; the requester never approves; approval unlocks and re-evaluates the days.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unlock_needs_someone_else_and_reopens_the_days(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let first = month_start(1);
    let body = json!({ "from": first, "to": first + Duration::days(1), "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);

    let uri = format!("/v1/periods/{last}/unlock-requests");
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &uri, Some(json!({ "reason": "wrong shift" })), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("UNLOCK_NOT_ALLOWED")), "{b}");
    assert_eq!(patch_settings(&app, &s, json!({ "unlock": "with_org_admin_approval", "reason": "audit finding" })).await.0, StatusCode::OK);
    assert_eq!(patch_settings(&app, &s, json!({ "unlock": "never", "reason": "again" })).await.0, StatusCode::CONFLICT, "one version per day");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &uri, Some(json!({ "reason": " " })), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, req) = common::as_user(&app, &s.hr_token, "POST", &uri, Some(json!({ "reason": "wrong shift" })), &key()).await;
    assert_eq!((st, req["state"].as_str()), (StatusCode::CREATED, Some("pending_l1")), "{req}");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &uri, Some(json!({ "reason": "again" })), &key()).await.0, StatusCode::CONFLICT, "one pending request");
    let id = req["id"].as_str().unwrap();
    let approve = format!("/v1/period/unlock-requests/{id}/approve");

    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &approve, None, &key()).await.0, StatusCode::FORBIDDEN, "HR holds no PER-05");
    let hr_role: Uuid = sqlx::query_scalar("SELECT role_id FROM iam.user_role WHERE user_id = $1").bind(s.hr_user_id).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'PER-05', 'scoped')").bind(s.org_id).bind(hr_role).execute(&pool).await.unwrap();
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &approve, None, &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "BR-07: {b}");

    // The Organisation Admin.
    let (admin, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'oa@test.invalid', 'x')").bind(s.org_id).bind(admin).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'Org Admin')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'PER-05', 'scoped')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(admin).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(s.org_id).bind(admin).execute(&pool).await.unwrap();
    let oa = common::token(admin, s.org_id);
    let (st, done) = common::as_user(&app, &oa, "POST", &approve, Some(json!({ "comment": "ok" })), &key()).await;
    assert_eq!((st, done["state"].as_str(), done["approval_steps"].as_array().map(Vec::len)), (StatusCode::OK, Some("approved"), Some(2)), "{done}");
    assert_eq!(common::as_user(&app, &oa, "POST", &approve, None, &key()).await.0, StatusCode::OK, "approve twice: same answer");
    assert_eq!(common::as_user(&app, &oa, "POST", &format!("/v1/period/unlock-requests/{id}/reject"), Some(json!({ "reason": "late" })), &key()).await.0, StatusCode::CONFLICT);

    let (_, p) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}"), None, &[]).await;
    let h = p["state_history"].as_array().unwrap().last().unwrap().clone();
    assert_eq!((p["state"].as_str(), h["changed_by"].as_str(), h["approved_by"].as_str()), (Some("unlocked"), Some(&*s.hr_user_id.to_string()), Some(&*admin.to_string())), "{p}");
    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM attendance.attendance_day WHERE processing_state = 'locked'").fetch_one(&pool).await.unwrap();
    let coverage: i64 = sqlx::query_scalar("SELECT count(*) FROM period.lock_coverage").fetch_one(&pool).await.unwrap();
    assert_eq!((left, coverage), (0, 0), "days re-evaluated, lock removed");

    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK, "P8: lock again");
    let frozen: i64 = sqlx::query_scalar("SELECT count(*) FROM period.period_totals").fetch_one(&pool).await.unwrap();
    assert_eq!(frozen, 2, "each lock freezes its own totals; the first stay as history");

    let (_, again) = common::as_user(&app, &s.hr_token, "POST", &uri, Some(json!({ "reason": "second look" })), &key()).await;
    let reject = format!("/v1/period/unlock-requests/{}/reject", again["id"].as_str().unwrap());
    let (st, r) = common::as_user(&app, &oa, "POST", &reject, Some(json!({ "reason": "not needed" })), &key()).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::OK, Some("rejected")), "{r}");
    let (_, p) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}"), None, &[]).await;
    assert_eq!(p["state"], "locked", "a rejection leaves the period locked");
}

/// Review focus 4: a branch-scoped reader sees only their branch's employees in the checklist and totals.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn checklist_and_totals_respect_scope(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    common::seed_worker(&pool, &app, &s, "Asha").await;
    let (last, _) = periods(&app, &s).await;
    let (user, role, other_branch) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO org.branch (org_id, id, code, name) VALUES ($1, $2, 'B2', 'Second')").bind(s.org_id).bind(other_branch).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'b2@test.invalid', 'x')").bind(s.org_id).bind(user).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'B2 HR')").bind(s.org_id).bind(role).execute(&pool).await.unwrap();
    for op in ["PER-02", "PER-03", "PER-08"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'scoped')").bind(s.org_id).bind(role).bind(op).execute(&pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(s.org_id).bind(user).bind(role).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id) VALUES ($1, $2, 'branch', $3)").bind(s.org_id).bind(user).bind(other_branch).execute(&pool).await.unwrap();
    let b2 = common::token(user, s.org_id);
    let (_, hq) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    assert_eq!(hq["items"].as_array().map(Vec::len), Some(1), "{hq}");
    let (_, t) = common::as_user(&app, &b2, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    assert_eq!(t["items"], json!([]), "{t}");
    let (st, c) = common::as_user(&app, &b2, "GET", &format!("/v1/periods/{last}/checklist"), None, &[]).await;
    assert_eq!((st, &c["items"]), (StatusCode::OK, &json!([])));
    assert_eq!(common::as_user(&app, &b2, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::FORBIDDEN, "lock is organisation-wide");
}

async fn hr_correct(app: &axum::Router, s: &common::SeedOrg, w: &common::Worker, d: NaiveDate) -> (StatusCode, Value) {
    let at = |t: &str| format!("{d}T{t}:00Z");
    let body = json!({ "employee_id": w.employee_id, "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at("03:30"), "proposed_out_at": at("12:30"), "reason": "device fault" });
    common::as_user(app, &s.hr_token, "POST", "/v1/attendance/hr-corrections", Some(body), &key()).await
}

/// E15 / AC-C12-06.1-06.5: a corrected locked day stays locked; the difference is posted once to the next open period,
/// frozen with it, and carried by the payroll input export, which is identical when taken twice (AC-C12-07.3).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn correction_after_lock_posts_an_adjustment_and_exports(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let store = std::sync::Arc::new(common::MemoryStore::default());
    let app = api::router(common::state_with_store(pool.clone(), store.clone()));
    org_policy(&app, &s).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let (_, list) = common::as_user(&app, &s.hr_token, "GET", "/v1/periods", None, &[]).await;
    let oldest = list["items"][2]["id"].as_str().unwrap().to_string();
    let d = month_start(2) + Duration::days(2);
    let body = json!({ "from": d, "to": d, "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{oldest}/lock"), None, &key()).await.0, StatusCode::OK);

    let (st, r) = hr_correct(&app, &s, &w, d).await;
    assert_eq!((st, r["state"].as_str()), (StatusCode::CREATED, Some("approved")), "{r}");
    let (_, adj) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/period/adjustments?employee_id={}", w.employee_id), None, &[]).await;
    let a = &adj["items"][0];
    assert_eq!(adj["items"].as_array().map(Vec::len), Some(1), "{adj}");
    assert_eq!(
        (a["original_date"].as_str(), a["source_period_id"].as_str(), a["target_period_id"].as_str(), a["days"].as_str(), a["state"].as_str()),
        (Some(&*d.to_string()), Some(&*oldest), Some(&*last), Some("1.00"), Some("posted")),
        "one-day arrear into the next open period: {a}"
    );
    assert_eq!((a["difference"]["day_status"]["to"].as_str(), a["difference"]["deduction_lines"][0]["days"].as_str()), (Some("present"), Some("-1.00")), "{a}");
    let (_, day) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/attendance/employees/{}/days/{d}", w.employee_id), None, &[]).await;
    assert_eq!(day["current_result"]["day_status"], "absent", "the locked day itself never changes: {day}");

    // The same inputs again post nothing (AC-C12-06.5).
    let body = json!({ "from": d, "to": d, "employee_ids": [w.employee_id], "reason": "again" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM period.period_adjustment").fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1);

    let export = format!("/v1/periods/{last}/payroll-input-exports");
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &export, None, &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "export after lock only: {b}");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);
    let state: String = sqlx::query_scalar("SELECT state FROM period.period_adjustment").fetch_one(&pool).await.unwrap();
    assert_eq!(state, "frozen", "SM-ADJ Q2");
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &export, Some(json!({ "format": "xlsx" })), &key()).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, job) = common::as_user(&app, &s.hr_token, "POST", &export, None, &key()).await;
    assert_eq!((st, job["state"].as_str()), (StatusCode::ACCEPTED, Some("succeeded")), "{job}");
    assert!(job["download_url"].as_str().unwrap().starts_with(&format!("https://storage.test/{}/payroll-input/{last}/", s.org_id)), "{job}");
    let (_, again) = common::as_user(&app, &s.hr_token, "POST", &export, None, &key()).await;

    let files: Vec<Value> = store.0.lock().unwrap().values().map(|b| serde_json::from_slice(b).unwrap()).collect();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["employees"], files[1]["employees"], "exported twice: identical content");
    let doc = files.iter().find(|f| f["export_id"] == job["id"]).unwrap();
    let e = &doc["employees"][0];
    assert_eq!(
        (doc["contract_version"].as_str(), e["employee_code"].as_str(), e["adjustments"][0]["original_date"].as_str(), e["adjustments"][0]["days"].as_str()),
        (Some("PayrollInput v1"), Some("HQ-0001"), Some(&*d.to_string()), Some("1.00")),
        "{doc}"
    );
    let state: String = sqlx::query_scalar("SELECT state FROM period.period_adjustment").fetch_one(&pool).await.unwrap();
    assert_eq!(state, "exported", "SM-ADJ Q3");

    let (_, exports) = common::as_user(&app, &s.hr_token, "GET", &export, None, &[]).await;
    assert_eq!((exports["items"].as_array().map(Vec::len), exports["items"][0]["adjustment_ids"].as_array().map(Vec::len)), (Some(2), Some(1)), "{exports}");
    let again_id = again["id"].as_str().unwrap();
    let (st, link) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/period/payroll-input-exports/{again_id}/download-link"), None, &[]).await;
    assert_eq!((st, link["url"].as_str().map(|u| u.contains(again_id))), (StatusCode::OK, Some(true)), "{link}");
    let downloads: i64 = sqlx::query_scalar("SELECT count(*) FROM audit.audit_entry WHERE action = 'period.payroll_input_downloaded'").fetch_one(&pool).await.unwrap();
    assert_eq!(downloads, 1, "every download is audited");
    let other = common::seed_org(&pool).await;
    let uri = format!("/v1/period/payroll-input-exports/{}/download-link", job["id"].as_str().unwrap());
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &uri, None, &[]).await.0, StatusCode::NOT_FOUND);
}

/// PL-07: a correction older than the window is refused and nothing is written (OQ-C12-04).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn correction_beyond_the_window_is_refused(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    org_policy(&app, &s).await;
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let d = month_start(1) + Duration::days(2);
    let body = json!({ "from": d, "to": d, "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);
    assert_eq!(patch_settings(&app, &s, json!({ "adjustment_past_periods": 0, "reason": "no corrections" })).await.0, StatusCode::OK);
    let (st, b) = hr_correct(&app, &s, &w, d).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ADJUSTMENT_WINDOW_EXCEEDED")), "{b}");
    let written: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM attendance.regularisation_request) + (SELECT count(*) FROM period.period_adjustment)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(written, 0, "refused as a whole");
}

/// Review fixes: an unchanged auto-debit absence measures the same loss of pay (no adjustment); corrections chain from
/// the latest measurement; a locked date with no result is refused; a period with posted corrections cannot be unlocked.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn measured_locked_days_keep_their_charge_and_chain(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    policy_ds10(&app, &s, json!({ "action": "auto_debit" })).await; // no leave type has balance: loss of pay
    org_settings(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    common::day_shift(&app, &s, w.employee_id).await;
    let (last, _) = periods(&app, &s).await;
    let d = month_start(1) + Duration::days(2);
    let body = json!({ "from": d, "to": d, "employee_ids": [w.employee_id], "reason": "check" });
    common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body.clone()), &key()).await;
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/lock"), None, &key()).await.0, StatusCode::OK);
    let (_, t) = common::as_user(&app, &s.hr_token, "GET", &format!("/v1/periods/{last}/totals"), None, &[]).await;
    assert_eq!(t["items"][0]["deduction_days"], "1.00", "auto-debit with no balance is loss of pay: {t}");

    // Inputs that look changed (fingerprint) but leave the day absent: measured with the same loss of pay, nothing posted.
    let mut conn = pool.acquire().await.unwrap();
    sqlx::raw_sql("SET session_replication_role = replica").execute(&mut *conn).await.unwrap();
    sqlx::query("UPDATE attendance.day_result SET input_fingerprint = '\\x00' WHERE employee_id = $1 AND attendance_date = $2").bind(w.employee_id).bind(d).execute(&mut *conn).await.unwrap();
    sqlx::raw_sql("SET session_replication_role = origin").execute(&mut *conn).await.unwrap();
    drop(conn);
    let (st, job) = common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/recalculations", Some(body), &key()).await;
    assert_eq!(st, StatusCode::ACCEPTED, "{job}");
    let measured: i64 = sqlx::query_scalar("SELECT count(*) FROM attendance.day_result WHERE for_adjustment").fetch_one(&pool).await.unwrap();
    let posted: i64 = sqlx::query_scalar("SELECT count(*) FROM period.period_adjustment").fetch_one(&pool).await.unwrap();
    assert_eq!((measured, posted), (1, 0), "measured again, same outcome: no adjustment");

    // Two different corrections chain: the second is measured against the first.
    assert_eq!(hr_correct(&app, &s, &w, d).await.0, StatusCode::CREATED);
    let at = |t: &str| format!("{d}T{t}:00Z");
    let late = json!({ "employee_id": w.employee_id, "attendance_date": d, "request_type": "whole_day", "proposed_in_at": at("05:30"), "proposed_out_at": at("12:30"), "reason": "came late" });
    assert_eq!(common::as_user(&app, &s.hr_token, "POST", "/v1/attendance/hr-corrections", Some(late), &key()).await.0, StatusCode::CREATED);
    let chain: Vec<(Uuid, Uuid)> = sqlx::query_as("SELECT locked_result_id, new_result_id FROM period.period_adjustment ORDER BY created_at, days DESC").fetch_all(&pool).await.unwrap();
    assert_eq!(chain.len(), 2, "two arrival times, two differences: {chain:?}");
    assert_eq!(chain[1].0, chain[0].1, "the second correction is measured against the first");

    // A locked date that never had a result cannot be corrected (nothing to measure against).
    let (st, b) = hr_correct(&app, &s, &w, d + Duration::days(4)).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("PERIOD_LOCKED")), "{b}");

    // OQ-C12-07: once its corrections are posted elsewhere, the period cannot be unlocked.
    assert_eq!(patch_settings(&app, &s, json!({ "unlock": "with_org_admin_approval", "reason": "audit" })).await.0, StatusCode::OK);
    let (st, b) = common::as_user(&app, &s.hr_token, "POST", &format!("/v1/periods/{last}/unlock-requests"), Some(json!({ "reason": "redo" })), &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("UNLOCK_NOT_ALLOWED")), "{b}");
}
