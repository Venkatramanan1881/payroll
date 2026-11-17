//! Shifts and versioned shift definitions (SH-01..SH-12, G5). Draft versions are editable; published ones are frozen.

use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{Map, Value};
use uuid::Uuid;

/// scheduling.yaml ShiftDefinition fields = sched.shift_version columns. Only these names ever reach SQL.
const DEFINITION: [&str; 17] = [
    "shift_type", "start_time", "end_time", "required_minutes", "required_per", "core_start", "core_end",
    "flex_earliest_in", "flex_latest_out", "window_before_min", "window_after_min", "is_night_shift",
    "night_attendance_date", "half_day_split", "half_day_split_time", "segments", "missed_punch_cutoff",
];

const VERSION_JSON: &str = "(to_jsonb(v) - 'org_id')";

fn check_fields(def: &Map<String, Value>, extra: &[&str]) -> Result<(), DomainError> {
    if let Some(k) = def.keys().find(|k| !DEFINITION.contains(&k.as_str()) && !extra.contains(&k.as_str())) {
        return Err(invalid(format!("unknown field {k}")));
    }
    // The expected-day resolver handles only the default (SH-06 = shift start date); refuse rather than resolve the wrong day.
    if def.get("night_attendance_date").and_then(Value::as_str) == Some("shift_end_date") {
        return Err(invalid("night_attendance_date = shift_end_date is not supported yet"));
    }
    Ok(())
}

/// Only drafts can change (G5): published versions (scheduled, in force or superseded) are frozen.
async fn require_draft(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(), DomainError> {
    let state: Option<String> = sqlx::query_scalar("SELECT state FROM sched.shift_version WHERE id = $1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    match state.as_deref() {
        None => Err(DomainError::NotFound),
        Some("draft") => Ok(()),
        Some(_) => Err(DomainError::VersionFrozen),
    }
}

/// Columns present in `def`, in a fixed order; values are cast by jsonb_populate_record.
fn columns(def: &Map<String, Value>) -> Vec<&'static str> {
    DEFINITION.iter().copied().filter(|c| def.contains_key(*c)).collect()
}

pub async fn get_version(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {VERSION_JSON} FROM sched.shift_version v WHERE v.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn get_shift(tx: &mut OrgTx<'_>, id: Uuid, today: NaiveDate) -> Result<Value, DomainError> {
    let sql = format!(
        "SELECT jsonb_build_object('id', s.id, 'code', s.code, 'name', s.name, 'retired_at', s.retired_at,
           'versions', coalesce((SELECT jsonb_agg({VERSION_JSON} ORDER BY v.effective_from) FROM sched.shift_version v WHERE v.shift_id = s.id), '[]'),
           'current_version', (SELECT {VERSION_JSON} FROM sched.shift_version v WHERE v.shift_id = s.id
                               AND v.state IN ('scheduled','in_force','superseded') AND v.effective_from <= $2
                               ORDER BY v.effective_from DESC LIMIT 1))
         FROM sched.shift s WHERE s.id = $1"
    );
    sqlx::query_scalar(&sql).bind(id).bind(today).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

// shortcut: no cursor paging (next_cursor is always null); add a cursor when organisations have hundreds of shifts.
pub async fn list_shifts(tx: &mut OrgTx<'_>, include_retired: bool, today: NaiveDate) -> Result<Vec<Value>, DomainError> {
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM sched.shift WHERE $1 OR retired_at IS NULL ORDER BY code")
        .bind(include_retired)
        .fetch_all(&mut ***tx)
        .await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(get_shift(tx, id, today).await?);
    }
    Ok(out)
}

pub async fn create_shift(tx: &mut OrgTx<'_>, actor: Uuid, code: &str, name: &str, from: NaiveDate, def: Map<String, Value>) -> Result<Uuid, DomainError> {
    if code.trim().is_empty() || name.trim().is_empty() {
        return Err(invalid("code and name are required"));
    }
    check_fields(&def, &[])?;
    let shift: Uuid = sqlx::query_scalar("INSERT INTO sched.shift (org_id, code, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $3) RETURNING id")
        .bind(code.trim())
        .bind(name.trim())
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    insert_version(tx, actor, shift, from, &def).await?;
    db::audit(tx, actor, "shift.create", "shift", Some(shift), Some(Value::Object(def))).await?;
    Ok(shift)
}

pub async fn create_version(tx: &mut OrgTx<'_>, actor: Uuid, shift: Uuid, from: NaiveDate, def: Map<String, Value>) -> Result<Uuid, DomainError> {
    check_fields(&def, &[])?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sched.shift WHERE id = $1)").bind(shift).fetch_one(&mut ***tx).await?;
    if !exists {
        return Err(DomainError::NotFound);
    }
    // A new version starts from the latest one; supplied fields override it.
    let latest: Option<Value> = sqlx::query_scalar(
        "SELECT to_jsonb(v) FROM sched.shift_version v WHERE v.shift_id = $1 ORDER BY v.effective_from DESC, v.created_at DESC LIMIT 1",
    )
    .bind(shift)
    .fetch_optional(&mut ***tx)
    .await?;
    let mut merged: Map<String, Value> = latest
        .and_then(|v| v.as_object().cloned())
        .map(|m| m.into_iter().filter(|(k, _)| DEFINITION.contains(&k.as_str())).collect())
        .unwrap_or_default();
    merged.extend(def.clone());
    let id = insert_version(tx, actor, shift, from, &merged).await?;
    db::audit(tx, actor, "shift_version.create", "shift_version", Some(id), Some(Value::Object(def))).await?;
    Ok(id)
}

async fn insert_version(tx: &mut OrgTx<'_>, actor: Uuid, shift: Uuid, from: NaiveDate, def: &Map<String, Value>) -> Result<Uuid, DomainError> {
    let cols = columns(def);
    let (names, picks) = if cols.is_empty() {
        (String::new(), String::new())
    } else {
        (format!(", {}", cols.join(", ")), format!(", {}", cols.iter().map(|c| format!("r.{c}")).collect::<Vec<_>>().join(", ")))
    };
    let sql = format!(
        "INSERT INTO sched.shift_version (org_id, shift_id, effective_from, created_by, updated_by{names})
         SELECT app.current_org(), $1, $2, $3, $3{picks} FROM jsonb_populate_record(NULL::sched.shift_version, $4) r RETURNING id"
    );
    Ok(sqlx::query_scalar(&sql).bind(shift).bind(from).bind(actor).bind(Value::Object(def.clone())).fetch_one(&mut ***tx).await?)
}

/// PATCH a draft (definition fields and effective_from). Published versions are refused by the G5 trigger.
pub async fn update_version(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, patch: Map<String, Value>) -> Result<(), DomainError> {
    if patch.is_empty() {
        return Err(invalid("at least one field"));
    }
    check_fields(&patch, &["effective_from"])?;
    require_draft(tx, id).await?;
    let mut cols = columns(&patch);
    if patch.contains_key("effective_from") {
        cols.push("effective_from");
    }
    let sql = format!(
        "UPDATE sched.shift_version v SET ({cols}, updated_by) = (SELECT {picks}, $3 FROM jsonb_populate_record(NULL::sched.shift_version, $2) r) WHERE v.id = $1",
        cols = cols.join(", "),
        picks = cols.iter().map(|c| format!("r.{c}")).collect::<Vec<_>>().join(", "),
    );
    let n = sqlx::query(&sql).bind(id).bind(Value::Object(patch.clone())).bind(actor).execute(&mut ***tx).await?.rows_affected();
    if n == 0 {
        return Err(DomainError::NotFound);
    }
    db::audit(tx, actor, "shift_version.update", "shift_version", Some(id), Some(Value::Object(patch))).await?;
    Ok(())
}

/// Draft -> in_force (effective_from <= today) or scheduled; earlier in_force versions of the shift become superseded.
/// Publishing an already-published version returns it unchanged (idempotent).
pub async fn publish_version(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, today: NaiveDate) -> Result<(), DomainError> {
    let row: Option<(Uuid, String, NaiveDate)> = sqlx::query_as("SELECT shift_id, state, effective_from FROM sched.shift_version WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?;
    let (shift, state, from) = row.ok_or(DomainError::NotFound)?;
    match state.as_str() {
        "draft" => {}
        "scheduled" | "in_force" | "superseded" => return Ok(()),
        other => return Err(DomainError::IllegalTransition(format!("cannot publish a {other} version"))),
    }
    // The version governs [from, next published version's start): refuse if that touches locked days (BR-06).
    let next: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT min(effective_from) FROM sched.shift_version WHERE shift_id = $1 AND state IN ('scheduled','in_force','superseded') AND effective_from > $2",
    )
    .bind(shift)
    .bind(from)
    .fetch_one(&mut ***tx)
    .await?;
    if db::any_locked(tx, from, next).await? {
        return Err(DomainError::PeriodLocked);
    }
    let new_state = if from <= today { "in_force" } else { "scheduled" };
    sqlx::query("UPDATE sched.shift_version SET state = $2, updated_by = $3 WHERE id = $1").bind(id).bind(new_state).bind(actor).execute(&mut ***tx).await?;
    if new_state == "in_force" {
        sqlx::query("UPDATE sched.shift_version SET state = 'superseded' WHERE shift_id = $1 AND id <> $2 AND state = 'in_force' AND effective_from < $3")
            .bind(shift)
            .bind(id)
            .bind(from)
            .execute(&mut ***tx)
            .await?;
    }
    db::audit(tx, actor, "shift_version.publish", "shift_version", Some(id), Some(serde_json::json!({ "state": new_state }))).await?;
    Ok(())
}
