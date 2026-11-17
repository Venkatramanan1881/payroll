//! Employee schedules (EM-R3), roster entries (SH-R2) and the expected-day resolver (DOMAIN_MODEL §4 ScheduleResolver).

use chrono::{Datelike, NaiveDate};
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// What was expected of an employee on an attendance date. Weekly off and holiday can both be true with a shift present.
#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedDay {
    pub source: &'static str, // "roster" | "fixed_shift" | "none"
    pub shift_id: Option<Uuid>,
    pub shift_version_id: Option<Uuid>,
    pub weekly_off: bool,
    pub holiday: Option<String>,
}

/// The published version of `shift` in force on `date` (by effective date, never by the stored state flag).
pub async fn version_on(tx: &mut OrgTx<'_>, shift: Uuid, date: NaiveDate) -> Result<Option<Uuid>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM sched.shift_version WHERE shift_id = $1 AND state IN ('scheduled','in_force','superseded') AND effective_from <= $2
         ORDER BY effective_from DESC, created_at DESC LIMIT 1",
    )
    .bind(shift)
    .bind(date)
    .fetch_optional(&mut ***tx)
    .await?)
}

/// `branch` gives the holiday calendar: the punch's branch when there is one, otherwise the master branch (D-04).
pub async fn expected_day(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, branch: Uuid) -> Result<ExpectedDay, DomainError> {
    let schedule: Option<(String, Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("SELECT source, shift_id, weekly_off_pattern_id FROM sched.employee_schedule WHERE employee_id = $1 AND valid @> $2::date")
            .bind(employee)
            .bind(date)
            .fetch_optional(&mut ***tx)
            .await?;
    let roster: Option<(Option<Uuid>, bool)> =
        sqlx::query_as("SELECT shift_version_id, is_weekly_off FROM sched.roster_entry WHERE employee_id = $1 AND roster_date = $2")
            .bind(employee)
            .bind(date)
            .fetch_optional(&mut ***tx)
            .await?;

    let mut day = ExpectedDay { source: "none", shift_id: None, shift_version_id: None, weekly_off: false, holiday: None };
    if let Some((version, off)) = roster {
        // A roster entry for the date wins over a fixed shift (SH-R2).
        day.source = "roster";
        day.weekly_off = off;
        day.shift_version_id = version;
        if let Some(v) = version {
            day.shift_id = sqlx::query_scalar("SELECT shift_id FROM sched.shift_version WHERE id = $1").bind(v).fetch_one(&mut ***tx).await?;
        }
    } else if let Some((source, shift, _)) = &schedule {
        if source == "fixed_shift" {
            day.source = "fixed_shift";
            day.shift_id = *shift;
            if let Some(s) = shift {
                day.shift_version_id = version_on(tx, *s, date).await?;
            }
        }
    }
    if day.source != "roster" {
        if let Some((_, _, Some(pattern))) = schedule {
            day.weekly_off = pattern_is_off(tx, pattern, date).await?;
        }
    }
    day.holiday = sqlx::query_scalar(
        "SELECT h.name FROM sched.holiday h JOIN org.branch b ON b.holiday_calendar_id = h.calendar_id
         WHERE b.id = $1 AND h.holiday_date = $2
           AND (h.kind <> 'optional' OR EXISTS (SELECT 1 FROM sched.optional_holiday_choice c WHERE c.holiday_id = h.id AND c.employee_id = $3))
         LIMIT 1",
    )
    .bind(branch)
    .bind(date)
    .bind(employee)
    .fetch_optional(&mut ***tx)
    .await?;
    Ok(day)
}

/// (kind, weekly_days, alt_day, alt_weeks) of sched.weekly_off_pattern.
type PatternRow = (String, Option<Vec<i16>>, Option<i16>, Option<Vec<i16>>);

async fn pattern_is_off(tx: &mut OrgTx<'_>, pattern: Uuid, date: NaiveDate) -> Result<bool, DomainError> {
    let (kind, days, alt_day, alt_weeks): PatternRow =
        sqlx::query_as("SELECT kind, weekly_days, alt_day, alt_weeks FROM sched.weekly_off_pattern WHERE id = $1")
            .bind(pattern)
            .fetch_one(&mut ***tx)
            .await?;
    let iso_day = date.weekday().number_from_monday() as i16;
    let week_of_month = ((date.day() - 1) / 7 + 1) as i16;
    Ok(match kind.as_str() {
        "fixed_days" => days.unwrap_or_default().contains(&iso_day),
        "alternating" => alt_day == Some(iso_day) && alt_weeks.unwrap_or_default().contains(&week_of_month),
        _ => false, // from_roster: only roster entries say so
    })
}

const SCHEDULE_JSON: &str = "jsonb_build_object('id', s.id, 'employee_id', s.employee_id, 'source', s.source, 'shift_id', s.shift_id,
    'weekly_off_pattern_id', s.weekly_off_pattern_id, 'valid_from', lower(s.valid), 'valid_to', upper(s.valid),
    'reason', s.reason, 'created_at', s.created_at, 'created_by', s.created_by)";

pub struct NewSchedule {
    pub employee: Uuid,
    pub source: String,
    pub shift: Option<Uuid>,
    pub pattern: Option<Uuid>,
    pub from: NaiveDate,
    pub reason: String,
}

/// Dated schedule (fixed shift or roster). A change ends the current range at `from` and starts the next one.
pub async fn assign_schedule(tx: &mut OrgTx<'_>, actor: Uuid, s: NewSchedule) -> Result<Value, DomainError> {
    let NewSchedule { employee, source, shift, pattern, from, reason } = s;
    if reason.trim().is_empty() {
        return Err(invalid("reason"));
    }
    let current: Option<(Uuid, NaiveDate)> =
        sqlx::query_as("SELECT id, lower(valid) FROM sched.employee_schedule WHERE employee_id = $1 AND valid @> $2::date FOR UPDATE")
            .bind(employee)
            .bind(from)
            .fetch_optional(&mut ***tx)
            .await?;
    if let Some((id, start)) = current {
        if start >= from {
            return Err(invalid("valid_from must be after the current schedule's start"));
        }
        sqlx::query("UPDATE sched.employee_schedule SET valid = daterange(lower(valid), $2) WHERE id = $1").bind(id).bind(from).execute(&mut ***tx).await?;
    }
    let sql = format!(
        "INSERT INTO sched.employee_schedule AS s (org_id, employee_id, source, shift_id, weekly_off_pattern_id, valid, reason, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4, daterange($5, NULL), $6, $7) RETURNING {SCHEDULE_JSON}"
    );
    let row: Value = sqlx::query_scalar(&sql)
        .bind(employee)
        .bind(&source)
        .bind(shift)
        .bind(pattern)
        .bind(from)
        .bind(reason.trim())
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    db::audit(tx, actor, "employee_schedule.assign", "employee", Some(employee), Some(row.clone())).await?;
    Ok(row)
}

pub async fn list_schedules(tx: &mut OrgTx<'_>, employee: Option<Uuid>, on: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {SCHEDULE_JSON} FROM sched.employee_schedule s
         WHERE ($1::uuid IS NULL OR s.employee_id = $1) AND ($2::date IS NULL OR s.valid @> $2::date) ORDER BY s.employee_id, lower(s.valid)"
    );
    Ok(sqlx::query_scalar(&sql).bind(employee).bind(on).fetch_all(&mut ***tx).await?)
}

pub struct RosterInput {
    pub employee: Uuid,
    pub date: NaiveDate,
    pub weekly_off: bool,
    pub shift: Option<Uuid>,
}

const ROSTER_JSON: &str = "(to_jsonb(r) - 'org_id') || jsonb_build_object('shift_id', v.shift_id, 'shift_code', sh.code)";

/// Upserts roster entries. The caller has already checked every employee's scope; one bad entry fails the whole call.
pub async fn set_roster(tx: &mut OrgTx<'_>, actor: Uuid, entries: Vec<RosterInput>, reason: Option<&str>) -> Result<Vec<Value>, DomainError> {
    let mut ids = Vec::with_capacity(entries.len());
    for e in &entries {
        let version = match (e.weekly_off, e.shift) {
            (true, None) => None,
            (false, Some(s)) => Some(version_on(tx, s, e.date).await?.ok_or_else(|| invalid(format!("no published version of shift {s} on {}", e.date)))?),
            _ => return Err(invalid("shift_id is required unless is_weekly_off, and must be absent when it is")),
        };
        let locked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.lock_coverage WHERE employee_id = $1 AND dates @> $2::date)")
            .bind(e.employee)
            .bind(e.date)
            .fetch_one(&mut ***tx)
            .await?;
        if locked {
            return Err(DomainError::PeriodLocked);
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO sched.roster_entry (org_id, employee_id, roster_date, shift_version_id, is_weekly_off, source, created_by, updated_by)
             VALUES (app.current_org(), $1, $2, $3, $4, 'screen', $5, $5)
             ON CONFLICT (org_id, employee_id, roster_date) DO UPDATE
               SET shift_version_id = EXCLUDED.shift_version_id, is_weekly_off = EXCLUDED.is_weekly_off, updated_at = now(), updated_by = EXCLUDED.updated_by
             RETURNING id",
        )
        .bind(e.employee)
        .bind(e.date)
        .bind(version)
        .bind(e.weekly_off)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
        ids.push(id);
    }
    let sql = format!(
        "SELECT {ROSTER_JSON} FROM sched.roster_entry r LEFT JOIN sched.shift_version v ON v.id = r.shift_version_id
         LEFT JOIN sched.shift sh ON sh.id = v.shift_id WHERE r.id = ANY($1) ORDER BY r.employee_id, r.roster_date"
    );
    let out: Vec<Value> = sqlx::query_scalar(&sql).bind(&ids).fetch_all(&mut ***tx).await?;
    db::audit(tx, actor, "roster.set", "roster_entry", None, Some(json!({ "entries": out, "reason": reason }))).await?;
    Ok(out)
}

pub async fn list_roster(tx: &mut OrgTx<'_>, employee: Option<Uuid>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {ROSTER_JSON} FROM sched.roster_entry r LEFT JOIN sched.shift_version v ON v.id = r.shift_version_id
         LEFT JOIN sched.shift sh ON sh.id = v.shift_id
         WHERE ($1::uuid IS NULL OR r.employee_id = $1) AND ($2::date IS NULL OR r.roster_date >= $2) AND ($3::date IS NULL OR r.roster_date <= $3)
         ORDER BY r.employee_id, r.roster_date"
    );
    Ok(sqlx::query_scalar(&sql).bind(employee).bind(from).bind(to).fetch_all(&mut ***tx).await?)
}

/// scheduling.yaml ScheduleDay for each date in [from, to], holidays from `branch`.
pub async fn schedule_days(tx: &mut OrgTx<'_>, employee: Uuid, branch: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Vec<Value>, DomainError> {
    if to < from || (to - from).num_days() > 62 {
        return Err(invalid("from/to must be ordered and at most 63 days apart"));
    }
    let mut out = Vec::new();
    let mut date = from;
    while date <= to {
        let day = expected_day(tx, employee, date, branch).await?;
        let shift: Option<Value> = match day.shift_version_id {
            Some(v) => sqlx::query_scalar(
                "SELECT jsonb_build_object('shift_code', s.code, 'shift_name', s.name, 'start_time', v.start_time, 'end_time', v.end_time)
                 FROM sched.shift_version v JOIN sched.shift s ON s.id = v.shift_id WHERE v.id = $1",
            )
            .bind(v)
            .fetch_optional(&mut ***tx)
            .await?,
            None => None,
        };
        let mut item = json!({ "date": date, "source": day.source, "shift_id": day.shift_id, "is_weekly_off": day.weekly_off, "holiday_name": day.holiday });
        if let Some(Value::Object(extra)) = shift {
            item.as_object_mut().expect("object").extend(extra);
        }
        out.push(item);
        date = date.succ_opt().expect("date in range");
    }
    Ok(out)
}
