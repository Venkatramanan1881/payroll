//! Idempotency-Key for POST commands (API_CONVENTIONS §5), stored in app.idempotency_key (migration 0009).
//! The key row is written in the command's own transaction, so a failed command leaves no key and can be retried.

use crate::ApiError;
use axum::http::{HeaderMap, StatusCode};
use db::OrgTx;
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn key(headers: &HeaderMap) -> Result<Uuid, ApiError> {
    let raw = headers.get("idempotency-key").ok_or(ApiError::PreconditionRequired)?;
    raw.to_str().ok().and_then(|s| Uuid::parse_str(s).ok()).ok_or_else(|| ApiError::ValidationFailed("Idempotency-Key must be a UUID".into()))
}

/// Claims the key. Ok(None): first use, run the command. Ok(Some(..)): stored response to replay.
/// `operation` is the method and path, so one key reused on another resource is a conflict, not a replay.
pub async fn begin(tx: &mut OrgTx<'_>, key: Uuid, operation: &str, body: &[u8]) -> Result<Option<(StatusCode, Value)>, ApiError> {
    let hash = Sha256::new().chain_update(operation.as_bytes()).chain_update([0u8]).chain_update(body).finalize().to_vec();
    // A concurrent request with the same key waits here until the first one commits.
    let inserted = sqlx::query("INSERT INTO app.idempotency_key (org_id, key, request_hash) VALUES (app.current_org(), $1, $2) ON CONFLICT DO NOTHING")
        .bind(key)
        .bind(&hash)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    if inserted == 1 {
        return Ok(None);
    }
    let (stored_hash, status, response): (Vec<u8>, Option<i32>, Option<Value>) =
        sqlx::query_as("SELECT request_hash, response_status, response_body FROM app.idempotency_key WHERE key = $1")
            .bind(key)
            .fetch_one(&mut ***tx)
            .await?;
    if stored_hash != hash {
        return Err(ApiError::IdempotencyConflict);
    }
    let status = status.and_then(|s| StatusCode::from_u16(s as u16).ok()).ok_or_else(|| ApiError::Internal(anyhow::anyhow!("idempotency row without response")))?;
    Ok(Some((status, response.unwrap_or(Value::Null))))
}

pub async fn finish(tx: &mut OrgTx<'_>, key: Uuid, status: StatusCode, body: &Value) -> Result<(), ApiError> {
    sqlx::query("UPDATE app.idempotency_key SET response_status = $2, response_body = $3 WHERE key = $1")
        .bind(key)
        .bind(i32::from(status.as_u16()))
        .bind(body)
        .execute(&mut ***tx)
        .await?;
    Ok(())
}
