//! attendance.yaml: regularisation requests, HR corrections, bulk outage regularisation.

use crate::routes::punches::{narrow, sees, visible_employees, FileRef};
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use domain_attendance::regularisation::{self as reg, NewRequest};
use domain_iam::authz::{require, Scope};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// RegularisationInput.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegIn {
    employee_id: Option<Uuid>,
    attendance_date: NaiveDate,
    request_type: String,
    proposed_in_at: Option<DateTime<Utc>>,
    proposed_out_at: Option<DateTime<Utc>>,
    target_raw_punch_id: Option<Uuid>,
    reason: String,
    attachment: Option<FileRef>,
}

impl RegIn {
    fn into_request(self, employee: Uuid) -> NewRequest {
        NewRequest {
            employee,
            date: self.attendance_date,
            request_type: self.request_type,
            proposed_in: self.proposed_in_at,
            proposed_out: self.proposed_out_at,
            target: self.target_raw_punch_id,
            reason: self.reason,
            attachment: self.attachment.map(|f| f.path),
        }
    }
}

/// POST /v1/attendance/regularisation-requests (ATT-02, self). The employee is always the caller.
pub async fn submit(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ATT-02").await?;
    let employee = p.employee_id.ok_or(ApiError::Forbidden)?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/attendance/regularisation-requests", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RegIn = parse(&raw)?;
    let id = reg::submit(&mut tx, p.user_id, r.into_request(employee)).await?;
    let body = reg::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ListQuery {
    employee_id: Option<Uuid>,
    state: Option<String>,
    origin: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

// shortcut: branch, department and sort filters are not offered yet; add them with the HR requests screen.
/// GET /v1/attendance/regularisation-requests (ATT-01).
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-01").await?;
    let f = reg::Filter { employees: narrow(visible, q.employee_id), state: q.state, origin: q.origin, from: q.from, to: q.to };
    Ok(Json(json!({ "items": reg::list(&mut tx, &f).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
pub struct PendingQuery {
    level: Option<i16>,
}

/// GET /v1/attendance/regularisation-requests/pending-approval (ATT-03 / ATT-04 / ATT-05).
pub async fn pending(Auth(p, mut tx): Auth, Query(q): Query<PendingQuery>) -> Result<Json<Value>, ApiError> {
    let mut any = false;
    for op in ["ATT-03", "ATT-04", "ATT-05"] {
        any |= require(&mut tx, &p, op).await.is_ok();
    }
    if !any {
        return Err(ApiError::Forbidden);
    }
    Ok(Json(json!({ "items": reg::pending_for(&mut tx, &p, q.level).await?, "next_cursor": null })))
}

/// GET /v1/attendance/regularisation-requests/{id} (ATT-01). The routed approver may also read it.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let body = reg::get(&mut tx, id).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    let visible = match visible_employees(&mut tx, &p, "ATT-01").await {
        Ok(v) => sees(&v, employee),
        Err(ApiError::Forbidden) => false,
        Err(e) => return Err(e),
    };
    let routed = reg::pending_for(&mut tx, &p, None).await?.iter().any(|r| r["id"] == body["id"]);
    if visible || routed { Ok(Json(body)) } else { Err(ApiError::NotFound) }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(p: domain_iam::authz::Principal, mut tx: db::OrgTx<'static>, id: Uuid, headers: HeaderMap, raw: Bytes, approve: bool) -> Result<Response, ApiError> {
    let op = format!("POST /v1/attendance/regularisation-requests/{id}/{}", if approve { "approve" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: DecisionIn = if raw.is_empty() { DecisionIn { comment: None, reason: None } } else { parse(&raw)? };
    let note = if approve { r.comment.as_deref() } else { r.reason.as_deref() };
    reg::decide(&mut tx, &p, id, approve, note).await?;
    let body = reg::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST .../{id}/approve (SM-REQ R4/R5).
pub async fn approve(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, true).await
}

/// POST .../{id}/reject (SM-REQ R6; reason required).
pub async fn reject(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CancelIn {
    reason: Option<String>,
}

/// POST .../{id}/cancel (ATT-02; the requester, while pending).
pub async fn cancel(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ATT-02").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/attendance/regularisation-requests/{id}/cancel"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: CancelIn = if raw.is_empty() { CancelIn { reason: None } } else { parse(&raw)? };
    reg::cancel(&mut tx, p.user_id, id, r.reason.as_deref()).await?;
    let body = reg::get(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/attendance/hr-corrections (ATT-06, RG-10).
pub async fn hr_correction(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-06").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/attendance/hr-corrections", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RegIn = parse(&raw)?;
    let employee = r.employee_id.ok_or_else(|| ApiError::ValidationFailed("employee_id is required".into()))?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::ValidationFailed("employee_id is not an employee you manage".into()));
    }
    let id = reg::hr_correct(&mut tx, &p, r.into_request(employee)).await?;
    let body = reg::get(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BulkIn {
    branch_id: Uuid,
    attendance_date: NaiveDate,
    #[allow(dead_code)] // the failed device, kept for the request record; the correction does not depend on it
    device_id: Option<Uuid>,
    correction: String,
    reason: String,
}

/// POST /v1/attendance/bulk-regularisations (ATT-06, RG-11): runs now and answers with a finished job.
pub async fn bulk(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-06").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/attendance/bulk-regularisations", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: BulkIn = parse(&raw)?;
    if !["use_shift_times", "mark_present"].contains(&r.correction.as_str()) || r.reason.trim().is_empty() {
        return Err(ApiError::ValidationFailed("correction must be use_shift_times or mark_present, and reason is required".into()));
    }
    if let Scope::Branches(b) = require(&mut tx, &p, "ATT-06").await?.scope {
        if !b.contains(&r.branch_id) {
            return Err(ApiError::ValidationFailed("branch_id is not a branch you manage".into()));
        }
    }
    let staff: Vec<Uuid> = sqlx::query_scalar("SELECT employee_id FROM workforce.employee_assignment WHERE kind = 'master_branch' AND branch_id = $1 AND valid @> $2::date")
        .bind(r.branch_id)
        .bind(r.attendance_date)
        .fetch_all(&mut **tx)
        .await?;
    let staff: Vec<Uuid> = staff.into_iter().filter(|e| sees(&visible, Some(*e))).collect();
    let (corrected, skipped) = reg::bulk_outage(&mut tx, &p, &staff, r.attendance_date, r.reason.trim()).await?;
    db::audit(&mut tx, p.user_id, "regularisation.bulk_outage_run", "branch", Some(r.branch_id),
        Some(json!({ "attendance_date": r.attendance_date, "correction": r.correction, "corrected": corrected, "skipped_no_shift": skipped, "reason": r.reason.trim() }))).await?;
    let now = Utc::now();
    // shortcut: skipped employees are counted in the audit entry; a downloadable error report comes with file storage.
    let body = json!({ "id": key, "kind": "bulk_regularisation", "state": "succeeded", "created_at": now, "finished_at": now,
                       "download_url": null, "download_expires_at": null, "error_report_url": null });
    done(tx, key, StatusCode::ACCEPTED, body).await
}
