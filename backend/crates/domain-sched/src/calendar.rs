//! Weekly-off patterns (SH-09) and holiday calendars (HD-R1..R3, SH-10).

use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

pub struct NewPattern {
    pub name: String,
    pub kind: String,
    pub weekly_days: Option<Vec<i16>>,
    pub alt_day: Option<i16>,
    pub alt_weeks: Option<Vec<i16>>,
}

pub async fn get_pattern(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar("SELECT to_jsonb(p) - 'org_id' FROM sched.weekly_off_pattern p WHERE p.id = $1")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn create_pattern(tx: &mut OrgTx<'_>, actor: Uuid, p: NewPattern) -> Result<Uuid, DomainError> {
    if p.name.trim().is_empty() {
        return Err(invalid("name"));
    }
    // The DB CHECK enforces which fields each kind needs (fixed_days: weekly_days; alternating: alt_day + alt_weeks).
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO sched.weekly_off_pattern (org_id, name, kind, weekly_days, alt_day, alt_weeks, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $6) RETURNING id",
    )
    .bind(p.name.trim())
    .bind(&p.kind)
    .bind(&p.weekly_days)
    .bind(p.alt_day)
    .bind(&p.alt_weeks)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "weekly_off_pattern.create", "weekly_off_pattern", Some(id), None).await?;
    Ok(id)
}

pub async fn list_patterns(tx: &mut OrgTx<'_>, include_retired: bool) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar("SELECT to_jsonb(p) - 'org_id' FROM sched.weekly_off_pattern p WHERE $1 OR retired_at IS NULL ORDER BY name")
        .bind(include_retired)
        .fetch_all(&mut ***tx)
        .await?)
}

pub async fn retire_pattern(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid) -> Result<(), DomainError> {
    let n = sqlx::query("UPDATE sched.weekly_off_pattern SET retired_at = coalesce(retired_at, now()), updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(id)
        .bind(actor)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(DomainError::NotFound);
    }
    db::audit(tx, actor, "weekly_off_pattern.retire", "weekly_off_pattern", Some(id), None).await?;
    Ok(())
}

pub async fn get_calendar(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(
        "SELECT (to_jsonb(c) - 'org_id') || jsonb_build_object('branch_ids',
           coalesce((SELECT jsonb_agg(b.id) FROM org.branch b WHERE b.holiday_calendar_id = c.id), '[]'))
         FROM sched.holiday_calendar c WHERE c.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?
    .ok_or(DomainError::NotFound)
}

pub async fn create_calendar(tx: &mut OrgTx<'_>, actor: Uuid, name: &str) -> Result<Uuid, DomainError> {
    if name.trim().is_empty() {
        return Err(invalid("name"));
    }
    let id: Uuid = sqlx::query_scalar("INSERT INTO sched.holiday_calendar (org_id, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $2) RETURNING id")
        .bind(name.trim())
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    db::audit(tx, actor, "holiday_calendar.create", "holiday_calendar", Some(id), None).await?;
    Ok(id)
}

pub async fn list_calendars(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM sched.holiday_calendar ORDER BY name").fetch_all(&mut ***tx).await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(get_calendar(tx, id).await?);
    }
    Ok(out)
}

pub async fn update_calendar(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, name: Option<String>, retired: Option<bool>) -> Result<(), DomainError> {
    if name.is_none() && retired.is_none() {
        return Err(invalid("at least one field"));
    }
    if name.as_deref().is_some_and(|n| n.trim().is_empty()) {
        return Err(invalid("name"));
    }
    let n = sqlx::query(
        "UPDATE sched.holiday_calendar SET name = coalesce($2, name),
           retired_at = CASE WHEN $3 IS NULL THEN retired_at WHEN $3 THEN coalesce(retired_at, now()) ELSE NULL END,
           updated_at = now(), updated_by = $4
         WHERE id = $1",
    )
    .bind(id)
    .bind(name.as_deref().map(str::trim))
    .bind(retired)
    .bind(actor)
    .execute(&mut ***tx)
    .await?
    .rows_affected();
    if n == 0 {
        return Err(DomainError::NotFound);
    }
    db::audit(tx, actor, "holiday_calendar.update", "holiday_calendar", Some(id), Some(json!({ "name": name, "retired": retired }))).await?;
    Ok(())
}

pub async fn get_holiday(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar("SELECT to_jsonb(h) - 'org_id' FROM sched.holiday h WHERE h.id = $1")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn add_holiday(tx: &mut OrgTx<'_>, actor: Uuid, calendar: Uuid, date: NaiveDate, name: &str, kind: &str, reason: &str) -> Result<Uuid, DomainError> {
    if name.trim().is_empty() || reason.trim().is_empty() {
        return Err(invalid("name and reason are required"));
    }
    get_calendar(tx, calendar).await?; // NotFound for another organisation's calendar
    holiday_unlocked(tx, date).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO sched.holiday (org_id, calendar_id, holiday_date, name, kind, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $5) RETURNING id",
    )
    .bind(calendar)
    .bind(date)
    .bind(name.trim())
    .bind(kind)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "holiday.add", "holiday", Some(id), Some(json!({ "reason": reason }))).await?;
    Ok(id)
}

pub async fn list_holidays(tx: &mut OrgTx<'_>, calendar: Uuid, kind: Option<&str>, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    get_calendar(tx, calendar).await?;
    Ok(sqlx::query_scalar(
        "SELECT to_jsonb(h) - 'org_id' FROM sched.holiday h
         WHERE h.calendar_id = $1 AND ($2::text IS NULL OR h.kind = $2) AND ($3::date IS NULL OR h.holiday_date >= $3) AND ($4::date IS NULL OR h.holiday_date <= $4)
         ORDER BY h.holiday_date",
    )
    .bind(calendar)
    .bind(kind)
    .bind(from)
    .bind(to)
    .fetch_all(&mut ***tx)
    .await?)
}

pub async fn update_holiday(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, date: Option<NaiveDate>, name: Option<String>, kind: Option<String>, reason: &str) -> Result<(), DomainError> {
    if reason.trim().is_empty() {
        return Err(invalid("reason"));
    }
    let before = get_holiday(tx, id).await?;
    holiday_unlocked(tx, date_of(&before)?).await?;
    if let Some(d) = date {
        holiday_unlocked(tx, d).await?;
    }
    sqlx::query(
        "UPDATE sched.holiday SET holiday_date = coalesce($2, holiday_date), name = coalesce($3, name), kind = coalesce($4, kind),
           updated_at = now(), updated_by = $5 WHERE id = $1",
    )
    .bind(id)
    .bind(date)
    .bind(name.as_deref().map(str::trim))
    .bind(kind)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "holiday.update", "holiday", Some(id), Some(json!({ "before": before, "reason": reason }))).await?;
    Ok(())
}

/// Deletes the holiday; its prior values stay in the audit entry (sched.holiday has no soft-remove column).
pub async fn remove_holiday(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid) -> Result<(), DomainError> {
    let before = get_holiday(tx, id).await?;
    holiday_unlocked(tx, date_of(&before)?).await?;
    sqlx::query("DELETE FROM sched.holiday WHERE id = $1").bind(id).execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO audit.audit_entry (org_id, actor_kind, actor_id, action, target_type, target_id, before_value)
         VALUES (app.current_org(), 'user', $1, 'holiday.remove', 'holiday', $2, $3)",
    )
    .bind(actor)
    .bind(id)
    .bind(before)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// A holiday on a locked date would change locked days (BR-06).
async fn holiday_unlocked(tx: &mut OrgTx<'_>, date: NaiveDate) -> Result<(), DomainError> {
    if db::any_locked(tx, date, date.succ_opt()).await? {
        return Err(DomainError::PeriodLocked);
    }
    Ok(())
}

fn date_of(holiday: &Value) -> Result<NaiveDate, DomainError> {
    holiday["holiday_date"].as_str().and_then(|d| d.parse().ok()).ok_or_else(|| invalid("holiday_date"))
}

/// Branches whose expected days this calendar drives (for the caller's scope check).
pub async fn calendar_branches(tx: &mut OrgTx<'_>, calendar: Uuid) -> Result<Vec<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT id FROM org.branch WHERE holiday_calendar_id = $1").bind(calendar).fetch_all(&mut ***tx).await?)
}

/// The calendar a holiday belongs to.
pub async fn holiday_calendar_of(tx: &mut OrgTx<'_>, holiday: Uuid) -> Result<Uuid, DomainError> {
    sqlx::query_scalar("SELECT calendar_id FROM sched.holiday WHERE id = $1").bind(holiday).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}
