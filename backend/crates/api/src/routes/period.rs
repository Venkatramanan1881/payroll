//! period.yaml: cycles, period settings, periods, pre-lock checklist, lock and unlock, period totals.

use crate::routes::org::require_org_wide;
use crate::routes::sched::{done, if_match, parse, reply, start};
use crate::{ApiError, AppState, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{Duration, NaiveDate, Utc};
use sha2::{Digest, Sha256};
use db::OrgTx;
use domain_attendance::period::{self, NewCycle, PeriodFilter, ScopeFilter};
use domain_iam::authz::{require, Principal, Scope};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

/// The caller's employees for `op` (x-scope: branch-department).
async fn scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<ScopeFilter, ApiError> {
    Ok(match require(tx, p, op).await?.scope {
        Scope::All => (None, None),
        Scope::Branches(b) => (Some(b), None),
        Scope::Departments(d) => (None, Some(d)),
        Scope::Nothing => (Some(vec![]), None),
    })
}

/// GET /v1/period/cycles (PER-01).
pub async fn list_cycles(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "PER-01").await?;
    Ok(Json(json!({ "items": period::list_cycles(&mut tx).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleIn {
    owner_kind: String,
    owner_id: Uuid,
    frequency: String,
    start_kind: String,
    custom_start_day: Option<i16>,
    cutoff_day: Option<i16>,
    valid_from: NaiveDate,
    valid_to: Option<NaiveDate>,
    reason: Option<String>,
}

/// POST /v1/period/cycles (PER-01, organisation-wide).
pub async fn create_cycle(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/period/cycles", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let c: CycleIn = parse(&raw)?;
    let body = period::create_cycle(
        &mut tx,
        p.user_id,
        NewCycle {
            owner_kind: c.owner_kind,
            owner_id: c.owner_id,
            frequency: c.frequency,
            start_kind: c.start_kind,
            custom_start_day: c.custom_start_day,
            cutoff_day: c.cutoff_day,
            valid_from: c.valid_from,
            valid_to: c.valid_to,
            reason: c.reason,
        },
    )
    .await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// GET /v1/period/settings (PER-01).
pub async fn settings(Auth(p, mut tx): Auth) -> Result<Response, ApiError> {
    require(&mut tx, &p, "PER-01").await?;
    Ok(reply(StatusCode::OK, period::settings(&mut tx).await?))
}

/// PATCH /v1/period/settings (PER-01, organisation-wide, If-Match).
pub async fn change_settings(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-01").await?;
    let mut body: Map<String, Value> = parse(&raw)?;
    if_match(&headers, &period::settings(&mut tx).await?)?;
    let reason = match body.remove("reason") {
        Some(Value::String(r)) if !r.trim().is_empty() && r.chars().count() <= 1000 => r,
        _ => return Err(ApiError::ValidationFailed("reason is required (1-1000 characters)".into())),
    };
    let out = period::change_settings(&mut tx, p.user_id, body, &reason).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, out))
}

#[derive(Deserialize)]
pub struct PeriodQuery {
    cycle_id: Option<Uuid>,
    state: Option<String>,
    contains_date: Option<NaiveDate>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/periods (PER-08).
pub async fn list_periods(Auth(p, mut tx): Auth, Query(q): Query<PeriodQuery>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "PER-08").await?;
    let f = PeriodFilter { cycle_id: q.cycle_id, state: q.state, contains_date: q.contains_date, from: q.from, to: q.to };
    Ok(Json(json!({ "items": period::list_periods(&mut tx, &f).await?, "next_cursor": null })))
}

/// GET /v1/periods/{id} (PER-08).
pub async fn get_period(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "PER-08").await?;
    Ok(Json(period::get_period(&mut tx, id).await?))
}

#[derive(Deserialize)]
pub struct ChecklistQuery {
    kind: Option<String>,
}

/// GET /v1/periods/{id}/checklist (PER-02, D-10), within scope.
pub async fn checklist(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, Query(q): Query<ChecklistQuery>) -> Result<Json<Value>, ApiError> {
    let s = scope(&mut tx, &p, "PER-02").await?;
    Ok(Json(period::checklist(&mut tx, id, &s, q.kind.as_deref()).await?))
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Note {
    comment: Option<String>,
}

fn optional<T: serde::de::DeserializeOwned + Default>(raw: &[u8]) -> Result<T, ApiError> {
    if raw.iter().all(u8::is_ascii_whitespace) { Ok(T::default()) } else { parse(raw) }
}

/// POST /v1/periods/{id}/lock (PER-03, D-10 HR, organisation-wide).
pub async fn lock(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-03").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/periods/{id}/lock"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let n: Note = optional(&raw)?;
    let body = period::lock(&mut tx, p.user_id, id, n.comment.as_deref()).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonIn {
    reason: String,
}

/// POST /v1/periods/{id}/unlock-requests (PER-04).
pub async fn request_unlock(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-04").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/periods/{id}/unlock-requests"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ReasonIn = parse(&raw)?;
    let body = period::request_unlock(&mut tx, p.user_id, id, &r.reason).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// GET /v1/periods/{id}/unlock-requests (PER-08).
pub async fn list_unlocks(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    require_org_wide(&mut tx, &p, "PER-08").await?; // x-scope: org
    Ok(Json(json!({ "items": period::list_unlocks(&mut tx, id).await?, "next_cursor": null })))
}

/// POST /v1/period/unlock-requests/{id}/approve (PER-05, Organisation Admin).
pub async fn approve_unlock(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-05").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/period/unlock-requests/{id}/approve"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let n: Note = optional(&raw)?;
    let body = period::decide_unlock(&mut tx, p.user_id, id, true, n.comment.as_deref()).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/period/unlock-requests/{id}/reject (PER-05).
pub async fn reject_unlock(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require_org_wide(&mut tx, &p, "PER-05").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/period/unlock-requests/{id}/reject"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ReasonIn = parse(&raw)?;
    if r.reason.trim().is_empty() {
        return Err(ApiError::ValidationFailed("a reason is required".into()));
    }
    let body = period::decide_unlock(&mut tx, p.user_id, id, false, Some(&r.reason)).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
pub struct TotalsQuery {
    employee_id: Option<Uuid>,
}

/// GET /v1/periods/{id}/totals (PER-08), within scope.
pub async fn totals(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, Query(q): Query<TotalsQuery>) -> Result<Json<Value>, ApiError> {
    let s = scope(&mut tx, &p, "PER-08").await?;
    Ok(Json(json!({ "items": period::totals(&mut tx, id, &s, q.employee_id).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
pub struct AdjustmentQuery {
    target_period_id: Option<Uuid>,
    source_period_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    state: Option<String>,
}

/// GET /v1/period/adjustments (PER-08), within scope.
pub async fn list_adjustments(Auth(p, mut tx): Auth, Query(q): Query<AdjustmentQuery>) -> Result<Json<Value>, ApiError> {
    let s = scope(&mut tx, &p, "PER-08").await?;
    let f = period::AdjustmentFilter { target_period_id: q.target_period_id, source_period_id: q.source_period_id, employee_id: q.employee_id, state: q.state };
    Ok(Json(json!({ "items": period::list_adjustments(&mut tx, &f, &s).await?, "next_cursor": null })))
}

/// Signed links live this long (TB-02, AC-C12-07.5).
const LINK_SECS: i64 = 900;

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ExportIn {
    format: Option<String>,
}

/// POST /v1/periods/{id}/payroll-input-exports (PER-06, G-31): `PayrollInput v1` for the employees in scope, stored
/// in Cloud Storage (D-19); the job answers at once with a short-lived signed link.
pub async fn produce_export(State(state): State<AppState>, Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let s = scope(&mut tx, &p, "PER-06").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/periods/{id}/payroll-input-exports"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let f: ExportIn = optional(&raw)?;
    if f.format.as_deref().is_some_and(|f| f != "json") {
        return Err(ApiError::ValidationFailed("only json is produced in V1: the spreadsheet layout is OQ-C12-11".into()));
    }
    let store = state.store.clone().ok_or_else(|| ApiError::Internal(anyhow::anyhow!("EXPORT_BUCKET is not set")))?;
    let (export_id, now) = (Uuid::now_v7(), Utc::now());
    let (doc, adjustments) = period::payroll_input(&mut tx, id, &s, export_id, now).await?;
    let bytes = serde_json::to_vec(&doc).map_err(|e| ApiError::Internal(e.into()))?;
    let checksum: String = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect();
    let org: Uuid = sqlx::query_scalar("SELECT app.current_org()").fetch_one(&mut **tx).await?;
    let path = format!("{org}/payroll-input/{id}/{export_id}.json"); // TB-02: the organisation id leads the path
    // shortcut: a failed commit after the upload leaves an unreferenced file; a bucket lifecycle rule can sweep those.
    store.put(&path, bytes, "application/json").await.map_err(ApiError::Internal)?;
    period::record_export(&mut tx, p.user_id, export_id, id, &path, &checksum, &adjustments).await?;
    let url = store.signed_url(&path, LINK_SECS as u64).await.map_err(ApiError::Internal)?;
    let mut body = json!({ "id": export_id, "kind": "payroll_input_export", "state": "succeeded", "created_at": now, "finished_at": Utc::now(),
                           "download_url": null, "download_expires_at": null, "error_report_url": null });
    // The stored answer (replayed for a retried key) carries no link: links expire and are bearer capabilities.
    // A retry gets a fresh one from GET /v1/period/payroll-input-exports/{id}/download-link.
    crate::idempotency::finish(&mut tx, key, StatusCode::ACCEPTED, &body).await?;
    tx.commit().await?;
    body["download_url"] = json!(url);
    body["download_expires_at"] = json!(now + Duration::seconds(LINK_SECS));
    Ok(reply(StatusCode::ACCEPTED, body))
}

/// GET /v1/periods/{id}/payroll-input-exports (PER-07).
pub async fn list_exports(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let all = matches!(require(&mut tx, &p, "PER-07").await?.scope, Scope::All);
    let items: Vec<Value> = period::list_exports(&mut tx, id).await?.into_iter().filter(|x| all || x["produced_by"] == json!(p.user_id)).collect();
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

/// GET /v1/period/payroll-input-exports/{id}/download-link (PER-07). An export holds the producer's scope, so a
/// branch-scoped caller gets only their own exports; every download is audited (DOMAIN_MODEL §10).
pub async fn download_link(State(state): State<AppState>, Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let all = matches!(require(&mut tx, &p, "PER-07").await?.scope, Scope::All);
    let export = period::get_export(&mut tx, id).await?;
    if !all && export["produced_by"] != json!(p.user_id) {
        return Err(ApiError::NotFound);
    }
    let store = state.store.clone().ok_or_else(|| ApiError::Internal(anyhow::anyhow!("EXPORT_BUCKET is not set")))?;
    let path = period::export_path(&mut tx, id).await?;
    let url = store.signed_url(&path, LINK_SECS as u64).await.map_err(ApiError::Internal)?;
    db::audit(&mut tx, p.user_id, "period.payroll_input_downloaded", "payroll_input_export", Some(id), None).await?;
    tx.commit().await?;
    Ok(Json(json!({ "url": url, "expires_at": Utc::now() + Duration::seconds(LINK_SECS) })))
}
