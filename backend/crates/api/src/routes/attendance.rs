//! attendance.yaml: attendance days, daily view, HR recalculation.

use crate::routes::punches::{narrow, sees, visible_employees};
use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

// shortcut: an employee sees the whole day; AC-05 filtering (own punches / status / deduction reason) comes with org settings endpoints.
/// GET /v1/attendance/employees/{employee_id}/days/{date} (ATT-01). Outside scope → 404.
pub async fn get_day(Auth(p, mut tx): Auth, Path((employee, date)): Path<(Uuid, NaiveDate)>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-01").await?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::NotFound);
    }
    Ok(Json(domain_attendance::get_day(&mut tx, employee, date).await?))
}

#[derive(Deserialize)]
pub struct DailyQuery {
    date: NaiveDate,
    day_status: Option<String>,
}

// shortcut: branch, department, processing_state and sort filters are not offered yet; add them with the team screen.
/// GET /v1/attendance/daily (ATT-01).
pub async fn daily(Auth(p, mut tx): Auth, Query(q): Query<DailyQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-01").await?;
    let items = domain_attendance::daily(&mut tx, visible, q.date, q.day_status.as_deref()).await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecalcIn {
    from: NaiveDate,
    to: NaiveDate,
    employee_ids: Option<Vec<Uuid>>,
    reason: String,
}

const MAX_RANGE_DAYS: i64 = 31;

/// POST /v1/attendance/recalculations (ATT-06). Runs now and answers with a finished job (202, Job).
// shortcut: runs inline for up to 31 days; becomes queued (Cloud Tasks) with the deployment slice.
pub async fn recalculate(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-06").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/attendance/recalculations", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RecalcIn = parse(&raw)?;
    if r.reason.trim().is_empty() || r.employee_ids.as_ref().is_some_and(|e| e.len() > 1000) {
        return Err(ApiError::ValidationFailed("reason is required and employee_ids holds at most 1000".into()));
    }
    let days = (r.to - r.from).num_days();
    if !(0..MAX_RANGE_DAYS).contains(&days) {
        return Err(ApiError::ValidationFailed(format!("to must be on or after from, at most {MAX_RANGE_DAYS} days in all")));
    }
    let employees: Vec<Uuid> = match r.employee_ids {
        // Requested employees outside scope are skipped, never reported (AC-07).
        Some(ids) => ids.into_iter().filter(|e| sees(&visible, Some(*e))).collect(),
        None => match narrow(visible, None) {
            Some(ids) => ids,
            None => sqlx::query_scalar("SELECT id FROM workforce.employee").fetch_all(&mut **tx).await?,
        },
    };
    let mut written = 0;
    for e in &employees {
        for d in r.from.iter_days().take(days as usize + 1) {
            if domain_attendance::recalculate(&mut tx, *e, d, "hr_request", Some(key)).await?.is_some() {
                written += 1;
            }
        }
    }
    db::audit(&mut tx, p.user_id, "attendance.recalculate", "attendance_day", None, Some(json!({ "from": r.from, "to": r.to, "employees": employees.len(), "reason": r.reason, "results_written": written }))).await?;
    let now = Utc::now();
    let body = json!({ "id": key, "kind": "attendance_recalculation", "state": "succeeded", "created_at": now, "finished_at": now,
                       "download_url": null, "download_expires_at": null, "error_report_url": null });
    done(tx, key, StatusCode::ACCEPTED, body).await
}
