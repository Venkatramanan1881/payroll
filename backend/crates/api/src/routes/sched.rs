//! scheduling.yaml: shifts, calendars, schedules and roster.

use crate::routes::employees::today;
use crate::{idempotency, ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::NaiveDate;
use db::OrgTx;
use domain_iam::authz::require;
use domain_sched::shift;
use serde::Deserialize;
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub(crate) fn reply(status: StatusCode, body: Value) -> Response {
    match body["updated_at"].as_str() {
        Some(t) => (status, [(header::ETAG, format!("W/\"{t}\""))], Json(body)).into_response(),
        None => (status, Json(body)).into_response(),
    }
}

pub(crate) fn parse<T: serde::de::DeserializeOwned>(raw: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(raw).map_err(|e| ApiError::ValidationFailed(e.to_string()))
}

/// Replays a stored answer for a retried command, or claims the key. Callers then run the command and `done` it.
pub(crate) async fn start(tx: &mut OrgTx<'_>, headers: &HeaderMap, op: &str, raw: &[u8]) -> Result<(Uuid, Option<Response>), ApiError> {
    let key = idempotency::key(headers)?;
    Ok((key, idempotency::begin(tx, key, op, raw).await?.map(|(s, b)| reply(s, b))))
}

pub(crate) async fn done(mut tx: OrgTx<'static>, key: Uuid, status: StatusCode, body: Value) -> Result<Response, ApiError> {
    idempotency::finish(&mut tx, key, status, &body).await?;
    tx.commit().await?;
    Ok(reply(status, body))
}

pub(crate) fn if_match(headers: &HeaderMap, current: &Value) -> Result<(), ApiError> {
    let sent = headers.get(header::IF_MATCH).ok_or(ApiError::PreconditionRequired)?;
    let expected = format!("W/\"{}\"", current["updated_at"].as_str().unwrap_or_default());
    if sent.to_str().ok() == Some(expected.as_str()) { Ok(()) } else { Err(ApiError::PreconditionFailed) }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewShift {
    code: String,
    name: String,
    effective_from: NaiveDate,
    #[serde(default)]
    definition: Map<String, Value>,
}

pub async fn create_shift(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/shifts", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let req: NewShift = parse(&raw)?;
    let id = shift::create_shift(&mut tx, p.user_id, &req.code, &req.name, req.effective_from, req.definition).await?;
    let body = shift::get_shift(&mut tx, id, today()).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ShiftFilter {
    #[serde(default)]
    include_retired: bool,
}

pub async fn list_shifts(Auth(p, mut tx): Auth, Query(f): Query<ShiftFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    let items = shift::list_shifts(&mut tx, f.include_retired, today()).await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

pub async fn get_shift(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    Ok(Json(shift::get_shift(&mut tx, id, today()).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewVersion {
    effective_from: NaiveDate,
    #[serde(default)]
    definition: Map<String, Value>,
}

pub async fn create_shift_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/shifts/{id}/versions"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let req: NewVersion = parse(&raw)?;
    let v = shift::create_version(&mut tx, p.user_id, id, req.effective_from, req.definition).await?;
    let body = shift::get_version(&mut tx, v).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

pub async fn get_shift_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    Ok(reply(StatusCode::OK, shift::get_version(&mut tx, id).await?))
}

pub async fn update_shift_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-01").await?;
    sqlx::query("SELECT 1 FROM sched.shift_version WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    let current = shift::get_version(&mut tx, id).await?;
    if_match(&headers, &current)?;
    let patch: Map<String, Value> = parse(&raw)?;
    shift::update_version(&mut tx, p.user_id, id, patch).await?;
    let body = shift::get_version(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

pub async fn publish_shift_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/shift-versions/{id}/publish"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    shift::publish_version(&mut tx, p.user_id, id, today()).await?;
    let body = shift::get_version(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

use domain_sched::calendar;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewPattern {
    name: String,
    kind: String,
    weekly_days: Option<Vec<i16>>,
    alt_day: Option<i16>,
    alt_weeks: Option<Vec<i16>>,
}

pub async fn create_pattern(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-04").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/weekly-off-patterns", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewPattern = parse(&raw)?;
    let id = calendar::create_pattern(&mut tx, p.user_id, calendar::NewPattern { name: r.name, kind: r.kind, weekly_days: r.weekly_days, alt_day: r.alt_day, alt_weeks: r.alt_weeks }).await?;
    let body = calendar::get_pattern(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct RetiredFilter {
    #[serde(default)]
    include_retired: bool,
}

pub async fn list_patterns(Auth(p, mut tx): Auth, Query(f): Query<RetiredFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    Ok(Json(json!({ "items": calendar::list_patterns(&mut tx, f.include_retired).await?, "next_cursor": null })))
}

pub async fn retire_pattern(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-04").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/weekly-off-patterns/{id}/retire"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    calendar::retire_pattern(&mut tx, p.user_id, id).await?;
    let body = calendar::get_pattern(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCalendar {
    name: String,
}

pub async fn create_calendar(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-05").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/holiday-calendars", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewCalendar = parse(&raw)?;
    let id = calendar::create_calendar(&mut tx, p.user_id, &r.name).await?;
    let body = calendar::get_calendar(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

pub async fn list_calendars(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-05").await?;
    Ok(Json(json!({ "items": calendar::list_calendars(&mut tx).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarPatch {
    name: Option<String>,
    retired: Option<bool>,
}

pub async fn update_calendar(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    calendar_in_scope(&mut tx, &p, id).await?;
    sqlx::query("SELECT 1 FROM sched.holiday_calendar WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &calendar::get_calendar(&mut tx, id).await?)?;
    let r: CalendarPatch = parse(&raw)?;
    calendar::update_calendar(&mut tx, p.user_id, id, r.name, r.retired).await?;
    let body = calendar::get_calendar(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
pub struct HolidayFilter {
    kind: Option<String>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

pub async fn list_holidays(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, Query(f): Query<HolidayFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-05").await?;
    Ok(Json(json!({ "items": calendar::list_holidays(&mut tx, id, f.kind.as_deref(), f.from, f.to).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewHoliday {
    holiday_date: NaiveDate,
    name: String,
    kind: String,
    reason: String,
}

pub async fn add_holiday(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    calendar_in_scope(&mut tx, &p, id).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/holiday-calendars/{id}/holidays"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewHoliday = parse(&raw)?;
    let h = calendar::add_holiday(&mut tx, p.user_id, id, r.holiday_date, &r.name, &r.kind, &r.reason).await?;
    let body = calendar::get_holiday(&mut tx, h).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HolidayPatch {
    holiday_date: Option<NaiveDate>,
    name: Option<String>,
    kind: Option<String>,
    reason: String,
}

pub async fn update_holiday(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let cal = calendar::holiday_calendar_of(&mut tx, id).await?;
    calendar_in_scope(&mut tx, &p, cal).await?;
    sqlx::query("SELECT 1 FROM sched.holiday WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &calendar::get_holiday(&mut tx, id).await?)?;
    let r: HolidayPatch = parse(&raw)?;
    calendar::update_holiday(&mut tx, p.user_id, id, r.holiday_date, r.name, r.kind, &r.reason).await?;
    let body = calendar::get_holiday(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

pub async fn remove_holiday(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let cal = calendar::holiday_calendar_of(&mut tx, id).await?;
    calendar_in_scope(&mut tx, &p, cal).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/holidays/{id}/remove"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    calendar::remove_holiday(&mut tx, p.user_id, id).await?;
    idempotency::finish(&mut tx, key, StatusCode::NO_CONTENT, &Value::Null).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

use crate::routes::employees::load_in_scope;
use domain_sched::schedule;

/// Whether the caller may see this employee for `op` (same rule as the employee routes: out of scope = not visible).
async fn visible(tx: &mut OrgTx<'_>, p: &domain_iam::authz::Principal, employee: Uuid, op: &str) -> Result<bool, ApiError> {
    match load_in_scope(tx, p, employee, op, false).await {
        Ok(_) => Ok(true),
        Err(ApiError::NotFound) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Keeps only rows whose employee the caller may see.
async fn only_visible(tx: &mut OrgTx<'_>, p: &domain_iam::authz::Principal, rows: Vec<Value>, op: &str) -> Result<Vec<Value>, ApiError> {
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let emp = r["employee_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or_else(|| ApiError::Internal(anyhow::anyhow!("row without employee")))?;
        if visible(tx, p, emp, op).await? {
            out.push(r);
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewSchedule {
    employee_id: Uuid,
    source: String,
    shift_id: Option<Uuid>,
    weekly_off_pattern_id: Option<Uuid>,
    valid_from: NaiveDate,
    reason: String,
}

pub async fn assign_schedule(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-02").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/employee-schedules", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewSchedule = parse(&raw)?;
    load_in_scope(&mut tx, &p, r.employee_id, "SCH-02", true).await?;
    let new = schedule::NewSchedule { employee: r.employee_id, source: r.source, shift: r.shift_id, pattern: r.weekly_off_pattern_id, from: r.valid_from, reason: r.reason };
    let body = schedule::assign_schedule(&mut tx, p.user_id, new).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct ScheduleFilter {
    employee_id: Option<Uuid>,
    on_date: Option<NaiveDate>,
}

// shortcut: scope is filtered row by row after the query; push it into SQL when lists get large.
pub async fn list_schedules(Auth(p, mut tx): Auth, Query(f): Query<ScheduleFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    let rows = schedule::list_schedules(&mut tx, f.employee_id, f.on_date).await?;
    Ok(Json(json!({ "items": only_visible(&mut tx, &p, rows, "SCH-03").await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterEntryIn {
    employee_id: Uuid,
    roster_date: NaiveDate,
    is_weekly_off: bool,
    shift_id: Option<Uuid>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterSet {
    reason: Option<String>,
    entries: Vec<RosterEntryIn>,
}

pub async fn set_roster(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "SCH-02").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/roster-entries/set", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: RosterSet = parse(&raw)?;
    if r.entries.is_empty() || r.entries.len() > 1000 {
        return Err(ApiError::ValidationFailed("entries: 1 to 1000".into()));
    }
    let mut employees: Vec<Uuid> = r.entries.iter().map(|e| e.employee_id).collect();
    employees.sort();
    employees.dedup();
    for e in employees {
        load_in_scope(&mut tx, &p, e, "SCH-02", true).await?; // any employee out of scope: 404, nothing written
    }
    let input = r.entries.into_iter().map(|e| schedule::RosterInput { employee: e.employee_id, date: e.roster_date, weekly_off: e.is_weekly_off, shift: e.shift_id }).collect();
    let entries = schedule::set_roster(&mut tx, p.user_id, input, r.reason.as_deref()).await?;
    done(tx, key, StatusCode::OK, json!({ "entries": entries })).await
}

#[derive(Deserialize)]
pub struct RosterFilter {
    employee_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

pub async fn list_roster(Auth(p, mut tx): Auth, Query(f): Query<RosterFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "SCH-03").await?;
    let rows = schedule::list_roster(&mut tx, f.employee_id, f.from, f.to).await?;
    Ok(Json(json!({ "items": only_visible(&mut tx, &p, rows, "SCH-03").await?, "next_cursor": null })))
}

#[derive(Deserialize)]
pub struct Range {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}

/// GET /v1/me/roster: the caller's own expected days; holidays from the master branch's calendar.
pub async fn my_roster(Auth(p, mut tx): Auth, Query(r): Query<Range>) -> Result<Json<Value>, ApiError> {
    let me = p.employee_id.ok_or(ApiError::NotFound)?;
    let emp = load_in_scope(&mut tx, &p, me, "SCH-03", false).await?;
    let branch = emp["current"]["master_branch_id"].as_str().and_then(|s| Uuid::parse_str(s).ok()).ok_or(ApiError::NotFound)?;
    let from = r.from.unwrap_or_else(today);
    let to = r.to.unwrap_or(from + chrono::Days::new(13));
    Ok(Json(json!({ "items": schedule::schedule_days(&mut tx, me, branch, from, to).await? })))
}

/// SCH-05 is scoped (PERMISSION_MATRIX): a calendar may be changed only when every branch it serves is in the caller's
/// scope. Out of scope reads as not found, as for employees.
async fn calendar_in_scope(tx: &mut OrgTx<'_>, p: &domain_iam::authz::Principal, calendar: Uuid) -> Result<(), ApiError> {
    use domain_iam::authz::{employee_in_scope, GrantKind};
    let grant = require(tx, p, "SCH-05").await?;
    calendar::get_calendar(tx, calendar).await?;
    let branches = calendar::calendar_branches(tx, calendar).await?;
    let ok = matches!(grant.kind, GrantKind::All | GrantKind::Scoped) && branches.iter().all(|b| employee_in_scope(&grant.scope, *b, None));
    if ok { Ok(()) } else { Err(ApiError::NotFound) }
}
