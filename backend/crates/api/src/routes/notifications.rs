//! notifications.yaml (C14): rules (NTF-01), the caller's inbox, push device registration.

use crate::routes::sched::{if_match, parse, reply};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use domain_attendance::notify::{self, Change};
use domain_iam::authz::require;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

/// GET /v1/notifications/rules (NTF-01), with the rule set's ETag.
pub async fn rules(Auth(p, mut tx): Auth) -> Result<Response, ApiError> {
    require(&mut tx, &p, "NTF-01").await?;
    Ok(reply(StatusCode::OK, notify::rules(&mut tx).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeIn {
    event_code: String,
    channel: String,
    recipient_kind: String,
    enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RulesIn {
    reason: String,
    changes: Vec<ChangeIn>,
}

/// PATCH /v1/notifications/rules (NTF-01; If-Match the rule set).
// shortcut: saved at once; AC-04 "with OA approval" for notification settings waits on G-08 (OQ-C03-11).
pub async fn change_rules(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "NTF-01").await?;
    notify::lock_rules(&mut tx).await?;
    if_match(&headers, &notify::rules(&mut tx).await?)?;
    let r: RulesIn = parse(&raw)?;
    let changes: Vec<Change> = r.changes.into_iter().map(|c| Change { event: c.event_code, channel: c.channel, recipient: c.recipient_kind, enabled: c.enabled }).collect();
    let body = notify::change_rules(&mut tx, p.user_id, &changes, &r.reason).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
pub struct InboxQuery {
    unread: Option<bool>,
    event_code: Option<String>,
}

/// GET /v1/notifications/inbox (own).
pub async fn inbox(Auth(p, mut tx): Auth, Query(q): Query<InboxQuery>) -> Result<Json<Value>, ApiError> {
    Ok(Json(notify::inbox(&mut tx, p.user_id, q.unread, q.event_code.as_deref()).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarkIn {
    ids: Option<Vec<Uuid>>,
    all: Option<bool>,
}

/// POST /v1/notifications/inbox/mark-read: listed ids, or all.
pub async fn mark_read(Auth(p, mut tx): Auth, raw: Bytes) -> Result<StatusCode, ApiError> {
    let m: MarkIn = parse(&raw)?;
    match (&m.ids, m.all) {
        (Some(ids), None | Some(false)) if (1..=200).contains(&ids.len()) => notify::mark_read(&mut tx, p.user_id, Some(ids)).await?,
        (None, Some(true)) => notify::mark_read(&mut tx, p.user_id, None).await?,
        _ => return Err(ApiError::ValidationFailed("give ids (1-200) or all = true".into())),
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIn {
    token: String,
    platform: String,
    app_version: Option<String>,
}

/// POST /v1/notifications/push-devices (own).
pub async fn register_device(Auth(p, mut tx): Auth, raw: Bytes) -> Result<Json<Value>, ApiError> {
    let d: DeviceIn = parse(&raw)?;
    let body = notify::register_device(&mut tx, p.user_id, &d.token, &d.platform, d.app_version.as_deref()).await?;
    tx.commit().await?;
    Ok(Json(body))
}

/// DELETE /v1/notifications/push-devices/{id}: 204 whatever the id (no disclosure).
pub async fn unregister_device(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<StatusCode, ApiError> {
    notify::unregister_device(&mut tx, p.user_id, id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
