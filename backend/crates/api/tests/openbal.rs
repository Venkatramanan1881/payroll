//! Opening balances (slice 12).

mod common;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn key() -> Vec<(&'static str, String)> {
    vec![("idempotency-key", Uuid::new_v4().to_string())]
}

async fn leave_type(app: &axum::Router, s: &common::SeedOrg) -> String {
    let version = json!({ "effective_from": "2026-01-01", "reason": "setup", "paid": true, "eligibility": {}, "credit_method": "annual_grant",
        "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00", "half_day_allowed": true, "min_notice_days": 0,
        "approval_levels": "manager", "timeout_days": 30, "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true });
    let (st, t, _) = common::call(app, s, "POST", "/v1/leave/types", Some(json!({ "code": "CL", "name": "Casual", "version": version })), None).await;
    assert_eq!(st, StatusCode::CREATED, "{t}");
    t["id"].as_str().unwrap().to_string()
}

async fn balance(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT to_char(coalesce(sum(quantity), 0), 'FM990.00') FROM leave.leave_ledger_entry").fetch_one(pool).await.unwrap()
}

/// An approver with OBL-02 (approve) over the whole organisation, as the matrix gives the Organisation Admin.
async fn approver(pool: &PgPool, org: Uuid) -> String {
    let (user, role) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO iam.user_account (org_id, id, login_kind, email, password_hash) VALUES ($1, $2, 'password', 'oa@test.invalid', 'x')").bind(org).bind(user).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role (org_id, id, name) VALUES ($1, $2, 'OA')").bind(org).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, 'OBL-02', 'approve')").bind(org).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(user).execute(pool).await.unwrap();
    common::token(user, org)
}

/// OB-01 = HR only (default): rows validated, confirmed once, posted once, changed only by superseding with a reason.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn confirm_posts_once_and_changes_supersede(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let cl = leave_type(&app, &s).await;
    let leave = json!({ "employee_id": w.employee_id, "balance_kind": "leave", "leave_type_id": cl, "value": "5.00", "as_of_date": "2026-04-01" });
    let late = json!({ "employee_id": w.employee_id, "balance_kind": "late_counter", "value": "2.00", "as_of_date": "2026-04-01" });

    let bad = json!({ "entries": [leave, { "employee_id": w.employee_id, "balance_kind": "comp_off", "value": "1.00", "as_of_date": "2026-04-01" },
                                  { "employee_id": Uuid::new_v4(), "balance_kind": "late_counter", "value": "1.0", "as_of_date": "2026-04-01" }] });
    let (st, b, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(bad), None).await;
    assert_eq!((st, b["errors"].as_array().map(Vec::len), b["errors"][0]["field"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some(2), Some("entries[1]")), "{b}");
    let batches: i64 = sqlx::query_scalar("SELECT count(*) FROM openbal.opening_balance_batch").fetch_one(&pool).await.unwrap();
    assert_eq!(batches, 0, "nothing saved when a row fails");

    let (st, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [leave.clone(), late] })), None).await;
    assert_eq!((st, batch["entry_count"].as_i64(), batch["state"].as_str()), (StatusCode::CREATED, Some(2), Some("draft")), "{batch}");
    let id = batch["id"].as_str().unwrap();
    let (st, _, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/entries"), Some(json!({ "entries": [leave] })), None).await;
    assert_eq!(st, StatusCode::CONFLICT, "one live balance per employee, kind and type");

    let (_, entries, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries?balance_kind=leave"), None, None).await;
    let entry = entries["items"][0].clone();
    let uri = format!("/v1/opening-balances/batches/{id}/entries/{}", entry["id"].as_str().unwrap());
    let tag = format!("W/\"{}\"", entry["updated_at"].as_str().unwrap());
    let (st, edited, _) = common::call(&app, &s, "PATCH", &uri, Some(json!({ "value": "6.50" })), Some(&tag)).await;
    assert_eq!((st, edited["value"].as_str()), (StatusCode::OK, Some("6.50")), "{edited}");

    let (st, b, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/submit"), None, None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("ILLEGAL_TRANSITION")), "OB-01 = HR only: confirm directly");
    let (st, conf, _) = common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/confirm"), None, None).await;
    assert_eq!((st, conf["state"].as_str()), (StatusCode::OK, Some("confirmed")), "{conf}");
    assert_eq!(balance(&pool).await, "6.50", "posted into the leave ledger");
    common::call(&app, &s, "POST", &format!("/v1/opening-balances/batches/{id}/confirm"), None, None).await;
    assert_eq!(balance(&pool).await, "6.50", "confirming twice posts once");
    let tag = format!("W/\"{}\"", common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries?balance_kind=leave"), None, None).await.1["items"][0]["updated_at"].as_str().unwrap());
    let (st, _, _) = common::call(&app, &s, "PATCH", &uri, Some(json!({ "value": "1.00" })), Some(&tag)).await;
    assert_eq!(st, StatusCode::CONFLICT, "confirmed entries never change in place");

    let change = format!("/v1/opening-balances/entries/{}/change", entry["id"].as_str().unwrap());
    assert_eq!(common::call(&app, &s, "POST", &change, Some(json!({ "value": "8.00", "reason": " " })), None).await.0, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, new, _) = common::call(&app, &s, "POST", &change, Some(json!({ "value": "8.00", "reason": "payroll file corrected" })), None).await;
    assert_eq!((st, new["value"].as_str(), new["change_reason"].as_str()), (StatusCode::CREATED, Some("8.00"), Some("payroll file corrected")), "{new}");
    assert_eq!(balance(&pool).await, "8.00", "old opening reversed, new one posted");
    let (_, all, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries?include_superseded=true"), None, None).await;
    assert_eq!(all["items"].as_array().map(Vec::len), Some(3), "{all}");
    assert_eq!(common::call(&app, &s, "POST", &change, Some(json!({ "value": "9.00", "reason": "again" })), None).await.0, StatusCode::CONFLICT, "a superseded entry is not changed again");
}

/// OB-01 = with Organisation Admin approval: submit, BR-07 for anyone involved, reject back to draft, confirm.
/// OB-02 = not allowed: no change after confirmation.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approval_flow_and_br07(pool: PgPool) {
    let s = common::seed_org(&pool).await;
    let app = common::app(pool.clone());
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version) VALUES ($1, '2026-01-01', $2, 1)")
        .bind(s.org_id).bind(json!({ "opening_balances": { "confirm_mode": "hr_with_org_admin_approval", "change_after_confirm": "not_allowed" } }))
        .execute(&pool).await.unwrap();
    let oa = approver(&pool, s.org_id).await;
    let w = common::seed_worker(&pool, &app, &s, "Asha").await;
    let late = json!({ "employee_id": w.employee_id, "balance_kind": "late_counter", "value": "3.00", "as_of_date": "2026-04-01" });
    let (_, batch, _) = common::call(&app, &s, "POST", "/v1/opening-balances/batches", Some(json!({ "entries": [late] })), None).await;
    let id = batch["id"].as_str().unwrap();
    let cmd = |what: &str| format!("/v1/opening-balances/batches/{id}/{what}");

    assert_eq!(common::call(&app, &s, "POST", &cmd("confirm"), None, None).await.0, StatusCode::CONFLICT, "approval needed: submit first");
    let (_, sub, _) = common::call(&app, &s, "POST", &cmd("submit"), None, None).await;
    assert_eq!(sub["state"], "pending_approval");
    let (st, b, _) = common::call(&app, &s, "POST", &cmd("confirm"), None, None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::FORBIDDEN, Some("SELF_APPROVAL_FORBIDDEN")), "the submitter never approves");
    let (st, rej) = common::as_user(&app, &oa, "POST", &cmd("reject"), Some(json!({ "reason": "wrong date" })), &key()).await;
    assert_eq!((st, rej["state"].as_str()), (StatusCode::OK, Some("draft")), "back for correction (GAP-SM-13): {rej}");
    common::call(&app, &s, "POST", &cmd("submit"), None, None).await;
    let (st, ok) = common::as_user(&app, &oa, "POST", &cmd("confirm"), Some(json!({ "comment": "checked" })), &key()).await;
    assert_eq!((st, ok["state"].as_str(), ok["approval_steps"].as_array().map(Vec::len)), (StatusCode::OK, Some("confirmed"), Some(4)), "{ok}");

    let (_, entries, _) = common::call(&app, &s, "GET", &format!("/v1/opening-balances/batches/{id}/entries"), None, None).await;
    let change = format!("/v1/opening-balances/entries/{}/change", entries["items"][0]["id"].as_str().unwrap());
    let (st, b, _) = common::call(&app, &s, "POST", &change, Some(json!({ "value": "1.00", "reason": "x" })), None).await;
    assert_eq!((st, b["code"].as_str()), (StatusCode::CONFLICT, Some("RULE_GROUP_DISABLED")), "OB-02 = not allowed");

    let other = common::seed_org(&pool).await;
    assert_eq!(common::as_user(&app, &other.hr_token, "GET", &format!("/v1/opening-balances/batches/{id}"), None, &[]).await.0, StatusCode::NOT_FOUND, "TB-01");
}
