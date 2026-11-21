//! permissions.yaml (C09, permissions part): types and versions, requests, decisions, cancellation, usage.
//! On-duty, the change route (PM-29 change, OQ-C09-10) and late/early conversion (LA-15 / EC-13) come later.

use crate::routes::org::require_org_wide;
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
use domain_attendance::permission::{self as pm, NewPermission};
use domain_iam::authz::{require, Principal};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// The employees the caller sees requests for: their own plus every HR-type read scope they hold.
async fn readable(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Option<Vec<Uuid>>, ApiError> {
    let mut out: Option<Vec<Uuid>> = Some(p.employee_id.into_iter().collect());
    for op in ["PRM-07", "PRM-13"] {
        match visible_employees(tx, p, op).await {
            Ok(None) => return Ok(None),
            Ok(Some(more)) => out.as_mut().expect("set").extend(more),
            Err(ApiError::Forbidden) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
pub struct TypesQuery {
    on_date: Option<NaiveDate>,
}

/// GET /v1/permissions/types (any signed-in user; eligibility rules PM-25 are not enforced yet, so all are offered).
pub async fn list_types(Auth(_p, mut tx): Auth, Query(q): Query<TypesQuery>) -> Result<Json<Value>, ApiError> {
    let on = q.on_date.unwrap_or_else(crate::routes::employees::today);
    Ok(Json(json!({ "items": pm::list_types(&mut tx, on).await?, "next_cursor": null })))
}

/// GET /v1/permissions/types/{id}.
pub async fn get_type(Auth(_p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    Ok(Json(pm::get_type(&mut tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewType {
    code: String,
    name: String,
    version: Value,
}

/// POST /v1/permissions/types (PRM-01).
pub async fn create_type(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PRM-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/permissions/types", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let t: NewType = parse(&raw)?;
    let id = pm::create_type(&mut tx, p.user_id, &t.code, &t.name, &t.version).await?;
    let body = pm::get_type(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// POST /v1/permissions/types/{id}/versions (PRM-01).
pub async fn publish_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PRM-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/permissions/types/{id}/versions"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let v: Value = parse(&raw)?;
    let vid = pm::publish_version(&mut tx, p.user_id, id, &v).await?;
    let body = pm::get_version(&mut tx, vid).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestIn {
    permission_type_id: Uuid,
    permission_date: NaiveDate,
    start_time: NaiveTime,
    end_time: NaiveTime,
    reason: String,
}

/// POST /v1/permissions/requests (PRM-02, own).
pub async fn submit(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "PRM-02").await?;
    let employee = p.employee_id.ok_or(ApiError::Forbidden)?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/permissions/requests", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RequestIn = parse(&raw)?;
    if r.start_time == r.end_time {
        return Err(ApiError::ValidationFailed("start_time and end_time differ".into()));
    }
    let id = pm::submit(&mut tx, p.user_id, employee, NewPermission { ptype: r.permission_type_id, date: r.permission_date, start: r.start_time, end: r.end_time, reason: r.reason }).await?;
    let body = pm::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ListQuery {
    employee_id: Option<Uuid>,
    state: Option<String>,
    origin: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    awaiting_me: Option<bool>,
}

/// GET /v1/permissions/requests: own, in scope, or routed to the caller; `awaiting_me` = those the caller may decide now.
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    if q.awaiting_me == Some(true) {
        return Ok(Json(json!({ "items": pm::awaiting(&mut tx, &p).await?, "next_cursor": null })));
    }
    let employees = readable(&mut tx, &p).await?;
    let f = pm::Filter { employees, approver: p.user_id, employee: q.employee_id, state: q.state, origin: q.origin, from: q.from, to: q.to };
    Ok(Json(json!({ "items": pm::list(&mut tx, &f).await?, "next_cursor": null })))
}

/// GET /v1/permissions/requests/{id}: own, in scope, or routed to / decided by the caller; otherwise NOT_FOUND.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let body = pm::get(&mut tx, id).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    let visible = sees(&readable(&mut tx, &p).await?, employee);
    let routed = body["current_approver_user_id"] == json!(p.user_id) || body["steps"].as_array().is_some_and(|s| s.iter().any(|x| x["actor_user_id"] == json!(p.user_id)));
    // A caller who may decide it (e.g. a delegate, or the HR pool) may also read it, as for leave.
    let can_decide = pm::can_decide(&mut tx, &p, id).await?;
    if !(visible || routed || can_decide) {
        return Err(ApiError::NotFound);
    }
    let mut body = body;
    // What this caller may do now, by the checks approve / reject / cancel make (clients never rebuild them).
    body["can_decide"] = json!(can_decide);
    body["can_cancel"] = json!(may_cancel(&mut tx, &p, id).await?);
    Ok(Json(body))
}

/// cancel()'s checks: one's own with PRM-06 before the start time (PM-29), or HR with the employee in PRM-07 scope.
async fn may_cancel(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid) -> Result<bool, ApiError> {
    let employee = pm::employee_of(tx, id).await?;
    let own = p.employee_id == Some(employee);
    if own && !holds(tx, p, "PRM-06").await? {
        return Ok(false);
    }
    let hr = !own
        && match visible_employees(tx, p, "PRM-07").await {
            Ok(v) => sees(&v, Some(employee)),
            Err(ApiError::Forbidden) => false,
            Err(e) => return Err(e),
        };
    if !own && !hr {
        return Ok(false);
    }
    Ok(pm::can_cancel(tx, p, id, hr).await?)
}

async fn holds(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<bool, ApiError> {
    match require(tx, p, op).await {
        Ok(_) => Ok(true),
        Err(domain_iam::authz::AuthzError::Forbidden) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(Auth(p, mut tx): Auth, id: Uuid, headers: HeaderMap, raw: Bytes, approve: bool) -> Result<Response, ApiError> {
    let op = format!("POST /v1/permissions/requests/{id}/{}", if approve { "approve" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let d: DecisionIn = if raw.iter().all(u8::is_ascii_whitespace) { DecisionIn::default() } else { parse(&raw)? };
    let note = if approve { d.comment } else { d.reason };
    pm::decide(&mut tx, &p, id, approve, note.as_deref()).await?;
    let body = pm::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/permissions/requests/{id}/approve (PRM-03 level 1, PRM-04 level 2).
pub async fn approve(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, true).await
}

/// POST /v1/permissions/requests/{id}/reject.
pub async fn reject(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonIn {
    reason: String,
}

/// POST /v1/permissions/requests/{id}/cancel: own before the start (PRM-06), HR within scope after it (PRM-07).
pub async fn cancel(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let employee = pm::employee_of(&mut tx, id).await?;
    let hr = match visible_employees(&mut tx, &p, "PRM-07").await {
        Ok(v) => sees(&v, Some(employee)),
        Err(ApiError::Forbidden) => false,
        Err(e) => return Err(e),
    };
    let own = p.employee_id == Some(employee);
    if !hr && !own {
        return Err(ApiError::NotFound);
    }
    if own {
        require(&mut tx, &p, "PRM-06").await?;
    }
    let hr = hr && !own; // one's own permission follows PM-29's employee rule, whatever else one holds
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/permissions/requests/{id}/cancel"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ReasonIn = parse(&raw)?;
    pm::cancel(&mut tx, &p, id, &r.reason, hr).await?;
    let body = pm::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
pub struct UsageQuery {
    on_date: Option<NaiveDate>,
}

/// GET /v1/permissions/employees/{employee_id}/usage (PRM-13: own, or within scope).
pub async fn usage(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>, Query(q): Query<UsageQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "PRM-13").await?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::NotFound);
    }
    let on = q.on_date.unwrap_or_else(crate::routes::employees::today);
    Ok(Json(json!({ "employee_id": employee, "items": pm::usage(&mut tx, employee, on).await? })))
}
