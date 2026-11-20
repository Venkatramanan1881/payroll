//! leave.yaml: leave types and versions, balances and ledger, leave requests.

use crate::routes::punches::{sees, visible_employees, FileRef};
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDate;
use db::OrgTx;
use domain_attendance::leave::{self, NewLeave};
use domain_iam::authz::{require, Principal};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

async fn holds(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<bool, ApiError> {
    match require(tx, p, op).await {
        Ok(_) => Ok(true),
        Err(domain_iam::authz::AuthzError::Forbidden) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

#[derive(Deserialize)]
pub struct TypesQuery {
    on_date: Option<NaiveDate>,
    include_retired: Option<bool>,
}

/// GET /v1/leave/types: LVE-01 holders see every type; others the types they are eligible for (LM-01).
pub async fn list_types(Auth(p, mut tx): Auth, Query(q): Query<TypesQuery>) -> Result<Json<Value>, ApiError> {
    let on = q.on_date.unwrap_or_else(crate::routes::employees::today);
    let items = if holds(&mut tx, &p, "LVE-01").await? {
        leave::list_types(&mut tx, on, q.include_retired.unwrap_or(false), None).await?
    } else {
        match p.employee_id {
            Some(e) => leave::list_types(&mut tx, on, false, Some(e)).await?,
            None => vec![],
        }
    };
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewType {
    code: String,
    name: String,
    version: Value,
}

/// POST /v1/leave/types (LVE-01).
pub async fn create_type(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "LVE-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/leave/types", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewType = parse(&raw)?;
    let id = leave::create_type(&mut tx, p.user_id, &r.code, &r.name, &r.version).await?;
    let body = leave::get_type(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// GET /v1/leave/types/{id}.
pub async fn get_type(Auth(_p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    Ok(Json(leave::get_type(&mut tx, id).await?))
}

/// POST /v1/leave/types/{id}/versions (LVE-01).
pub async fn publish_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "LVE-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/leave/types/{id}/versions"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let v: Value = parse(&raw)?;
    let version = leave::publish_version(&mut tx, p.user_id, id, &v).await?;
    let body = leave::get_version(&mut tx, version).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// LVE-08: the employee themself or within scope; otherwise NOT_FOUND.
async fn may_view(tx: &mut OrgTx<'_>, p: &Principal, employee: Uuid) -> Result<(), ApiError> {
    let visible = visible_employees(tx, p, "LVE-08").await?;
    if sees(&visible, Some(employee)) { Ok(()) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize)]
pub struct OnDate {
    on_date: Option<NaiveDate>,
}

/// GET /v1/leave/employees/{employee_id}/balances (LVE-08).
pub async fn balances(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>, Query(q): Query<OnDate>) -> Result<Json<Value>, ApiError> {
    may_view(&mut tx, &p, employee).await?;
    let balances = leave::balances(&mut tx, employee, q.on_date).await?;
    tx.commit().await?; // keeps the accounts opened for the reply
    Ok(Json(json!({ "employee_id": employee, "balances": balances })))
}

#[derive(Deserialize)]
pub struct LedgerQuery {
    leave_type_id: Option<Uuid>,
    entry_kind: Option<String>,
}

/// GET /v1/leave/employees/{employee_id}/ledger (LVE-08).
pub async fn ledger(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>, Query(q): Query<LedgerQuery>) -> Result<Json<Value>, ApiError> {
    may_view(&mut tx, &p, employee).await?;
    let items = leave::ledger(&mut tx, employee, q.leave_type_id, q.entry_kind.as_deref()).await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdjustIn {
    leave_type_id: Uuid,
    quantity: String,
    entry_date: NaiveDate,
    reason: String,
}

/// POST /v1/leave/employees/{employee_id}/ledger-adjustments (LVE-07, within scope; not your own balance).
pub async fn adjust(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "LVE-07").await?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::NotFound);
    }
    if p.employee_id == Some(employee) {
        return Err(ApiError::SelfApprovalForbidden); // nobody adjusts their own balance (BR-07 spirit, G-26 open)
    }
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/leave/employees/{employee}/ledger-adjustments"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: AdjustIn = parse(&raw)?;
    let body = leave::adjust(&mut tx, p.user_id, employee, r.leave_type_id, &r.quantity, r.entry_date, &r.reason).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// LeaveRequestInput (+ employee_id for on-behalf).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeaveIn {
    employee_id: Option<Uuid>,
    leave_type_id: Uuid,
    from_date: NaiveDate,
    to_date: NaiveDate,
    from_part: Option<String>,
    to_part: Option<String>,
    reason: Option<String>,
    document: Option<FileRef>,
}

impl LeaveIn {
    fn request(self) -> NewLeave {
        NewLeave {
            leave_type: self.leave_type_id,
            from: self.from_date,
            to: self.to_date,
            from_part: self.from_part.unwrap_or_else(|| "full".into()),
            to_part: self.to_part.unwrap_or_else(|| "full".into()),
            reason: self.reason,
            document: self.document.map(|d| d.path),
        }
    }
}

/// POST /v1/leave/requests (LVE-02, own: the employee comes from the token).
pub async fn submit(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "LVE-02").await?;
    let employee = p.employee_id.ok_or(ApiError::Forbidden)?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/leave/requests", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: LeaveIn = parse(&raw)?;
    if r.employee_id.is_some() {
        return Err(ApiError::ValidationFailed("employee_id is not accepted here; use /v1/leave/requests/on-behalf".into()));
    }
    let id = leave::submit(&mut tx, p.user_id, employee, r.request()).await?;
    let body = leave::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// POST /v1/leave/requests/on-behalf (LVE-03, within scope).
pub async fn submit_on_behalf(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "LVE-03").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/leave/requests/on-behalf", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: LeaveIn = parse(&raw)?;
    let employee = r.employee_id.ok_or_else(|| ApiError::ValidationFailed("employee_id is required".into()))?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::ValidationFailed("employee_id is not an employee you manage".into()));
    }
    let id = leave::submit(&mut tx, p.user_id, employee, r.request()).await?;
    let body = leave::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ListQuery {
    employee_id: Option<Uuid>,
    state: Option<String>,
    leave_type_id: Option<Uuid>,
    awaiting_me: Option<bool>,
}

/// GET /v1/leave/requests: own requests, employees in LVE-08 scope, and requests routed to the caller.
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    if q.awaiting_me == Some(true) {
        return Ok(Json(json!({ "items": leave::awaiting(&mut tx, &p).await?, "next_cursor": null })));
    }
    let mut employees: Option<Vec<Uuid>> = Some(p.employee_id.into_iter().collect());
    match visible_employees(&mut tx, &p, "LVE-08").await {
        Ok(None) => employees = None,
        Ok(Some(more)) => employees.as_mut().expect("set above").extend(more),
        Err(ApiError::Forbidden) => {}
        Err(e) => return Err(e),
    }
    if let Some(e) = q.employee_id {
        employees = Some(match employees {
            None => vec![e],
            Some(v) => v.into_iter().filter(|x| *x == e).collect(),
        });
    }
    let f = leave::Filter { employees, employee: q.employee_id, state: q.state, leave_type: q.leave_type_id };
    Ok(Json(json!({ "items": leave::list(&mut tx, &p, &f).await?, "next_cursor": null })))
}

/// GET /v1/leave/requests/{id}: own, in LVE-08 scope, or routed to the caller; otherwise NOT_FOUND.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let body = leave::get(&mut tx, id).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    let visible = match visible_employees(&mut tx, &p, "LVE-08").await {
        Ok(v) => sees(&v, employee),
        Err(ApiError::Forbidden) => false,
        Err(e) => return Err(e),
    };
    let can_decide = leave::can_decide(&mut tx, &p, id).await?;
    if p.employee_id.is_some() && p.employee_id == employee || visible || can_decide {
        let mut body = body;
        // What this caller may do now, by the same checks approve/reject/cancel make (clients never rebuild them).
        body["can_decide"] = json!(can_decide);
        body["can_cancel"] = json!(leave::can_cancel(&mut tx, &p, id).await?);
        Ok(Json(body))
    } else {
        Err(ApiError::NotFound)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(p: Principal, mut tx: OrgTx<'static>, id: Uuid, headers: HeaderMap, raw: Bytes, approve: bool) -> Result<Response, ApiError> {
    let op = format!("POST /v1/leave/requests/{id}/{}", if approve { "approve" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: DecisionIn = if raw.is_empty() { DecisionIn { comment: None, reason: None } } else { parse(&raw)? };
    let note = if approve { r.comment.as_deref() } else { r.reason.as_deref() };
    leave::decide(&mut tx, &p, id, approve, note).await?;
    let body = leave::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/leave/requests/{id}/approve (LVE-04 / LVE-05).
pub async fn approve(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, true).await
}

/// POST /v1/leave/requests/{id}/reject (LVE-04 / LVE-05; reason required).
pub async fn reject(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkIn {
    decision: String,
    request_ids: Vec<Uuid>,
    reason: Option<String>,
    comment: Option<String>,
}

/// POST /v1/leave/requests/bulk-decisions: one decision per request, each on its own (not atomic).
pub async fn bulk(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let (key, replay) = start(&mut tx, &headers, "POST /v1/leave/requests/bulk-decisions", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: BulkIn = parse(&raw)?;
    let approve = match r.decision.as_str() {
        "approve" => true,
        "reject" => false,
        _ => return Err(ApiError::ValidationFailed("decision must be approve or reject".into())),
    };
    if r.request_ids.is_empty() || r.request_ids.len() > 200 {
        return Err(ApiError::ValidationFailed("request_ids must hold 1 to 200 ids".into()));
    }
    let mut items = Vec::new();
    for id in r.request_ids {
        sqlx::query("SAVEPOINT bulk_item").execute(&mut **tx).await?;
        let note = if approve { r.comment.as_deref() } else { r.reason.as_deref() };
        let outcome = leave::decide(&mut tx, &p, id, approve, note).await;
        let sp = if outcome.is_err() { "ROLLBACK TO SAVEPOINT bulk_item" } else { "RELEASE SAVEPOINT bulk_item" };
        sqlx::query(sp).execute(&mut **tx).await?;
        items.push(match outcome {
            Ok(()) => {
                let state: String = sqlx::query_scalar("SELECT state FROM leave.leave_request WHERE id = $1").bind(id).fetch_one(&mut **tx).await?;
                json!({ "request_id": id, "outcome": "done", "state": state })
            }
            Err(db::DomainError::Db(e)) => return Err(e.into()),
            Err(e) => {
                let problem = ApiError::from(e).into_response();
                let status = problem.status().as_u16();
                let body = axum::body::to_bytes(problem.into_body(), 64 * 1024).await.map_err(|e| ApiError::Internal(anyhow::anyhow!(e)))?;
                let mut err: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
                err["status"] = json!(status);
                json!({ "request_id": id, "outcome": "refused", "error": err })
            }
        });
    }
    done(tx, key, StatusCode::OK, json!({ "items": items })).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelIn {
    reason: String,
}

/// POST /v1/leave/requests/{id}/cancel (requester, or LVE-06 within scope; G-24 open).
pub async fn cancel(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/leave/requests/{id}/cancel"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: CancelIn = parse(&raw)?;
    leave::cancel(&mut tx, &p, id, &r.reason).await?;
    let body = leave::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}
