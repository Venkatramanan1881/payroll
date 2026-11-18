//! Attendance days: gathers each day's inputs, runs the engine and stores an append-only, explained result (C06).

use chrono::{DateTime, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use domain_timecap::punch::{branch_tz, master_branch_on};
use engine::day::{compute_day, fingerprint, DayInput, Effect, Expected, LeaveCover, PunchIn, ENGINE_VERSION};
use engine::settings::{AttendanceSettings, GROUPS};
use serde_json::{json, Value};
use uuid::Uuid;

pub mod approval;
pub mod leave;
pub mod regularisation;

/// Settings of a policy version for the engine. A switched-off LA, EC or BK group evaluates nothing (G2, BR-14);
/// the other groups cannot be "off" for a day, so their defaults apply.
pub async fn load_settings(tx: &mut OrgTx<'_>, version: Uuid) -> Result<AttendanceSettings, DomainError> {
    let rows: Vec<(String, bool, Value)> =
        sqlx::query_as("SELECT rule_group, enabled, settings FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = ANY($2)")
            .bind(version)
            .bind(GROUPS.to_vec())
            .fetch_all(&mut ***tx)
            .await?;
    let mut s = AttendanceSettings::default();
    for (group, enabled, doc) in rows {
        if enabled {
            // A stored document saved before typed validation must not block punches: that group keeps its defaults.
            // shortcut: no visible warning on the day yet; surface it when policy health checks are built.
            let mut typed = s.clone();
            if typed.apply(&group, &doc).is_ok() {
                s = typed;
            }
        } else {
            match group.as_str() {
                "LA" => s.late_on = false,
                "EC" => s.early_on = false,
                "BK" => s.break_deduct = false,
                _ => {}
            }
        }
    }
    Ok(s)
}

/// start, end, window before, window after, cut-off, required minutes, shift type.
type ShiftRow = (Option<DateTime<Utc>>, Option<DateTime<Utc>>, i32, i32, Option<DateTime<Utc>>, i32, String);

/// What one date looks like for an employee, as instants in the schedule's zone (BR-11).
struct Shape {
    expected: Expected,
    shift_version: Option<Uuid>,
    /// The day's own punch window before its neighbours clip it (SH-07 for a timed shift, else the local day).
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    cutoff_at: DateTime<Utc>,
    /// Shift start and end when the day has set times.
    times: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

async fn local_bounds(tx: &mut OrgTx<'_>, date: NaiveDate, tz: &str) -> Result<(DateTime<Utc>, DateTime<Utc>, DateTime<Utc>), DomainError> {
    Ok(sqlx::query_as(
        "SELECT ($1::date::timestamp AT TIME ZONE $2), (($1::date + 1)::timestamp AT TIME ZONE $2), ((($1::date + 1) + time '02:30') AT TIME ZONE $2)",
    )
    .bind(date)
    .bind(tz)
    .fetch_one(&mut ***tx)
    .await?)
}

async fn shape(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, tz: &str, holiday_branch: Uuid) -> Result<Shape, DomainError> {
    let (day_from, day_to, plain_cutoff) = local_bounds(tx, date, tz).await?;
    let local_day = |expected, shift_version| Shape { expected, shift_version, from: day_from, to: day_to, cutoff_at: plain_cutoff, times: None };
    let day = domain_sched::schedule::expected_day(tx, employee, date, holiday_branch).await?;
    if day.holiday.is_some() {
        return Ok(local_day(Expected::Holiday, None));
    }
    if day.weekly_off {
        return Ok(local_day(Expected::WeeklyOff, None));
    }
    let Some(version) = day.shift_version_id else { return Ok(local_day(Expected::Unscheduled, None)) };
    // A shift crossing midnight ends the next day (SH-06 default: dated by its start); D-08: the missed-punch cut-off is
    // that time of day on the calendar day after the shift's end date.
    let row: ShiftRow = sqlx::query_as(
        "SELECT (($2::date + v.start_time) AT TIME ZONE $3),
                (($2::date + v.end_time + CASE WHEN v.crosses_midnight THEN interval '1 day' ELSE interval '0' END) AT TIME ZONE $3),
                v.window_before_min, v.window_after_min,
                (($2::date + CASE WHEN v.crosses_midnight THEN 2 ELSE 1 END + v.missed_punch_cutoff) AT TIME ZONE $3),
                coalesce(v.required_minutes, 480), v.shift_type
         FROM sched.shift_version v WHERE v.id = $1",
    )
    .bind(version)
    .bind(date)
    .bind(tz)
    .fetch_one(&mut ***tx)
    .await?;
    let (start, end, before, after, cutoff, required, kind) = row;
    let required_min = i64::from(required);
    Ok(match (start, end, cutoff) {
        (Some(start_at), Some(end_at), Some(cutoff_at)) => Shape {
            expected: Expected::Working { start_at, end_at, required_min, flexible: matches!(kind.as_str(), "flexible" | "flexible_core" | "open") },
            shift_version: Some(version),
            from: start_at - chrono::Duration::minutes(before.into()),
            to: end_at + chrono::Duration::minutes(after.into()),
            cutoff_at,
            times: Some((start_at, end_at)),
        },
        // An open or band-less shift: hours only, over the local day (DS-01 b).
        _ => local_day(Expected::HoursOnly { required_min }, Some(version)),
    })
}

/// Where the punches of `a` (the earlier day) stop and those of `b` start, so no punch counts on two days.
/// Two timed shifts split their overlap halfway between a's end and b's start (OQ-C05-04 open; this keeps it deterministic).
fn split(a: &Shape, b: &Shape) -> DateTime<Utc> {
    match (a.times, b.times) {
        (Some((_, a_end)), Some((b_start, _))) => a_end + (b_start - a_end) / 2,
        (Some(_), None) => a.to,
        (None, _) => b.from,
    }
}

/// Recalculates one employee's day (BR-03). Returns the new result id, or None when nothing was written:
/// unchanged inputs (BR-02, SM-DAY A11), a locked date (BR-06; adjustments come with period close), a future date,
/// not employed, nothing expected and nothing punched, or no policy version in force (the day is marked `exception`).
pub async fn recalculate(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, trigger_kind: &str, trigger_ref: Option<Uuid>) -> Result<Option<Uuid>, DomainError> {
    if db::employee_locked(tx, employee, date, date.succ_opt()).await? {
        return Ok(None);
    }
    let Some(master) = master_branch_on(tx, employee, date).await? else { return Ok(None) };
    let noon = date.and_hms_opt(12, 0, 0).expect("valid time").and_utc();
    let master_tz = branch_tz(tx, master, noon).await?;
    let future: bool = sqlx::query_scalar("SELECT $1::date > (now() AT TIME ZONE $2)::date").bind(date).bind(&master_tz).fetch_one(&mut ***tx).await?;
    if future {
        return Ok(None); // a day not yet lived has no result (no absence for tomorrow)
    }

    let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
    let Some(policy_version) = resolved.and_then(|r| r.version_id) else {
        sqlx::query("INSERT INTO attendance.attendance_day (org_id, employee_id, attendance_date, processing_state) VALUES (app.current_org(), $1, $2, 'exception') ON CONFLICT DO NOTHING")
            .bind(employee)
            .bind(date)
            .execute(&mut ***tx)
            .await?;
        sqlx::query("UPDATE attendance.attendance_day SET processing_state = 'exception', updated_at = now() WHERE employee_id = $1 AND attendance_date = $2 AND current_result_id IS NULL")
            .bind(employee)
            .bind(date)
            .execute(&mut ***tx)
            .await?;
        return Ok(None);
    };
    let settings = load_settings(tx, policy_version).await?;

    // D-04: the branch where the employee punched decides the holiday calendar and the zone; else the master branch.
    let (day_from, day_to, _) = local_bounds(tx, date, &master_tz).await?;
    let punch_branch: Option<Uuid> = sqlx::query_scalar(
        "SELECT branch_id FROM timecap.raw_punch WHERE employee_id = $1 AND event_at >= $2 AND event_at < $3 AND branch_id IS NOT NULL ORDER BY event_at LIMIT 1",
    )
    .bind(employee)
    .bind(day_from)
    .bind(day_to)
    .fetch_optional(&mut ***tx)
    .await?;
    let branch = punch_branch.unwrap_or(master);
    let tz = if branch == master { master_tz } else { branch_tz(tx, branch, noon).await? };
    let cur = shape(tx, employee, date, &tz, branch).await?;
    let mut window = (cur.from, cur.to);
    if let Some(prev) = date.pred_opt() {
        let p = shape(tx, employee, prev, &tz, master).await?;
        window.0 = window.0.max(split(&p, &cur));
    }
    if let Some(next) = date.succ_opt() {
        let n = shape(tx, employee, next, &tz, master).await?;
        window.1 = window.1.min(split(&cur, &n));
    }

    // A punch replaced by an approved correction stays as evidence but is left out (BR-01). D-05c: a rejected flag,
    // or a raised flag that does not count before its decision, keeps the punch out.
    let rows: Vec<(Uuid, DateTime<Utc>, Option<String>)> = sqlx::query_as(
        "SELECT p.id, p.event_at,
                coalesce(
                  (SELECT 'replaced_by_regularisation' FROM attendance.regularisation_request q
                   WHERE q.target_raw_punch_id = p.id AND q.state = 'approved' LIMIT 1),
                  -- An approved whole-day correction stands for the whole day: only its own punches count.
                  (SELECT 'replaced_by_whole_day_correction' FROM (
                     SELECT q.id FROM attendance.regularisation_request q
                     WHERE q.employee_id = p.employee_id AND q.attendance_date = $4 AND q.request_type = 'whole_day' AND q.state = 'approved'
                     ORDER BY q.decided_at DESC, q.id LIMIT 1) w
                   WHERE p.manual_approval_ref IS DISTINCT FROM w.id),
                  (SELECT 'flag_' || f.reason || '_' || f.state FROM timecap.punch_flag f
                   WHERE f.punch_id = p.id AND (f.state = 'rejected' OR (f.state = 'raised' AND NOT f.counts_before_decision))
                   ORDER BY f.state DESC, f.reason LIMIT 1))
         FROM timecap.raw_punch p WHERE p.employee_id = $1 AND p.event_at >= $2 AND p.event_at < $3 ORDER BY p.event_at, p.id",
    )
    .bind(employee)
    .bind(window.0)
    .bind(window.1)
    .bind(date)
    .fetch_all(&mut ***tx)
    .await?;
    let existing: Option<Uuid> = sqlx::query_scalar("SELECT current_result_id FROM attendance.attendance_day WHERE employee_id = $1 AND attendance_date = $2")
        .bind(employee)
        .bind(date)
        .fetch_optional(&mut ***tx)
        .await?
        .flatten();
    if cur.expected == Expected::Unscheduled && rows.is_empty() && existing.is_none() {
        return Ok(None); // nothing expected, nothing punched
    }
    // Approved leave on the day (full, or a half with the shift's SH-11 split point).
    let leave = match leave::leave_on(tx, employee, date).await? {
        None => None,
        Some(None) => Some(LeaveCover { half: None, split_at: None }),
        Some(Some(half)) => {
            let split_at: Option<DateTime<Utc>> = match cur.shift_version {
                Some(v) => sqlx::query_scalar(
                    "SELECT CASE WHEN half_day_split = 'fixed_time' AND half_day_split_time IS NOT NULL
                                 THEN (($2::date + half_day_split_time) AT TIME ZONE $3) END FROM sched.shift_version WHERE id = $1",
                )
                .bind(v)
                .bind(date)
                .bind(&tz)
                .fetch_one(&mut ***tx)
                .await?,
                None => None,
            };
            Some(LeaveCover { half: Some(half), split_at })
        }
    };
    // DS-10: an absence is charged once its waiting days have passed, never on the day itself (D-07b).
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(&tz).fetch_one(&mut ***tx).await?;
    let absence_due = (today - date).num_days() >= settings.absence_wait_days().max(1);
    let input = DayInput {
        expected: cur.expected,
        punches: rows.into_iter().map(|(id, at, excluded)| PunchIn { id, at, excluded }).collect(),
        cutoff_passed: Utc::now() >= cur.cutoff_at,
        leave,
        absence_due,
        settings,
    };
    let print = fingerprint(&input);

    sqlx::query("INSERT INTO attendance.attendance_day (org_id, employee_id, attendance_date) VALUES (app.current_org(), $1, $2) ON CONFLICT DO NOTHING")
        .bind(employee)
        .bind(date)
        .execute(&mut ***tx)
        .await?;
    let (day_id, current): (Uuid, Option<Uuid>) =
        sqlx::query_as("SELECT id, current_result_id FROM attendance.attendance_day WHERE employee_id = $1 AND attendance_date = $2 FOR UPDATE")
            .bind(employee)
            .bind(date)
            .fetch_one(&mut ***tx)
            .await?;
    if let Some(c) = current {
        let same: bool = sqlx::query_scalar(
            "SELECT input_fingerprint = $2 AND policy_version_id = $3 AND shift_version_id IS NOT DISTINCT FROM $4 AND schedule_tz = $5
                AND costed_branch_id IS NOT DISTINCT FROM $6
             FROM attendance.day_result WHERE id = $1",
        )
        .bind(c)
        .bind(&print)
        .bind(policy_version)
        .bind(cur.shift_version)
        .bind(&tz)
        .bind(branch)
        .fetch_one(&mut ***tx)
        .await?;
        if same {
            return Ok(None);
        }
    }
    let mut out = compute_day(&input);
    settle_absence_debit(tx, employee, date, day_id, &input.settings.leave_debit_order, &mut out).await?;
    let costed_branch = branch; // EM-08 default: the branch worked
    let result: Uuid = sqlx::query_scalar(
        "INSERT INTO attendance.day_result (org_id, attendance_day_id, employee_id, attendance_date, supersedes_result_id, policy_version_id,
           shift_version_id, schedule_tz, costed_branch_id, input_fingerprint, engine_version, trigger_kind, trigger_ref,
           gross_minutes, break_minutes, net_minutes, late_minutes, early_minutes, early_arrival_minutes, overtime_identified_minutes,
           late_mark, early_mark, day_status, day_fraction_present, flags, sessions, explanation)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, NULL,
           $19, $20, $21, $22::numeric / 100, $23, $24, $25) RETURNING id",
    )
    .bind(day_id)
    .bind(employee)
    .bind(date)
    .bind(current)
    .bind(policy_version)
    .bind(cur.shift_version)
    .bind(&tz)
    .bind(costed_branch)
    .bind(&print)
    .bind(ENGINE_VERSION)
    .bind(trigger_kind)
    .bind(trigger_ref)
    .bind(out.gross_min as i32)
    .bind(out.break_min as i32)
    .bind(out.net_min as i32)
    .bind(out.late_min as i32)
    .bind(out.early_min as i32)
    .bind(out.early_arrival_min as i32)
    .bind(out.late_mark)
    .bind(out.early_mark)
    .bind(out.status)
    .bind(out.fraction_pct as i32)
    .bind(&out.flags)
    .bind(json!(out.sessions))
    .bind(json!(out.explanation))
    .fetch_one(&mut ***tx)
    .await?;
    for u in &out.punches_used {
        sqlx::query(
            "INSERT INTO attendance.day_result_punch (org_id, result_id, raw_punch_id, assigned_direction, session_no, excluded, excluded_reason)
             VALUES (app.current_org(), $1, $2, $3, $4, $5, $6)",
        )
        .bind(result)
        .bind(u.id)
        .bind(u.direction)
        .bind(u.session_no.map(|n| n as i16))
        .bind(u.excluded.is_some())
        .bind(&u.excluded)
        .execute(&mut ***tx)
        .await?;
    }
    for e in &out.effects {
        sqlx::query(
            "INSERT INTO attendance.day_effect (org_id, result_id, effect_kind, days, minutes, reason_code, rule_ref)
             VALUES (app.current_org(), $1, $2, $3::numeric / 100, $4, $5, $6)",
        )
        .bind(result)
        .bind(e.kind)
        .bind(e.days_pct.map(|d| d as i32))
        .bind(e.minutes.map(|m| m as i32))
        .bind(&e.reason_code)
        .bind(e.rule_ref)
        .execute(&mut ***tx)
        .await?;
    }
    sqlx::query("UPDATE attendance.attendance_day SET current_result_id = $2, processing_state = $3, updated_at = now() WHERE id = $1")
        .bind(day_id)
        .bind(result)
        .bind(out.state)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), 'DayResultChanged', $1, $2)")
        .bind(day_id)
        .bind(json!({ "employee_id": employee, "attendance_date": date, "result_id": result }))
        .execute(&mut ***tx)
        .await?;
    Ok(Some(result))
}

/// DS-10 auto-debit (D-07b, LV-04): an absence due for charging debits one day from the first leave type in the
/// LV-04 order that has the balance; with none, it is loss of pay. A day that stops being such an absence gets its
/// debit reversed. One debit per day ever (the ledger key is the attendance day); once reversed, a later absence on
/// the same day is loss of pay.
async fn settle_absence_debit(
    tx: &mut OrgTx<'_>,
    employee: Uuid,
    date: NaiveDate,
    day_id: Uuid,
    order: &[Uuid],
    out: &mut engine::day::DayOutput,
) -> Result<(), DomainError> {
    let open_debit: Option<Uuid> = sqlx::query_scalar(
        "SELECT e.id FROM leave.leave_ledger_entry e
         WHERE e.cause_kind = 'attendance_day' AND e.cause_id = $1 AND e.entry_kind = 'absence_auto_debit'
           AND NOT EXISTS (SELECT 1 FROM leave.leave_ledger_entry r WHERE r.reverses_entry_id = e.id)",
    )
    .bind(day_id)
    .fetch_optional(&mut ***tx)
    .await?;
    let due = out.flags.iter().any(|f| f == "absence_charge_due");
    out.flags.retain(|f| f != "absence_charge_due");
    if !due {
        if let Some(debit) = open_debit {
            sqlx::query(
                "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reverses_entry_id, reason)
                 SELECT app.current_org(), account_id, entry_date, 'reversal', -quantity, 'attendance_day', $2, id, 'the day is no longer an absence'
                 FROM leave.leave_ledger_entry WHERE id = $1",
            )
            .bind(debit)
            .bind(day_id)
            .execute(&mut ***tx)
            .await?;
        }
        return Ok(());
    }
    if open_debit.is_some() {
        out.flags.push("absence_auto_debited".into());
        return Ok(());
    }
    for leave_type in order {
        let Some(v) = leave::version_on(tx, *leave_type, date).await? else { continue };
        if !v.paid {
            continue;
        }
        let account = leave::lock_account(tx, employee, *leave_type).await?;
        if leave::balance_pct(tx, account).await? < 100 {
            continue;
        }
        let posted = sqlx::query(
            "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reason)
             VALUES (app.current_org(), $1, $2, 'absence_auto_debit', -1, 'attendance_day', $3, 'absent with no leave (DS-10, LV-04)')
             ON CONFLICT DO NOTHING",
        )
        .bind(account)
        .bind(date)
        .bind(day_id)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
        if posted == 1 {
            out.flags.push("absence_auto_debited".into());
            return Ok(());
        }
    }
    out.effects.push(Effect { kind: "absence_lop", days_pct: Some(100), minutes: None, reason_code: "absent_no_leave_balance".into(), rule_ref: "DS-10" });
    Ok(())
}

/// Recalculates the days a punch at `at` can belong to: its local date and its neighbours.
// shortcut: inline, in the caller's transaction; moves behind the Cloud Tasks queue (TSD §5.2) with the deployment slice.
pub async fn recalculate_around(tx: &mut OrgTx<'_>, employee: Uuid, at: DateTime<Utc>, trigger_kind: &str, trigger_ref: Option<Uuid>) -> Result<(), DomainError> {
    let Some(master) = master_branch_on(tx, employee, at.date_naive()).await? else { return Ok(()) };
    let tz = branch_tz(tx, master, at).await?;
    let local: NaiveDate = sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE $2)::date").bind(at).bind(&tz).fetch_one(&mut ***tx).await?;
    // A night shift started the day before; a shift starting just after midnight opens its window the evening before.
    for d in [local.pred_opt(), Some(local), local.succ_opt()].into_iter().flatten() {
        recalculate(tx, employee, d, trigger_kind, trigger_ref).await?;
    }
    Ok(())
}

const RESULT_JSON: &str = "jsonb_build_object('id', r.id, 'attendance_date', r.attendance_date, 'is_current', r.id IS NOT DISTINCT FROM d.current_result_id,
    'supersedes_result_id', r.supersedes_result_id, 'day_status', r.day_status, 'day_fraction_present', to_char(r.day_fraction_present, 'FM990.00'),
    'gross_minutes', r.gross_minutes, 'break_minutes', r.break_minutes, 'net_minutes', r.net_minutes, 'late_minutes', r.late_minutes,
    'early_minutes', r.early_minutes, 'early_arrival_minutes', r.early_arrival_minutes, 'overtime_identified_minutes', r.overtime_identified_minutes,
    'late_mark', r.late_mark, 'early_mark', r.early_mark, 'flags', to_jsonb(r.flags), 'sessions', r.sessions,
    'punches_used', (SELECT coalesce(jsonb_agg(jsonb_build_object('raw_punch_id', u.raw_punch_id, 'assigned_direction', u.assigned_direction,
        'session_no', u.session_no, 'excluded', u.excluded, 'excluded_reason', u.excluded_reason) ORDER BY rp.event_at, u.raw_punch_id), '[]')
        FROM attendance.day_result_punch u JOIN timecap.raw_punch rp ON rp.id = u.raw_punch_id WHERE u.result_id = r.id),
    'day_effects', (SELECT coalesce(jsonb_agg(jsonb_build_object('effect_kind', e.effect_kind, 'days', to_char(e.days, 'FM990.00'), 'minutes', e.minutes,
        'reason_code', e.reason_code, 'rule_ref', e.rule_ref) ORDER BY e.created_at, e.id), '[]') FROM attendance.day_effect e WHERE e.result_id = r.id),
    'explanation', r.explanation, 'policy_version_id', r.policy_version_id, 'shift_version_id', r.shift_version_id, 'schedule_tz', r.schedule_tz,
    'costed_branch_id', r.costed_branch_id, 'engine_version', r.engine_version, 'trigger_kind', r.trigger_kind, 'trigger_ref', r.trigger_ref,
    'for_adjustment', r.for_adjustment, 'produced_at', r.produced_at)";

/// AttendanceDay: current result and the superseded ones, newest first (BRX-03).
pub async fn get_day(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Value, DomainError> {
    let day: Option<(Uuid, String)> = sqlx::query_as("SELECT id, processing_state FROM attendance.attendance_day WHERE employee_id = $1 AND attendance_date = $2")
        .bind(employee)
        .bind(date)
        .fetch_optional(&mut ***tx)
        .await?;
    let (id, state) = day.ok_or(DomainError::NotFound)?;
    let sql = format!(
        "SELECT {RESULT_JSON} FROM attendance.day_result r JOIN attendance.attendance_day d ON d.id = r.attendance_day_id
         WHERE r.attendance_day_id = $1 AND NOT r.for_adjustment ORDER BY r.produced_at DESC, r.id"
    );
    let mut results: Vec<Value> = sqlx::query_scalar(&sql).bind(id).fetch_all(&mut ***tx).await?;
    let pos = results.iter().position(|r| r["is_current"] == true);
    let current = pos.map(|i| results.remove(i));
    Ok(json!({
        "id": id, "employee_id": employee, "attendance_date": date, "processing_state": state, "period_id": null,
        "current_result": current, "history": results,
        "permission_conversion_suggested": null, "check_in_held": false, "open_regularisation_request_ids": [],
    }))
}

pub struct DailyFilter {
    /// None = every employee.
    pub employees: Option<Vec<Uuid>>,
    pub date: NaiveDate,
    pub day_status: Option<String>,
    pub processing_state: Option<String>,
    pub branch: Option<Uuid>,
    pub department: Option<Uuid>,
    /// employee_code, -employee_code, day_status, -day_status.
    pub sort: Option<String>,
}

// shortcut: no cursor paging, first 500 rows.
/// Daily rows (R1) on a date.
pub async fn daily(tx: &mut OrgTx<'_>, f: &DailyFilter) -> Result<Vec<Value>, DomainError> {
    // Fixed list of orderings; never input text in SQL.
    let order = match f.sort.as_deref() {
        None | Some("employee_code") => "e.employee_code",
        Some("-employee_code") => "e.employee_code DESC",
        Some("day_status") => "r.day_status NULLS LAST, e.employee_code",
        Some("-day_status") => "r.day_status DESC NULLS LAST, e.employee_code",
        Some(other) => return Err(db::invalid(format!("sort {other} is not supported"))),
    };
    let sql = format!(
        "SELECT row FROM (SELECT jsonb_build_object('employee_id', e.id, 'employee_code', e.employee_code, 'employee_name', e.full_name,
           'branch_id', (SELECT a.branch_id FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'master_branch' AND a.valid @> $2::date),
           'department_id', (SELECT a.list_entry_id FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'department' AND a.valid @> $2::date),
           'attendance_date', d.attendance_date, 'processing_state', d.processing_state, 'day_status', r.day_status,
           'first_in_at', r.sessions->0->'in_at', 'last_out_at', r.sessions->-1->'out_at',
           'net_minutes', coalesce(r.net_minutes, 0), 'late_minutes', coalesce(r.late_minutes, 0), 'early_minutes', coalesce(r.early_minutes, 0),
           'flags', coalesce(to_jsonb(r.flags), '[]')) AS row, e.employee_code, r.day_status
         FROM attendance.attendance_day d JOIN workforce.employee e ON e.id = d.employee_id
         LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
         WHERE d.attendance_date = $2 AND ($1::uuid[] IS NULL OR d.employee_id = ANY($1)) AND ($3::text IS NULL OR r.day_status = $3)
           AND ($4::text IS NULL OR d.processing_state = $4)
           AND ($5::uuid IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'master_branch' AND a.branch_id = $5 AND a.valid @> $2::date))
           AND ($6::uuid IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'department' AND a.list_entry_id = $6 AND a.valid @> $2::date))
         ) x ORDER BY {} LIMIT 500",
        order.replace("e.", "x.").replace("r.", "x.")
    );
    Ok(sqlx::query_scalar(&sql)
        .bind(&f.employees)
        .bind(f.date)
        .bind(&f.day_status)
        .bind(&f.processing_state)
        .bind(f.branch)
        .bind(f.department)
        .fetch_all(&mut ***tx)
        .await?)
}
