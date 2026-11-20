//! Local development seed for the web app: one organisation with an HR Manager who signs in with a password and no
//! authenticator step, two branches and a few departments. Refuses any database that is not on localhost.
//!
//! DATABASE_URL=postgres://postgres@localhost:55432/payroll_dev DEV_ADMIN_PASSWORD=... cargo run -p api --example dev_seed

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL")?;
    let host = url.parse::<sqlx::postgres::PgConnectOptions>()?.get_host().to_string();
    anyhow::ensure!(host == "localhost" || host == "127.0.0.1", "dev_seed only runs against a local database, not {host}");
    let password = std::env::var("DEV_ADMIN_PASSWORD").map_err(|_| anyhow::anyhow!("DEV_ADMIN_PASSWORD is required"))?;
    anyhow::ensure!(password.len() >= 12, "DEV_ADMIN_PASSWORD needs at least 12 characters");
    let email = std::env::var("DEV_ADMIN_EMAIL").unwrap_or_else(|_| "hr@dev.invalid".into());
    let pool = sqlx::PgPool::connect(&url).await?;
    db::MIGRATOR.run(&pool).await?;
    let mut tx = pool.begin().await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE lower(email) = lower($1))").bind(&email).fetch_one(&mut *tx).await?;
    anyhow::ensure!(!exists, "{email} already exists: drop the dev database to seed again");

    let vendor: Uuid = sqlx::query_scalar("INSERT INTO platform.vendor_user (email, full_name, vendor_role) VALUES ($1, 'Dev vendor', 'full_admin') RETURNING id")
        .bind(format!("vendor-{}@dev.invalid", Uuid::new_v4())).fetch_one(&mut *tx).await?;
    let org: Uuid = sqlx::query_scalar("INSERT INTO platform.organisation (legal_name, display_name) VALUES ('Dev Bakery Pvt Ltd', 'Dev Bakery') RETURNING id").fetch_one(&mut *tx).await?;
    for m in ["C01", "C02", "C03"] {
        sqlx::query("INSERT INTO platform.module_entitlement (org_id, module_code, enabled, valid, set_by) VALUES ($1, $2, true, daterange('2000-01-01', NULL), $3)")
            .bind(org).bind(m).bind(vendor).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason) VALUES ($1, '2000-01-01', '{\"admin_second_step\": false}', 1, 'dev seed')")
        .bind(org).execute(&mut *tx).await?;
    for (code, name) in [("HQ", "Head office"), ("ANN", "Anna Nagar")] {
        sqlx::query("INSERT INTO org.branch (org_id, code, name) VALUES ($1, $2, $3)").bind(org).bind(code).bind(name).execute(&mut *tx).await?;
    }
    for (kind, name) in [("department", "Production"), ("department", "Sales"), ("designation", "Baker"), ("designation", "Cashier")] {
        sqlx::query("INSERT INTO org.list_entry (org_id, kind, name) VALUES ($1, $2, $3)").bind(org).bind(kind).bind(name).execute(&mut *tx).await?;
    }
    let role: Uuid = sqlx::query_scalar("INSERT INTO iam.role (org_id, name, system_code) VALUES ($1, 'HR Manager', 'hr_manager') RETURNING id").bind(org).fetch_one(&mut *tx).await?;
    for (_, op, kind, setting) in domain_iam::defaults::SYSTEM_ROLE_GRANTS.iter().filter(|g| g.0 == "hr_manager") {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind, setting_ref) VALUES ($1, $2, $3, $4, $5)")
            .bind(org).bind(role).bind(op).bind(kind).bind(setting).execute(&mut *tx).await?;
    }
    let hash = argon2::Argon2::default().hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng)).map_err(|e| anyhow::anyhow!("{e}"))?.to_string();
    let user: Uuid = sqlx::query_scalar("INSERT INTO iam.user_account (org_id, login_kind, email, password_hash) VALUES ($1, 'password', $2, $3) RETURNING id")
        .bind(org).bind(&email).bind(hash).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(user).bind(role).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(user).execute(&mut *tx).await?;
    tx.commit().await?;
    println!("seeded organisation {org}; sign in as {email}");
    Ok(())
}
