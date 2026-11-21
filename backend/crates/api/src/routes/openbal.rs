//! opening-balances.yaml (C15): batches, entries, submit / confirm / reject, change after confirmation.

use crate::routes::punches::{sees, visible_employees};
use crate::routes::sched::{done, if_match, parse, reply, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDate;
use db::OrgTx;
use domain_attendance::openbal::{self as ob, EntryIn};
use domain_iam::authz::{require, GrantKind, Principal, Scope};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// The caller's employees for `op`: None = the whole organisation (any grant kind over scope `all`, so an
/// approve-type grant of the Organisation Admin covers every batch), else the listed employees.
async fn scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Option<Vec<Uuid>>, ApiError> {
    let g = require(tx, p, op).await?;
    if matches!((g.kind, &g.scope), (GrantKind::All | GrantKind::Scoped | GrantKind::Approve, Scope::All)) {
        return Ok(None);
    }
    visible_employees(tx, p, op).await
}

/// Reading batches and entries: OBL-04, or the OBL-02 decider (whoever confirms or rejects a batch may read it).
async fn read_scope(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Option<Vec<Uuid>>, ApiError> {
    match scope(tx, p, "OBL-04").await {
        Err(ApiError::Forbidden) => scope(tx, p, "OBL-02").await,
        other => other,
    }
}

/// The grant kinds the caller holds for `op` across roles (approving and entering are different rights, OB-01).
async fn kinds(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Vec<String>, ApiError> {
    Ok(sqlx::query_scalar("SELECT g.grant_kind FROM iam.role_grant g JOIN iam.user_role r ON r.role_id = g.role_id WHERE r.user_id = $1 AND g.operation_code = $2")
        .bind(p.user_id)
        .bind(op)
        .fetch_all(&mut ***tx)
        .await?)
}

/// Batch-wide commands need every employee of the batch in scope.
async fn whole_batch(tx: &mut OrgTx<'_>, p: &Principal, op: &str, batch: Uuid) -> Result<(), ApiError> {
    let visible = scope(tx, p, op).await?;
    ob::get_batch(tx, batch).await?;
    if ob::batch_employees(tx, batch).await?.into_iter().any(|e| !sees(&visible, Some(e))) {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryBody {
    employee_id: Uuid,
    balance_kind: String,
    leave_type_id: Option<Uuid>,
    permission_type_id: Option<Uuid>,
    value: String,
    as_of_date: NaiveDate,
}

fn entries(raw: Vec<EntryBody>, visible: &Option<Vec<Uuid>>) -> Result<(Vec<EntryIn>, Vec<usize>), ApiError> {
    if raw.len() > 500 {
        return Err(ApiError::ValidationFailed("at most 500 entries at a time".into()));
    }
    let out_of_scope = raw.iter().enumerate().filter(|(_, e)| !sees(visible, Some(e.employee_id))).map(|(i, _)| i).collect();
    let list = raw
        .into_iter()
        .map(|e| EntryIn { employee: e.employee_id, kind: e.balance_kind, leave_type: e.leave_type_id, permission_type: e.permission_type_id, value: e.value, as_of: e.as_of_date })
        .collect();
    Ok((list, out_of_scope))
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct NewBatch {
    #[serde(default)]
    entries: Vec<EntryBody>,
}

/// POST /v1/opening-balances/batches (OBL-01).
pub async fn create_batch(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = scope(&mut tx, &p, "OBL-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/opening-balances/batches", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let b: NewBatch = if raw.iter().all(u8::is_ascii_whitespace) { NewBatch::default() } else { parse(&raw)? };
    let (list, out) = entries(b.entries, &visible)?;
    let body = ob::create_batch(&mut tx, p.user_id, &list, &out).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct BatchQuery {
    state: Option<String>,
}

/// GET /v1/opening-balances/batches (OBL-04).
pub async fn list_batches(Auth(p, mut tx): Auth, Query(q): Query<BatchQuery>) -> Result<Json<Value>, ApiError> {
    let visible = read_scope(&mut tx, &p).await?;
    Ok(Json(json!({ "items": ob::list_batches(&mut tx, q.state.as_deref(), visible.as_deref()).await?, "next_cursor": null })))
}

/// GET /v1/opening-balances/batches/{id} (OBL-04).
pub async fn get_batch(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    let visible = read_scope(&mut tx, &p).await?;
    let batch = ob::get_batch(&mut tx, id).await?;
    if !ob::batch_visible(&mut tx, id, visible.as_deref()).await? {
        return Err(ApiError::NotFound);
    }
    Ok(reply(StatusCode::OK, batch))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddEntries {
    entries: Vec<EntryBody>,
}

/// POST /v1/opening-balances/batches/{id}/entries (OBL-01, draft only).
pub async fn add_entries(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = scope(&mut tx, &p, "OBL-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/opening-balances/batches/{id}/entries"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let b: AddEntries = parse(&raw)?;
    if b.entries.is_empty() {
        return Err(ApiError::ValidationFailed("at least one entry".into()));
    }
    ob::batch_state(&mut tx, id).await?; // locks the batch before the If-Match comparison
    if_match(&headers, &ob::get_batch(&mut tx, id).await?)?;
    let (list, out) = entries(b.entries, &visible)?;
    let body = ob::add_entries(&mut tx, p.user_id, id, &list, &out).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
pub struct EntryQuery {
    employee_id: Option<Uuid>,
    balance_kind: Option<String>,
    include_superseded: Option<bool>,
}

/// GET /v1/opening-balances/batches/{id}/entries (OBL-04), within scope.
pub async fn list_entries(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, Query(q): Query<EntryQuery>) -> Result<Json<Value>, ApiError> {
    let visible = read_scope(&mut tx, &p).await?;
    let mut items: Vec<Value> = ob::list_entries(&mut tx, id, q.employee_id, q.balance_kind.as_deref(), q.include_superseded.unwrap_or(false))
        .await?
        .into_iter()
        .filter(|e| sees(&visible, e["employee_id"].as_str().and_then(|s| s.parse().ok())))
        .collect();
    crate::routes::with_names(&mut tx, &mut items).await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

/// The entry, visible to the caller (outside scope reads as not found).
async fn entry_in_scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str, entry: Uuid) -> Result<Value, ApiError> {
    let visible = scope(tx, p, op).await?;
    let e = ob::get_entry(tx, entry).await?;
    if !sees(&visible, e["employee_id"].as_str().and_then(|s| s.parse().ok())) {
        return Err(ApiError::NotFound);
    }
    Ok(e)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditIn {
    value: Option<String>,
    as_of_date: Option<NaiveDate>,
}

/// PATCH /v1/opening-balances/batches/{id}/entries/{entry_id} (OBL-01, If-Match, draft only).
pub async fn edit_entry(Auth(p, mut tx): Auth, Path((id, entry)): Path<(Uuid, Uuid)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let current = entry_in_scope(&mut tx, &p, "OBL-01", entry).await?;
    let b: EditIn = parse(&raw)?;
    if b.value.is_none() && b.as_of_date.is_none() {
        return Err(ApiError::ValidationFailed("give value and/or as_of_date".into()));
    }
    if_match(&headers, &current)?;
    let out = ob::edit_entry(&mut tx, p.user_id, id, entry, b.value.as_deref(), b.as_of_date).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, out))
}

/// DELETE /v1/opening-balances/batches/{id}/entries/{entry_id} (OBL-01, If-Match, draft only).
pub async fn remove_entry(Auth(p, mut tx): Auth, Path((id, entry)): Path<(Uuid, Uuid)>, headers: HeaderMap) -> Result<Response, ApiError> {
    let current = entry_in_scope(&mut tx, &p, "OBL-01", entry).await?;
    if_match(&headers, &current)?;
    ob::remove_entry(&mut tx, p.user_id, id, entry).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

async fn batch_command(Auth(p, mut tx): Auth, id: Uuid, headers: HeaderMap, raw: Bytes, what: &str) -> Result<Response, ApiError> {
    whole_batch(&mut tx, &p, "OBL-02", id).await?;
    let held = kinds(&mut tx, &p, "OBL-02").await?;
    let approver = held.iter().any(|k| k == "approve");
    let enterer = held.iter().any(|k| k == "all" || k == "scoped");
    // OB-01: HR enters and confirms (or submits); only an approve-type holder decides a pending batch.
    let allowed = match (what, ob::batch_state(&mut tx, id).await?.as_str()) {
        ("confirm", "pending_approval") | ("reject", _) => approver,
        ("confirm", "confirmed") => approver || enterer,
        _ => enterer,
    };
    if !allowed {
        return Err(ApiError::Forbidden);
    }
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/opening-balances/batches/{id}/{what}"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let note: Value = if raw.iter().all(u8::is_ascii_whitespace) { json!({}) } else { parse(&raw)? };
    let body = match what {
        "submit" => ob::submit(&mut tx, p.user_id, id).await?,
        "confirm" => ob::confirm(&mut tx, p.user_id, id, note["comment"].as_str()).await?,
        _ => {
            let reason = note["reason"].as_str().map(str::trim).filter(|r| !r.is_empty() && r.chars().count() <= 1000);
            ob::reject(&mut tx, p.user_id, id, reason.ok_or_else(|| ApiError::ValidationFailed("reason is required".into()))?).await?
        }
    };
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/opening-balances/batches/{id}/submit (OBL-02).
pub async fn submit(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    batch_command(auth, id, headers, raw, "submit").await
}

/// POST /v1/opening-balances/batches/{id}/confirm (OBL-02).
pub async fn confirm(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    batch_command(auth, id, headers, raw, "confirm").await
}

/// POST /v1/opening-balances/batches/{id}/reject (OBL-02).
pub async fn reject(auth: Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    batch_command(auth, id, headers, raw, "reject").await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeIn {
    value: String,
    as_of_date: Option<NaiveDate>,
    reason: String,
}

/// POST /v1/opening-balances/entries/{entry_id}/change (OBL-03, OB-02).
pub async fn change_entry(Auth(p, mut tx): Auth, Path(entry): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    entry_in_scope(&mut tx, &p, "OBL-03", entry).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/opening-balances/entries/{entry}/change"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let b: ChangeIn = parse(&raw)?;
    let body = ob::change_entry(&mut tx, p.user_id, entry, &b.value, b.as_of_date, &b.reason).await?;
    done(tx, key, StatusCode::CREATED, body).await
}
