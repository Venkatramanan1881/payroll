//! identity.yaml support-access grants, organisation side (VC-R6): list, approve, reject, end, action log.
//! Organisation-wide only; RLS shows the organisation its own grants.

use crate::routes::org::require_org_wide;
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, Utc};
use domain_iam::authz::{require, AuthzError};
use domain_iam::support;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ListQuery {
    state: Option<String>,
}

/// GET /v1/support-access-grants: approvers (PLT-14) or log readers (PLT-18).
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    if let Err(e) = require(&mut tx, &p, "PLT-14").await {
        if !matches!(e, AuthzError::Forbidden) {
            return Err(e.into());
        }
        require(&mut tx, &p, "PLT-18").await?;
    }
    Ok(Json(json!({ "items": support::list(&mut tx, q.state.as_deref()).await?, "next_cursor": null })))
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ApproveIn {
    window_end: Option<DateTime<Utc>>,
    comment: Option<String>,
}

/// POST /v1/support-access-grants/{id}/approve (PLT-14).
pub async fn approve(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "PLT-14").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/support-access-grants/{id}/approve"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let b: ApproveIn = if raw.iter().all(u8::is_ascii_whitespace) { ApproveIn::default() } else { parse(&raw)? };
    let body = support::approve(&mut tx, p.user_id, id, b.window_end, b.comment.as_deref()).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonIn {
    reason: String,
}

fn reason(raw: &[u8]) -> Result<String, ApiError> {
    let r: ReasonIn = parse(raw)?;
    let t = r.reason.trim().to_string();
    if t.is_empty() || t.chars().count() > 1000 {
        return Err(ApiError::ValidationFailed("reason is required (1-1000 characters)".into()));
    }
    Ok(t)
}

/// POST /v1/support-access-grants/{id}/reject (PLT-14).
pub async fn reject(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "PLT-14").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/support-access-grants/{id}/reject"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let body = support::reject(&mut tx, p.user_id, id, &reason(&raw)?).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/support-access-grants/{id}/end (PLT-15; no role holds it by default, G-03).
pub async fn end(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PLT-15").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/support-access-grants/{id}/end"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let body = support::end(&mut tx, p.user_id, id, &reason(&raw)?).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// GET /v1/support-access-grants/{id}/actions (PLT-18).
pub async fn actions(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "PLT-18").await?;
    Ok(Json(json!({ "items": support::actions(&mut tx, id).await?, "next_cursor": null })))
}
