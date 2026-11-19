//! comp-off.yaml (M02): credits (claims) with approval, the pay-or-comp-off choice, balance and ledger.
//! Taking comp-off is a leave request of the reserved type COMP_OFF (D-25), through leave.yaml.

use crate::routes::punches::{sees, visible_employees};
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::NaiveDate;
use db::{DomainError, OrgTx};
use domain_attendance::comp_off as co;
use domain_iam::authz::{require, Principal};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// Own credits plus the CMP-07 read scope and the CMP-05 approver scope.
async fn readable(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Option<Vec<Uuid>>, ApiError> {
    let mut out: Vec<Uuid> = p.employee_id.into_iter().collect();
    for op in ["CMP-05", "CMP-07"] {
        match visible_employees(tx, p, op).await {
            Ok(None) => return Ok(None),
            Ok(Some(more)) => out.extend(more),
            Err(ApiError::Forbidden) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(Some(out))
}

#[derive(Deserialize)]
pub struct ListQuery {
    employee_id: Option<Uuid>,
    state: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/comp-off/credits.
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    let employees = readable(&mut tx, &p).await?;
    let f = co::Filter { employees, approver: p.user_id, employee: q.employee_id, state: q.state, from: q.from, to: q.to };
    Ok(Json(json!({ "items": co::list(&mut tx, &f).await?, "next_cursor": null })))
}

/// GET /v1/comp-off/credits/{id}: own, in scope, or routed to / decided by the caller; otherwise NOT_FOUND.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let body = co::get(&mut tx, id).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    let visible = sees(&readable(&mut tx, &p).await?, employee);
    let routed = body["current_approver_user_id"] == json!(p.user_id) || body["decided_by"] == json!(p.user_id);
    if visible || routed { Ok(Json(body)) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(Auth(p, mut tx): Auth, id: Uuid, headers: HeaderMap, raw: Bytes, approve: bool) -> Result<Response, ApiError> {
    let op = format!("POST /v1/comp-off/credits/{id}/{}", if approve { "approve" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let d: DecisionIn = if raw.iter().all(u8::is_ascii_whitespace) { DecisionIn::default() } else { parse(&raw)? };
    let note = if approve { d.comment } else { d.reason };
    co::decide(&mut tx, &p, id, approve, note.as_deref()).await?;
    let body = co::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/comp-off/credits/{id}/approve (CMP-05).
pub async fn approve(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, true).await
}

/// POST /v1/comp-off/credits/{id}/reject (CMP-05).
pub async fn reject(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(auth, id, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceIn {
    worked_date: NaiveDate,
    choice: String,
}

/// POST /v1/comp-off/choices (CMP-04). HW-01 = employee chooses cannot be saved yet (D-28), so no choice is offered:
/// RULE_GROUP_DISABLED (AC-M02-04.3).
pub async fn choose(Auth(p, mut tx): Auth, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "CMP-04").await?;
    p.employee_id.ok_or(ApiError::Forbidden)?;
    let c: ChoiceIn = parse(&raw)?;
    if !matches!(c.choice.as_str(), "pay" | "comp_off") {
        return Err(ApiError::ValidationFailed("choice is pay or comp_off".into()));
    }
    let _ = c.worked_date;
    Err(DomainError::Refused("RULE_GROUP_DISABLED", "HW-01 is not \"employee chooses\": no choice is offered".into()).into())
}

async fn visible(tx: &mut OrgTx<'_>, p: &Principal, employee: Uuid) -> Result<(), ApiError> {
    let v = visible_employees(tx, p, "CMP-07").await?;
    if sees(&v, Some(employee)) { Ok(()) } else { Err(ApiError::NotFound) }
}

/// GET /v1/comp-off/employees/{employee_id}/balance (CMP-07).
pub async fn balance(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    visible(&mut tx, &p, employee).await?;
    Ok(Json(co::balance(&mut tx, employee).await?))
}

#[derive(Deserialize)]
pub struct LedgerQuery {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/comp-off/employees/{employee_id}/ledger (CMP-07).
pub async fn ledger(Auth(p, mut tx): Auth, Path(employee): Path<Uuid>, Query(q): Query<LedgerQuery>) -> Result<Json<Value>, ApiError> {
    visible(&mut tx, &p, employee).await?;
    Ok(Json(json!({ "items": co::ledger(&mut tx, employee, q.from, q.to).await?, "next_cursor": null })))
}
