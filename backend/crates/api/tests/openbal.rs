//! Opening balances (slice 12).

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn leave_type(app: &axum::Router, s: &common::SeedOrg, code: &str) -> String {
    let version = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 30, "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true });
    let (st, t, _) = common::call(app, s, "POST", "/v1/leave/types", Some(json!({ "code": code, "name": code, "version": version })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    t["id"].as_str().unwrap().to_string()
}

async fn balance(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT to_char(coalesce(sum(quantity), 0), 'FM990.00') FROM leave.leave_ledger_entry").fetch_one(pool).await.unwrap()
}

/// A user over the whole organisation with the given OBL grants (the Organisation Admin holds OBL-02 approve).
async fn user_with(pool: &PgPool, org: Uuid, grants: &[(&str, &str)]) -> String {
    let (user, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', $3, 'x')").bind(org).bind(user).bind(format!("{user}@t.invalid")).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, $3)").bind(org).bind(role).bind(format!("R{role}")).execute(pool).await.unwrap();
    for (op, kind) in grants {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, $4)").bind(org).bind(role).bind(op).bind(kind).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(user).execute(pool).await.unwrap();
    common::token(user, org)
}

async fn batch_tag(app: &axum::Router, s: &common::SeedOrg, id: &str) -> String {
    let (_, _, tag) = common::call(app, s, "GET", &format!("/v1/opening-balances/batches/{id}"), None, None).await;
    tag.unwrap()
}

/// OB-01 = HR only (default): rows validated, confirmed once, posted once, changed only by superseding with a reason.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirm_posts_once_and_changes_supersede(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let cl = leave_type(&app, &s, "CL").await;
    let leave = json!({ "employee_id": w.employee_id, "balance_kind": "leave", "leave_type_id": cl, "value": "5.00", "as_of_date": "2026-04-01" });

    let bad = json!({ "entries": [leave, { "employee_id": w.employee_id, "balance_kind": "late_counter", "value": "2.50", "as_of_date": "2026-04-01" },
                                  { "employee_id": Uuid::new_v4(), "balance_kind": "leave", "leave_type_id": cl, "value": "1.0", "as_of_date": "2026-04-01" }] });
    let (st, b, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(bad), None).await;
    assert_eq!((st, b["errors"].as_array().map(Vec::len), b["errors"][0]["field"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some(2), Some("entries[1]")),
               "a counter is whole marks; unknown employees and bad values are row errors: {b}");
    let batches: i64 = sqlx::query_scalar("SELECT count(*) FROM openbal.opening_balance_batch").fetch_one(&pool).await.unwrap();
    assert_eq!(batches, 0, "nothing saved when a row fails");

    let (st, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [leave.clone()] })), None).await;
    assert_eq!((st, batch["entry_count"].as_i64(), batch["state"].as_str()), (StatusCode::CREATED, Some(1), Some("draft")), "{batch}");
    let id = batch["id"].as_str().unwrap();
    let add = format!("/v1/opening-balances/batches/{id}/entries");
    assert_eq!(common::call(&app, &s, "POST", &add, Some(json!({ "entries": [leave.clone()] })), None).await.0, StatusCode::PRECONDITION_REQUIRED);
    let tag = batch_tag(&app, &s, id).await;
    assert_eq!(common::call(&app, &s, "POST", &add, Some(json!({ "entries": [leave] })), Some(&tag)).await.0, StatusCode::CONFLICT, "one live balance per employee, kind and type");

    let (_, entries, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries"), None, None).await;
    let entry = entries["items"][0].clone();
    let uri = format!("/v1/opening-balances/batches/{id}/entries/{}", entry["id"].as_str().unwrap());
    let tag = format!("W/\"{}\"", entry["updated_at"].as_str().unwrap());
    let (st, edited, _) = common::call(&app, &s, "PATCH", &uri, Some(json!({ "value": "6.50" })), Some(&tag)).await;
    assert_eq!((st, edited["value"].as_str()), (StatusCode::OK, Some("6.50")), "{edited}");
    assert_eq!(common::call(&app, &s, "PATCH", &format!("/v1/opening-balances/batches/{}/entries/{}", Uuid::new_v4(), entry["id"].as_str().unwrap()),
                            Some(json!({ "value": "1.00" })), Some(&format!("W/\"{}\"", edited["updated_at"].as_str().unwrap()))).await.0, StatusCode::NOT_FOUND, "entry of another batch");

    let (st, b, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/submit"), None, None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "OB-01 = HR only: confirm directly");
    let (st, conf, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/confirm"), None, None).await;
    assert_eq!((st, conf["state"].as_str()), (StatusCode::OK, Some("confirmed")), "{conf}");
    assert_eq!(balance(&pool).await, "6.50", "posted into the leave ledger");
    common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/confirm"), None, None).await;
    assert_eq!(balance(&pool).await, "6.50", "confirming twice posts once");

    let change = format!("/v1/opening-balances/entries/{}/change", entry["id"].as_str().unwrap());
    assert_eq!(common::call(&app, &s, "POST", &change, Some(json!({ "value": "8.00", "reason": " " })), None).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, new, _) = common::call(&app, &s, "POST", &change, Some(json!({ "value": "8.00", "as_of_date": "2026-03-01", "reason": "payroll file corrected" })), None).await;
    assert_eq!((st, new["value"].as_str()), (StatusCode::CREATED, Some("8.00")), "{new}");
    assert_eq!(balance(&pool).await, "8.00", "old opening reversed, new one posted");
    let reversal_date: chrono::NaiveDate = sqlx::query_scalar("SELECT entry_date FROM leave.leave_ledger_entry WHERE entry_kind = 'reversal'").fetch_one(&pool).await.unwrap();
    assert_eq!(reversal_date.to_string(), "2026-04-01", "the reversal sits on the old as-of date");
    let (st, zero, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/entries/{}/change", new["id"].as_str().unwrap()), Some(json!({ "value": "0.00", "reason": "no balance after all" })), None).await;
    assert_eq!((st, zero["value"].as_str()), (StatusCode::CREATED, Some("0.00")));
    assert_eq!(balance(&pool).await, "0.00", "a change in a chain reverses the replacement; zero posts nothing");
    let (_, all, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries?include_superseded=true"), None, None).await;
    assert_eq!(all["items"].as_array().map(Vec::len), Some(3), "{all}");
    assert_eq!(common::call(&app, &s, "POST", &change, Some(json!({ "value": "9.00", "reason": "again" })), None).await.0, StatusCode::CONFLICT, "a superseded entry is not changed again");
}

/// OB-01 = with Organisation Admin approval: HR enters and submits; only an approve-type holder who touched nothing
/// decides; reject returns to draft. OB-02 = not allowed. A lock between draft and confirm blocks the confirmation.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approval_flow_br07_and_locks(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(s.org_id).bind(json!({ "opening_balances": { "confirm_mode": "hr_with_org_admin_approval", "change_after_confirm": "not_allowed" } }))
        .execute(&pool).await.unwrap();
    let editor = user_with(&pool, s.org_id, &[("OBL-01", "scoped"), ("OBL-02", "approve"), ("OBL-04", "scoped")]).await;
    let oa = user_with(&pool, s.org_id, &[("OBL-02", "approve")]).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let cl = leave_type(&app, &s, "CL").await;
    let leave = json!({ "employee_id": w.employee_id, "balance_kind": "leave", "leave_type_id": cl, "value": "3.00", "as_of_date": "2026-04-01" });
    let (_, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [leave] })), None).await;
    let id = batch["id"].as_str().unwrap();
    let cmd = |what: &str| format!("/v1/opening-balances/batches/{id}/{what}");

    // The approver-to-be edits a value: from then on they may not approve the batch.
    let (_, entries) = common::as_user(&app, &editor, "GET", &format!("/v1/opening-balances/batches/{id}/entries"), None, &[]).await;
    let e = &entries["items"][0];
    let tag = format!("W/\"{}\"", e["updated_at"].as_str().unwrap());
    let (st, _) = common::as_user(&app, &editor, "PATCH", &format!("/v1/opening-balances/batches/{id}/entries/{}", e["id"].as_str().unwrap()),
                                  Some(json!({ "value": "4.00" })), &[("if-match", tag)]).await;
    assert_eq!(st, StatusCode::OK);

    assert_eq!(common::call(&app, &s, "POST", &cmd("confirm"), None, None).await.0, StatusCode::CONFLICT, "approval needed: submit first");
    let (_, sub, _) = common::call(&app, &s, "POST", &cmd("submit"), None, None).await;
    assert_eq!(sub["state"], "pending_approval");
    assert_eq!(common::call(&app, &s, "POST", &cmd("confirm"), None, None).await.0, StatusCode::FORBIDDEN, "an HR Manager never decides a pending batch");
    let (st, b) = common::as_user(&app, &editor, "POST", &cmd("confirm"), None, &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "the editor never approves: {b}");
    let (st, rej) = common::as_user(&app, &oa, "POST", &cmd("reject"), Some(json!({ "reason": "wrong date" })), &key()).await;
    assert_eq!((st, rej["state"].as_str()), (StatusCode::OK, Some("draft")), "back for correction (GAP-SM-13): {rej}");
    common::call(&app, &s, "POST", &cmd("submit"), None, None).await;

    // April gets locked while the batch waits: confirmation is refused (BR-06).
    let (cycle, period) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO period.attendance_cycle (org_id, id, owner_kind, owner_id, valid) VALUES ($1, $2, 'organisation', $1, daterange('2026-01-01', NULL))").bind(s.org_id).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.attendance_period (org_id, id, cycle_id, period_start, period_end) VALUES ($1, $2, $3, '2026-04-01', '2026-04-30')").bind(s.org_id).bind(period).bind(cycle).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO period.lock_coverage (org_id, period_id, employee_id, dates) VALUES ($1, $2, $3, '[2026-04-01,2026-05-01)')").bind(s.org_id).bind(period).bind(w.employee_id).execute(&pool).await.unwrap();
    let (st, b) = common::as_user(&app, &oa, "POST", &cmd("confirm"), None, &key()).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("PERIOD_LOCKED")), "{b}");
    sqlx::query("DELETE FROM period.lock_coverage").execute(&pool).await.unwrap();

    let (st, ok) = common::as_user(&app, &oa, "POST", &cmd("confirm"), Some(json!({ "comment": "checked" })), &key()).await;
    assert_eq!((st, ok["state"].as_str(), ok["approval_steps"].as_array().map(Vec::len)), (StatusCode::OK, Some("confirmed"), Some(4)), "{ok}");
    assert_eq!(balance(&pool).await, "4.00");

    let (_, entries, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries"), None, None).await;
    let change = format!("/v1/opening-balances/entries/{}/change", entries["items"][0]["id"].as_str().unwrap());
    let (st, b, _) = common::call(&app, &s, "POST", &change, Some(json!({ "value": "1.00", "reason": "x" })), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("RULE_GROUP_DISABLED")), "OB-02 = not allowed");

    // A self-only reader with scope `all` sees only their own batches, never the organisation's.
    let reader = user_with(&pool, s.org_id, &[("OBL-04", "self")]).await;
    let (_, mine) = common::as_user(&app, &reader, "GET", "/v1/opening-balances/batches", None, &[]).await;
    assert_eq!(mine["items"], json!([]), "{mine}");
    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/opening-balances/batches/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}
