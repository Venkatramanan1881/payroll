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
