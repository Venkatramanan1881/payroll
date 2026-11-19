//! Vendor support access (VC-R6, VC-03, SM-SUPPORT): requested by a vendor user, decided and ended inside the
//! organisation, time-limited, every action logged. An active grant past its window reads as `expired` (S5).
//! Data access under a grant (a grant-bound token, PLT-16) waits on G-02 / OQ-V01-12: a grant opens nothing yet.

use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// OQ-V01-13 (decided for now): a requested window, default 2 hours, at most 8.
pub const DEFAULT_WINDOW_MINUTES: i64 = 120;
pub const MAX_WINDOW_MINUTES: i64 = 480;

const GRANT_JSON: &str = "jsonb_build_object('id', g.id, 'org_id', g.org_id, 'vendor_user_id', g.vendor_user_id, 'reason', g.reason,
    'state', CASE WHEN g.state = 'active' AND g.window_end <= now() THEN 'expired' ELSE g.state END,
    'requested_at', g.requested_at, 'decided_at', g.decided_at, 'decided_by', g.decided_by, 'window_start', g.window_start, 'window_end', g.window_end,
    'ended_at', g.ended_at)";

// ---------------------------------------------------------------------------------------------------------------------
// Vendor side (runs as app_vendor)

/// S1 / S2: requested (VC-03 on) or active at once (off). Every request is new (AC-V01-09.3).
pub async fn request(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, reason: &str, window_minutes: Option<i64>, key: Uuid) -> Result<Value, DomainError> {
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > 1000 {
        return Err(DomainError::Validation("reason is required (1-1000 characters)".into()));
    }
    let minutes = window_minutes.unwrap_or(DEFAULT_WINDOW_MINUTES);
    if !(1..=MAX_WINDOW_MINUTES).contains(&minutes) {
        return Err(DomainError::Validation(format!("requested_window_minutes is 1-{MAX_WINDOW_MINUTES}")));
    }
    let needs: Option<bool> = sqlx::query_scalar("SELECT support_access_needs_approval FROM platform.organisation WHERE id = $1").bind(org).fetch_optional(&mut ***tx).await?;
    let needs = needs.ok_or(DomainError::NotFound)?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO platform.support_access_grant (org_id, vendor_user_id, reason, state, window_start, window_end)
         VALUES ($1, $2, $3, $4, CASE WHEN $4 = 'active' THEN now() END, CASE WHEN $4 = 'active' THEN now() + make_interval(mins => $5) END) RETURNING id",
    )
    .bind(org)
    .bind(vendor_user)
    .bind(reason)
    .bind(if needs { "requested" } else { "active" })
    .bind(minutes as i32)
    .fetch_one(&mut ***tx)
    .await?;
    sqlx::query(
        "INSERT INTO audit.audit_entry (org_id, actor_kind, actor_id, action, target_type, target_id, after_value, change_ref)
         VALUES ($1, 'vendor_user', $2, 'support.requested', 'support_access_grant', $3, $4, $5)",
    )
    .bind(org)
    .bind(vendor_user)
    .bind(id)
    .bind(json!({ "reason": reason, "window_minutes": minutes, "needs_approval": needs }))
    .bind(key.to_string())
    .execute(&mut ***tx)
    .await?;
    get(tx, id).await
}

pub async fn list_for_vendor(tx: &mut OrgTx<'_>, vendor_user: Uuid, state: Option<&str>, org: Option<Uuid>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT e FROM (SELECT {GRANT_JSON} AS e, g.requested_at FROM platform.support_access_grant g
                        WHERE g.vendor_user_id = $1 AND ($3::uuid IS NULL OR g.org_id = $3)) x
         WHERE $2::text IS NULL OR e->>'state' = $2 ORDER BY requested_at DESC"
    ))
    .bind(vendor_user)
    .bind(state)
    .bind(org)
    .fetch_all(&mut ***tx)
    .await?)
}

// ---------------------------------------------------------------------------------------------------------------------
// Organisation side (runs as app_api under the organisation: RLS shows its own grants only)

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {GRANT_JSON} FROM platform.support_access_grant g WHERE g.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list(tx: &mut OrgTx<'_>, state: Option<&str>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT e FROM (SELECT {GRANT_JSON} AS e, g.requested_at FROM platform.support_access_grant g) x
         WHERE $1::text IS NULL OR e->>'state' = $1 ORDER BY requested_at DESC"
    ))
    .bind(state)
    .fetch_all(&mut ***tx)
    .await?)
}

/// (stored state, effective state) under a row lock.
async fn lock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<String, DomainError> {
    let row: Option<String> = sqlx::query_scalar(
        "SELECT CASE WHEN state = 'active' AND window_end <= now() THEN 'expired' ELSE state END FROM platform.support_access_grant WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    row.ok_or(DomainError::NotFound)
}

/// S3: requested -> active for a window from now (`window_end`, else the default length, at most 8 hours).
pub async fn approve(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, window_end: Option<chrono::DateTime<chrono::Utc>>, comment: Option<&str>) -> Result<Value, DomainError> {
    match lock(tx, id).await?.as_str() {
        "active" => return get(tx, id).await,
        "requested" => {}
        s => return Err(DomainError::IllegalTransition(format!("the grant is {s}"))),
    }
    let now = chrono::Utc::now();
    // Without a window_end the requested length applies (kept on the request's audit entry).
    let requested: Option<i64> = sqlx::query_scalar(
        "SELECT (after_value->>'window_minutes')::bigint FROM audit.audit_entry WHERE action = 'support.requested' AND target_id = $1 ORDER BY occurred_at LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?
    .flatten();
    let end = window_end.unwrap_or(now + chrono::Duration::minutes(requested.unwrap_or(DEFAULT_WINDOW_MINUTES)));
    if end <= now || end > now + chrono::Duration::minutes(MAX_WINDOW_MINUTES) {
        return Err(DomainError::Validation(format!("window_end is in the next {MAX_WINDOW_MINUTES} minutes")));
    }
    sqlx::query("UPDATE platform.support_access_grant SET state = 'active', decided_by = $2, decided_at = now(), window_start = now(), window_end = $3 WHERE id = $1")
        .bind(id)
        .bind(actor)
        .bind(end)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "support.approved", "support_access_grant", Some(id), Some(json!({ "window_end": end, "comment": comment }))).await?;
    get(tx, id).await
}

/// S4: requested -> rejected, with a reason.
pub async fn reject(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, reason: &str) -> Result<Value, DomainError> {
    match lock(tx, id).await?.as_str() {
        "rejected" => return get(tx, id).await,
        "requested" => {}
        s => return Err(DomainError::IllegalTransition(format!("the grant is {s}"))),
    }
    sqlx::query("UPDATE platform.support_access_grant SET state = 'rejected', decided_by = $2, decided_at = now() WHERE id = $1").bind(id).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, "support.rejected", "support_access_grant", Some(id), Some(json!({ "reason": reason }))).await?;
    get(tx, id).await
}

/// S6: active -> ended now.
pub async fn end(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, reason: &str) -> Result<Value, DomainError> {
    match lock(tx, id).await?.as_str() {
        "ended" => return get(tx, id).await,
        "active" => {}
        s => return Err(DomainError::IllegalTransition(format!("the grant is {s}"))),
    }
    sqlx::query("UPDATE platform.support_access_grant SET state = 'ended', ended_at = now() WHERE id = $1").bind(id).execute(&mut ***tx).await?;
    db::audit(tx, actor, "support.ended", "support_access_grant", Some(id), Some(json!({ "reason": reason }))).await?;
    get(tx, id).await
}

/// VC-R6: every audited action carrying the grant (none until grant-bound access exists, G-02).
pub async fn actions(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Vec<Value>, DomainError> {
    get(tx, id).await?;
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('occurred_at', occurred_at, 'vendor_user_id', actor_id, 'action', action, 'target_type', target_type, 'target_id', target_id)
         FROM audit.audit_entry WHERE support_grant_id = $1 ORDER BY occurred_at",
    )
    .bind(id)
    .fetch_all(&mut ***tx)
    .await?)
}
