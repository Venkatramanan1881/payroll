//! Local development seed for the web app. Refuses any database that is not on localhost. It creates:
//! - one organisation, with two branches and a few lists;
//! - an HR Manager (hr@dev.invalid) who signs in with a password and no authenticator step. The HR Manager is also
//!   employee Meena, the manager of employee Asha;
//! - Asha (asha@dev.invalid), a password login for local trials only (employees sign in by phone in real use);
//! - an organisation policy and a 09:00-18:00 shift;
//! - yesterday's punches for Asha;
//! - leave types CL and SL with balances for Asha, and a pending CL request of hers;
//! - a "Dev leave extras" role for HR (LVE-03 apply on behalf, LVE-06 cancel, LVE-07 adjust): no default role holds
//!   these while G-24..G-26 are open, so they exist here only for local trials.
//!
//! Organisation rows go straight in as the superuser; everything else goes through the API in-process.
//!
//! DATABASE_URL=postgres://postgres@localhost:55432/payroll_dev DEV_ADMIN_PASSWORD=... cargo run -p api --example dev_seed

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

async fn call(app: &axum::Router, token: &str, method: &str, uri: &str, body: Option<Value>) -> anyhow::Result<Value> {
    let req = Request::builder()
        .method(method)
        .uri(uri)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", Uuid::new_v4().to_string())
        .header("content-type", "application/json")
        .body(body.map(|b| Body::from(b.to_string())).unwrap_or_else(Body::empty))?;
    let res = app.clone().oneshot(req).await?;
    let status = res.status();
    let bytes = res.into_body().collect().await?.to_bytes();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    anyhow::ensure!(status.is_success(), "{method} {uri}: {status} {json}");
    Ok(json)
}

async fn role(pool: &PgPool, org: Uuid, code: &str, name: &str) -> anyhow::Result<Uuid> {
    let role: Uuid = sqlx::query_scalar("INSERT INTO iam.role (org_id, name, system_code) VALUES ($1, $2, $3) RETURNING id").bind(org).bind(name).bind(code).fetch_one(pool).await?;
    for (_, op, kind, setting) in domain_iam::defaults::SYSTEM_ROLE_GRANTS.iter().filter(|g| g.0 == code) {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind, setting_ref) VALUES ($1, $2, $3, $4, $5)")
            .bind(org).bind(role).bind(op).bind(kind).bind(setting).execute(pool).await?;
    }
    Ok(role)
}

async fn user(pool: &PgPool, org: Uuid, email: &str, hash: &str, roles: &[Uuid], employee: Option<Uuid>) -> anyhow::Result<Uuid> {
    let id: Uuid = sqlx::query_scalar("INSERT INTO iam.user_account (org_id, login_kind, email, password_hash, employee_id) VALUES ($1, 'password', $2, $3, $4) RETURNING id")
        .bind(org).bind(email).bind(hash).bind(employee).fetch_one(pool).await?;
    for r in roles {
        sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(id).bind(r).execute(pool).await?;
    }
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(id).execute(pool).await?;
    Ok(id)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")?;
    let host = url.parse::<sqlx::postgres::PgConnectOptions>()?.get_host().to_string();
    anyhow::ensure!(host == "localhost" || host == "127.0.0.1", "dev_seed only runs against a local database, not {host}");
    let password = std::env::var("DEV_ADMIN_PASSWORD").map_err(|_| anyhow::anyhow!("DEV_ADMIN_PASSWORD is required"))?;
    anyhow::ensure!(password.len() >= 12, "DEV_ADMIN_PASSWORD needs at least 12 characters");
    let pool = PgPool::connect(&url).await?;
    db::MIGRATOR.run(&pool).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE email = 'hr@dev.invalid')").fetch_one(&pool).await?;
    anyhow::ensure!(!exists, "already seeded: drop the dev database to seed again");

    // Organisation rows, as the vendor's set-up would make them.
    let vendor: Uuid = sqlx::query_scalar("INSERT INTO platform.vendor_user (email, full_name, vendor_role) VALUES ($1, 'Dev vendor', 'full_admin') RETURNING id")
        .bind(format!("vendor-{}@dev.invalid", Uuid::new_v4())).fetch_one(&pool).await?;
    let org: Uuid = sqlx::query_scalar("INSERT INTO platform.organisation (legal_name, display_name) VALUES ('Dev Bakery Pvt Ltd', 'Dev Bakery') RETURNING id").fetch_one(&pool).await?;
    for m in (1..=16).map(|n| format!("C{n:02}")) {
        sqlx::query("INSERT INTO platform.module_entitlement (org_id, module_code, enabled, valid, set_by) VALUES ($1, $2, true, daterange('2000-01-01', NULL), $3)")
            .bind(org).bind(m).bind(vendor).execute(&pool).await?;
    }
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason) VALUES ($1, '2000-01-01', '{\"admin_second_step\": false}', 1, 'dev seed')")
        .bind(org).execute(&pool).await?;
    let hq: Uuid = sqlx::query_scalar("INSERT INTO org.branch (org_id, code, name) VALUES ($1, 'HQ', 'Head office') RETURNING id").bind(org).fetch_one(&pool).await?;
    sqlx::query("INSERT INTO org.branch (org_id, code, name) VALUES ($1, 'ANN', 'Anna Nagar')").bind(org).execute(&pool).await?;
    for (kind, name) in [("department", "Production"), ("department", "Sales"), ("designation", "Baker"), ("designation", "Cashier")] {
        sqlx::query("INSERT INTO org.list_entry (org_id, kind, name) VALUES ($1, $2, $3)").bind(org).bind(kind).bind(name).execute(&pool).await?;
    }
    let hash = argon2::Argon2::default().hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng)).map_err(|e| anyhow::anyhow!("{e}"))?.to_string();
    let (hr_role, mgr_role, emp_role) = (role(&pool, org, "hr_manager", "HR Manager").await?, role(&pool, org, "manager", "Manager").await?, role(&pool, org, "employee", "Employee").await?);
    let hr = user(&pool, org, "hr@dev.invalid", &hash, &[hr_role, mgr_role], None).await?;

    // Everything else through the API, as HR.
    let secret = Uuid::new_v4().to_string().into_bytes();
    let app = api::router(api::AppState {
        pool: pool.clone(),
        jwt_secret: secret.clone(),
        phone_verifier: std::sync::Arc::new(domain_iam::firebase::GoogleVerifier::new("dev-local")),
        job_secret: None,
        store: None,
        vendor_verifier: None,
        mfa_key: None,
    });
    let now = chrono::Utc::now().timestamp();
    let token = domain_iam::tokens::issue_access(&secret, hr, org, None, now);
    let t = token.as_str();

    let set = call(&app, t, "POST", "/v1/policy-sets", Some(json!({ "name": "Default", "effective_from": "2026-01-01" }))).await?;
    let v = set["versions"][0]["id"].as_str().unwrap_or_default().to_string();
    call(&app, t, "POST", &format!("/v1/policy-versions/{v}/publish"), None).await?;
    call(&app, t, "POST", "/v1/policy-assignments", Some(json!({ "policy_set_id": set["id"], "level": "organisation", "valid_from": "2026-01-01", "reason": "dev seed" }))).await?;
    let shift = call(&app, t, "POST", "/v1/shifts", Some(json!({ "code": "GEN", "name": "General 09-18", "effective_from": "2026-01-01",
        "definition": { "shift_type": "fixed", "start_time": "09:00", "end_time": "18:00" } }))).await?;
    let sv = shift["versions"][0]["id"].as_str().unwrap_or_default().to_string();
    call(&app, t, "POST", &format!("/v1/shift-versions/{sv}/publish"), None).await?;

    let mut ids = vec![];
    for name in ["Meena", "Asha"] {
        let e = call(&app, t, "POST", "/v1/employees", Some(json!({ "full_name": name, "joining_date": "2026-01-01", "employment_status": "active", "master_branch_id": hq }))).await?;
        let id: Uuid = e["id"].as_str().unwrap_or_default().parse()?;
        call(&app, t, "POST", "/v1/employee-schedules", Some(json!({ "employee_id": id, "source": "fixed_shift", "shift_id": shift["id"], "valid_from": "2026-01-01", "reason": "joining" }))).await?;
        ids.push(id);
    }
    let (meena, asha) = (ids[0], ids[1]);
    sqlx::query("UPDATE iam.user_account SET employee_id = $1 WHERE id = $2").bind(meena).bind(hr).execute(&pool).await?;
    sqlx::query("INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, manager_employee_id, valid) VALUES ($1, $2, 'reporting_manager', $3, daterange('2026-01-01', NULL))")
        .bind(org).bind(asha).bind(meena).execute(&pool).await?;
    let asha_user = user(&pool, org, "asha@dev.invalid", &hash, &[emp_role], Some(asha)).await?;
    let extras: Uuid = sqlx::query_scalar("INSERT INTO iam.role (org_id, name) VALUES ($1, 'Dev leave extras') RETURNING id").bind(org).fetch_one(&pool).await?;
    for op in ["LVE-03", "LVE-06", "LVE-07"] {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind) VALUES ($1, $2, $3, 'scoped')").bind(org).bind(extras).bind(op).execute(&pool).await?;
    }
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(hr).bind(extras).execute(&pool).await?;

    // Yesterday in India: in late at 09:20, out at 17:30 (03:50Z and 12:00Z).
    let d = (chrono::Utc::now() + chrono::Duration::minutes(330)).date_naive() - chrono::Duration::days(1);
    let entries = [format!("{d}T03:50:00Z"), format!("{d}T12:00:00Z")].map(|at| json!({ "employee_id": asha, "event_at": at }));
    call(&app, t, "POST", "/v1/manual-attendance", Some(json!({ "branch_id": hq, "reason": "dev seed: device down", "entries": entries }))).await?;

    // Leave: two types, balances for Asha, and a pending request of hers three days ahead.
    let version = |name: &str| json!({ "effective_from": "2026-01-01", "reason": format!("dev seed: {name}"), "paid": true, "eligibility": {},
        "credit_method": "annual_grant", "accrual_timing": "end", "proration": "none", "year_end": "lapse", "negative_allowed_days": "0.00",
        "half_day_allowed": true, "min_notice_days": 0, "approval_levels": "manager", "timeout_action": "escalate", "during_notice": "allow", "combine_allowed": true });
    let mut types = vec![];
    for (code, name, days) in [("CL", "Casual leave", "6.00"), ("SL", "Sick leave", "4.00")] {
        let ty = call(&app, t, "POST", "/v1/leave/types", Some(json!({ "code": code, "name": name, "version": version(name) }))).await?;
        call(&app, t, "POST", &format!("/v1/leave/employees/{asha}/ledger-adjustments"),
            Some(json!({ "leave_type_id": ty["id"], "quantity": days, "entry_date": "2026-01-01", "reason": "dev seed: opening balance" }))).await?;
        types.push(ty["id"].clone());
    }
    // A permission type: up to two hours, twice a month, the manager approves, applied for until two days after.
    call(&app, t, "POST", "/v1/permissions/types", Some(json!({ "code": "PER", "name": "Personal permission", "version": {
        "effective_from": "2026-01-01", "reason": "dev seed", "duration_unit": "block_30", "min_minutes": 30, "max_minutes": 120,
        "limit_basis": "count", "limit_count": 2, "limit_period": "calendar_month", "max_per_day": 1, "allowed_position": "any",
        "paid": true, "counts_as_worked": true, "counts_toward_overtime": false, "apply_window": "after_within_days", "after_event_days": 2,
        "min_notice_minutes": 0, "approval": "manager", "timeout_action": "escalate", "over_limit": "block", "over_stay": "late_early_minutes",
        "same_day_half_leave_allowed": false, "adjacent_off_allowed": true, "mid_period_joiner": "full", "eligibility": {},
        "blackout_enabled": false, "blackout_dates": [], "punch_requirement": {}, "converted_uses_limit": true } }))).await?;
    let asha_token = domain_iam::tokens::issue_access(&secret, asha_user, org, None, now);
    let ahead = d + chrono::Duration::days(4);
    call(&app, &asha_token, "POST", "/v1/leave/requests", Some(json!({ "leave_type_id": types[0], "from_date": ahead, "to_date": ahead, "reason": "Family function" }))).await?;

    println!("seeded organisation {org}: sign in as hr@dev.invalid (HR, manager of Asha) or asha@dev.invalid; punches on {d}");
    Ok(())
}
