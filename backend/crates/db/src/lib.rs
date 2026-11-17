//! Database access: migrations and organisation-scoped transactions (DATA_MODEL §4).

use sqlx::{PgPool, Postgres, Transaction};
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

/// A transaction confined to one organisation by row-level security.
pub struct OrgTx<'a>(pub Transaction<'a, Postgres>);

impl<'a> Deref for OrgTx<'a> {
    type Target = Transaction<'a, Postgres>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OrgTx<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> OrgTx<'a> {
    pub async fn commit(self) -> sqlx::Result<()> {
        self.0.commit().await
    }
}

/// Starts a transaction as `app_api` (never bypasses RLS, even if the pool logs in as the owner)
/// with the organisation taken from the verified caller, never from the request body (TB-01).
pub async fn begin_org(pool: &PgPool, org_id: Uuid) -> sqlx::Result<OrgTx<'static>> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut *tx).await?;
    sqlx::query("SELECT app.set_org($1)").bind(org_id).execute(&mut *tx).await?;
    Ok(OrgTx(tx))
}

/// Writes one insert-only audit entry in the caller's transaction, so it commits or rolls back with the change (TSD §3.3).
pub async fn audit(
    tx: &mut OrgTx<'_>,
    actor_id: Uuid,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
    after: Option<serde_json::Value>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO audit.audit_entry (org_id, actor_kind, actor_id, action, target_type, target_id, after_value)
         VALUES (app.current_org(), 'user', $1, $2, $3, $4, $5)",
    )
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(after)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// Error shared by the domain crates added from slice 2 on.
#[derive(Debug)]
pub enum DomainError {
    Validation(String),
    NotFound,
    IllegalTransition(String),
    VersionFrozen,
    PeriodLocked,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for DomainError {
    /// Constraint violations come from the caller's input (422); the G5 freeze trigger is VERSION_FROZEN.
    fn from(e: sqlx::Error) -> Self {
        if let Some(db) = e.as_database_error() {
            match db.code().as_deref() {
                Some("23503" | "23505" | "23514" | "23P01" | "22007" | "22008" | "22P02") => {
                    return DomainError::Validation(db.constraint().unwrap_or(db.message()).to_string())
                }
                // RLS violations share 42501, so only the trigger's own "(G5)" message counts as frozen.
                Some("42501") if db.message().contains("(G5)") => return DomainError::VersionFrozen,
                _ => {}
            }
        }
        DomainError::Db(e)
    }
}

pub fn invalid(msg: impl Into<String>) -> DomainError {
    DomainError::Validation(msg.into())
}

/// True when any employee's locked attendance (period.lock_coverage) overlaps [from, to) — `to` None = open-ended.
/// For organisation-wide changes (shift and policy versions, assignments, holidays) that could alter locked days (BR-06).
pub async fn any_locked(tx: &mut OrgTx<'_>, from: chrono::NaiveDate, to: Option<chrono::NaiveDate>) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.lock_coverage WHERE dates && daterange($1, $2))")
        .bind(from)
        .bind(to)
        .fetch_one(&mut ***tx)
        .await
}

/// True when this employee's locked attendance overlaps [from, to).
pub async fn employee_locked(tx: &mut OrgTx<'_>, employee: Uuid, from: chrono::NaiveDate, to: Option<chrono::NaiveDate>) -> sqlx::Result<bool> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.lock_coverage WHERE employee_id = $1 AND dates && daterange($2, $3))")
        .bind(employee)
        .bind(from)
        .bind(to)
        .fetch_one(&mut ***tx)
        .await
}
