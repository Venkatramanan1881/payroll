//! punches.yaml: app/web punches, offline sync, punch flags, manual attendance, punch reads.

use crate::routes::sched::{done, parse, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{DateTime, NaiveDate, Utc};
use db::OrgTx;
use domain_iam::authz::{require, GrantKind, Principal, Scope};
use domain_timecap::evaluate::Loc;
use domain_timecap::flags;
use domain_timecap::punch::{self, Capture, Outcome};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationIn {
    latitude: f64,
    longitude: f64,
    accuracy_m: i64,
}

#[derive(Deserialize)]
pub struct FileRef {
    path: String,
}

/// PunchCapture. There is no inside/outside or distance field: unknown fields are refused (AC-C04-11.1).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PunchIn {
    idempotency_key: Uuid,
    source: String,
    client_time: DateTime<Utc>,
    captured_tz: String,
    location: Option<LocationIn>,
    location_unavailable_reason: Option<String>,
    selfie: Option<FileRef>,
    client_ref: Option<String>,
}

fn location(l: Option<LocationIn>) -> Result<Option<Loc>, ApiError> {
    let Some(l) = l else { return Ok(None) };
    if !(-90.0..=90.0).contains(&l.latitude) || !(-180.0..=180.0).contains(&l.longitude) || l.accuracy_m < 0 {
        return Err(ApiError::ValidationFailed("location is out of range".into()));
    }
    Ok(Some(Loc { lat: l.latitude, lon: l.longitude, accuracy_m: l.accuracy_m }))
}

fn check_common(reason: &Option<String>, client_ref: &Option<String>) -> Result<(), ApiError> {
    if reason.as_deref().is_some_and(|r| !["permission_denied", "no_fix", "not_supported"].contains(&r)) {
        return Err(ApiError::ValidationFailed("location_unavailable_reason".into()));
    }
    if client_ref.as_ref().is_some_and(|c| c.chars().count() > 200) {
        return Err(ApiError::ValidationFailed("client_ref is longer than 200 characters".into()));
    }
    Ok(())
}

// shortcut: trusts the last X-Forwarded-For hop (the address our front proxy saw); set the trusted hop count at deployment.
fn client_ip(headers: &HeaderMap) -> Option<String> {
    let last = headers.get("x-forwarded-for")?.to_str().ok()?.rsplit(',').next()?.trim();
    last.parse::<std::net::IpAddr>().ok().map(|ip| ip.to_string())
}

/// Whether the caller holds an operation (for optional extras such as location, TCP-08).
async fn holds(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<bool, ApiError> {
    match require(tx, p, op).await {
        Ok(_) => Ok(true),
        Err(domain_iam::authz::AuthzError::Forbidden) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// TCP-03 is the caller's own punch: the employee comes from the signed-in user, never the body.
async fn own_employee(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Uuid, ApiError> {
    require(tx, p, "TCP-03").await?;
    p.employee_id.ok_or(ApiError::Forbidden)
}

/// POST /v1/punches (TCP-03).
pub async fn record(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<(StatusCode, Json<Value>), ApiError> {
    let employee = own_employee(&mut tx, &p).await?;
    let r: PunchIn = parse(&raw)?;
    check_common(&r.location_unavailable_reason, &r.client_ref)?;
    let now = Utc::now();
    let cap = Capture {
        idempotency_key: r.idempotency_key,
        source: r.source,
        // A live punch happens now; the phone clock is only compared (PC-08).
        event_at: now,
        received_at: now,
        captured_tz: r.captured_tz,
        location: location(r.location)?,
        location_unavailable_reason: r.location_unavailable_reason,
        selfie_path: r.selfie.map(|f| f.path),
        client_ref: r.client_ref,
        client_ip: client_ip(&headers),
        device_uptime_ms: None,
        clock_skew_ms: Some((r.client_time - now).num_milliseconds()),
    };
    match punch::record(&mut tx, p.user_id, employee, &cap).await? {
        Outcome::Refused { attempt_id, rule_ref, message } => {
            tx.commit().await?; // the refused attempt is evidence (E18)
            Err(ApiError::PunchRefused(rule_ref.to_string(), attempt_id, message))
        }
        Outcome::Stored { punch_id, flags, duplicate } => {
            let with_location = holds(&mut tx, &p, "TCP-08").await?;
            let body = json!({
                "outcome": if flags.is_empty() { "accepted" } else { "flagged" },
                "punch": punch::get_punch(&mut tx, punch_id, with_location).await?,
            });
            tx.commit().await?;
            Ok((if duplicate { StatusCode::OK } else { StatusCode::CREATED }, Json(body)))
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Scope

/// Employees the caller may see for `op`: None = every employee. Scope is by today's assignment, as for employees.
async fn visible_employees(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Option<Vec<Uuid>>, ApiError> {
    let g = require(tx, p, op).await?;
    let today = crate::routes::employees::today();
    let by_assignment = |kind: &'static str, column: &'static str| {
        format!("SELECT employee_id FROM workforce.employee_assignment WHERE kind = '{kind}' AND {column} = ANY($1) AND valid @> $2::date")
    };
    let ids: Vec<Uuid> = match (g.kind, g.scope) {
        (GrantKind::SelfOnly, _) => p.employee_id.into_iter().collect(),
        // A manager decides for the employees who report to them (AC-02).
        (GrantKind::Approve, _) => sqlx::query_scalar(&by_assignment("reporting_manager", "manager_employee_id"))
            .bind(p.employee_id.into_iter().collect::<Vec<_>>())
            .bind(today)
            .fetch_all(&mut ***tx)
            .await?,
        (_, Scope::All) => return Ok(None),
        (_, Scope::Branches(b)) => sqlx::query_scalar(&by_assignment("master_branch", "branch_id")).bind(b).bind(today).fetch_all(&mut ***tx).await?,
        (_, Scope::Departments(d)) => sqlx::query_scalar(&by_assignment("department", "list_entry_id")).bind(d).bind(today).fetch_all(&mut ***tx).await?,
        (_, Scope::Nothing) => vec![],
    };
    Ok(Some(ids))
}

fn sees(visible: &Option<Vec<Uuid>>, employee: Option<Uuid>) -> bool {
    match visible {
        None => true,
        Some(ids) => employee.is_some_and(|e| ids.contains(&e)),
    }
}

/// Narrows the visible set to one requested employee (outside scope = empty result, never an error).
fn narrow(visible: Option<Vec<Uuid>>, wanted: Option<Uuid>) -> Option<Vec<Uuid>> {
    match (visible, wanted) {
        (v, None) => v,
        (None, Some(e)) => Some(vec![e]),
        (Some(ids), Some(e)) => Some(ids.into_iter().filter(|i| *i == e).collect()),
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Offline sync

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OfflineIn {
    idempotency_key: Uuid,
    client_time: DateTime<Utc>,
    captured_tz: String,
    device_uptime_ms: i64,
    location: Option<LocationIn>,
    location_unavailable_reason: Option<String>,
    selfie: Option<FileRef>,
    client_ref: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyncIn {
    #[allow(dead_code)] // in the contract; the server's own clock anchors the PC-08 check (OQ-C04-05 ruling)
    sync_client_time: DateTime<Utc>,
    sync_uptime_ms: i64,
    /// Items stay raw so one malformed item is reported on its own, not as a 422 for the batch.
    punches: Vec<Value>,
}

fn item_problem(key: &Value, code: &str, rule_ref: Option<&str>, message: String) -> Value {
    json!({ "idempotency_key": key, "outcome": "refused", "code": null, "punch_id": null, "flags": [],
            "refusal": { "code": code, "rule_ref": rule_ref, "message": message } })
}

fn parse_item(v: &Value) -> Result<(OfflineIn, Option<Loc>), String> {
    let mut o: OfflineIn = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    if o.device_uptime_ms < 0 {
        return Err("device_uptime_ms must not be negative".into());
    }
    check_common(&o.location_unavailable_reason, &o.client_ref).map_err(|_| "location_unavailable_reason or client_ref is not valid".to_string())?;
    let loc = location(o.location.take()).map_err(|_| "location is out of range".to_string())?;
    Ok((o, loc))
}

/// POST /v1/punches/sync (TCP-03; TSD §9.3).
pub async fn sync(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Json<Value>, ApiError> {
    let employee = own_employee(&mut tx, &p).await?;
    let r: SyncIn = parse(&raw)?;
    if r.punches.is_empty() || r.punches.len() > 500 || r.sync_uptime_ms < 0 {
        return Err(ApiError::ValidationFailed("punches must hold 1 to 500 items and sync_uptime_ms must not be negative".into()));
    }
    let now = Utc::now();
    let ip = client_ip(&headers);
    let mut results: Vec<Value> = vec![Value::Null; r.punches.len()];
    let mut items = Vec::new();
    for (i, v) in r.punches.iter().enumerate() {
        match parse_item(v) {
            Ok(item) => items.push((i, item)),
            Err(m) => results[i] = item_problem(v.get("idempotency_key").unwrap_or(&Value::Null), "VALIDATION_FAILED", None, m),
        }
    }
    // Stored in the original time order for recalculation (PC-R7).
    items.sort_by_key(|(_, (o, _))| o.client_time);
    for (i, (o, loc)) in items {
        let key = json!(o.idempotency_key);
        // PC-08: the uptime clock says how long ago the punch happened; a phone clock that disagrees was changed.
        // A smaller uptime at sync means the phone restarted, so the check cannot be made.
        let skew = (r.sync_uptime_ms >= o.device_uptime_ms)
            .then(|| (o.client_time - (now - chrono::Duration::milliseconds(r.sync_uptime_ms - o.device_uptime_ms))).num_milliseconds());
        let cap = Capture {
            idempotency_key: o.idempotency_key,
            source: "app".into(),
            event_at: o.client_time,
            received_at: now,
            captured_tz: o.captured_tz,
            location: loc,
            location_unavailable_reason: o.location_unavailable_reason,
            selfie_path: o.selfie.map(|f| f.path),
            client_ref: o.client_ref,
            client_ip: ip.clone(),
            device_uptime_ms: Some(o.device_uptime_ms),
            clock_skew_ms: skew,
        };
        sqlx::query("SAVEPOINT sync_item").execute(&mut **tx).await?;
        let outcome = punch::record(&mut tx, p.user_id, employee, &cap).await;
        let sp = if outcome.is_err() { "ROLLBACK TO SAVEPOINT sync_item" } else { "RELEASE SAVEPOINT sync_item" };
        sqlx::query(sp).execute(&mut **tx).await?;
        results[i] = match outcome {
            Ok(Outcome::Stored { punch_id, flags, duplicate }) => json!({
                "idempotency_key": key,
                "outcome": if duplicate { "duplicate" } else if flags.is_empty() { "accepted" } else { "flagged" },
                "code": if duplicate { json!("DUPLICATE_PUNCH") } else { Value::Null },
                "punch_id": punch_id, "flags": flags, "refusal": null,
            }),
            Ok(Outcome::Refused { rule_ref, message, .. }) => item_problem(&key, "PUNCH_REFUSED", Some(rule_ref), message),
            Err(db::DomainError::Validation(m)) => item_problem(&key, "VALIDATION_FAILED", None, m),
            Err(e) => return Err(e.into()),
        };
    }
    tx.commit().await?;
    Ok(Json(json!({ "results": results })))
}

// ---------------------------------------------------------------------------------------------------------------------
// Reads

/// GET /v1/punches/{id} (TCP-07; location only with TCP-08). Outside scope → 404.
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-07").await?;
    let with_location = holds(&mut tx, &p, "TCP-08").await?;
    let body = punch::get_punch(&mut tx, id, with_location).await?;
    let employee = body["employee_id"].as_str().and_then(|s| s.parse().ok());
    if !sees(&visible, employee) {
        return Err(ApiError::NotFound);
    }
    Ok(Json(body))
}

#[derive(Deserialize)]
pub struct PunchQuery {
    employee_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    source: Option<String>,
    has_open_flag: Option<bool>,
}

// shortcut: branch_id, department_id, device_id and sort filters are not offered yet; add them with the HR punch screen.
/// GET /v1/punches (TCP-07).
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<PunchQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-07").await?;
    let with_location = holds(&mut tx, &p, "TCP-08").await?;
    let f = punch::PunchFilter { employees: narrow(visible, q.employee_id), from: q.from, to: q.to, source: q.source, has_open_flag: q.has_open_flag };
    Ok(Json(json!({ "items": punch::list_punches(&mut tx, &f, with_location).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
pub struct AttemptQuery {
    employee_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    rule_ref: Option<String>,
}

/// GET /v1/punch-attempts (TCP-09).
pub async fn attempts(Auth(p, mut tx): Auth, Query(q): Query<AttemptQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-09").await?;
    let with_location = holds(&mut tx, &p, "TCP-08").await?;
    let f = punch::AttemptFilter { employees: narrow(visible, q.employee_id), from: q.from, to: q.to, rule_ref: q.rule_ref };
    Ok(Json(json!({ "items": punch::list_attempts(&mut tx, &f, with_location).await?, "next_cursor": null })))
}

// ---------------------------------------------------------------------------------------------------------------------
// Flags

#[derive(Deserialize)]
pub struct FlagQuery {
    state: Option<String>,
    reason: Option<String>,
    employee_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/punch-flags (TCP-06), oldest first, each with its punch.
pub async fn list_flags(Auth(p, mut tx): Auth, Query(q): Query<FlagQuery>) -> Result<Json<Value>, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-06").await?;
    let with_location = holds(&mut tx, &p, "TCP-08").await?;
    let state = q.state.unwrap_or_else(|| "raised".into());
    if !["raised", "accepted", "rejected"].contains(&state.as_str()) {
        return Err(ApiError::ValidationFailed("state".into()));
    }
    let f = flags::FlagFilter { employees: narrow(visible, q.employee_id), state, reason: q.reason, from: q.from, to: q.to };
    let mut items = flags::list_flags(&mut tx, &f).await?;
    for item in &mut items {
        let punch_id: Uuid = item["punch_id"].as_str().and_then(|s| s.parse().ok()).ok_or_else(|| ApiError::Internal(anyhow::anyhow!("flag without punch")))?;
        item["punch"] = punch::get_punch(&mut tx, punch_id, with_location).await?;
    }
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionIn {
    comment: Option<String>,
    reason: Option<String>,
}

async fn decide(p: Principal, mut tx: OrgTx<'static>, id: Uuid, headers: HeaderMap, raw: Bytes, accept: bool) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-06").await?;
    let employee = flags::flag_employee(&mut tx, id).await?;
    if !sees(&visible, Some(employee)) {
        return Err(ApiError::NotFound);
    }
    if p.employee_id == Some(employee) {
        return Err(ApiError::SelfApprovalForbidden); // BR-07
    }
    let op = format!("POST /v1/punch-flags/{id}/{}", if accept { "accept" } else { "reject" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: DecisionIn = if raw.is_empty() { DecisionIn { comment: None, reason: None } } else { parse(&raw)? };
    let note = if accept { r.comment.as_deref() } else { r.reason.as_deref() };
    flags::decide(&mut tx, p.user_id, id, accept, note).await?;
    let body = flags::get_flag(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/punch-flags/{id}/accept (TCP-06, SM-FLAG F2).
pub async fn accept_flag(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, true).await
}

/// POST /v1/punch-flags/{id}/reject (TCP-06, SM-FLAG F3; reason required).
pub async fn reject_flag(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    decide(p, tx, id, headers, raw, false).await
}

// ---------------------------------------------------------------------------------------------------------------------
// Manual attendance

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualEntry {
    employee_id: Uuid,
    event_at: DateTime<Utc>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManualIn {
    branch_id: Uuid,
    device_id: Option<Uuid>,
    reason: String,
    entries: Vec<ManualEntry>,
}

/// POST /v1/manual-attendance (TCP-04; PC-R9). One transaction: any invalid entry refuses the whole request.
pub async fn manual(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "TCP-04").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/manual-attendance", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ManualIn = parse(&raw)?;
    if r.entries.is_empty() || r.entries.len() > 500 {
        return Err(ApiError::ValidationFailed("entries must hold 1 to 500 items".into()));
    }
    if let Some(i) = r.entries.iter().position(|e| !sees(&visible, Some(e.employee_id))) {
        return Err(ApiError::ValidationFailed(format!("entries[{i}].employee_id is not an employee you manage")));
    }
    let entries: Vec<(Uuid, DateTime<Utc>)> = r.entries.iter().map(|e| (e.employee_id, e.event_at)).collect();
    let ids = punch::record_manual(&mut tx, p.user_id, key, r.branch_id, r.device_id, &r.reason, &entries).await?;
    let with_location = holds(&mut tx, &p, "TCP-08").await?;
    let mut punches = Vec::with_capacity(ids.len());
    for id in ids {
        punches.push(punch::get_punch(&mut tx, id, with_location).await?);
    }
    done(tx, key, StatusCode::CREATED, json!({ "punches": punches })).await
}
