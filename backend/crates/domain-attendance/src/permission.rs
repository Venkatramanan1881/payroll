//! Permissions (C09, PM-R1..R4): types with append-only versions, requests on the shared approval flow (SM-REQ),
//! the usage ledger (PM-R4) and the effect on the day (approved minutes count as worked; no late or early mark for a
//! permission at either edge of the shift). On-duty comes with slice 14.

use crate::approval::{self, Step};
use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc};
use db::{DomainError, OrgTx};
use domain_iam::authz::Principal;
use serde_json::{json, Map, Value};
use uuid::Uuid;

const KIND: &str = "permission";

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

fn refused(code: &'static str, detail: impl Into<String>) -> DomainError {
    DomainError::Refused(code, detail.into())
}

// ---------------------------------------------------------------------------------------------------------------------
// Types and versions

/// Columns of a version as written by the API (PM-02..PM-30), with what this slice can honour.
const COLUMNS: &[&str] = &[
    "duration_unit", "min_minutes", "max_minutes", "limit_basis", "limit_count", "limit_minutes", "limit_period", "max_per_day", "max_per_week",
    "allowed_position", "paid", "counts_as_worked", "counts_toward_overtime", "apply_window", "after_event_days", "min_notice_minutes", "approval",
    "timeout_minutes", "timeout_action", "over_limit", "over_stay", "same_day_half_leave_allowed", "adjacent_off_allowed", "max_consecutive_days",
    "carry_forward_max", "mid_period_joiner", "eligibility", "blackout_enabled", "blackout_dates", "punch_requirement", "converted_uses_limit",
];

fn one_of(v: &Map<String, Value>, k: &str, allowed: &[&str]) -> Result<(), DomainError> {
    match v.get(k).and_then(Value::as_str) {
        Some(s) if allowed.contains(&s) => Ok(()),
        _ => Err(invalid(format!("{k} must be one of {}", allowed.join(", ")))),
    }
}

fn check_version(v: &Map<String, Value>) -> Result<(), DomainError> {
    for k in v.keys() {
        if !COLUMNS.contains(&k.as_str()) && k != "effective_from" && k != "reason" {
            return Err(invalid(format!("{k} is not a permission type setting")));
        }
    }
    one_of(v, "duration_unit", &["any_minutes", "block_30", "block_60"])?;
    one_of(v, "limit_basis", &["count", "hours", "both"])?;
    one_of(v, "limit_period", &["week", "calendar_month", "payroll_period", "quarter", "year"])?;
    one_of(v, "allowed_position", &["start_of_shift", "end_of_shift", "middle_of_shift", "any"])?;
    one_of(v, "apply_window", &["before_only", "before_or_on_day", "after_within_days"])?;
    one_of(v, "approval", &["auto_within_limit", "manager", "manager_then_hr"])?;
    one_of(v, "timeout_action", &["escalate", "auto_approve", "auto_reject"])?;
    // Options whose behaviour is not built yet are refused rather than stored and silently ignored.
    let open = [
        ("over_limit", "block", "PM-18 options other than block need their conversions (OQ-C09-05)"),
        ("over_stay", "late_early_minutes", "PM-19 options other than late_early_minutes (OQ-C09-08)"),
        ("mid_period_joiner", "full", "PM-24 proration (OQ-C09-02)"),
    ];
    for (k, only, why) in open {
        if v.get(k).and_then(Value::as_str) != Some(only) {
            return Err(invalid(format!("{k}: only {only} is available yet: {why}")));
        }
    }
    let fixed = [
        ("paid", json!(true), "PM-11 unpaid permissions are not available yet"),
        ("adjacent_off_allowed", json!(true), "PM-21 is not enforced yet"),
    ];
    for (k, only, why) in fixed {
        if v.get(k) != Some(&only) {
            return Err(invalid(format!("{k}: only {only} is available yet ({why})")));
        }
    }
    if v.get("max_consecutive_days").is_some_and(|c| !c.is_null()) {
        return Err(invalid("max_consecutive_days: PM-22 is not enforced yet"));
    }
    if v.get("timeout_action").and_then(Value::as_str) != Some("escalate") {
        return Err(invalid("timeout_action: only escalate is available yet (no permission timer runs, PM-17)"));
    }
    if v.get("limit_period").and_then(Value::as_str) == Some("payroll_period") {
        return Err(invalid("limit_period: payroll_period waits on OQ-C09-01 (which cycle applies)"));
    }
    for k in ["min_minutes", "max_minutes", "limit_count", "limit_minutes", "max_per_day", "max_per_week", "after_event_days", "min_notice_minutes", "timeout_minutes"] {
        if v.get(k).and_then(Value::as_i64).is_some_and(|n| !(0..=100_000).contains(&n)) {
            return Err(invalid(format!("{k} is out of range")));
        }
    }
    if v.get("carry_forward_max").is_some_and(|c| !c.is_null()) {
        return Err(invalid("carry_forward_max: PM-23 carry forward is not available yet (OQ-C09-02)"));
    }
    if v.get("eligibility").and_then(Value::as_object).is_none_or(|e| !e.is_empty()) {
        return Err(invalid("eligibility: PM-25 rules are not enforced yet; send {} (everyone eligible)"));
    }
    let int = |k: &str| v.get(k).and_then(Value::as_i64);
    let (min, max) = (int("min_minutes").unwrap_or(0), int("max_minutes").unwrap_or(0));
    if min < 1 || max < min {
        return Err(invalid("min_minutes is at least 1 and max_minutes at least min_minutes (PM-04)"));
    }
    let basis = v["limit_basis"].as_str().unwrap_or("");
    if basis != "hours" && int("limit_count").is_none_or(|c| c < 0) {
        return Err(invalid("limit_count is required unless limit_basis = hours (PM-06)"));
    }
    if basis != "count" && int("limit_minutes").is_none_or(|c| c < 0) {
        return Err(invalid("limit_minutes is required unless limit_basis = count (PM-07)"));
    }
    if v["apply_window"] == "after_within_days" && int("after_event_days").is_none_or(|d| d < 0) {
        return Err(invalid("after_event_days is required when apply_window = after_within_days (PM-14)"));
    }
    for k in ["paid", "counts_as_worked", "counts_toward_overtime", "same_day_half_leave_allowed", "adjacent_off_allowed", "blackout_enabled", "converted_uses_limit"] {
        if !v.get(k).is_some_and(Value::is_boolean) {
            return Err(invalid(format!("{k} is required (true or false)")));
        }
    }
    if !v.get("blackout_dates").and_then(Value::as_array).is_some_and(|d| d.iter().all(|x| x.as_str().is_some_and(|s| s.parse::<NaiveDate>().is_ok()))) {
        return Err(invalid("blackout_dates is a list of dates"));
    }
    if v.get("reason").and_then(Value::as_str).is_none_or(|r| r.trim().is_empty() || r.chars().count() > 1000) {
        return Err(invalid("reason is required (1-1000 characters)"));
    }
    v.get("effective_from").and_then(Value::as_str).and_then(|d| d.parse::<NaiveDate>().ok()).ok_or_else(|| invalid("effective_from is a date"))?;
    Ok(())
}

async fn insert_version(tx: &mut OrgTx<'_>, actor: Uuid, ptype: Uuid, v: &Map<String, Value>) -> Result<Uuid, DomainError> {
    check_version(v)?;
    let mut v = v.clone();
    v.entry("max_per_day").or_insert(json!(1)); // PM-09 BRD default; an explicit null means no cap
    let v = &v;
    // jsonb_populate_record maps the checked document onto the version row's columns (unknown keys were refused above).
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO perm.permission_type_version (org_id, permission_type_id, effective_from, reason, created_by,
            duration_unit, min_minutes, max_minutes, limit_basis, limit_count, limit_minutes, limit_period, max_per_day, max_per_week, allowed_position,
            paid, counts_as_worked, counts_toward_overtime, apply_window, after_event_days, min_notice_minutes, approval, timeout_minutes, timeout_action,
            over_limit, over_stay, same_day_half_leave_allowed, adjacent_off_allowed, max_consecutive_days, carry_forward_max, mid_period_joiner,
            eligibility, blackout_enabled, blackout_dates, punch_requirement, converted_uses_limit)
         SELECT app.current_org(), $1, r.effective_from, r.reason, $3,
            r.duration_unit, r.min_minutes, r.max_minutes, r.limit_basis, r.limit_count, r.limit_minutes, r.limit_period, r.max_per_day, r.max_per_week,
            r.allowed_position, r.paid, r.counts_as_worked, r.counts_toward_overtime, r.apply_window, r.after_event_days, r.min_notice_minutes, r.approval,
            r.timeout_minutes, r.timeout_action, r.over_limit, r.over_stay, r.same_day_half_leave_allowed, r.adjacent_off_allowed, r.max_consecutive_days,
            r.carry_forward_max, r.mid_period_joiner, coalesce(r.eligibility, '{}'), r.blackout_enabled, coalesce(r.blackout_dates, '{}'),
            coalesce(r.punch_requirement, '{\"middle_of_shift\": true, \"start_of_shift\": false, \"end_of_shift\": false}'), r.converted_uses_limit
         FROM jsonb_populate_record(NULL::perm.permission_type_version, $2) r RETURNING id",
    )
    .bind(ptype)
    .bind(Value::Object(v.clone()))
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
            DomainError::VersionFrozen // the same effective_from again (G5)
        } else {
            e.into()
        }
    })?;
    db::audit(tx, actor, "permission_type.version_published", "permission_type", Some(ptype), Some(Value::Object(v.clone()))).await?;
    Ok(id)
}

pub async fn create_type(tx: &mut OrgTx<'_>, actor: Uuid, code: &str, name: &str, version: &Value) -> Result<Uuid, DomainError> {
    let (code, name) = (code.trim(), name.trim());
    if code.is_empty() || code.chars().count() > 20 || name.is_empty() || name.chars().count() > 100 {
        return Err(invalid("code (1-20) and name (1-100) are required"));
    }
    let v = version.as_object().ok_or_else(|| invalid("version is an object"))?;
    check_version(v)?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO perm.permission_type (org_id, code, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $3) RETURNING id")
        .bind(code)
        .bind(name)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await
        .map_err(|e| {
            if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
                refused("ALREADY_EXISTS", "a permission type with that code or name exists")
            } else {
                e.into()
            }
        })?;
    insert_version(tx, actor, id, v).await?;
    Ok(id)
}

pub async fn publish_version(tx: &mut OrgTx<'_>, actor: Uuid, ptype: Uuid, version: &Value) -> Result<Uuid, DomainError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM perm.permission_type WHERE id = $1)").bind(ptype).fetch_one(&mut ***tx).await?;
    if !exists {
        return Err(DomainError::NotFound);
    }
    let v = version.as_object().ok_or_else(|| invalid("version is an object"))?;
    insert_version(tx, actor, ptype, v).await
}

const VERSION_JSON: &str = "jsonb_build_object('id', v.id, 'permission_type_id', v.permission_type_id, 'effective_from', v.effective_from,
    'created_at', v.created_at, 'created_by', v.created_by,
    'settings', to_jsonb(v) - ARRAY['org_id','id','permission_type_id','effective_from','created_at','created_by'])";

pub async fn get_version(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {VERSION_JSON} FROM perm.permission_type_version v WHERE v.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

fn type_json(on: &str) -> String {
    format!(
        "jsonb_build_object('id', t.id, 'code', t.code, 'name', t.name, 'retired_at', t.retired_at,
           'current_version', (SELECT {VERSION_JSON} FROM perm.permission_type_version v WHERE v.permission_type_id = t.id AND v.effective_from <= {on}
                               ORDER BY v.effective_from DESC LIMIT 1))"
    )
}

pub async fn get_type(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let mut t: Value = sqlx::query_scalar(&format!("SELECT {} FROM perm.permission_type t WHERE t.id = $1", type_json("(now() AT TIME ZONE 'Asia/Kolkata')::date")))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)?;
    let versions: Vec<Value> = sqlx::query_scalar(&format!("SELECT {VERSION_JSON} FROM perm.permission_type_version v WHERE v.permission_type_id = $1 ORDER BY v.effective_from"))
        .bind(id)
        .fetch_all(&mut ***tx)
        .await?;
    t["versions"] = versions.into();
    Ok(t)
}

pub async fn list_types(tx: &mut OrgTx<'_>, on: NaiveDate) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!("SELECT {} FROM perm.permission_type t WHERE t.retired_at IS NULL ORDER BY t.code", type_json("$1")))
        .bind(on)
        .fetch_all(&mut ***tx)
        .await?)
}

/// The version in force on a date whether or not the type was retired since (for what was already approved).
async fn version_any(tx: &mut OrgTx<'_>, ptype: Uuid, date: NaiveDate) -> Result<Option<Value>, DomainError> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(v) FROM perm.permission_type_version v WHERE v.permission_type_id = $1 AND v.effective_from <= $2 ORDER BY v.effective_from DESC LIMIT 1")
        .bind(ptype)
        .bind(date)
        .fetch_optional(&mut ***tx)
        .await?)
}

/// The version in force on a date, as its stored row (retired types offer nothing new).
async fn version_on(tx: &mut OrgTx<'_>, ptype: Uuid, date: NaiveDate) -> Result<Option<Value>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT to_jsonb(v) FROM perm.permission_type_version v JOIN perm.permission_type t ON t.id = v.permission_type_id
         WHERE v.permission_type_id = $1 AND v.effective_from <= $2 AND t.retired_at IS NULL ORDER BY v.effective_from DESC LIMIT 1",
    )
    .bind(ptype)
    .bind(date)
    .fetch_optional(&mut ***tx)
    .await?)
}

// ---------------------------------------------------------------------------------------------------------------------
// Limit periods and usage (PM-05..PM-08, PM-R4)

/// The PM-08 period containing `date`. `payroll_period` is the attendance period containing it (OQ-C09-01), or the
/// calendar month where none exists.
async fn limit_period(tx: &mut OrgTx<'_>, kind: &str, date: NaiveDate) -> Result<(NaiveDate, NaiveDate), DomainError> {
    let month = |d: NaiveDate| {
        let first = d.with_day(1).expect("valid");
        let next = if d.month() == 12 { NaiveDate::from_ymd_opt(d.year() + 1, 1, 1) } else { NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1) }.expect("valid");
        (first, next.pred_opt().expect("valid"))
    };
    Ok(match kind {
        "week" => {
            // shortcut: weeks start on Monday until OQ-C09-03 says otherwise (or the organisation's week_start is read).
            let start = date - Duration::days(date.weekday().num_days_from_monday() as i64);
            (start, start + Duration::days(6))
        }
        "quarter" => {
            let q0 = (date.month0() / 3) * 3 + 1;
            let first = NaiveDate::from_ymd_opt(date.year(), q0, 1).expect("valid");
            (first, month(first + Duration::days(62)).1)
        }
        "year" => (NaiveDate::from_ymd_opt(date.year(), 1, 1).expect("valid"), NaiveDate::from_ymd_opt(date.year(), 12, 31).expect("valid")),
        "payroll_period" => {
            let p: Option<(NaiveDate, NaiveDate)> = sqlx::query_as("SELECT period_start, period_end FROM period.attendance_period WHERE $1 BETWEEN period_start AND period_end LIMIT 1")
                .bind(date)
                .fetch_optional(&mut ***tx)
                .await?;
            p.unwrap_or_else(|| month(date))
        }
        _ => month(date),
    })
}

/// (count, minutes) used in a period, from the usage ledger (opening balances included).
async fn used(tx: &mut OrgTx<'_>, employee: Uuid, ptype: Uuid, from: NaiveDate, to: NaiveDate) -> Result<(i64, i64), DomainError> {
    Ok(sqlx::query_as(
        "SELECT coalesce(sum(count_delta), 0)::int8, coalesce(sum(minutes_delta), 0)::int8 FROM perm.permission_usage_entry
         WHERE employee_id = $1 AND permission_type_id = $2 AND limit_period && daterange($3, $4, '[]')",
    )
    .bind(employee)
    .bind(ptype)
    .bind(from)
    .bind(to)
    .fetch_one(&mut ***tx)
    .await?)
}

/// PM-05..PM-07 with PM-18 = block: would one more permission of `minutes` exceed the period's limit?
fn over_limit(v: &Value, used: (i64, i64), minutes: i64) -> Option<String> {
    let basis = v["limit_basis"].as_str().unwrap_or("count");
    if basis != "hours" {
        if let Some(c) = v["limit_count"].as_i64() {
            if used.0 + 1 > c {
                return Some(format!("{c} permission(s) allowed in the period; {} used (PM-06)", used.0));
            }
        }
    }
    if basis != "count" {
        if let Some(m) = v["limit_minutes"].as_i64() {
            if used.1 + minutes > m {
                return Some(format!("{m} minutes allowed in the period; {} used (PM-07)", used.1));
            }
        }
    }
    None
}

pub async fn usage(tx: &mut OrgTx<'_>, employee: Uuid, on: NaiveDate) -> Result<Vec<Value>, DomainError> {
    let types: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM perm.permission_type WHERE retired_at IS NULL ORDER BY code").fetch_all(&mut ***tx).await?;
    let mut out = vec![];
    for t in types {
        let Some(v) = version_on(tx, t, on).await? else { continue };
        let (from, to) = limit_period(tx, v["limit_period"].as_str().unwrap_or("calendar_month"), on).await?;
        let (count, minutes) = used(tx, employee, t, from, to).await?;
        let (lc, lm) = (v["limit_count"].as_i64(), v["limit_minutes"].as_i64());
        out.push(json!({
            "permission_type_id": t, "limit_basis": v["limit_basis"], "period_from": from, "period_to": to, "used_count": count, "used_minutes": minutes,
            "limit_count": lc, "limit_minutes": lm, "remaining_count": lc.map(|c| (c - count).max(0)), "remaining_minutes": lm.map(|m| (m - minutes).max(0)),
        }));
    }
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------------------------
// Requests

pub struct NewPermission {
    pub ptype: Uuid,
    pub date: NaiveDate,
    pub start: NaiveTime,
    pub end: NaiveTime,
    pub reason: String,
}

/// Minutes between two times of day; an end before the start runs past midnight (overnight shifts).
pub fn span_minutes(start: NaiveTime, end: NaiveTime) -> i64 {
    let m = (end - start).num_minutes();
    if m <= 0 { m + 24 * 60 } else { m }
}

/// The permission's instants in the employee's zone (master branch on the date).
pub(crate) async fn instants(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, start: NaiveTime, end: NaiveTime) -> Result<(DateTime<Utc>, DateTime<Utc>), DomainError> {
    let tz = crate::employee_tz(tx, employee, date).await?;
    let mut from: DateTime<Utc> = sqlx::query_scalar("SELECT ($1::date + $2::time) AT TIME ZONE $3").bind(date).bind(start).bind(&tz).fetch_one(&mut ***tx).await?;
    // A night shift (e.g. 22:00-06:00 on D): a time before the shift's start is the after-midnight part, on D+1.
    if let Some((s, e)) = crate::shift_times(tx, employee, date).await? {
        let next = from + Duration::days(1);
        if from < s && next >= s && next < e {
            from = next;
        }
    }
    Ok((from, from + Duration::minutes(span_minutes(start, end))))
}

async fn event(tx: &mut OrgTx<'_>, kind: &str, id: Uuid, employee: Uuid) -> Result<(), DomainError> {
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), $1, $2, $3)")
        .bind(kind)
        .bind(id)
        .bind(json!({ "request_type": KIND, "employee_id": employee }))
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

/// PM-R1/PM-R2: an employee's own application, refused before anything is written when any rule fails.
/// Serialises one employee's permission writes, so limits, caps and overlap checks see every concurrent change.
async fn lock_employee(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('permission:' || $1::text))").bind(employee).execute(&mut ***tx).await?;
    Ok(())
}

pub async fn submit(tx: &mut OrgTx<'_>, user: Uuid, employee: Uuid, r: NewPermission) -> Result<Uuid, DomainError> {
    lock_employee(tx, employee).await?;
    let reason = r.reason.trim();
    if reason.is_empty() || reason.chars().count() > 500 {
        return Err(invalid("reason is required (1-500 characters, PM-28)"));
    }
    let v = version_on(tx, r.ptype, r.date).await?.ok_or_else(|| invalid("permission_type_id has no version in force on that date"))?;
    let minutes = span_minutes(r.start, r.end);
    let unit = match v["duration_unit"].as_str() {
        Some("block_30") => 30,
        Some("block_60") => 60,
        _ => 1,
    };
    if minutes % unit != 0 {
        return Err(invalid(format!("the permission is taken in blocks of {unit} minutes (PM-03)")));
    }
    let (min, max) = (v["min_minutes"].as_i64().unwrap_or(1), v["max_minutes"].as_i64().unwrap_or(i64::MAX));
    if minutes < min || minutes > max {
        return Err(invalid(format!("a permission is {min}-{max} minutes (PM-04)")));
    }
    if v["blackout_enabled"] == true && v["blackout_dates"].as_array().is_some_and(|d| d.iter().any(|x| x.as_str() == Some(&r.date.to_string()))) {
        return Err(invalid("the date is a blackout date (PM-26)"));
    }
    let (from, to) = instants(tx, employee, r.date, r.start, r.end).await?;
    // PM-10: the position inside the shift.
    let shift = crate::shift_times(tx, employee, r.date).await?.ok_or_else(|| invalid("there is no shift with set times on that date"))?;
    let position_ok = match v["allowed_position"].as_str() {
        Some("start_of_shift") => from == shift.0 && to < shift.1,
        Some("end_of_shift") => to == shift.1 && from > shift.0,
        Some("middle_of_shift") => from > shift.0 && to < shift.1,
        _ => from >= shift.0 && to <= shift.1,
    };
    if !position_ok {
        return Err(invalid(format!("the permission must fall {} (PM-10)", v["allowed_position"].as_str().unwrap_or("within the shift").replace('_', " "))));
    }
    // PM-14 / PM-15: when it may be applied.
    let now = Utc::now();
    let tz = crate::employee_tz(tx, employee, r.date).await?;
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(&tz).fetch_one(&mut ***tx).await?;
    let window_ok = match v["apply_window"].as_str() {
        Some("before_only") => now < from,
        Some("before_or_on_day") => today <= r.date,
        _ => today <= r.date + Duration::days(v["after_event_days"].as_i64().unwrap_or(0)),
    };
    let notice = v["min_notice_minutes"].as_i64().unwrap_or(0);
    if !window_ok || (now < from && (from - now).num_minutes() < notice) {
        return Err(refused("OUTSIDE_REQUEST_WINDOW", "the permission is outside its application window (PM-14 / PM-15)"));
    }
    if db::employee_locked(tx, employee, r.date, r.date.succ_opt()).await? {
        return Err(DomainError::PeriodLocked);
    }
    // PM-20: a half-day leave on the same day.
    match crate::leave::leave_on(tx, employee, r.date).await? {
        Some(None) => return Err(refused("OVERLAPPING_REQUEST", "the day is a full-day leave")),
        Some(Some(_)) if v["same_day_half_leave_allowed"] == false => return Err(refused("OVERLAPPING_REQUEST", "a half-day leave is taken that day (PM-20)")),
        _ => {}
    }
    // Overlap with another active permission, and the PM-09 per-day / per-week caps (pending requests count).
    let overlapping: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM perm.permission_request WHERE employee_id = $1 AND permission_date = $2 AND state IN ('pending_l1','pending_l2','approved')
                        AND tsrange($2 + start_time, $2 + start_time + make_interval(mins => minutes)) && tsrange($2 + $3::time, $2 + $3::time + make_interval(mins => $4)))",
    )
    .bind(employee)
    .bind(r.date)
    .bind(r.start)
    .bind(minutes as i32)
    .fetch_one(&mut ***tx)
    .await?;
    if overlapping {
        return Err(refused("OVERLAPPING_REQUEST", "another permission covers that time"));
    }
    let week = limit_period(tx, "week", r.date).await?;
    let (per_day, per_week): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE permission_date = $2), count(*) FROM perm.permission_request
         WHERE employee_id = $1 AND permission_date BETWEEN $3 AND $4 AND state IN ('pending_l1','pending_l2','approved')",
    )
    .bind(employee)
    .bind(r.date)
    .bind(week.0)
    .bind(week.1)
    .fetch_one(&mut ***tx)
    .await?;
    if v["max_per_day"].as_i64().is_some_and(|m| per_day + 1 > m) || v["max_per_week"].as_i64().is_some_and(|m| per_week + 1 > m) {
        return Err(refused("LIMIT_EXCEEDED", "the per-day or per-week cap is reached (PM-09)"));
    }
    let period = limit_period(tx, v["limit_period"].as_str().unwrap_or("calendar_month"), r.date).await?;
    let u = used(tx, employee, r.ptype, period.0, period.1).await?;
    if let Some(why) = over_limit(&v, u, minutes) {
        return Err(refused("LIMIT_EXCEEDED", why)); // PM-18 = block
    }

    let auto = v["approval"] == "auto_within_limit";
    let today_date = Utc::now().date_naive();
    let approver = if auto { None } else { approval::route(tx, KIND, employee, 1, today_date).await?.filter(|a| *a != user) };
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO perm.permission_request (org_id, employee_id, permission_type_id, permission_date, start_time, end_time, minutes, origin, reason,
           state, decided_basis, current_approver_user_id, requested_by, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, 'applied', $7, $8, $9, $10, $11, $11, $11) RETURNING id",
    )
    .bind(employee)
    .bind(r.ptype)
    .bind(r.date)
    .bind(r.start)
    .bind(r.end)
    .bind(minutes as i32)
    .bind(reason)
    .bind(if auto { "approved" } else { "pending_l1" })
    .bind(if auto { Some("auto_on_submit") } else { None })
    .bind(approver)
    .bind(user)
    .fetch_one(&mut ***tx)
    .await?;
    approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action: "submitted", requester: user, actor: Some(user), on_behalf_of: None, comment: None }).await?;
    db::audit(tx, user, "permission_request.submit", "permission_request", Some(id), Some(json!({ "minutes": minutes, "auto": auto }))).await?;
    event(tx, "RequestSubmitted", id, employee).await?;
    if auto {
        approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action: "auto_approved", requester: user, actor: None, on_behalf_of: None, comment: Some("PM-16 within limit") }).await?;
        post_usage(tx, id, user).await?;
        crate::recalculate(tx, employee, r.date, "permission_approved", Some(id)).await?;
        event(tx, "RequestApproved", id, employee).await?;
    }
    Ok(id)
}

/// PM-R4: one usage entry per approved request, counted in the period of its date (idempotent by the request).
async fn post_usage(tx: &mut OrgTx<'_>, id: Uuid, actor: Uuid) -> Result<(), DomainError> {
    let (employee, ptype, date, minutes): (Uuid, Uuid, NaiveDate, i32) =
        sqlx::query_as("SELECT employee_id, permission_type_id, permission_date, minutes FROM perm.permission_request WHERE id = $1").bind(id).fetch_one(&mut ***tx).await?;
    let v = version_any(tx, ptype, date).await?.ok_or_else(|| invalid("permission type has no version in force"))?;
    let (from, to) = limit_period(tx, v["limit_period"].as_str().unwrap_or("calendar_month"), date).await?;
    sqlx::query(
        "INSERT INTO perm.permission_usage_entry (org_id, employee_id, permission_type_id, limit_period, entry_kind, origin, count_delta, minutes_delta, cause_kind, cause_id, created_by)
         VALUES (app.current_org(), $1, $2, daterange($3, $4, '[]'), 'usage', 'request', 1, $5, 'permission_request', $6, $7) ON CONFLICT DO NOTHING",
    )
    .bind(employee)
    .bind(ptype)
    .bind(from)
    .bind(to)
    .bind(minutes)
    .bind(id)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

struct Row {
    employee: Uuid,
    state: String,
    requester: Uuid,
    approver: Option<Uuid>,
    ptype: Uuid,
    date: NaiveDate,
    start: NaiveTime,
    minutes: i32,
}

async fn lock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Row, DomainError> {
    type R = (Uuid, String, Option<Uuid>, Option<Uuid>, Uuid, NaiveDate, NaiveTime, i32);
    let r: Option<R> = sqlx::query_as(
        "SELECT employee_id, state, requested_by, current_approver_user_id, permission_type_id, permission_date, start_time, minutes
         FROM perm.permission_request WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let (employee, state, requester, approver, ptype, date, start, minutes) = r.ok_or(DomainError::NotFound)?;
    Ok(Row { employee, state, requester: requester.unwrap_or_default(), approver, ptype, date, start, minutes })
}

/// SM-REQ R4/R5/R6 (PRM-03 level 1, PRM-04 level 2). Final approval re-checks the limit, posts usage and recalculates.
pub async fn decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, approve: bool, note: Option<&str>) -> Result<(), DomainError> {
    let employee = employee_of(tx, id).await?;
    lock_employee(tx, employee).await?;
    let r = lock(tx, id).await?;
    if p.user_id == r.requester || p.employee_id == Some(r.employee) {
        return Err(DomainError::SelfApproval); // BR-07
    }
    let done = if approve { "approved" } else { "rejected" };
    let level: i16 = match r.state.as_str() {
        "pending_l1" => 1,
        "pending_l2" => 2,
        s => {
            let decider: Option<Uuid> = sqlx::query_scalar(
                "SELECT coalesce((SELECT actor_user_id FROM app.approval_step WHERE request_type = 'permission' AND request_id = $1 AND action IN ('approved','rejected')
                                  ORDER BY acted_at DESC LIMIT 1), $2)",
            )
            .bind(id)
            .bind(r.approver)
            .fetch_one(&mut ***tx)
            .await?;
            if decider != Some(p.user_id) {
                return Err(DomainError::NotFound);
            }
            return if s == done { Ok(()) } else { Err(DomainError::IllegalTransition(format!("request is {s}"))) };
        }
    };
    let on_behalf_of = approval::may_decide(tx, p, r.employee, if level == 1 { "PRM-03" } else { "PRM-04" }, r.approver).await?;
    if level == 2 {
        let twice: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM app.approval_step WHERE request_type = 'permission' AND request_id = $1 AND action = 'approved' AND actor_user_id = $2)")
            .bind(id)
            .bind(p.user_id)
            .fetch_one(&mut ***tx)
            .await?;
        if twice {
            return Err(DomainError::IllegalTransition("the first-level approver cannot also decide the second level".into()));
        }
    }
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    if !approve && note.is_none() {
        return Err(invalid("reason is required to reject"));
    }
    let v = version_on(tx, r.ptype, r.date).await?.ok_or_else(|| invalid("permission type has no version in force"))?;
    if approve && level == 1 && v["approval"] == "manager_then_hr" {
        approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
        let next = approval::route(tx, KIND, r.employee, 2, Utc::now().date_naive()).await?.filter(|a| *a != r.requester);
        sqlx::query("UPDATE perm.permission_request SET state = 'pending_l2', current_approver_user_id = $2, updated_at = now(), updated_by = $3 WHERE id = $1")
            .bind(id)
            .bind(next)
            .bind(p.user_id)
            .execute(&mut ***tx)
            .await?;
        db::audit(tx, p.user_id, "permission_request.approve_level_1", "permission_request", Some(id), Some(json!({ "comment": note }))).await?;
        event(tx, "RequestApprovedAtLevel", id, r.employee).await?;
        return Ok(());
    }
    if approve {
        if db::employee_locked(tx, r.employee, r.date, r.date.succ_opt()).await? {
            return Err(DomainError::PeriodLocked); // BR-06: no new effect on a locked day
        }
        // PM-R2 at decision: the period may have filled since submission.
        let period = limit_period(tx, v["limit_period"].as_str().unwrap_or("calendar_month"), r.date).await?;
        let u = used(tx, r.employee, r.ptype, period.0, period.1).await?;
        if let Some(why) = over_limit(&v, u, i64::from(r.minutes)) {
            return Err(refused("LIMIT_EXCEEDED", why));
        }
    }
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
    sqlx::query("UPDATE perm.permission_request SET state = $2, decided_basis = 'approver', current_approver_user_id = NULL, updated_at = now(), updated_by = $3 WHERE id = $1")
        .bind(id)
        .bind(done)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, p.user_id, &format!("permission_request.{}", if approve { "approve" } else { "reject" }), "permission_request", Some(id), Some(json!({ "comment": note }))).await?;
    if approve {
        post_usage(tx, id, p.user_id).await?;
        crate::recalculate(tx, r.employee, r.date, "permission_approved", Some(id)).await?;
        event(tx, "RequestApproved", id, r.employee).await?;
    }
    Ok(())
}

/// PM-29: the employee before the start time (PRM-06); HR afterwards with a reason (PRM-07, checked by the caller).
/// An approved permission gets a usage reversal and the day is recalculated.
pub async fn cancel(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, reason: &str, hr: bool) -> Result<(), DomainError> {
    let employee = employee_of(tx, id).await?;
    lock_employee(tx, employee).await?;
    let r = lock(tx, id).await?;
    match r.state.as_str() {
        "cancelled" => return Ok(()),
        "rejected" => return Err(DomainError::IllegalTransition("request is rejected".into())),
        _ => {}
    }
    let own = p.employee_id == Some(r.employee);
    let (start, _) = instants(tx, r.employee, r.date, r.start, r.start).await?;
    if !hr && (!own || Utc::now() >= start) {
        return Err(DomainError::IllegalTransition("after the start time only HR may cancel (PM-29)".into()));
    }
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(invalid("reason is required"));
    }
    if r.state == "approved" && db::employee_locked(tx, r.employee, r.date, r.date.succ_opt()).await? {
        return Err(DomainError::PeriodLocked);
    }
    let level = if r.state == "pending_l2" { 2 } else { 1 };
    sqlx::query("UPDATE perm.permission_request SET state = 'cancelled', cancelled_by = $2, cancelled_at = now(), updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(id)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: "cancelled", requester: r.requester, actor: Some(p.user_id), on_behalf_of: None, comment: Some(reason.trim()) }).await?;
    if r.state == "approved" {
        sqlx::query(
            "INSERT INTO perm.permission_usage_entry (org_id, employee_id, permission_type_id, limit_period, entry_kind, origin, count_delta, minutes_delta,
                                                      cause_kind, cause_id, reverses_entry_id, reason, created_by)
             SELECT app.current_org(), e.employee_id, e.permission_type_id, e.limit_period, 'reversal', e.origin, -e.count_delta, -e.minutes_delta,
                    'permission_request', $1, e.id, $2, $3
             FROM perm.permission_usage_entry e WHERE e.cause_kind = 'permission_request' AND e.cause_id = $1 AND e.entry_kind = 'usage'
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(reason.trim())
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
        crate::recalculate(tx, r.employee, r.date, "permission_cancelled", Some(id)).await?;
    }
    db::audit(tx, p.user_id, "permission_request.cancel", "permission_request", Some(id), Some(json!({ "reason": reason.trim(), "was": r.state }))).await?;
    event(tx, "RequestCancelled", id, r.employee).await?;
    Ok(())
}

fn request_json() -> String {
    format!(
        "jsonb_build_object('id', q.id, 'employee_id', q.employee_id, 'permission_type_id', q.permission_type_id, 'permission_date', q.permission_date,
           'start_time', to_char(q.start_time, 'HH24:MI'), 'end_time', to_char(q.end_time, 'HH24:MI'), 'minutes', q.minutes, 'reason', q.reason,
           'document_path', q.document_path, 'origin', q.origin, 'requested_by', q.requested_by, 'state', q.state, 'escalated', q.escalated,
           'decided_basis', q.decided_basis, 'current_approver_user_id', q.current_approver_user_id, 'submitted_at', q.submitted_at,
           'cancelled_at', q.cancelled_at, 'cancelled_by', q.cancelled_by, 'steps', {})",
        approval::steps_sql(KIND, "q.id")
    )
}

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {} FROM perm.permission_request q WHERE q.id = $1", request_json()))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub struct Filter {
    /// Employees whose requests the caller sees (None = all); requests routed to `approver` are always included.
    pub employees: Option<Vec<Uuid>>,
    pub approver: Uuid,
    pub employee: Option<Uuid>,
    pub state: Option<String>,
    pub origin: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

pub async fn list(tx: &mut OrgTx<'_>, f: &Filter) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {} FROM perm.permission_request q
         WHERE ($1::uuid[] IS NULL OR q.employee_id = ANY($1) OR q.current_approver_user_id = $7
                OR EXISTS (SELECT 1 FROM app.approval_step st WHERE st.request_type = 'permission' AND st.request_id = q.id AND st.actor_user_id = $7))
           AND ($2::uuid IS NULL OR q.employee_id = $2) AND ($3::text IS NULL OR q.state = $3)
           AND ($4::text IS NULL OR q.origin = $4) AND ($5::date IS NULL OR q.permission_date >= $5) AND ($6::date IS NULL OR q.permission_date <= $6)
         ORDER BY q.permission_date DESC, q.start_time",
        request_json()
    ))
    .bind(&f.employees)
    .bind(f.employee)
    .bind(&f.state)
    .bind(&f.origin)
    .bind(f.from)
    .bind(f.to)
    .bind(f.approver)
    .fetch_all(&mut ***tx)
    .await?)
}

/// The request's employee (callers check scope).
pub async fn employee_of(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Uuid, DomainError> {
    sqlx::query_scalar("SELECT employee_id FROM perm.permission_request WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// Approved permissions on a day, for the engine: (start, end, counts as worked).
pub async fn approved_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>, bool)>, DomainError> {
    let rows: Vec<(Uuid, NaiveTime, NaiveTime)> = sqlx::query_as(
        "SELECT permission_type_id, start_time, end_time FROM perm.permission_request WHERE employee_id = $1 AND permission_date = $2 AND state = 'approved' ORDER BY start_time",
    )
    .bind(employee)
    .bind(date)
    .fetch_all(&mut ***tx)
    .await?;
    let mut out = vec![];
    for (ptype, start, end) in rows {
        let worked = version_any(tx, ptype, date).await?.is_none_or(|v| v["counts_as_worked"] != false);
        let (from, to) = instants(tx, employee, date, start, end).await?;
        out.push((from, to, worked));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_and_limits() {
        let t = |s: &str| NaiveTime::parse_from_str(s, "%H:%M").unwrap();
        assert_eq!((span_minutes(t("09:00"), t("10:30")), span_minutes(t("23:00"), t("01:00"))), (90, 120));
        let v = json!({ "limit_basis": "both", "limit_count": 2, "limit_minutes": 120 });
        assert!(over_limit(&v, (1, 60), 60).is_none());
        assert!(over_limit(&v, (2, 60), 30).is_some(), "count");
        assert!(over_limit(&v, (1, 90), 60).is_some(), "minutes");
        assert!(over_limit(&json!({ "limit_basis": "hours", "limit_minutes": 120 }), (9, 0), 60).is_none(), "count ignored");
    }
}
