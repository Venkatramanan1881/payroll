//! Attendance periods (C12, SM-PERIOD): cycles, period generation and housekeeping (J7), period settings PL-03..07,
//! the pre-lock checklist (PL-R2), period totals (layer 5), lock (PL-R3, BR-06) and unlock through Organisation Admin
//! approval (PL-06, BR-07), adjustments after lock (PL-R4, SM-ADJ) and the payroll input document (MD-06).

use crate::approval::{step, steps_sql, Step};
use chrono::{Datelike, Duration, NaiveDate};
use db::{DomainError, OrgTx};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use uuid::Uuid;

/// The organisation's date.
// shortcut: India time for every organisation; read the organisation's zone when one outside India signs up.
pub async fn org_today(tx: &mut OrgTx<'_>) -> Result<NaiveDate, DomainError> {
    Ok(sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Kolkata')::date").fetch_one(&mut ***tx).await?)
}

// ---------------------------------------------------------------------------------------------------------------------
// Period arithmetic (PL-01 monthly: calendar month, or a custom start day such as 26th -> 25th)

fn days_in(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).expect("valid").pred_opt().expect("valid").day()
}

/// The start of the period in month (y, m): the start day, or the month's last day when the month is shorter.
fn start_in(y: i32, m: u32, start_day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, start_day.min(days_in(y, m))).expect("valid")
}

fn shift_month(y: i32, m: u32, by: i32) -> (i32, u32) {
    let n = y * 12 + m as i32 - 1 + by;
    (n.div_euclid(12), n.rem_euclid(12) as u32 + 1)
}

/// The monthly period containing `d`.
pub fn bounds(start_day: u32, d: NaiveDate) -> (NaiveDate, NaiveDate) {
    let here = start_in(d.year(), d.month(), start_day);
    let (from_month, by) = if d >= here { (d, 1) } else { (d, -1) };
    let (y, m) = shift_month(from_month.year(), from_month.month(), by);
    let other = start_in(y, m, start_day);
    if d >= here { (here, other.pred_opt().expect("valid")) } else { (other, here.pred_opt().expect("valid")) }
}

/// PL-02 cut-off date inside the period: the last date carrying the cut-off day (a shorter month uses its last day).
pub fn cutoff_in(from: NaiveDate, to: NaiveDate, cutoff_day: u32) -> NaiveDate {
    let mut d = to;
    while d >= from {
        if d.day() == cutoff_day.min(days_in(d.year(), d.month())) {
            return d;
        }
        d = d.pred_opt().expect("valid");
    }
    to
}

// ---------------------------------------------------------------------------------------------------------------------
// Cycles and periods

const CYCLE_JSON: &str = "jsonb_build_object('id', c.id, 'owner_kind', c.owner_kind, 'owner_id', c.owner_id, 'frequency', c.frequency,
    'start_kind', c.start_kind, 'custom_start_day', c.custom_start_day, 'cutoff_day', c.cutoff_day, 'valid_from', lower(c.valid),
    'valid_to', upper(c.valid) - 1, 'pending_approval', false)";

pub async fn list_cycles(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!("SELECT {CYCLE_JSON} FROM period.attendance_cycle c ORDER BY lower(c.valid)")).fetch_all(&mut ***tx).await?)
}

pub struct NewCycle {
    pub owner_kind: String,
    pub owner_id: Uuid,
    pub frequency: String,
    pub start_kind: String,
    pub custom_start_day: Option<i16>,
    pub cutoff_day: Option<i16>,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub reason: Option<String>,
}

/// PL-01, PL-02. Creating a cycle generates its periods up to today (SM-PERIOD P1).
pub async fn create_cycle(tx: &mut OrgTx<'_>, actor: Uuid, c: NewCycle) -> Result<Value, DomainError> {
    let bad = |m: &str| Err(DomainError::Validation(m.into()));
    if c.frequency != "monthly" {
        return bad("only monthly periods are offered in V1 (AC-C12-01.3)");
    }
    if c.owner_kind != "organisation" {
        return bad("cycles are owned by the organisation until DP-01 decides branch or policy-set cycles");
    }
    let org: Uuid = sqlx::query_scalar("SELECT app.current_org()").fetch_one(&mut ***tx).await?;
    if c.owner_id != org {
        return bad("owner_id must be the organisation's id when owner_kind = organisation");
    }
    match (c.start_kind.as_str(), c.custom_start_day) {
        ("calendar", None) | ("custom", Some(1..=31)) => {}
        ("calendar", Some(_)) => return bad("custom_start_day is set only when start_kind = custom"),
        ("custom", _) => return bad("custom_start_day (1-31) is required when start_kind = custom"),
        _ => return bad("start_kind must be calendar or custom"),
    }
    if c.cutoff_day.is_some_and(|d| !(1..=31).contains(&d)) {
        return bad("cutoff_day must be 1-31");
    }
    if c.valid_to.is_some_and(|t| t < c.valid_from) {
        return bad("valid_to is before valid_from");
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO period.attendance_cycle (org_id, owner_kind, owner_id, frequency, start_kind, custom_start_day, cutoff_day, valid, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, daterange($7, $8 + 1), $9, $9) RETURNING id",
    )
    .bind(&c.owner_kind)
    .bind(c.owner_id)
    .bind(&c.frequency)
    .bind(&c.start_kind)
    .bind(c.custom_start_day)
    .bind(c.cutoff_day)
    .bind(c.valid_from)
    .bind(c.valid_to)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await
    .map_err(|e| match DomainError::from(e) {
        DomainError::Db(e) if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23P01") => {
            DomainError::Validation("the organisation already has a cycle in that date range".into())
        }
        other => other,
    })?;
    db::audit(tx, actor, "period.cycle_created", "attendance_cycle", Some(id), Some(json!({ "reason": c.reason }))).await?;
    let today = org_today(tx).await?;
    generate(tx, id, today).await?;
    Ok(sqlx::query_scalar(&format!("SELECT {CYCLE_JSON} FROM period.attendance_cycle c WHERE c.id = $1")).bind(id).fetch_one(&mut ***tx).await?)
}

/// P1: every period of the cycle from its start up to the one containing `today` (clipped to the cycle's range).
async fn generate(tx: &mut OrgTx<'_>, cycle: Uuid, today: NaiveDate) -> Result<u32, DomainError> {
    let (start_day, cutoff, from, to): (Option<i16>, Option<i16>, NaiveDate, Option<NaiveDate>) = sqlx::query_as(
        "SELECT custom_start_day, cutoff_day, lower(valid), upper(valid) - 1 FROM period.attendance_cycle WHERE id = $1",
    )
    .bind(cycle)
    .fetch_one(&mut ***tx)
    .await?;
    let start_day = start_day.map_or(1, |d| d as u32);
    let mut made = 0;
    let mut d = from;
    while d <= today && to.is_none_or(|t| d <= t) {
        let (s, e) = bounds(start_day, d);
        let (s, e) = (s.max(from), to.map_or(e, |t| e.min(t)));
        let cut = cutoff.map(|c| cutoff_in(s, e, c as u32));
        let id: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO period.attendance_period (org_id, cycle_id, period_start, period_end, cutoff_date)
             VALUES (app.current_org(), $1, $2, $3, $4) ON CONFLICT DO NOTHING RETURNING id",
        )
        .bind(cycle)
        .bind(s)
        .bind(e)
        .bind(cut)
        .fetch_optional(&mut ***tx)
        .await?;
        if let Some(id) = id {
            sqlx::query("INSERT INTO period.period_state_history (org_id, period_id, from_state, to_state) VALUES (app.current_org(), $1, NULL, 'open')")
                .bind(id)
                .execute(&mut ***tx)
                .await?;
            made += 1;
        }
        d = e + Duration::days(1);
    }
    Ok(made)
}

/// J7 period housekeeping: missing periods generated; Open -> Past cut-off -> Awaiting lock by the organisation's date
/// (P1-P3). Idempotent: a second run changes nothing.
pub async fn housekeeping(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let today = org_today(tx).await?;
    let cycles: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM period.attendance_cycle ORDER BY lower(valid)").fetch_all(&mut ***tx).await?;
    let mut generated = 0;
    for c in cycles {
        generated += generate(tx, c, today).await?;
    }
    let mut moved = Map::new();
    for (to, from_states, due) in [
        ("awaiting_lock", "'open','past_cut_off'", "period_end < $1"),
        ("past_cut_off", "'open'", "cutoff_date < $1"),
    ] {
        let n: i64 = sqlx::query_scalar(&format!(
            "WITH due AS (SELECT id, state FROM period.attendance_period WHERE state IN ({from_states}) AND {due} FOR UPDATE),
                  moved AS (UPDATE period.attendance_period p SET state = '{to}', updated_at = now() FROM due WHERE p.id = due.id RETURNING p.id, due.state AS from_state),
                  logged AS (INSERT INTO period.period_state_history (org_id, period_id, from_state, to_state)
                             SELECT app.current_org(), id, from_state, '{to}' FROM moved RETURNING 1)
             SELECT count(*) FROM logged"
        ))
        .bind(today)
        .fetch_one(&mut ***tx)
        .await?;
        moved.insert(to.into(), n.into());
    }
    let summary = json!({ "periods_generated": generated, "moved": moved });
    db::audit_system(tx, "job.j7_period_housekeeping", "organisation", None, Some(summary.clone())).await?;
    Ok(summary)
}

const PERIOD_JSON: &str = "jsonb_build_object('id', p.id, 'cycle_id', p.cycle_id, 'period_start', p.period_start, 'period_end', p.period_end,
    'cutoff_date', p.cutoff_date, 'state', p.state, 'locked_at', p.locked_at, 'locked_by', p.locked_by,
    'state_history', coalesce((SELECT jsonb_agg(jsonb_build_object('from_state', h.from_state, 'to_state', h.to_state, 'changed_by', h.changed_by,
        'approved_by', h.approved_by, 'reason', h.reason, 'created_at', h.created_at) ORDER BY h.created_at, h.id)
      FROM period.period_state_history h WHERE h.period_id = p.id), '[]'))";

pub struct PeriodFilter {
    pub cycle_id: Option<Uuid>,
    pub state: Option<String>,
    pub contains_date: Option<NaiveDate>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// Newest first (AC-C12-01.4).
pub async fn list_periods(tx: &mut OrgTx<'_>, f: &PeriodFilter) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {PERIOD_JSON} FROM period.attendance_period p
         WHERE ($1::uuid IS NULL OR p.cycle_id = $1) AND ($2::text IS NULL OR p.state = $2)
           AND ($3::date IS NULL OR $3 BETWEEN p.period_start AND p.period_end)
           AND ($4::date IS NULL OR p.period_end >= $4) AND ($5::date IS NULL OR p.period_start <= $5)
         ORDER BY p.period_start DESC"
    ))
    .bind(f.cycle_id)
    .bind(&f.state)
    .bind(f.contains_date)
    .bind(f.from)
    .bind(f.to)
    .fetch_all(&mut ***tx)
    .await?)
}

pub async fn get_period(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {PERIOD_JSON} FROM period.attendance_period p WHERE p.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

// ---------------------------------------------------------------------------------------------------------------------
// Settings PL-03..07, stored under `period` in the organisation-settings versions (DP-10)

fn defaults() -> Map<String, Value> {
    json!({ "after_cutoff_days": "assumed_present", "lock_mode": "manual", "auto_lock_day": null, "pending_at_lock": "block",
            "unlock": "never", "adjustment_past_periods": 3 })
    .as_object()
    .cloned()
    .expect("object")
}

/// The settings in force today, with the version's creation time as `updated_at` (the ETag).
pub async fn settings(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let today = org_today(tx).await?;
    let row: Option<(Value, String)> = sqlx::query_as(
        "SELECT coalesce(settings->'period', '{}'), to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"')
         FROM org.organisation_settings_version WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(today)
    .fetch_optional(&mut ***tx)
    .await?;
    let mut s = defaults();
    if let Some((Value::Object(stored), _)) = &row {
        s.extend(stored.clone());
    }
    s.insert("pending_approval".into(), false.into());
    s.insert("updated_at".into(), row.map_or(Value::Null, |r| r.1.into()));
    Ok(Value::Object(s))
}

/// PATCH: a new organisation-settings version from today that keeps every other key (append-only versions).
pub async fn change_settings(tx: &mut OrgTx<'_>, actor: Uuid, changes: Map<String, Value>, reason: &str) -> Result<Value, DomainError> {
    let bad = |m: String| Err(DomainError::Validation(m));
    let allowed: [(&str, &[&str]); 4] = [
        ("after_cutoff_days", &["assumed_present", "held_until_actuals"]),
        ("lock_mode", &["manual", "automatic"]),
        ("pending_at_lock", &["block", "allow_as_absent", "allow_as_present"]),
        ("unlock", &["never", "with_org_admin_approval"]),
    ];
    for (k, v) in &changes {
        let ok = match k.as_str() {
            "auto_lock_day" => v.is_null() || v.as_i64().is_some_and(|d| (1..=31).contains(&d)),
            "adjustment_past_periods" => v.as_i64().is_some_and(|d| (0..=24).contains(&d)),
            _ => allowed.iter().find(|(n, _)| n == k).is_some_and(|(_, vals)| v.as_str().is_some_and(|s| vals.contains(&s))),
        };
        if !ok {
            return bad(format!("{k} has a value that is not allowed"));
        }
    }
    let today = org_today(tx).await?;
    let prev: Option<(NaiveDate, Value, i32)> = sqlx::query_as(
        "SELECT effective_from, settings, schema_version FROM org.organisation_settings_version WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(today)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((from, mut doc, schema_version)) = prev else {
        return bad("set up the organisation settings first (POST /v1/organisation-settings/versions)".into());
    };
    if from == today {
        return Err(DomainError::VersionFrozen); // versions are append-only: one change per day
    }
    // A later version already exists and would silently bring the old period settings back on its date.
    let later: Option<NaiveDate> = sqlx::query_scalar("SELECT min(effective_from) FROM org.organisation_settings_version WHERE effective_from > $1")
        .bind(today)
        .fetch_one(&mut ***tx)
        .await?;
    if let Some(d) = later {
        return Err(DomainError::IllegalTransition(format!("an organisation-settings version from {d} is already scheduled; change the period settings after it takes effect")));
    }
    let mut period = doc.get("period").and_then(Value::as_object).cloned().unwrap_or_default();
    period.extend(changes);
    let merged: Map<String, Value> = defaults().into_iter().map(|(k, v)| (k.clone(), period.get(&k).cloned().unwrap_or(v))).collect();
    if merged["lock_mode"] == "automatic" && merged["auto_lock_day"].is_null() {
        return bad("auto_lock_day is required when lock_mode = automatic".into());
    }
    doc["period"] = Value::Object(merged);
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(today)
    .bind(&doc)
    .bind(schema_version)
    .bind(reason)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "period.settings_changed", "organisation_settings_version", Some(id), Some(json!({ "period": doc["period"], "reason": reason }))).await?;
    settings(tx).await
}

// ---------------------------------------------------------------------------------------------------------------------
// Scope: the caller's employees (branch / department scope), as SQL array filters

/// (branches, departments): both None = everyone; otherwise an employee matches through either list.
pub type ScopeFilter = (Option<Vec<Uuid>>, Option<Vec<Uuid>>);

/// Employees employed at some point of [from, to] and in scope on `to`.
async fn employees(tx: &mut OrgTx<'_>, from: NaiveDate, to: NaiveDate, scope: &ScopeFilter) -> Result<Vec<(Uuid, String, NaiveDate, NaiveDate)>, DomainError> {
    Ok(sqlx::query_as(
        "SELECT e.id, e.employee_code, greatest(e.joining_date, $1), least(coalesce(e.exit_date, $2), $2)
         FROM workforce.employee e
         WHERE e.joining_date <= $2 AND (e.exit_date IS NULL OR e.exit_date >= $1)
           AND (($3::uuid[] IS NULL AND $4::uuid[] IS NULL) OR EXISTS (
                 SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.valid @> least(coalesce(e.exit_date, $2), $2)
                   AND ((a.kind = 'master_branch' AND a.branch_id = ANY($3)) OR (a.kind = 'department' AND a.list_entry_id = ANY($4)))))
         ORDER BY e.employee_code",
    )
    .bind(from)
    .bind(to)
    .bind(&scope.0)
    .bind(&scope.1)
    .fetch_all(&mut ***tx)
    .await?)
}

async fn bounds_of(tx: &mut OrgTx<'_>, period: Uuid) -> Result<(String, NaiveDate, NaiveDate, Option<Value>), DomainError> {
    sqlx::query_as("SELECT state, period_start, period_end, checklist_snapshot FROM period.attendance_period WHERE id = $1")
        .bind(period)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

// ---------------------------------------------------------------------------------------------------------------------
// Checklist (PL-R2)

/// Live checklist items for the employees given.
async fn live_items(tx: &mut OrgTx<'_>, from: NaiveDate, to: NaiveDate, emps: &[Uuid]) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('kind', i.kind, 'employee_id', i.employee_id, 'employee_code', e.employee_code, 'date', i.date,
                                   'request_type', i.request_type, 'request_id', i.request_id)
         FROM (
           SELECT 'pending_request' AS kind, q.employee_id, min(d.leave_date) AS date, 'leave' AS request_type, q.id AS request_id
             FROM leave.leave_request q JOIN leave.leave_request_day d ON d.leave_request_id = q.id
            WHERE q.state IN ('pending_l1','pending_l2') AND d.leave_date BETWEEN $1 AND $2 GROUP BY q.employee_id, q.id
           UNION
           SELECT 'pending_request', q.employee_id, q.attendance_date, 'regularisation', q.id FROM attendance.regularisation_request q
            WHERE q.state IN ('pending_l1','pending_l2') AND q.attendance_date BETWEEN $1 AND $2
           UNION
           -- An absence still inside its DS-10 waiting days is unsettled too (SM-DAY A7).
           SELECT 'flagged_day', d.employee_id, d.attendance_date, NULL, NULL FROM attendance.attendance_day d
            WHERE d.processing_state IN ('exception','absence_pending') AND d.attendance_date BETWEEN $1 AND $2
           UNION
           -- A raised punch flag belongs to the attendance day whose result uses the punch (a night shift's morning
           -- punch is the previous day's); a punch no result uses yet falls back to its India date.
           SELECT 'flagged_day', x.employee_id, x.date, 'punch_flag', x.id FROM (
             SELECT p.employee_id, f.id, coalesce(
                      (SELECT ad.attendance_date FROM attendance.day_result_punch rp
                         JOIN attendance.attendance_day ad ON ad.current_result_id = rp.result_id WHERE rp.raw_punch_id = p.id LIMIT 1),
                      (p.event_at AT TIME ZONE 'Asia/Kolkata')::date) AS date
               FROM timecap.punch_flag f JOIN timecap.raw_punch p ON p.id = f.punch_id WHERE f.state = 'raised') x
            WHERE x.date BETWEEN $1 AND $2
           UNION
           SELECT 'unresolved_missed_punch', d.employee_id, d.attendance_date, NULL, NULL
             FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
            WHERE 'missed_punch' = ANY(r.flags) AND d.attendance_date BETWEEN $1 AND $2
         ) i JOIN workforce.employee e ON e.id = i.employee_id
         WHERE i.employee_id = ANY($3)
         ORDER BY i.date, e.employee_code, i.kind, i.request_id",
    )
    .bind(from)
    .bind(to)
    .bind(emps)
    .fetch_all(&mut ***tx)
    .await?)
}

/// GET checklist: live for an unlocked period, the snapshot stored at lock for a locked one; always within scope.
pub async fn checklist(tx: &mut OrgTx<'_>, period: Uuid, scope: &ScopeFilter, kind: Option<&str>) -> Result<Value, DomainError> {
    let (state, from, to, snapshot) = bounds_of(tx, period).await?;
    let emps: Vec<Uuid> = employees(tx, from, to, scope).await?.into_iter().map(|e| e.0).collect();
    let is_snapshot = state == "locked";
    let all = match snapshot {
        Some(Value::Array(items)) if is_snapshot => items,
        _ => live_items(tx, from, to, &emps).await?,
    };
    let in_scope = |i: &Value| i["employee_id"].as_str().and_then(|s| s.parse().ok()).is_some_and(|e: Uuid| emps.contains(&e));
    let items: Vec<Value> = all.into_iter().filter(|i| in_scope(i)).collect();
    let count = |k: &str| items.iter().filter(|i| i["kind"] == k).count();
    let counts = json!({ "pending_request": count("pending_request"), "flagged_day": count("flagged_day"), "unresolved_missed_punch": count("unresolved_missed_punch") });
    let items: Vec<Value> = items.into_iter().filter(|i| kind.is_none_or(|k| i["kind"] == k)).collect();
    Ok(json!({ "period_id": period, "generated_at": chrono::Utc::now(), "is_snapshot": is_snapshot, "counts": counts, "items": items, "next_cursor": null }))
}

// ---------------------------------------------------------------------------------------------------------------------
// Totals (layer 5)

/// Deduction reason line for a day effect counted in days (DOMAIN_MODEL §7).
fn reason_of(effect: &str) -> &'static str {
    match effect {
        "uninformed_absence_cut" => "uninformed_absence_extra_cut",
        "late_slab" | "early_slab" => "late_early_penalty",
        _ => "lop", // absence_lop, hours_shortfall, missed_punch
    }
}

fn days_text(cents: i64) -> String {
    format!("{}.{:02}", cents / 100, cents % 100)
}

/// How pending days are counted at lock (PL-05 allow): absent = a full LOP day, present = no deduction.
#[derive(Clone, Copy, PartialEq)]
enum Pending {
    Leave,
    Absent,
    Present,
}

/// Every (employee, date) a checklist item stands for: each day of a pending leave, the date of every other item.
async fn pending_dates(tx: &mut OrgTx<'_>, from: NaiveDate, to: NaiveDate, items: &[Value]) -> Result<Vec<(String, String)>, DomainError> {
    let mut out: Vec<(String, String)> = items
        .iter()
        .filter(|i| i["request_type"] != "leave")
        .map(|i| (i["employee_id"].as_str().unwrap_or("").to_string(), i["date"].as_str().unwrap_or("").to_string()))
        .collect();
    let leaves: Vec<Uuid> = items.iter().filter(|i| i["request_type"] == "leave").filter_map(|i| i["request_id"].as_str()?.parse().ok()).collect();
    let days: Vec<(Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT q.employee_id, d.leave_date FROM leave.leave_request q JOIN leave.leave_request_day d ON d.leave_request_id = q.id
         WHERE q.id = ANY($1) AND d.leave_date BETWEEN $2 AND $3",
    )
    .bind(&leaves)
    .bind(from)
    .bind(to)
    .fetch_all(&mut ***tx)
    .await?;
    out.extend(days.into_iter().map(|(e, d)| (e.to_string(), d.to_string())));
    out.sort();
    out.dedup();
    Ok(out)
}

/// Totals per employee from the current day results and their effects (BR-08: lines carry the result ids).
// shortcut: effects measured only in minutes (DS-05 "deduct actual minutes") are not totalled; add a minutes line when PayrollInput v1 carries one.
async fn compute(tx: &mut OrgTx<'_>, period: Uuid, from: NaiveDate, to: NaiveDate, scope: &ScopeFilter, pending: Pending, items: &[Value]) -> Result<Vec<Value>, DomainError> {
    let emps = employees(tx, from, to, scope).await?;
    let ids: Vec<Uuid> = emps.iter().map(|e| e.0).collect();
    type Row = (Uuid, NaiveDate, Uuid, i32, i32, Option<Uuid>, i64, Value);
    let rows: Vec<Row> = sqlx::query_as(
        "SELECT d.employee_id, d.attendance_date, r.id, r.late_minutes, r.early_minutes, r.costed_branch_id, (r.day_fraction_present * 100)::bigint,
                coalesce((SELECT jsonb_agg(jsonb_build_object('kind', x.effect_kind, 'days', (x.days * 100)::bigint) ORDER BY x.id)
                          FROM attendance.day_effect x WHERE x.result_id = r.id AND x.days > 0), '[]')
         FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
         WHERE d.attendance_date BETWEEN $1 AND $2 AND d.employee_id = ANY($3)
         ORDER BY d.employee_id, d.attendance_date",
    )
    .bind(from)
    .bind(to)
    .bind(&ids)
    .fetch_all(&mut ***tx)
    .await?;
    let pending_days = pending_dates(tx, from, to, items).await?;
    let is_pending = |e: Uuid, d: NaiveDate| pending != Pending::Leave && pending_days.contains(&(e.to_string(), d.to_string()));

    let mut out = Vec::with_capacity(emps.len());
    for (emp, _code, start, end) in emps {
        let mut lines: BTreeMap<&str, (i64, Vec<String>)> = BTreeMap::new();
        let (mut late, mut early) = (0i64, 0i64);
        let mut branches: BTreeMap<Uuid, i64> = BTreeMap::new();
        let mut charged_pending = vec![];
        for (_, date, result, l, e, branch, fraction, effects) in rows.iter().filter(|r| r.0 == emp) {
            if let Some(b) = branch {
                *branches.entry(*b).or_default() += fraction;
            }
            if is_pending(emp, *date) {
                charged_pending.push(*date);
                if pending == Pending::Absent {
                    let line = lines.entry("lop").or_default();
                    line.0 += 100;
                    line.1.push(format!("PL-05:{date}"));
                }
                continue;
            }
            late += i64::from(*l);
            early += i64::from(*e);
            for x in effects.as_array().into_iter().flatten() {
                let line = lines.entry(reason_of(x["kind"].as_str().unwrap_or(""))).or_default();
                line.0 += x["days"].as_i64().unwrap_or(0);
                line.1.push(result.to_string());
            }
        }
        // A pending day without any result yet (e.g. no policy) still counts under PL-05 = absent.
        if pending == Pending::Absent {
            for (_, d) in pending_days.iter().filter(|(e, _)| *e == emp.to_string()) {
                if let Ok(d) = d.parse::<NaiveDate>() {
                    if !charged_pending.contains(&d) && !rows.iter().any(|r| r.0 == emp && r.1 == d) && (start..=end).contains(&d) {
                        charged_pending.push(d);
                        let line = lines.entry("lop").or_default();
                        line.0 += 100;
                        line.1.push(format!("PL-05:{d}"));
                    }
                }
            }
        }
        let deduction: i64 = lines.values().map(|l| l.0).sum();
        let employed = (end - start).num_days() + 1;
        let payable = (employed * 100 - deduction).max(0);
        out.push(json!({
            "employee_id": emp, "period_id": period, "basis": "provisional",
            "payable_days": days_text(payable), "deduction_days": days_text(deduction),
            "deduction_lines": lines.into_iter().map(|(r, (d, refs))| json!({ "reason": r, "days": days_text(d), "explanation_refs": refs })).collect::<Vec<_>>(),
            "late_minutes": late, "early_minutes": early, "overtime_identified_minutes": null, "pay_hold": false,
            "branch_days": branches.into_iter().map(|(b, d)| json!({ "branch_id": b, "days": days_text(d) })).collect::<Vec<_>>(),
            "frozen_at": null,
        }));
    }
    Ok(out)
}

/// GET totals: frozen at lock (the latest freeze), provisional otherwise.
pub async fn totals(tx: &mut OrgTx<'_>, period: Uuid, scope: &ScopeFilter, employee: Option<Uuid>) -> Result<Vec<Value>, DomainError> {
    let (state, from, to, _) = bounds_of(tx, period).await?;
    let items = if state == "locked" {
        let emps: Vec<Uuid> = employees(tx, from, to, scope).await?.into_iter().map(|e| e.0).collect();
        sqlx::query_scalar(
            "SELECT jsonb_build_object('employee_id', t.employee_id, 'period_id', t.period_id, 'basis', 'frozen',
                 'payable_days', to_char(t.payable_days, 'FM9990.00'), 'deduction_days', to_char(t.deduction_days, 'FM9990.00'),
                 'deduction_lines', t.deduction_lines, 'late_minutes', t.late_minutes, 'early_minutes', t.early_minutes,
                 'overtime_identified_minutes', NULL, 'pay_hold', t.pay_hold, 'branch_days', t.branch_days, 'frozen_at', t.frozen_at)
             FROM (SELECT DISTINCT ON (employee_id) * FROM period.period_totals WHERE period_id = $1 ORDER BY employee_id, frozen_at DESC) t
             JOIN workforce.employee e ON e.id = t.employee_id
             WHERE t.employee_id = ANY($2) ORDER BY e.employee_code",
        )
        .bind(period)
        .bind(&emps)
        .fetch_all(&mut ***tx)
        .await?
    } else {
        compute(tx, period, from, to, scope, Pending::Leave, &[]).await?
    };
    Ok(items.into_iter().filter(|t| employee.is_none_or(|e| t["employee_id"] == json!(e))).collect())
}

// ---------------------------------------------------------------------------------------------------------------------
// Lock (P5, P6, P8)

pub async fn lock(tx: &mut OrgTx<'_>, actor: Uuid, period: Uuid, comment: Option<&str>) -> Result<Value, DomainError> {
    let (state, from, to): (String, NaiveDate, NaiveDate) =
        sqlx::query_as("SELECT state, period_start, period_end FROM period.attendance_period WHERE id = $1 FOR UPDATE")
            .bind(period)
            .fetch_optional(&mut ***tx)
            .await?
            .ok_or(DomainError::NotFound)?;
    match state.as_str() {
        "locked" => return get_period(tx, period).await, // P6: no second effect
        "awaiting_lock" | "unlocked" => {}
        _ => return Err(DomainError::IllegalTransition("a period is locked after its end date (GAP-SM-12)".into())),
    }
    let everyone: ScopeFilter = (None, None);
    let emps: Vec<Uuid> = employees(tx, from, to, &everyone).await?.into_iter().map(|e| e.0).collect();
    let items = live_items(tx, from, to, &emps).await?;
    let mode = settings(tx).await?["pending_at_lock"].as_str().unwrap_or("block").to_string();
    let pending = match mode.as_str() {
        _ if items.is_empty() => Pending::Leave,
        "allow_as_absent" => Pending::Absent,
        "allow_as_present" => Pending::Present,
        _ => {
            return Err(DomainError::RefusedWith(
                "PENDING_ITEMS_BLOCK_LOCK",
                format!("{} checklist item(s) must be resolved before the lock (PL-05 = block)", items.len()),
                Value::Array(items),
            ))
        }
    };
    let totals = compute(tx, period, from, to, &everyone, pending, &items).await?;
    for t in &totals {
        sqlx::query(
            "INSERT INTO period.period_totals (org_id, period_id, employee_id, payable_days, deduction_days, deduction_lines, late_minutes, early_minutes,
                                             branch_days, pay_hold, created_by)
             VALUES (app.current_org(), $1, $2, $3::numeric, $4::numeric, $5, $6, $7, $8, false, $9)",
        )
        .bind(period)
        .bind(t["employee_id"].as_str().and_then(|s| s.parse::<Uuid>().ok()))
        .bind(t["payable_days"].as_str())
        .bind(t["deduction_days"].as_str())
        .bind(&t["deduction_lines"])
        .bind(t["late_minutes"].as_i64().map(|m| m as i32))
        .bind(t["early_minutes"].as_i64().map(|m| m as i32))
        .bind(&t["branch_days"])
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    }
    sqlx::query(
        "INSERT INTO period.lock_coverage (org_id, period_id, employee_id, dates, created_by)
         SELECT app.current_org(), $1, e, daterange($2, $3 + 1), $5 FROM unnest($4::uuid[]) e",
    )
    .bind(period)
    .bind(from)
    .bind(to)
    .bind(&emps)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    sqlx::query("UPDATE period.period_adjustment SET state = 'frozen', updated_at = now(), updated_by = $2 WHERE target_period_id = $1 AND state = 'posted'")
        .bind(period)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("UPDATE attendance.attendance_day SET processing_state = 'locked', updated_at = now() WHERE attendance_date BETWEEN $1 AND $2 AND employee_id = ANY($3)")
        .bind(from)
        .bind(to)
        .bind(&emps)
        .execute(&mut ***tx)
        .await?;
    sqlx::query(
        "UPDATE period.attendance_period SET state = 'locked', checklist_snapshot = $2, locked_at = now(), locked_by = $3, updated_at = now(), updated_by = $3 WHERE id = $1",
    )
    .bind(period)
    .bind(Value::Array(items.clone()))
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    sqlx::query("INSERT INTO period.period_state_history (org_id, period_id, from_state, to_state, reason, changed_by) VALUES (app.current_org(), $1, $2, 'locked', $3, $4)")
        .bind(period)
        .bind(&state)
        .bind(comment)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    let summary = json!({ "employees": emps.len(), "pending_items": items.len(), "pending_at_lock": mode });
    db::audit(tx, actor, "period.locked", "attendance_period", Some(period), Some(summary)).await?;
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), 'PeriodLocked', $1, $2)")
        .bind(period)
        .bind(json!({ "period_start": from, "period_end": to }))
        .execute(&mut ***tx)
        .await?;
    get_period(tx, period).await
}

// ---------------------------------------------------------------------------------------------------------------------
// Unlock (P7): request, approve, reject

fn unlock_json() -> String {
    format!(
        "jsonb_build_object('id', q.id, 'period_id', q.period_id, 'reason', q.reason, 'requested_by_user_id', q.requested_by, 'state', q.state,
           'escalated', q.escalated, 'decided_basis', q.decided_basis, 'current_approver_user_id', q.current_approver_user_id,
           'submitted_at', q.requested_at, 'approval_steps', {})",
        steps_sql("period_unlock", "q.id")
    )
}

pub async fn get_unlock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {} FROM period.unlock_request q WHERE q.id = $1", unlock_json()))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_unlocks(tx: &mut OrgTx<'_>, period: Uuid) -> Result<Vec<Value>, DomainError> {
    bounds_of(tx, period).await?;
    Ok(sqlx::query_scalar(&format!("SELECT {} FROM period.unlock_request q WHERE q.period_id = $1 ORDER BY q.requested_at DESC", unlock_json()))
        .bind(period)
        .fetch_all(&mut ***tx)
        .await?)
}

async fn unlock_allowed(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    if settings(tx).await?["unlock"] == "never" {
        return Err(DomainError::Refused("UNLOCK_NOT_ALLOWED", "the organisation's period settings never allow an unlock (PL-06)".into()));
    }
    Ok(())
}

pub async fn request_unlock(tx: &mut OrgTx<'_>, actor: Uuid, period: Uuid, reason: &str) -> Result<Value, DomainError> {
    let state: String = sqlx::query_scalar("SELECT state FROM period.attendance_period WHERE id = $1 FOR UPDATE")
        .bind(period)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)?;
    unlock_allowed(tx).await?;
    if reason.trim().is_empty() {
        return Err(DomainError::Validation("a reason is required (AC-C12-05.2)".into()));
    }
    if state != "locked" {
        return Err(DomainError::IllegalTransition("only a locked period can be unlocked".into()));
    }
    let pending: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.unlock_request WHERE period_id = $1 AND state IN ('pending_l1','pending_l2'))")
        .bind(period)
        .fetch_one(&mut ***tx)
        .await?;
    if pending {
        return Err(DomainError::IllegalTransition("an unlock request for this period is already pending".into()));
    }
    let id: Uuid = sqlx::query_scalar("INSERT INTO period.unlock_request (org_id, period_id, reason, requested_by) VALUES (app.current_org(), $1, $2, $3) RETURNING id")
        .bind(period)
        .bind(reason)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    step(tx, Step { kind: "period_unlock", request_id: id, level: 1, action: "submitted", requester: actor, actor: Some(actor), on_behalf_of: None, comment: Some(reason) }).await?;
    db::audit(tx, actor, "period.unlock_requested", "period_unlock_request", Some(id), Some(json!({ "period_id": period, "reason": reason }))).await?;
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), 'RequestSubmitted', $1, $2)")
        .bind(id)
        .bind(json!({ "request_type": "period_unlock", "period_id": period }))
        .execute(&mut ***tx)
        .await?;
    get_unlock(tx, id).await
}

/// Approve or reject (PER-05). The requester never decides (BR-07). Approve: lock coverage removed, every day of the
/// period re-evaluated (SM-DAY A8), period -> Unlocked. Frozen totals stay as history; a re-lock freezes new rows.
pub async fn decide_unlock(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, approve: bool, note: Option<&str>) -> Result<Value, DomainError> {
    let (period, state, requester, reason): (Uuid, String, Uuid, String) =
        sqlx::query_as("SELECT period_id, state, requested_by, reason FROM period.unlock_request WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut ***tx)
            .await?
            .ok_or(DomainError::NotFound)?;
    match (state.as_str(), approve) {
        ("approved", true) | ("rejected", false) => return get_unlock(tx, id).await,
        ("pending_l1" | "pending_l2", _) => {}
        _ => return Err(DomainError::IllegalTransition(format!("the request is already {state}"))),
    }
    if requester == actor {
        return Err(DomainError::SelfApproval);
    }
    if approve {
        unlock_allowed(tx).await?;
    }
    let to = if approve { "approved" } else { "rejected" };
    sqlx::query("UPDATE period.unlock_request SET state = $2, decided_basis = 'approver', current_approver_user_id = NULL WHERE id = $1")
        .bind(id)
        .bind(to)
        .execute(&mut ***tx)
        .await?;
    step(tx, Step { kind: "period_unlock", request_id: id, level: 1, action: to, requester, actor: Some(actor), on_behalf_of: None, comment: note }).await?;
    db::audit(tx, actor, &format!("period.unlock_{to}"), "period_unlock_request", Some(id), Some(json!({ "period_id": period, "note": note }))).await?;
    if approve {
        let (from, until): (NaiveDate, NaiveDate) = sqlx::query_as("SELECT period_start, period_end FROM period.attendance_period WHERE id = $1 FOR UPDATE")
            .bind(period)
            .fetch_one(&mut ***tx)
            .await?;
        sqlx::query("DELETE FROM period.lock_coverage WHERE period_id = $1").bind(period).execute(&mut ***tx).await?;
        // Adjustments frozen into this period may change again until it is re-locked; exported ones stay exported (OQ-C12-07).
        sqlx::query("UPDATE period.period_adjustment SET state = 'posted', updated_at = now(), updated_by = $2 WHERE target_period_id = $1 AND state = 'frozen'")
            .bind(period)
            .bind(actor)
            .execute(&mut ***tx)
            .await?;
        sqlx::query("UPDATE period.attendance_period SET state = 'unlocked', updated_at = now(), updated_by = $2 WHERE id = $1")
            .bind(period)
            .bind(actor)
            .execute(&mut ***tx)
            .await?;
        sqlx::query(
            "INSERT INTO period.period_state_history (org_id, period_id, from_state, to_state, reason, changed_by, approved_by)
             VALUES (app.current_org(), $1, 'locked', 'unlocked', $2, $3, $4)",
        )
        .bind(period)
        .bind(&reason)
        .bind(requester)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
        // shortcut: re-evaluates every day in this transaction; move to recalculation work items when periods grow large.
        let days: Vec<(Uuid, NaiveDate)> = sqlx::query_as("SELECT employee_id, attendance_date FROM attendance.attendance_day WHERE attendance_date BETWEEN $1 AND $2 ORDER BY 1, 2")
            .bind(from)
            .bind(until)
            .fetch_all(&mut ***tx)
            .await?;
        for (e, d) in days {
            crate::recalculate(tx, e, d, "period_unlock", Some(id)).await?;
        }
        // A day the engine did not revisit (not employed, no policy) leaves the locked state too.
        sqlx::query("UPDATE attendance.attendance_day SET processing_state = 'provisional', updated_at = now() WHERE processing_state = 'locked' AND attendance_date BETWEEN $1 AND $2")
            .bind(from)
            .bind(until)
            .execute(&mut ***tx)
            .await?;
        sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), 'PeriodUnlocked', $1, $2)")
            .bind(period)
            .bind(json!({ "unlock_request_id": id }))
            .execute(&mut ***tx)
            .await?;
    }
    get_unlock(tx, id).await
}

// ---------------------------------------------------------------------------------------------------------------------
// Adjustments (PL-R4, E15, SM-ADJ)

fn signed_days(cents: i64) -> String {
    format!("{}{}", if cents < 0 { "-" } else { "" }, days_text(cents.abs()))
}

/// (day status, late, early, deduction cents by reason) of one result.
async fn result_view(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(String, i64, i64, BTreeMap<&'static str, i64>), DomainError> {
    let (status, late, early, effects): (String, i32, i32, Value) = sqlx::query_as(
        "SELECT r.day_status, r.late_minutes, r.early_minutes,
                coalesce((SELECT jsonb_agg(jsonb_build_object('kind', x.effect_kind, 'days', (x.days * 100)::bigint))
                          FROM attendance.day_effect x WHERE x.result_id = r.id AND x.days > 0), '[]')
         FROM attendance.day_result r WHERE r.id = $1",
    )
    .bind(id)
    .fetch_one(&mut ***tx)
    .await?;
    let mut lines: BTreeMap<&'static str, i64> = BTreeMap::new();
    for x in effects.as_array().into_iter().flatten() {
        *lines.entry(reason_of(x["kind"].as_str().unwrap_or(""))).or_default() += x["days"].as_i64().unwrap_or(0);
    }
    Ok((status, i64::from(late), i64::from(early), lines))
}

/// SM-ADJ Q1: a changed input for a locked date posts only the difference to the first open period after the locked
/// one (PL-R4, AC-C12-06.3), tagged with the original date. The difference is measured against the date's latest
/// measured result, else the locked one (OQ-C12-06), so repeated corrections add up and the same inputs twice post
/// nothing (AC-C12-06.5). PL-07: a source more than that many periods back -> ADJUSTMENT_WINDOW_EXCEEDED (OQ-C12-04).
/// Returns the adjustment id, or None when the date is not locked or nothing changed.
pub async fn post_adjustment(tx: &mut OrgTx<'_>, actor: Option<Uuid>, employee: Uuid, date: NaiveDate, trigger: &str, trigger_ref: Option<Uuid>) -> Result<Option<Uuid>, DomainError> {
    let source: Option<(Uuid, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT p.id, p.period_start, p.period_end FROM period.lock_coverage c JOIN period.attendance_period p ON p.id = c.period_id
         WHERE c.employee_id = $1 AND c.dates @> $2::date",
    )
    .bind(employee)
    .bind(date)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((source, source_start, source_end)) = source else { return Ok(None) };
    // Locking the target row keeps a concurrent lock from freezing it while the adjustment is posted.
    let target: Option<(Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT id, period_start FROM period.attendance_period WHERE period_start > $1 AND state <> 'locked' ORDER BY period_start LIMIT 1 FOR UPDATE",
    )
    .bind(source_end)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((target, target_start)) = target else {
        return Err(DomainError::IllegalTransition("there is no open period after the locked one to post the adjustment into".into()));
    };
    let back: i64 = sqlx::query_scalar("SELECT count(*) FROM period.attendance_period WHERE period_start >= $1 AND period_start < $2")
        .bind(source_start)
        .bind(target_start)
        .fetch_one(&mut ***tx)
        .await?;
    let window = settings(tx).await?["adjustment_past_periods"].as_i64().unwrap_or(3);
    if back > window {
        return Err(DomainError::Refused(
            "ADJUSTMENT_WINDOW_EXCEEDED",
            format!("{date} is {back} periods back; the organisation allows corrections {window} periods back (PL-07)"),
        ));
    }
    let Some((previous, new)) = crate::measure_locked(tx, employee, date, trigger, trigger_ref).await? else { return Ok(None) };
    let (old_status, old_late, old_early, old_lines) = result_view(tx, previous).await?;
    let (new_status, new_late, new_early, new_lines) = result_view(tx, new).await?;
    let mut reasons: Vec<&str> = old_lines.keys().chain(new_lines.keys()).copied().collect();
    reasons.sort();
    reasons.dedup();
    let lines: Vec<Value> = reasons
        .iter()
        .map(|r| (*r, new_lines.get(r).copied().unwrap_or(0) - old_lines.get(r).copied().unwrap_or(0)))
        .filter(|(_, d)| *d != 0)
        .map(|(r, d)| json!({ "reason": r, "days": signed_days(d), "explanation_refs": [new.to_string()] }))
        .collect();
    let deduction_change: i64 = new_lines.values().sum::<i64>() - old_lines.values().sum::<i64>();
    if lines.is_empty() && old_status == new_status && old_late == new_late && old_early == new_early {
        return Ok(None); // the inputs changed, the outcome did not
    }
    let difference = json!({
        "day_status": { "from": old_status, "to": new_status },
        "late_minutes": new_late - old_late, "early_minutes": new_early - old_early,
        "deduction_lines": lines, "measured_against_result_id": previous,
    });
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO period.period_adjustment (org_id, employee_id, original_date, source_period_id, locked_result_id, new_result_id, difference, days,
                                               target_period_id, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7::numeric, $8, $9, $9) RETURNING id",
    )
    .bind(employee)
    .bind(date)
    .bind(source)
    .bind(previous)
    .bind(new)
    .bind(&difference)
    .bind(signed_days(-deduction_change)) // payable days move opposite to deductions
    .bind(target)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    match actor {
        Some(a) => db::audit(tx, a, "period.adjustment_posted", "period_adjustment", Some(id), Some(difference.clone())).await?,
        None => db::audit_system(tx, "period.adjustment_posted", "period_adjustment", Some(id), Some(difference.clone())).await?,
    }
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), 'AdjustmentPosted', $1, $2)")
        .bind(id)
        .bind(json!({ "employee_id": employee, "original_date": date, "target_period_id": target }))
        .execute(&mut ***tx)
        .await?;
    Ok(Some(id))
}

const ADJUSTMENT_JSON: &str = "jsonb_build_object('id', a.id, 'employee_id', a.employee_id, 'original_date', a.original_date,
    'source_period_id', a.source_period_id, 'target_period_id', a.target_period_id, 'locked_result_id', a.locked_result_id,
    'new_result_id', a.new_result_id, 'days', to_char(a.days, 'FM9990.00'), 'difference', a.difference, 'state', a.state,
    'created_at', a.created_at, 'explanation', (SELECT r.explanation FROM attendance.day_result r WHERE r.id = a.new_result_id))";

pub struct AdjustmentFilter {
    pub target_period_id: Option<Uuid>,
    pub source_period_id: Option<Uuid>,
    pub employee_id: Option<Uuid>,
    pub state: Option<String>,
}

/// GET adjustments, for employees in scope on their original date.
pub async fn list_adjustments(tx: &mut OrgTx<'_>, f: &AdjustmentFilter, scope: &ScopeFilter) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {ADJUSTMENT_JSON} FROM period.period_adjustment a
         WHERE ($1::uuid IS NULL OR a.target_period_id = $1) AND ($2::uuid IS NULL OR a.source_period_id = $2)
           AND ($3::uuid IS NULL OR a.employee_id = $3) AND ($4::text IS NULL OR a.state = $4)
           AND (($5::uuid[] IS NULL AND $6::uuid[] IS NULL) OR EXISTS (
                 SELECT 1 FROM workforce.employee_assignment s WHERE s.employee_id = a.employee_id AND s.valid @> a.original_date
                   AND ((s.kind = 'master_branch' AND s.branch_id = ANY($5)) OR (s.kind = 'department' AND s.list_entry_id = ANY($6)))))
         ORDER BY a.original_date, a.created_at"
    ))
    .bind(f.target_period_id)
    .bind(f.source_period_id)
    .bind(f.employee_id)
    .bind(&f.state)
    .bind(&scope.0)
    .bind(&scope.1)
    .fetch_all(&mut ***tx)
    .await?)
}

// ---------------------------------------------------------------------------------------------------------------------
// PayrollInput v1 (MD-06, DOMAIN_MODEL §7)

/// The `PayrollInput v1` document for a locked period and the employees in scope, with the ids of the adjustments it
/// carries. Built only from frozen data, so exporting twice gives the same employees section (AC-C12-07.3).
pub async fn payroll_input(tx: &mut OrgTx<'_>, period: Uuid, scope: &ScopeFilter, export_id: Uuid, produced_at: chrono::DateTime<chrono::Utc>) -> Result<(Value, Vec<Uuid>), DomainError> {
    let (state, from, to, _) = bounds_of(tx, period).await?;
    if state != "locked" {
        return Err(DomainError::IllegalTransition("the payroll input is produced from a locked period (OQ-C12-11)".into()));
    }
    let totals = totals(tx, period, scope, None).await?;
    let adjustments = list_adjustments(
        tx,
        &AdjustmentFilter { target_period_id: Some(period), source_period_id: None, employee_id: None, state: None },
        scope,
    )
    .await?;
    let mut people: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for t in totals {
        let mut e = Map::new();
        for k in ["employee_id", "payable_days", "deduction_days", "deduction_lines", "late_minutes", "early_minutes", "overtime_identified_minutes", "pay_hold", "branch_days"] {
            e.insert(k.into(), t[k].clone());
        }
        e.insert("adjustments".into(), json!([]));
        people.insert(t["employee_id"].as_str().unwrap_or("").to_string(), e);
    }
    let mut ids = vec![];
    for a in &adjustments {
        let emp = a["employee_id"].as_str().unwrap_or("").to_string();
        // Someone who left in the earlier period still receives their arrear here, with no totals of their own.
        let e = people.entry(emp.clone()).or_insert_with(|| {
            json!({ "employee_id": emp, "payable_days": "0.00", "deduction_days": "0.00", "deduction_lines": [], "late_minutes": 0, "early_minutes": 0,
                    "overtime_identified_minutes": null, "pay_hold": false, "branch_days": [], "adjustments": [] })
            .as_object()
            .cloned()
            .expect("object")
        });
        let d = &a["difference"];
        e["adjustments"].as_array_mut().expect("array").push(json!({
            "adjustment_id": a["id"], "original_date": a["original_date"], "source_period_id": a["source_period_id"], "days": a["days"],
            "deduction_lines": d["deduction_lines"], "late_minutes": d["late_minutes"], "early_minutes": d["early_minutes"],
            "explanation_refs": [a["new_result_id"]],
        }));
        if let Some(id) = a["id"].as_str().and_then(|s| s.parse().ok()) {
            ids.push(id);
        }
    }
    let codes: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, employee_code FROM workforce.employee WHERE id::text = ANY($1)")
        .bind(people.keys().cloned().collect::<Vec<_>>())
        .fetch_all(&mut ***tx)
        .await?;
    let mut employees: Vec<Value> = people
        .into_values()
        .map(|mut e| {
            let id = e["employee_id"].as_str().unwrap_or("").to_string();
            let code = codes.iter().find(|c| c.0.to_string() == id).map_or(String::new(), |c| c.1.clone());
            e.insert("employee_code".into(), code.into());
            let refs: Vec<Value> = e["deduction_lines"].as_array().into_iter().flatten().flat_map(|l| l["explanation_refs"].as_array().cloned().unwrap_or_default()).collect();
            e.insert("explanation_refs".into(), refs.into());
            Value::Object(e)
        })
        .collect();
    employees.sort_by(|a, b| a["employee_code"].as_str().cmp(&b["employee_code"].as_str()));
    let doc = json!({
        "contract_version": "PayrollInput v1", "export_id": export_id, "produced_at": produced_at,
        "period": { "id": period, "period_start": from, "period_end": to }, "employees": employees,
    });
    Ok((doc, ids))
}

/// Records an export once its file is stored: the export row, its adjustments (SM-ADJ Q3: frozen -> exported).
pub async fn record_export(tx: &mut OrgTx<'_>, actor: Uuid, export_id: Uuid, period: Uuid, path: &str, checksum: &str, adjustments: &[Uuid]) -> Result<Value, DomainError> {
    sqlx::query(
        "INSERT INTO period.payroll_input_export (org_id, id, period_id, file_path, checksum, produced_by, created_by) VALUES (app.current_org(), $1, $2, $3, $4, $5, $5)",
    )
    .bind(export_id)
    .bind(period)
    .bind(path)
    .bind(checksum)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    sqlx::query("INSERT INTO period.payroll_input_export_adjustment (org_id, export_id, adjustment_id, created_by) SELECT app.current_org(), $1, a, $3 FROM unnest($2::uuid[]) a")
        .bind(export_id)
        .bind(adjustments)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("UPDATE period.period_adjustment SET state = 'exported', updated_at = now(), updated_by = $2 WHERE id = ANY($1) AND state = 'frozen'")
        .bind(adjustments)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "period.payroll_input_exported", "payroll_input_export", Some(export_id), Some(json!({ "period_id": period, "checksum": checksum, "adjustments": adjustments.len() }))).await?;
    get_export(tx, export_id).await
}

const EXPORT_JSON: &str = "jsonb_build_object('id', x.id, 'period_id', x.period_id, 'contract_version', x.contract_version, 'produced_by', x.produced_by,
    'produced_at', x.produced_at, 'checksum', x.checksum,
    'adjustment_ids', coalesce((SELECT jsonb_agg(j.adjustment_id ORDER BY j.adjustment_id) FROM period.payroll_input_export_adjustment j WHERE j.export_id = x.id), '[]'))";

pub async fn get_export(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {EXPORT_JSON} FROM period.payroll_input_export x WHERE x.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_exports(tx: &mut OrgTx<'_>, period: Uuid) -> Result<Vec<Value>, DomainError> {
    bounds_of(tx, period).await?;
    Ok(sqlx::query_scalar(&format!("SELECT {EXPORT_JSON} FROM period.payroll_input_export x WHERE x.period_id = $1 ORDER BY x.produced_at DESC"))
        .bind(period)
        .fetch_all(&mut ***tx)
        .await?)
}

/// The stored file of an export (TB-02: the path starts with the organisation id).
pub async fn export_path(tx: &mut OrgTx<'_>, id: Uuid) -> Result<String, DomainError> {
    sqlx::query_scalar("SELECT file_path FROM period.payroll_input_export WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    #[test]
    fn monthly_bounds_and_cutoff() {
        assert_eq!(bounds(1, d("2026-02-14")), (d("2026-02-01"), d("2026-02-28")));
        assert_eq!(bounds(26, d("2026-10-28")), (d("2026-10-26"), d("2026-11-25")));
        assert_eq!(bounds(26, d("2026-11-25")), (d("2026-10-26"), d("2026-11-25")));
        assert_eq!(bounds(26, d("2026-01-03")), (d("2025-12-26"), d("2026-01-25")));
        // A start day beyond February's length uses its last day.
        assert_eq!(bounds(31, d("2026-03-10")), (d("2026-02-28"), d("2026-03-30")));
        assert_eq!(cutoff_in(d("2026-10-26"), d("2026-11-25"), 20), d("2026-11-20"));
        assert_eq!(cutoff_in(d("2026-02-01"), d("2026-02-28"), 31), d("2026-02-28"));
    }

    #[test]
    fn reason_lines() {
        assert_eq!((reason_of("absence_lop"), reason_of("uninformed_absence_cut"), reason_of("late_slab")), ("lop", "uninformed_absence_extra_cut", "late_early_penalty"));
        assert_eq!((days_text(150), signed_days(-100), signed_days(-25)), ("1.50".into(), "-1.00".into(), "-0.25".into()));
    }
}
