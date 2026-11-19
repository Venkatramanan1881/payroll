//! attendance.yaml: attendance days, daily view, HR recalculation.

use crate::routes::punches::{sees, visible_employees};
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
    processing_state: Option<String>,
    branch_id: Option<Uuid>,
    department_id: Option<Uuid>,
    sort: Option<String>,
    #[allow(dead_code)] // paging arrives with cursors; one page of up to 500 rows meanwhile
    limit: Option<u32>,
    #[allow(dead_code)]
    cursor: Option<String>,
}

/// GET /v1/attendance/daily (ATT-01). Filters apply after scope (AC-07).
pub async fn daily(Auth(p, mut tx): Auth, Query(q): Query<DailyQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "ATT-01").await?;
    let f = domain_attendance::DailyFilter {
        employees: visible,
        date: q.date,
        day_status: q.day_status,
        processing_state: q.processing_state,
        branch: q.branch_id,
        department: q.department_id,
        sort: q.sort,
    };
    Ok(Json(json!({ "items": domain_attendance::daily(&mut tx, &f).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecalcIn {
    from: NaiveDate,
    to: NaiveDate,
    employee_ids: Option<Vec<Uuid>>,
    branch_id: Option<Uuid>,
    department_id: Option<Uuid>,
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
    let candidates: Vec<Uuid> = match r.employee_ids {
        Some(ids) => ids,
        None => sqlx::query_scalar("SELECT id FROM workforce.employee").fetch_all(&mut **tx).await?,
    };
    // Branch and department filters by the assignment on the range's first day; out-of-scope employees are skipped (AC-07).
    let filtered: Vec<Uuid> = sqlx::query_scalar(
        "SELECT e FROM unnest($1::uuid[]) e
         WHERE ($2::uuid IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e AND a.kind = 'master_branch' AND a.branch_id = $2 AND a.valid @> $4::date))
           AND ($3::uuid IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e AND a.kind = 'department' AND a.list_entry_id = $3 AND a.valid @> $4::date))",
    )
    .bind(&candidates)
    .bind(r.branch_id)
    .bind(r.department_id)
    .bind(r.from)
    .fetch_all(&mut **tx)
    .await?;
    let employees: Vec<Uuid> = filtered.into_iter().filter(|e| sees(&visible, Some(*e))).collect();
    let (mut written, mut adjusted, mut beyond_window) = (0, 0, 0);
    for e in &employees {
        for d in r.from.iter_days().take(days as usize + 1) {
            if !db::employee_locked(&mut tx, *e, d, d.succ_opt()).await? {
                if domain_attendance::recalculate(&mut tx, *e, d, "hr_request", Some(key)).await?.is_some() {
                    written += 1;
                }
                continue;
            }
            // SM-DAY A10: a locked date posts its difference as an adjustment; one older than PL-07 is skipped and counted.
            match domain_attendance::period::post_adjustment(&mut tx, Some(p.user_id), *e, d, "hr_request", Some(key)).await {
                Ok(Some(_)) => adjusted += 1,
                Ok(None) => {}
                Err(db::DomainError::Refused("ADJUSTMENT_WINDOW_EXCEEDED", _)) => beyond_window += 1,
                Err(err) => return Err(err.into()),
            }
        }
    }
    let summary = json!({ "from": r.from, "to": r.to, "employees": employees.len(), "reason": r.reason, "results_written": written,
                          "adjustments_posted": adjusted, "beyond_adjustment_window": beyond_window });
    db::audit(&mut tx, p.user_id, "attendance.recalculate", "attendance_day", None, Some(summary)).await?;
    let now = Utc::now();
    let body = json!({ "id": key, "kind": "attendance_recalculation", "state": "succeeded", "created_at": now, "finished_at": now,
                       "download_url": null, "download_expires_at": null, "error_report_url": null });
    done(tx, key, StatusCode::ACCEPTED, body).await
}
