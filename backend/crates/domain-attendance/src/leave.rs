//! Leave (C08): types with append-only versions, balances as a ledger (LM-R4), and leave requests on the shared
//! approval flow (SM-REQ). Approval posts one debit and recalculates the days; cancellation posts a reversal.

use crate::approval::{self, Step};
use chrono::{NaiveDate, Utc};
use db::{invalid, DomainError, OrgTx};
use domain_iam::authz::Principal;
use domain_timecap::punch::{branch_tz, master_branch_on};
use serde_json::{json, Map, Value};
use uuid::Uuid;

const KIND: &str = "leave";

/// leave_type_version columns a client may set (LeaveTypeVersionInput); fixed names, never input text in SQL.
const VERSION_FIELDS: [&str; 23] = [
    "effective_from", "reason", "paid", "eligibility", "credit_method", "credit_days", "earn_per_worked_days", "accrual_timing", "proration",
    "max_balance", "year_end", "carry_forward_max", "negative_allowed_days", "half_day_allowed", "min_notice_days", "max_consecutive_days",
    "after_event_days", "document_after_days", "approval_levels", "timeout_days", "timeout_action", "combine_allowed", "during_notice",
];
const REQUIRED: [&str; 15] = [
    "effective_from", "reason", "paid", "eligibility", "credit_method", "accrual_timing", "proration", "year_end", "negative_allowed_days",
    "half_day_allowed", "min_notice_days", "approval_levels", "timeout_action", "during_notice", "combine_allowed",
];
const DAY_FIELDS: [&str; 5] = ["credit_days", "max_balance", "carry_forward_max", "negative_allowed_days", "max_consecutive_days"];

fn refused(code: &'static str, detail: impl Into<String>) -> DomainError {
    DomainError::Refused(code, detail.into())
}

/// DayQuantity: an exact decimal string such as "1.50".
fn is_day_quantity(s: &str) -> bool {
    let (int, frac) = s.strip_prefix('-').unwrap_or(s).split_once('.').unwrap_or(("", ""));
    (1..=3).contains(&int.len()) && frac.len() == 2 && int.bytes().chain(frac.bytes()).all(|b| b.is_ascii_digit())
}

fn check_version(v: &Map<String, Value>) -> Result<(), DomainError> {
    if let Some(k) = v.keys().find(|k| !VERSION_FIELDS.contains(&k.as_str())) {
        return Err(invalid(format!("unknown field {k}")));
    }
    if let Some(k) = REQUIRED.iter().find(|k| v.get(**k).is_none_or(Value::is_null)) {
        return Err(invalid(format!("{k} is required")));
    }
    if v["reason"].as_str().is_none_or(|r| r.trim().is_empty()) {
        return Err(invalid("reason is required (G9)"));
    }
    for k in DAY_FIELDS.iter().chain(["document_after_days"].iter()) {
        if let Some(x) = v.get(*k).filter(|x| !x.is_null()) {
            if !x.as_str().is_some_and(|s| is_day_quantity(s) && !s.starts_with('-')) {
                return Err(invalid(format!("{k} must be a day quantity such as \"1.50\"")));
            }
        }
    }
    let e = v["eligibility"].as_object().ok_or_else(|| invalid("eligibility must be an object"))?;
    if let Some(k) = e.keys().find(|k| !["category_ids", "employee_type_ids", "after_probation"].contains(&k.as_str())) {
        return Err(invalid(format!("eligibility.{k} is not a known field")));
    }
    Ok(())
}

async fn insert_version(tx: &mut OrgTx<'_>, actor: Uuid, leave_type: Uuid, v: &Map<String, Value>) -> Result<Uuid, DomainError> {
    let from: NaiveDate = v["effective_from"].as_str().and_then(|s| s.parse().ok()).ok_or_else(|| invalid("effective_from"))?;
    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.leave_type_version WHERE leave_type_id = $1 AND effective_from = $2)")
        .bind(leave_type)
        .bind(from)
        .fetch_one(&mut ***tx)
        .await?;
    if taken {
        return Err(DomainError::VersionFrozen); // versions are never edited (G5)
    }
    let cols: Vec<&str> = VERSION_FIELDS.iter().copied().filter(|c| v.contains_key(*c)).collect();
    let sql = format!(
        "INSERT INTO leave.leave_type_version (org_id, leave_type_id, created_by, {cols})
         SELECT app.current_org(), $2, $3, {vals} FROM jsonb_populate_record(NULL::leave.leave_type_version, $1) r RETURNING id",
        cols = cols.join(", "),
        vals = cols.iter().map(|c| format!("r.{c}")).collect::<Vec<_>>().join(", ")
    );
    Ok(sqlx::query_scalar(&sql).bind(Value::Object(v.clone())).bind(leave_type).bind(actor).fetch_one(&mut ***tx).await?)
}

pub async fn create_type(tx: &mut OrgTx<'_>, actor: Uuid, code: &str, name: &str, version: &Value) -> Result<Uuid, DomainError> {
    let v = version.as_object().ok_or_else(|| invalid("version must be an object"))?;
    check_version(v)?;
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(invalid("code and name are required"));
    }
    let id: Uuid = sqlx::query_scalar("INSERT INTO leave.leave_type (org_id, code, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $3) RETURNING id")
        .bind(code.trim())
        .bind(name.trim())
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    insert_version(tx, actor, id, v).await?;
    db::audit(tx, actor, "leave_type.create", "leave_type", Some(id), Some(version.clone())).await?;
    Ok(id)
}

pub async fn publish_version(tx: &mut OrgTx<'_>, actor: Uuid, leave_type: Uuid, version: &Value) -> Result<Uuid, DomainError> {
    let v = version.as_object().ok_or_else(|| invalid("version must be an object"))?;
    check_version(v)?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.leave_type WHERE id = $1)").bind(leave_type).fetch_one(&mut ***tx).await?;
    if !exists {
        return Err(DomainError::NotFound);
    }
    let id = insert_version(tx, actor, leave_type, v).await?;
    db::audit(tx, actor, "leave_type.publish_version", "leave_type", Some(leave_type), Some(version.clone())).await?;
    Ok(id)
}

const VERSION_JSON: &str = "jsonb_build_object('id', v.id, 'leave_type_id', v.leave_type_id, 'effective_from', v.effective_from, 'created_at', v.created_at,
    'created_by', v.created_by, 'settings', to_jsonb(v) - ARRAY['id','org_id','leave_type_id','effective_from','created_at','created_by'])";

pub async fn get_version(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {VERSION_JSON} FROM leave.leave_type_version v WHERE v.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

fn type_json(on: &str) -> String {
    format!(
        "jsonb_build_object('id', t.id, 'code', t.code, 'name', t.name, 'retired_at', t.retired_at,
           'current_version', (SELECT {VERSION_JSON} FROM leave.leave_type_version v WHERE v.leave_type_id = t.id AND v.effective_from <= {on}
                               ORDER BY v.effective_from DESC LIMIT 1))"
    )
}

pub async fn get_type(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!(
        "SELECT {} || jsonb_build_object('versions', coalesce((SELECT jsonb_agg({VERSION_JSON} ORDER BY v.effective_from) FROM leave.leave_type_version v WHERE v.leave_type_id = t.id), '[]'))
         FROM leave.leave_type t WHERE t.id = $1",
        type_json("current_date")
    );
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// Leave types with the version in force on `on`; `eligible_for` limits them to an employee's eligible types (LM-01).
pub async fn list_types(tx: &mut OrgTx<'_>, on: NaiveDate, include_retired: bool, eligible_for: Option<Uuid>) -> Result<Vec<Value>, DomainError> {
    let sql = format!("SELECT t.id, {} FROM leave.leave_type t WHERE $2 OR t.retired_at IS NULL ORDER BY t.name", type_json("$1"));
    let rows: Vec<(Uuid, Value)> = sqlx::query_as(&sql).bind(on).bind(include_retired).fetch_all(&mut ***tx).await?;
    let mut out = Vec::new();
    for (id, t) in rows {
        if let Some(e) = eligible_for {
            match version_on(tx, id, on).await? {
                Some(v) if eligible(tx, e, &v, on).await? => {}
                _ => continue,
            }
        }
        out.push(t);
    }
    Ok(out)
}

/// The rules of the version in force on a date (days in hundredths).
pub struct Version {
    pub paid: bool,
    eligibility: Value,
    pub(crate) negative_allowed: i64,
    half_day_allowed: bool,
    min_notice_days: i32,
    max_consecutive: Option<i64>,
    after_event_days: Option<i32>,
    document_after: Option<i64>,
    two_levels: bool,
    combine_allowed: bool,
}

type VersionRow = (bool, Value, i64, bool, i32, Option<i64>, Option<i32>, Option<i64>, String, bool);

/// Includes retired types: requests already raised can still be decided; submission refuses retired types itself.
pub async fn version_on(tx: &mut OrgTx<'_>, leave_type: Uuid, date: NaiveDate) -> Result<Option<Version>, DomainError> {
    let row: Option<VersionRow> = sqlx::query_as(
        "SELECT v.paid, v.eligibility, (v.negative_allowed_days * 100)::int8, v.half_day_allowed, v.min_notice_days, (v.max_consecutive_days * 100)::int8,
                v.after_event_days, (v.document_after_days * 100)::int8, v.approval_levels, v.combine_allowed
         FROM leave.leave_type_version v JOIN leave.leave_type t ON t.id = v.leave_type_id
         WHERE v.leave_type_id = $1 AND v.effective_from <= $2 ORDER BY v.effective_from DESC LIMIT 1",
    )
    .bind(leave_type)
    .bind(date)
    .fetch_optional(&mut ***tx)
    .await?;
    Ok(row.map(|r| Version {
        paid: r.0,
        eligibility: r.1,
        negative_allowed: r.2,
        half_day_allowed: r.3,
        min_notice_days: r.4,
        max_consecutive: r.5,
        after_event_days: r.6,
        document_after: r.7,
        two_levels: r.8 == "manager_then_hr",
        combine_allowed: r.9,
    }))
}

/// LM-01: category, employee type and after probation, on the date.
async fn eligible(tx: &mut OrgTx<'_>, employee: Uuid, v: &Version, date: NaiveDate) -> Result<bool, DomainError> {
    let ids = |k: &str| -> Vec<Uuid> { v.eligibility[k].as_array().map(|a| a.iter().filter_map(|x| x.as_str()?.parse().ok()).collect()).unwrap_or_default() };
    for (field, kind) in [("category_ids", "category"), ("employee_type_ids", "employee_type")] {
        let allowed = ids(field);
        if allowed.is_empty() {
            continue;
        }
        let has: Option<Uuid> = sqlx::query_scalar("SELECT list_entry_id FROM workforce.employee_assignment WHERE employee_id = $1 AND kind = $2 AND valid @> $3::date")
            .bind(employee)
            .bind(kind)
            .bind(date)
            .fetch_optional(&mut ***tx)
            .await?
            .flatten();
        if !has.is_some_and(|h| allowed.contains(&h)) {
            return Ok(false);
        }
    }
    if v.eligibility["after_probation"].as_bool() == Some(true) {
        let done: bool = sqlx::query_scalar("SELECT probation_end_date IS NULL OR probation_end_date < $2 FROM workforce.employee WHERE id = $1")
            .bind(employee)
            .bind(date)
            .fetch_one(&mut ***tx)
            .await?;
        if !done {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The employee's account for a type, locked for the balance check and posting (LM-07).
pub async fn lock_account(tx: &mut OrgTx<'_>, employee: Uuid, leave_type: Uuid) -> Result<Uuid, DomainError> {
    sqlx::query("INSERT INTO leave.leave_account (org_id, employee_id, leave_type_id) VALUES (app.current_org(), $1, $2) ON CONFLICT DO NOTHING")
        .bind(employee)
        .bind(leave_type)
        .execute(&mut ***tx)
        .await?;
    Ok(sqlx::query_scalar("SELECT id FROM leave.leave_account WHERE employee_id = $1 AND leave_type_id = $2 FOR UPDATE")
        .bind(employee)
        .bind(leave_type)
        .fetch_one(&mut ***tx)
        .await?)
}

/// Balance in hundredths of a day (sum of the ledger, LM-R4).
pub async fn balance_pct(tx: &mut OrgTx<'_>, account: Uuid) -> Result<i64, DomainError> {
    Ok(sqlx::query_scalar("SELECT coalesce((sum(quantity) * 100)::int8, 0) FROM leave.leave_ledger_entry WHERE account_id = $1").bind(account).fetch_one(&mut ***tx).await?)
}

/// Balances per active type. Missing accounts are opened (the caller commits), so every row carries its account.
pub async fn balances(tx: &mut OrgTx<'_>, employee: Uuid, on: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    sqlx::query(
        "INSERT INTO leave.leave_account (org_id, employee_id, leave_type_id)
         SELECT app.current_org(), $1, t.id FROM leave.leave_type t WHERE t.retired_at IS NULL ON CONFLICT DO NOTHING",
    )
    .bind(employee)
    .execute(&mut ***tx)
    .await?;
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('leave_type_id', t.id, 'leave_type_name', t.name, 'account_id', a.id,
           'balance', to_char(coalesce((SELECT sum(e.quantity) FROM leave.leave_ledger_entry e WHERE e.account_id = a.id AND ($2::date IS NULL OR e.entry_date <= $2)), 0), 'FM990.00'),
           'pending_days', to_char(coalesce((SELECT sum(d.fraction) FROM leave.leave_request q JOIN leave.leave_request_day d ON d.leave_request_id = q.id
                                             WHERE q.employee_id = $1 AND q.leave_type_id = t.id AND q.state IN ('pending_l1','pending_l2')), 0), 'FM990.00'))
         FROM leave.leave_type t JOIN leave.leave_account a ON a.leave_type_id = t.id AND a.employee_id = $1
         WHERE t.retired_at IS NULL ORDER BY t.name",
    )
    .bind(employee)
    .bind(on)
    .fetch_all(&mut ***tx)
    .await?)
}

const ENTRY_JSON: &str = "jsonb_build_object('id', e.id, 'account_id', e.account_id, 'leave_type_id', a.leave_type_id, 'entry_date', e.entry_date,
    'entry_kind', e.entry_kind, 'quantity', to_char(e.quantity, 'FM990.00'), 'cause_kind', e.cause_kind, 'cause_id', e.cause_id, 'reason', e.reason,
    'reverses_entry_id', e.reverses_entry_id, 'created_at', e.created_at, 'created_by', e.created_by)";

// shortcut: no cursor paging, newest 500.
pub async fn ledger(tx: &mut OrgTx<'_>, employee: Uuid, leave_type: Option<Uuid>, kind: Option<&str>) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {ENTRY_JSON} FROM leave.leave_ledger_entry e JOIN leave.leave_account a ON a.id = e.account_id
         WHERE a.employee_id = $1 AND ($2::uuid IS NULL OR a.leave_type_id = $2) AND ($3::text IS NULL OR e.entry_kind = $3)
         ORDER BY e.created_at DESC, e.id LIMIT 500"
    );
    Ok(sqlx::query_scalar(&sql).bind(employee).bind(leave_type).bind(kind).fetch_all(&mut ***tx).await?)
}

/// LVE-07: a manual `adjustment` with a reason. Nothing is overwritten.
pub async fn adjust(tx: &mut OrgTx<'_>, actor: Uuid, employee: Uuid, leave_type: Uuid, quantity: &str, entry_date: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    if !is_day_quantity(quantity) || quantity.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') {
        return Err(invalid("quantity must be a non-zero day quantity such as \"1.50\""));
    }
    if reason.trim().is_empty() {
        return Err(invalid("reason is required"));
    }
    let known: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.leave_type WHERE id = $1)").bind(leave_type).fetch_one(&mut ***tx).await?;
    if !known {
        return Err(invalid("leave_type_id"));
    }
    if crate::comp_off::is_comp_off_type(tx, leave_type).await? {
        return Err(invalid("comp-off balance comes from comp-off credits, not leave adjustments (D-25)"));
    }
    if db::employee_locked(tx, employee, entry_date, entry_date.succ_opt()).await? {
        return Err(DomainError::PeriodLocked);
    }
    let account = lock_account(tx, employee, leave_type).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reason, created_by)
         VALUES (app.current_org(), $1, $2, 'adjustment', $3::numeric, 'manual_adjustment', $4, $5, $6) RETURNING id",
    )
    .bind(account)
    .bind(entry_date)
    .bind(quantity)
    .bind(Uuid::new_v4()) // each adjustment is its own cause; retries are caught by the request's Idempotency-Key
    .bind(reason.trim())
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "leave_ledger.adjust", "leave_account", Some(account), Some(json!({ "quantity": quantity, "reason": reason.trim() }))).await?;
    let sql = format!("SELECT {ENTRY_JSON} FROM leave.leave_ledger_entry e JOIN leave.leave_account a ON a.id = e.account_id WHERE e.id = $1");
    Ok(sqlx::query_scalar(&sql).bind(id).fetch_one(&mut ***tx).await?)
}

pub struct NewLeave {
    pub leave_type: Uuid,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub from_part: String,
    pub to_part: String,
    pub reason: Option<String>,
    pub document: Option<String>,
}

struct Day {
    date: NaiveDate,
    /// 50 or 100.
    pct: i64,
    half: Option<String>,
}

/// The requested working days: weekly offs and holidays are not leave (sandwich LM-16 is off).
async fn request_days(tx: &mut OrgTx<'_>, employee: Uuid, master: Uuid, r: &NewLeave) -> Result<Vec<Day>, DomainError> {
    let mut out = Vec::new();
    for d in r.from.iter_days().take_while(|d| *d <= r.to) {
        let day = domain_sched::schedule::expected_day(tx, employee, d, master).await?;
        if day.weekly_off || day.holiday.is_some() {
            continue;
        }
        let part = if d == r.from && r.from_part != "full" {
            Some(r.from_part.clone())
        } else if d == r.to && r.to_part != "full" {
            Some(r.to_part.clone())
        } else {
            None
        };
        out.push(Day { date: d, pct: if part.is_some() { 50 } else { 100 }, half: part });
    }
    Ok(out)
}

/// SM-REQ R1: own request (LVE-02) or on behalf (LVE-03). Every refusal creates nothing.
pub async fn submit(tx: &mut OrgTx<'_>, user: Uuid, employee: Uuid, r: NewLeave) -> Result<Uuid, DomainError> {
    if r.to < r.from {
        return Err(invalid("to_date must be on or after from_date"));
    }
    let parts = ["full", "first_half", "second_half"];
    if !parts.contains(&r.from_part.as_str()) || !parts.contains(&r.to_part.as_str()) {
        return Err(invalid("from_part and to_part are full, first_half or second_half"));
    }
    if r.from == r.to && r.from_part != r.to_part {
        return Err(invalid("a one-day request has one part"));
    }
    if r.from < r.to && (r.from_part == "first_half" || r.to_part == "second_half") {
        return Err(invalid("a longer request may start on a second half and end on a first half only"));
    }
    if (r.to - r.from).num_days() > 366 {
        return Err(invalid("a request covers at most a year"));
    }
    if let Some(path) = &r.document {
        let ours: bool = sqlx::query_scalar("SELECT starts_with($1, app.current_org()::text || '/') AND position('..' in $1) = 0").bind(path).fetch_one(&mut ***tx).await?;
        if !ours {
            return Err(invalid("document.path must be a file of this organisation"));
        }
    }
    let master = master_branch_on(tx, employee, r.from).await?.ok_or_else(|| invalid("not employed on from_date"))?;
    let retired: Option<bool> = sqlx::query_scalar("SELECT retired_at IS NOT NULL FROM leave.leave_type WHERE id = $1").bind(r.leave_type).fetch_optional(&mut ***tx).await?;
    if retired != Some(false) {
        return Err(invalid("leave_type_id is not an active leave type"));
    }
    // One submission at a time per employee, so two overlapping requests can never both pass the overlap check.
    sqlx::query("SELECT 1 FROM workforce.employee WHERE id = $1 FOR UPDATE").bind(employee).execute(&mut ***tx).await?;
    let v = version_on(tx, r.leave_type, r.from).await?.ok_or_else(|| invalid("leave_type_id has no version in force on from_date"))?;
    if !eligible(tx, employee, &v, r.from).await? {
        return Err(refused("NOT_ELIGIBLE", "the employee is not eligible for this leave type (LM-01)"));
    }
    if (r.from_part != "full" || r.to_part != "full") && !v.half_day_allowed {
        return Err(invalid("half-day leave is off for this type (LM-08)"));
    }
    let days = request_days(tx, employee, master, &r).await?;
    let total: i64 = days.iter().map(|d| d.pct).sum();
    if total == 0 {
        return Err(invalid("the request covers no working day"));
    }
    let tz = branch_tz(tx, master, Utc::now()).await?;
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(&tz).fetch_one(&mut ***tx).await?;
    if r.from >= today && (r.from - today).num_days() < i64::from(v.min_notice_days) {
        return Err(refused("OUTSIDE_REQUEST_WINDOW", format!("this leave needs {} days' notice (LM-09)", v.min_notice_days)));
    }
    if r.from < today && v.after_event_days.is_none_or(|n| (today - r.from).num_days() > i64::from(n)) {
        return Err(refused("OUTSIDE_REQUEST_WINDOW", "leave for past days is outside the allowed window (LM-11)"));
    }
    if v.max_consecutive.is_some_and(|m| total > m) {
        return Err(refused("LIMIT_EXCEEDED", "more days than this leave type allows in one request (LM-10)"));
    }
    if v.document_after.is_some_and(|n| total > n) && r.document.is_none() {
        return Err(invalid("a supporting document is required for leave this long (LM-12)"));
    }
    for d in &days {
        if db::employee_locked(tx, employee, d.date, d.date.succ_opt()).await? {
            return Err(DomainError::PeriodLocked);
        }
        let clash: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM leave.leave_request_day x JOIN leave.leave_request q ON q.id = x.leave_request_id
                            WHERE q.employee_id = $1 AND q.state IN ('pending_l1','pending_l2','approved') AND x.leave_date = $2
                              AND (x.half IS NULL OR $3::text IS NULL OR x.half = $3))",
        )
        .bind(employee)
        .bind(d.date)
        .bind(&d.half)
        .fetch_one(&mut ***tx)
        .await?;
        if clash {
            return Err(refused("OVERLAPPING_REQUEST", format!("{} is already requested", d.date)));
        }
        // A day covered by an active full-day on-duty is a working day away, not a leave day.
        let on_duty: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM perm.on_duty_request WHERE employee_id = $1 AND state IN ('pending_l1','pending_l2','approved')
                            AND duration_kind IN ('full_day','several_days') AND $2 BETWEEN from_date AND to_date)",
        )
        .bind(employee)
        .bind(d.date)
        .fetch_one(&mut ***tx)
        .await?;
        if on_duty {
            return Err(refused("OVERLAPPING_REQUEST", format!("{} is covered by on-duty", d.date)));
        }
    }
    if !v.combine_allowed {
        // LM-14 block: no other leave type right before or after this stretch.
        let next_to: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM leave.leave_request_day x JOIN leave.leave_request q ON q.id = x.leave_request_id
                            WHERE q.employee_id = $1 AND q.state IN ('pending_l1','pending_l2','approved') AND q.leave_type_id <> $2
                              AND x.leave_date IN ($3::date - 1, $4::date + 1))",
        )
        .bind(employee)
        .bind(r.leave_type)
        .bind(r.from)
        .bind(r.to)
        .fetch_one(&mut ***tx)
        .await?;
        if next_to {
            return Err(refused("OVERLAPPING_REQUEST", "this leave type cannot be combined with another (LM-14)"));
        }
    }
    if crate::comp_off::is_comp_off_type(tx, r.leave_type).await? {
        // D-25: comp-off is taken from comp-off credits, never below zero.
        if crate::comp_off::available_pct(tx, employee, r.from).await? < total {
            return Err(refused("LIMIT_EXCEEDED", "not enough comp-off balance (CO-R2)"));
        }
    } else if v.paid {
        let account = lock_account(tx, employee, r.leave_type).await?;
        if balance_pct(tx, account).await? - total < -v.negative_allowed {
            return Err(refused("LIMIT_EXCEEDED", "not enough leave balance (LM-07)"));
        }
    }
    let approver = approval::route(tx, KIND, employee, 1, today).await?.filter(|a| *a != user);
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO leave.leave_request (org_id, employee_id, leave_type_id, from_date, to_date, from_part, to_part, reason, document_path, state,
           current_approver_user_id, requested_by, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, 'pending_l1', $9, $10, $10, $10) RETURNING id",
    )
    .bind(employee)
    .bind(r.leave_type)
    .bind(r.from)
    .bind(r.to)
    .bind(&r.from_part)
    .bind(&r.to_part)
    .bind(r.reason.as_deref().map(str::trim).filter(|s| !s.is_empty()))
    .bind(&r.document)
    .bind(approver)
    .bind(user)
    .fetch_one(&mut ***tx)
    .await?;
    for d in &days {
        sqlx::query("INSERT INTO leave.leave_request_day (org_id, leave_request_id, leave_date, fraction, half, created_by) VALUES (app.current_org(), $1, $2, $3::numeric / 100, $4, $5)")
            .bind(id)
            .bind(d.date)
            .bind(d.pct as i32)
            .bind(&d.half)
            .bind(user)
            .execute(&mut ***tx)
            .await?;
    }
    approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action: "submitted", requester: user, actor: Some(user), on_behalf_of: None, comment: None }).await?;
    db::audit(tx, user, "leave_request.submit", "leave_request", Some(id), None).await?;
    event(tx, "RequestSubmitted", id, employee).await?;
    Ok(id)
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

struct Row {
    employee: Uuid,
    state: String,
    requester: Uuid,
    approver: Option<Uuid>,
    leave_type: Uuid,
    from: NaiveDate,
}

async fn lock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Row, DomainError> {
    let r: Option<(Uuid, String, Uuid, Option<Uuid>, Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT employee_id, state, requested_by, current_approver_user_id, leave_type_id, from_date FROM leave.leave_request WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let (employee, state, requester, approver, leave_type, from) = r.ok_or(DomainError::NotFound)?;
    Ok(Row { employee, state, requester, approver, leave_type, from })
}

async fn request_dates(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Vec<NaiveDate>, DomainError> {
    Ok(sqlx::query_scalar("SELECT leave_date FROM leave.leave_request_day WHERE leave_request_id = $1 ORDER BY leave_date").bind(id).fetch_all(&mut ***tx).await?)
}

pub(crate) async fn total_pct(tx: &mut OrgTx<'_>, id: Uuid) -> Result<i64, DomainError> {
    Ok(sqlx::query_scalar("SELECT coalesce((sum(fraction) * 100)::int8, 0) FROM leave.leave_request_day WHERE leave_request_id = $1").bind(id).fetch_one(&mut ***tx).await?)
}

pub(crate) async fn recalc_days(tx: &mut OrgTx<'_>, employee: Uuid, id: Uuid, trigger: &str) -> Result<(), DomainError> {
    for d in request_dates(tx, id).await? {
        crate::recalculate(tx, employee, d, trigger, Some(id)).await?;
    }
    Ok(())
}

/// SM-REQ R4/R5/R6. Final approval re-checks the balance, posts one debit and recalculates the days.
pub async fn decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, approve: bool, note: Option<&str>) -> Result<(), DomainError> {
    let r = lock(tx, id).await?;
    if p.user_id == r.requester || p.employee_id == Some(r.employee) {
        return Err(DomainError::SelfApproval); // BR-07, including whoever submitted on behalf
    }
    let done = if approve { "approved" } else { "rejected" };
    let level: i16 = match r.state.as_str() {
        "pending_l1" => 1,
        "pending_l2" => 2,
        s => {
            // A closed request answers only to whoever decided it (or the approver it was waiting on when cancelled).
            let decider: Option<Uuid> = sqlx::query_scalar(
                "SELECT coalesce((SELECT actor_user_id FROM app.approval_step WHERE request_type = 'leave' AND request_id = $1 AND action IN ('approved','rejected')
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
    let on_behalf_of = approval::may_decide(tx, p, r.employee, if level == 1 { "LVE-04" } else { "LVE-05" }, r.approver).await?;
    if level == 2 {
        let twice: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM app.approval_step WHERE request_type = 'leave' AND request_id = $1 AND action = 'approved' AND actor_user_id = $2)")
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
    let v = version_on(tx, r.leave_type, r.from).await?.ok_or_else(|| invalid("leave type has no version in force"))?;
    if approve && level == 1 && v.two_levels {
        approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
        let today = Utc::now().date_naive();
        let next = approval::route(tx, KIND, r.employee, 2, today).await?.filter(|a| *a != r.requester);
        sqlx::query("UPDATE leave.leave_request SET state = 'pending_l2', current_approver_user_id = $2, updated_at = now(), updated_by = $3 WHERE id = $1")
            .bind(id)
            .bind(next)
            .bind(p.user_id)
            .execute(&mut ***tx)
            .await?;
        return Ok(());
    }
    if approve {
        // BR-06: until period adjustments exist, a locked day is never moved by leave (no debit without a recalculation).
        for d in request_dates(tx, id).await? {
            if db::employee_locked(tx, r.employee, d, d.succ_opt()).await? {
                return Err(DomainError::PeriodLocked);
            }
        }
        let overlap: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM leave.leave_request_day x JOIN leave.leave_request_day o ON o.leave_date = x.leave_date
                            JOIN leave.leave_request q ON q.id = o.leave_request_id
                            WHERE x.leave_request_id = $1 AND q.id <> $1 AND q.employee_id = $2 AND q.state = 'approved'
                              AND (x.half IS NULL OR o.half IS NULL OR x.half = o.half))",
        )
        .bind(id)
        .bind(r.employee)
        .fetch_one(&mut ***tx)
        .await?;
        if overlap {
            return Err(refused("OVERLAPPING_REQUEST", "another approved leave already covers these days"));
        }
    }
    let comp_off = crate::comp_off::is_comp_off_type(tx, r.leave_type).await?;
    if approve && comp_off {
        let total = total_pct(tx, id).await?;
        crate::comp_off::debit(tx, Some(p.user_id), r.employee, id, r.from, total).await?;
    } else if approve && v.paid {
        // LM-07 at decision: the balance may have moved since submission.
        let account = lock_account(tx, r.employee, r.leave_type).await?;
        let total = total_pct(tx, id).await?;
        if balance_pct(tx, account).await? - total < -v.negative_allowed {
            return Err(refused("LIMIT_EXCEEDED", "not enough leave balance (LM-07)"));
        }
        sqlx::query(
            "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, created_by)
             VALUES (app.current_org(), $1, $2, 'debit', -($3::numeric / 100), 'leave_request', $4, $5) ON CONFLICT DO NOTHING",
        )
        .bind(account)
        .bind(r.from)
        .bind(total as i32)
        .bind(id)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    }
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
    sqlx::query("UPDATE leave.leave_request SET state = $2, decided_basis = 'approver', current_approver_user_id = NULL, updated_at = now(), updated_by = $3 WHERE id = $1")
        .bind(id)
        .bind(done)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, p.user_id, &format!("leave_request.{}", if approve { "approve" } else { "reject" }), "leave_request", Some(id), Some(json!({ "comment": note }))).await?;
    if approve {
        recalc_days(tx, r.employee, id, "leave_approved").await?;
        event(tx, "RequestApproved", id, r.employee).await?;
    }
    Ok(())
}

/// SM-REQ R10/R11 (G-24 open: the requester, or LVE-06 within scope). Approved leave gets a reversal, never a delete.
pub async fn cancel(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, reason: &str) -> Result<(), DomainError> {
    let r = lock(tx, id).await?;
    let own = p.user_id == r.requester || p.employee_id == Some(r.employee);
    if !own {
        approval::may_decide(tx, p, r.employee, "LVE-06", None).await?;
    }
    match r.state.as_str() {
        "cancelled" => return Ok(()),
        "rejected" => return Err(DomainError::IllegalTransition("request is rejected".into())),
        _ => {}
    }
    if reason.trim().is_empty() {
        return Err(invalid("reason is required"));
    }
    if r.state == "approved" {
        // BR-06: cancelling approved leave in a locked period would refund days payroll already used.
        for d in request_dates(tx, id).await? {
            if db::employee_locked(tx, r.employee, d, d.succ_opt()).await? {
                return Err(DomainError::PeriodLocked);
            }
        }
    }
    let level = if r.state == "pending_l2" { 2 } else { 1 };
    // The approver it was waiting on is kept, so they may still ask about it (ILLEGAL_TRANSITION, not NOT_FOUND).
    sqlx::query("UPDATE leave.leave_request SET state = 'cancelled', cancelled_by = $2, cancelled_at = now(), updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(id)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    let on_behalf = None; // cancellation is never "on behalf" of the requester (BR-07 CHECK on the step)
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: "cancelled", requester: r.requester, actor: Some(p.user_id), on_behalf_of: on_behalf, comment: Some(reason.trim()) }).await?;
    if r.state == "approved" {
        sqlx::query(
            "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reverses_entry_id, reason, created_by)
             SELECT app.current_org(), e.account_id, e.entry_date, 'reversal', -e.quantity, 'leave_request', $1, e.id, $2, $3
             FROM leave.leave_ledger_entry e WHERE e.cause_kind = 'leave_request' AND e.cause_id = $1 AND e.entry_kind = 'debit'
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(reason.trim())
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
        crate::comp_off::reverse(tx, p.user_id, r.employee, id, reason.trim()).await?;
        recalc_days(tx, r.employee, id, "leave_cancelled").await?;
    }
    db::audit(tx, p.user_id, "leave_request.cancel", "leave_request", Some(id), Some(json!({ "reason": reason.trim(), "was": r.state }))).await?;
    event(tx, "RequestCancelled", id, r.employee).await?;
    Ok(())
}

fn request_json() -> String {
    format!(
        "jsonb_build_object('id', q.id, 'employee_id', q.employee_id, 'leave_type_id', q.leave_type_id, 'from_date', q.from_date, 'to_date', q.to_date,
           'from_part', q.from_part, 'to_part', q.to_part, 'reason', q.reason, 'document_path', q.document_path, 'requested_by', q.requested_by,
           'state', q.state, 'escalated', q.escalated, 'decided_basis', q.decided_basis, 'current_approver_user_id', q.current_approver_user_id,
           'submitted_at', q.submitted_at, 'cancelled_at', q.cancelled_at, 'cancelled_by', q.cancelled_by,
           'days', coalesce((SELECT jsonb_agg(jsonb_build_object('leave_date', d.leave_date, 'fraction', to_char(d.fraction, 'FM990.00'), 'half', d.half) ORDER BY d.leave_date)
                             FROM leave.leave_request_day d WHERE d.leave_request_id = q.id), '[]'),
           'steps', {})",
        approval::steps_sql(KIND, "q.id")
    )
}

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {} FROM leave.leave_request q WHERE q.id = $1", request_json());
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// Whether the caller may decide this pending request now.
pub async fn can_decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid) -> Result<bool, DomainError> {
    let r: Option<(Uuid, String, Uuid, Option<Uuid>)> =
        sqlx::query_as("SELECT employee_id, state, requested_by, current_approver_user_id FROM leave.leave_request WHERE id = $1 AND state IN ('pending_l1','pending_l2')")
            .bind(id)
            .fetch_optional(&mut ***tx)
            .await?;
    let Some((employee, state, requester, approver)) = r else { return Ok(false) };
    if requester == p.user_id || p.employee_id == Some(employee) {
        return Ok(false);
    }
    let op = if state == "pending_l1" { "LVE-04" } else { "LVE-05" };
    match approval::may_decide(tx, p, employee, op, approver).await {
        Ok(_) => Ok(true),
        Err(DomainError::NotFound | DomainError::Forbidden) => Ok(false),
        Err(e) => Err(e),
    }
}

pub struct Filter {
    pub employees: Option<Vec<Uuid>>,
    /// A single employee asked for (applies to routed requests too).
    pub employee: Option<Uuid>,
    pub state: Option<String>,
    pub leave_type: Option<Uuid>,
}

// shortcut: no cursor paging, newest 500.
pub async fn list(tx: &mut OrgTx<'_>, p: &Principal, f: &Filter) -> Result<Vec<Value>, DomainError> {
    // Visible: employees in scope, plus requests routed to the caller (or someone who delegated to them).
    let sql = format!(
        "SELECT {} FROM leave.leave_request q
         WHERE (($1::uuid[] IS NULL OR q.employee_id = ANY($1)) OR q.current_approver_user_id = $4
                OR (q.current_approver_user_id IN (SELECT approver_user_id FROM iam.delegation WHERE delegate_user_id = $4 AND valid @> now())
                    AND coalesce((SELECT (settings->>'delegation_enabled')::boolean FROM org.organisation_settings_version WHERE effective_from <= current_date
                                  ORDER BY effective_from DESC LIMIT 1), true)))
           AND ($2::text IS NULL OR q.state = $2) AND ($3::uuid IS NULL OR q.leave_type_id = $3) AND ($5::uuid IS NULL OR q.employee_id = $5)
         ORDER BY q.submitted_at DESC, q.id LIMIT 500",
        request_json()
    );
    Ok(sqlx::query_scalar(&sql).bind(&f.employees).bind(&f.state).bind(f.leave_type).bind(p.user_id).bind(f.employee).fetch_all(&mut ***tx).await?)
}

/// Approved leave on a date for the day calculation: (full day?, half, split time if the shift fixes one).
pub async fn leave_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Option<Option<String>>, DomainError> {
    let rows: Vec<(i64, Option<String>)> = sqlx::query_as(
        "SELECT (d.fraction * 100)::int8, d.half FROM leave.leave_request_day d JOIN leave.leave_request q ON q.id = d.leave_request_id
         WHERE q.employee_id = $1 AND d.leave_date = $2 AND q.state = 'approved'",
    )
    .bind(employee)
    .bind(date)
    .fetch_all(&mut ***tx)
    .await?;
    let total: i64 = rows.iter().map(|r| r.0).sum();
    Ok(match total {
        0 => None,
        100.. => Some(None),
        _ => Some(rows.into_iter().find_map(|r| r.1)),
    })
}
