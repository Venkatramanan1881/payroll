//! permissions.yaml, on-duty part (C09, PM-R5): apply, list, get, approve/reject, cancel.

use crate::routes::punches::{sees, visible_employees};
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveDate, NaiveTime};
use db::OrgTx;
use domain_attendance::onduty::{self as od, NewOnDuty};
use domain_iam::authz::{require, Principal};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// Own requests plus the PRM-11 scope; approvers also see what is routed to or was decided by them.
async fn readable(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Option<Vec<Uuid>>, ApiError> {
    let mut out: Vec<Uuid> = p.employee_id.into_iter().collect();
    match visible_employees(tx, p, "PRM-11").await {
        Ok(None) => return Ok(None),
        Ok(Some(more)) => out.extend(more),
        Err(ApiError::Forbidden) => {}
        Err(e) => return Err(e),
    }
    Ok(Some(out))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestIn {
    duration_kind: String,
    from_date: NaiveDate,
    to_date: NaiveDate,
    from_time: Option<NaiveTime>,
    to_time: Option<NaiveTime>,
    purpose: String,
    place: String,
    proof: Option<Value>,
}

/// POST /v1/on-duty/requests (PRM-08, own).
pub async fn submit(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "PRM-08").await?;
    let employee = p.employee_id.ok_or(ApiError::Forbidden)?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/on-duty/requests", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RequestIn = parse(&raw)?;
    if r.proof.is_some() {
        return Err(ApiError::ValidationFailed("proof attachments are not available yet".into()));
    }
    let new = NewOnDuty { kind: r.duration_kind, from: r.from_date, to: r.to_date, from_time: r.from_time, to_time: r.to_time, purpose: r.purpose, place: r.place };
    let id = od::submit(&mut tx, p.user_id, employee, new).await?;
    let body = od::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ListQuery {
    employee_id: Option<Uuid>,
    state: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/on-duty/requests.
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    let employees = readable(&mut tx, &p).await?;
    let f = od::Filter { employees, approver: p.user_id, employee: q.employee_id, state: q.state, from: q.from, to: q.to };
    Ok(Json(json!({ "items": od::list(&mut tx, &f).await?, "next_cursor": null })))
}

/// GET /v1/on-duty/requests/{id}: own, in scope, or routed to / decided by the caller; otherwise NOT_FOUND.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let body = od::get(&mut tx, id).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    let visible = sees(&readable(&mut tx, &p).await?, employee);
    let routed = body["current_approver_user_id"] == json!(p.user_id) || body["steps"].as_array().is_some_and(|s| s.iter().any(|x| x["actor_user_id"] == json!(p.user_id)));
    if visible || routed { Ok(Json(body)) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(Auth(p, mut tx): Auth, id: Uuid, headers: HeaderMap, raw: Bytes, approve: bool) -> Result<Response, ApiError> {
    let op = format!("POST /v1/on-duty/requests/{id}/{}", if approve { "approve" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let d: DecisionIn = if raw.iter().all(u8::is_ascii_whitespace) { DecisionIn::default() } else { parse(&raw)? };
    let note = if approve { d.comment } else { d.reason };
    od::decide(&mut tx, &p, id, approve, note.as_deref()).await?;
    let body = od::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/on-duty/requests/{id}/approve (PRM-09 level 1, PRM-10 level 2).
pub async fn approve(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, true).await
}

/// POST /v1/on-duty/requests/{id}/reject.
pub async fn reject(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonIn {
    reason: String,
}

/// POST /v1/on-duty/requests/{id}/cancel (PRM-11 within scope; G-27 open, so no role holds it by default).
pub async fn cancel(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let employee = od::employee_of(&mut tx, id).await?;
    let in_scope = match visible_employees(&mut tx, &p, "PRM-11").await {
        Ok(v) => sees(&v, Some(employee)),
        Err(ApiError::Forbidden) if p.employee_id == Some(employee) => return Err(ApiError::Forbidden),
        Err(ApiError::Forbidden) => false,
        Err(e) => return Err(e),
    };
    if !in_scope {
        return Err(ApiError::NotFound);
    }
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/on-duty/requests/{id}/cancel"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ReasonIn = parse(&raw)?;
    od::cancel(&mut tx, &p, id, &r.reason).await?;
    let body = od::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}
