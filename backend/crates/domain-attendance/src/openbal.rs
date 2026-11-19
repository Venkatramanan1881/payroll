//! Opening balances (C15, SM-OB): draft batches of entries, confirmation that posts them into their ledgers once,
//! and changes after confirmation as a superseding entry with a reason (OB-R5, OB-02). Nothing is overwritten.

use crate::approval::{step, steps_sql, Step};
use chrono::NaiveDate;
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// OB-01 (approval needed) and OB-02 (changes after confirmation allowed), BRD defaults: HR only; allowed with a reason.
pub async fn settings(tx: &mut OrgTx<'_>) -> Result<(bool, bool), DomainError> {
    let s: Option<Value> = sqlx::query_scalar(
        "SELECT settings->'opening_balances' FROM org.organisation_settings_version WHERE effective_from <= (now() AT TIME ZONE 'Asia/Kolkata')::date
         ORDER BY effective_from DESC LIMIT 1",
    )
    .fetch_optional(&mut ***tx)
    .await?
    .flatten();
    let s = s.unwrap_or(Value::Null);
    Ok((s["confirm_mode"] == "hr_with_org_admin_approval", s["change_after_confirm"] != "not_allowed"))
}

fn batch_sql() -> String {
    format!(
        "jsonb_build_object('id', b.id, 'state', b.state, 'upload', NULL, 'created_by', b.created_by, 'created_at', b.created_at,
           'submitted_by', b.submitted_by, 'confirmed_by', b.confirmed_by, 'confirmed_at', b.confirmed_at, 'updated_at', b.updated_at,
           'entry_count', (SELECT count(*) FROM openbal.opening_balance_entry e WHERE e.batch_id = b.id AND e.superseded_by_entry_id IS NULL),
           'approval_steps', {})",
        steps_sql("opening_balance", "b.id")
    )
}

const ENTRY_JSON: &str = "jsonb_build_object('id', e.id, 'batch_id', e.batch_id, 'employee_id', e.employee_id, 'balance_kind', e.balance_kind,
    'leave_type_id', e.leave_type_id, 'permission_type_id', e.permission_type_id, 'value', to_char(e.value, 'FM99990.00'), 'as_of_date', e.as_of_date,
    'change_reason', e.change_reason, 'superseded_by_entry_id', e.superseded_by_entry_id, 'updated_at', e.updated_at)";

pub async fn get_batch(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {} FROM openbal.opening_balance_batch b WHERE b.id = $1", batch_sql()))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_batches(tx: &mut OrgTx<'_>, state: Option<&str>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!("SELECT {} FROM openbal.opening_balance_batch b WHERE ($1::text IS NULL OR b.state = $1) ORDER BY b.created_at DESC", batch_sql()))
        .bind(state)
        .fetch_all(&mut ***tx)
        .await?)
}

pub async fn get_entry(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {ENTRY_JSON} FROM openbal.opening_balance_entry e WHERE e.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_entries(tx: &mut OrgTx<'_>, batch: Uuid, employee: Option<Uuid>, kind: Option<&str>, superseded: bool) -> Result<Vec<Value>, DomainError> {
    get_batch(tx, batch).await?;
    Ok(sqlx::query_scalar(&format!(
        "SELECT {ENTRY_JSON} FROM openbal.opening_balance_entry e
         WHERE e.batch_id = $1 AND ($2::uuid IS NULL OR e.employee_id = $2) AND ($3::text IS NULL OR e.balance_kind = $3)
           AND ($4 OR e.superseded_by_entry_id IS NULL)
         ORDER BY e.employee_id, e.balance_kind, e.created_at"
    ))
    .bind(batch)
    .bind(employee)
    .bind(kind)
    .bind(superseded)
    .fetch_all(&mut ***tx)
    .await?)
}

/// The employees an entry list touches (callers check scope on them).
pub async fn batch_employees(tx: &mut OrgTx<'_>, batch: Uuid) -> Result<Vec<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT DISTINCT employee_id FROM openbal.opening_balance_entry WHERE batch_id = $1").bind(batch).fetch_all(&mut ***tx).await?)
}

pub struct EntryIn {
    pub employee: Uuid,
    pub kind: String,
    pub leave_type: Option<Uuid>,
    pub permission_type: Option<Uuid>,
    pub value: String,
    pub as_of: NaiveDate,
}

fn value_ok(v: &str) -> bool {
    let body = v.strip_prefix('-').unwrap_or(v);
    body.split_once('.').is_some_and(|(i, d)| (1..=5).contains(&i.len()) && d.len() == 2 && i.bytes().chain(d.bytes()).all(|b| b.is_ascii_digit()))
}

/// Problems with one entry, or None.
async fn check_entry(tx: &mut OrgTx<'_>, e: &EntryIn) -> Result<Option<String>, DomainError> {
    if !value_ok(&e.value) {
        return Ok(Some("value is a decimal with two places, e.g. 12.50".into()));
    }
    match (e.kind.as_str(), e.leave_type, e.permission_type) {
        ("leave", Some(t), None) => {
            let known: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.leave_type WHERE id = $1)").bind(t).fetch_one(&mut ***tx).await?;
            if !known {
                return Ok(Some("leave_type_id is not a leave type of the organisation".into()));
            }
        }
        ("late_counter" | "early_counter", None, None) => {}
        ("comp_off" | "permission_used", ..) => return Ok(Some(format!("{} opening balances arrive with their module (not yet available)", e.kind))),
        ("leave", ..) => return Ok(Some("leave needs leave_type_id (and no permission_type_id)".into())),
        _ => return Ok(Some("balance_kind is leave, comp_off, permission_used, late_counter or early_counter; counters take no type".into())),
    }
    let employed: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM workforce.employee WHERE id = $1)").bind(e.employee).fetch_one(&mut ***tx).await?;
    if !employed {
        return Ok(Some("employee not found".into()));
    }
    if db::employee_locked(tx, e.employee, e.as_of, e.as_of.succ_opt()).await? {
        return Ok(Some("as_of_date is in a locked period (BR-06)".into()));
    }
    Ok(None)
}

fn row_error(i: usize, message: &str) -> Value {
    json!({ "field": format!("entries[{i}]"), "code": "INVALID_ROW", "message": message })
}

/// Rows refused before any is written. `out_of_scope` rows (by index) read as "not found" (AC-C15-02.3).
async fn insert_entries(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, entries: &[EntryIn], out_of_scope: &[usize]) -> Result<(), DomainError> {
    let mut errors = vec![];
    for (i, e) in entries.iter().enumerate() {
        let problem = if out_of_scope.contains(&i) { Some("employee not found".into()) } else { check_entry(tx, e).await? };
        if let Some(p) = problem {
            errors.push(row_error(i, &p));
        }
    }
    if !errors.is_empty() {
        return Err(DomainError::ValidationRows(format!("{} row(s) are not valid; nothing was saved", errors.len()), Value::Array(errors)));
    }
    for e in entries {
        sqlx::query(
            "INSERT INTO openbal.opening_balance_entry (org_id, batch_id, employee_id, balance_kind, leave_type_id, permission_type_id, value, as_of_date, created_by, updated_by)
             VALUES (app.current_org(), $1, $2, $3, $4, $5, $6::numeric, $7, $8, $8)",
        )
        .bind(batch)
        .bind(e.employee)
        .bind(&e.kind)
        .bind(e.leave_type)
        .bind(e.permission_type)
        .bind(&e.value)
        .bind(e.as_of)
        .bind(actor)
        .execute(&mut ***tx)
        .await
        .map_err(|err| {
            if err.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
                DomainError::Refused("ALREADY_EXISTS", "the batch already has a balance of that kind for that employee".into())
            } else {
                err.into()
            }
        })?;
    }
    Ok(())
}

pub async fn create_batch(tx: &mut OrgTx<'_>, actor: Uuid, entries: &[EntryIn], out_of_scope: &[usize]) -> Result<Value, DomainError> {
    let id: Uuid = sqlx::query_scalar("INSERT INTO openbal.opening_balance_batch (org_id, created_by, updated_by) VALUES (app.current_org(), $1, $1) RETURNING id")
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    insert_entries(tx, actor, id, entries, out_of_scope).await?;
    db::audit(tx, actor, "openbal.batch_created", "opening_balance_batch", Some(id), Some(json!({ "entries": entries.len() }))).await?;
    get_batch(tx, id).await
}

/// The batch row, locked; draft only for edits.
async fn lock_batch(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(String, Uuid, Option<Uuid>), DomainError> {
    let row: Option<(String, Option<Uuid>, Option<Uuid>)> = sqlx::query_as("SELECT state, created_by, submitted_by FROM openbal.opening_balance_batch WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?;
    let (state, created_by, submitted_by) = row.ok_or(DomainError::NotFound)?;
    Ok((state, created_by.unwrap_or_default(), submitted_by))
}

fn draft_only(state: &str) -> Result<(), DomainError> {
    if state != "draft" {
        return Err(DomainError::IllegalTransition("entries change only while the batch is a draft; a confirmed balance changes through /change (OB-02)".into()));
    }
    Ok(())
}

pub async fn add_entries(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, entries: &[EntryIn], out_of_scope: &[usize]) -> Result<Value, DomainError> {
    draft_only(&lock_batch(tx, batch).await?.0)?;
    insert_entries(tx, actor, batch, entries, out_of_scope).await?;
    sqlx::query("UPDATE openbal.opening_balance_batch SET updated_by = $2 WHERE id = $1").bind(batch).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, "openbal.entries_added", "opening_balance_batch", Some(batch), Some(json!({ "entries": entries.len() }))).await?;
    get_batch(tx, batch).await
}

/// Entry within its batch (NotFound when the ids do not belong together).
async fn entry_of(tx: &mut OrgTx<'_>, batch: Uuid, entry: Uuid) -> Result<(Uuid, String, Option<Uuid>), DomainError> {
    let row: Option<(Uuid, String, Option<Uuid>)> = sqlx::query_as("SELECT employee_id, balance_kind, leave_type_id FROM openbal.opening_balance_entry WHERE id = $1 AND batch_id = $2")
        .bind(entry)
        .bind(batch)
        .fetch_optional(&mut ***tx)
        .await?;
    row.ok_or(DomainError::NotFound)
}

pub async fn edit_entry(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, entry: Uuid, value: Option<&str>, as_of: Option<NaiveDate>) -> Result<Value, DomainError> {
    draft_only(&lock_batch(tx, batch).await?.0)?;
    let (employee, kind, leave_type) = entry_of(tx, batch, entry).await?;
    let current = get_entry(tx, entry).await?;
    let e = EntryIn {
        employee,
        kind,
        leave_type,
        permission_type: None,
        value: value.map(str::to_string).unwrap_or_else(|| current["value"].as_str().unwrap_or("").to_string()),
        as_of: as_of.or_else(|| current["as_of_date"].as_str().and_then(|d| d.parse().ok())).ok_or_else(|| DomainError::Validation("as_of_date".into()))?,
    };
    if let Some(p) = check_entry(tx, &e).await? {
        return Err(DomainError::Validation(p));
    }
    sqlx::query("UPDATE openbal.opening_balance_entry SET value = $2::numeric, as_of_date = $3, updated_by = $4 WHERE id = $1")
        .bind(entry)
        .bind(&e.value)
        .bind(e.as_of)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "openbal.entry_edited", "opening_balance_entry", Some(entry), Some(json!({ "before": current, "value": e.value, "as_of_date": e.as_of }))).await?;
    get_entry(tx, entry).await
}

pub async fn remove_entry(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, entry: Uuid) -> Result<(), DomainError> {
    draft_only(&lock_batch(tx, batch).await?.0)?;
    entry_of(tx, batch, entry).await?;
    let before = get_entry(tx, entry).await?;
    sqlx::query("DELETE FROM openbal.opening_balance_entry WHERE id = $1").bind(entry).execute(&mut ***tx).await?;
    db::audit(tx, actor, "openbal.entry_removed", "opening_balance_entry", Some(entry), Some(before)).await?;
    Ok(())
}

/// Whoever created the batch or any of its entries, or submitted it, never approves it (BR-07).
async fn involved(tx: &mut OrgTx<'_>, batch: Uuid, actor: Uuid, created_by: Uuid, submitted_by: Option<Uuid>) -> Result<bool, DomainError> {
    let entered: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM openbal.opening_balance_entry WHERE batch_id = $1 AND created_by = $2)")
        .bind(batch)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    Ok(entered || actor == created_by || submitted_by == Some(actor))
}

/// B4: draft -> pending approval (OB-01 = with Organisation Admin approval only).
pub async fn submit(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid) -> Result<Value, DomainError> {
    let (state, ..) = lock_batch(tx, batch).await?;
    if state == "pending_approval" {
        return get_batch(tx, batch).await;
    }
    if !settings(tx).await?.0 {
        return Err(DomainError::IllegalTransition("OB-01 = HR Manager: confirm the batch directly".into()));
    }
    draft_only(&state)?;
    sqlx::query("UPDATE openbal.opening_balance_batch SET state = 'pending_approval', submitted_by = $2, updated_by = $2 WHERE id = $1").bind(batch).bind(actor).execute(&mut ***tx).await?;
    step(tx, Step { kind: "opening_balance", request_id: batch, level: 1, action: "submitted", requester: actor, actor: Some(actor), on_behalf_of: None, comment: None }).await?;
    db::audit(tx, actor, "openbal.batch_submitted", "opening_balance_batch", Some(batch), None).await?;
    get_batch(tx, batch).await
}

/// B3 (HR, OB-01 = HR only) or B5 (approval). Posts each leave entry into its ledger once; counters are read from
/// confirmed entries by the counter rebuild. Confirming again -> unchanged, no second posting.
pub async fn confirm(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, comment: Option<&str>) -> Result<Value, DomainError> {
    let (state, created_by, submitted_by) = lock_batch(tx, batch).await?;
    let approval = settings(tx).await?.0;
    match state.as_str() {
        "confirmed" => return get_batch(tx, batch).await,
        "draft" if approval => return Err(DomainError::IllegalTransition("OB-01 needs Organisation Admin approval: submit the batch first".into())),
        "draft" => {}
        _ => {
            if involved(tx, batch, actor, created_by, submitted_by).await? {
                return Err(DomainError::SelfApproval);
            }
            let requester = submitted_by.unwrap_or(created_by);
            step(tx, Step { kind: "opening_balance", request_id: batch, level: 1, action: "approved", requester, actor: Some(actor), on_behalf_of: None, comment }).await?;
        }
    }
    let leave: Vec<(Uuid, Uuid, Uuid, String, NaiveDate)> = sqlx::query_as(
        "SELECT id, employee_id, leave_type_id, value::text, as_of_date FROM openbal.opening_balance_entry
         WHERE batch_id = $1 AND balance_kind = 'leave' AND superseded_by_entry_id IS NULL AND value <> 0",
    )
    .bind(batch)
    .fetch_all(&mut ***tx)
    .await?;
    for (entry, employee, leave_type, value, as_of) in leave {
        post_opening(tx, actor, entry, employee, leave_type, &value, as_of).await?;
    }
    sqlx::query("UPDATE openbal.opening_balance_batch SET state = 'confirmed', confirmed_by = $2, confirmed_at = now(), updated_by = $2 WHERE id = $1")
        .bind(batch)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "openbal.batch_confirmed", "opening_balance_batch", Some(batch), Some(json!({ "comment": comment }))).await?;
    get_batch(tx, batch).await
}

async fn post_opening(tx: &mut OrgTx<'_>, actor: Uuid, entry: Uuid, employee: Uuid, leave_type: Uuid, value: &str, as_of: NaiveDate) -> Result<(), DomainError> {
    let account = crate::leave::lock_account(tx, employee, leave_type).await?;
    sqlx::query(
        "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reason, created_by)
         VALUES (app.current_org(), $1, $2, 'opening', $3::numeric, 'opening_balance_entry', $4, 'opening balance (OB-R4)', $5) ON CONFLICT DO NOTHING",
    )
    .bind(account)
    .bind(as_of)
    .bind(value)
    .bind(entry)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// B6 (GAP-SM-13, decided here): pending approval -> back to draft for correction, with a reason.
pub async fn reject(tx: &mut OrgTx<'_>, actor: Uuid, batch: Uuid, reason: &str) -> Result<Value, DomainError> {
    let (state, created_by, submitted_by) = lock_batch(tx, batch).await?;
    if state != "pending_approval" {
        return Err(DomainError::IllegalTransition(format!("the batch is {state}, not waiting for approval")));
    }
    if involved(tx, batch, actor, created_by, submitted_by).await? {
        return Err(DomainError::SelfApproval);
    }
    step(tx, Step { kind: "opening_balance", request_id: batch, level: 1, action: "rejected", requester: submitted_by.unwrap_or(created_by), actor: Some(actor), on_behalf_of: None, comment: Some(reason) }).await?;
    sqlx::query("UPDATE openbal.opening_balance_batch SET state = 'draft', submitted_by = NULL, updated_by = $2 WHERE id = $1").bind(batch).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, "openbal.batch_rejected", "opening_balance_batch", Some(batch), Some(json!({ "reason": reason }))).await?;
    get_batch(tx, batch).await
}

/// batch, employee, kind, leave type, as-of date, superseded by.
type EntryRow = (Uuid, Uuid, String, Option<Uuid>, NaiveDate, Option<Uuid>);

/// B7 (OB-02): a new entry supersedes a confirmed one; leave reverses the old opening and posts the new one.
pub async fn change_entry(tx: &mut OrgTx<'_>, actor: Uuid, entry: Uuid, value: &str, as_of: Option<NaiveDate>, reason: &str) -> Result<Value, DomainError> {
    if !settings(tx).await?.1 {
        return Err(DomainError::Refused("RULE_GROUP_DISABLED", "the organisation does not allow changes after confirmation (OB-02)".into()));
    }
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(DomainError::Validation("reason is required (1-1000 characters)".into()));
    }
    let row: Option<EntryRow> = sqlx::query_as(
        "SELECT batch_id, employee_id, balance_kind, leave_type_id, as_of_date, superseded_by_entry_id FROM openbal.opening_balance_entry WHERE id = $1 FOR UPDATE",
    )
    .bind(entry)
    .fetch_optional(&mut ***tx)
    .await?;
    let (batch, employee, kind, leave_type, old_as_of, superseded) = row.ok_or(DomainError::NotFound)?;
    if lock_batch(tx, batch).await?.0 != "confirmed" || superseded.is_some() {
        return Err(DomainError::IllegalTransition("only a live entry of a confirmed batch is changed this way".into()));
    }
    let new = EntryIn { employee, kind: kind.clone(), leave_type, permission_type: None, value: value.to_string(), as_of: as_of.unwrap_or(old_as_of) };
    if let Some(p) = check_entry(tx, &new).await? {
        return Err(DomainError::Validation(p));
    }
    let before = get_entry(tx, entry).await?;
    let id = Uuid::new_v4();
    // The pointer first (deferred FK), then the replacement: the live-entry index never sees two.
    sqlx::query("UPDATE openbal.opening_balance_entry SET superseded_by_entry_id = $2, updated_by = $3 WHERE id = $1").bind(entry).bind(id).bind(actor).execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO openbal.opening_balance_entry (org_id, id, batch_id, employee_id, balance_kind, leave_type_id, value, as_of_date, change_reason, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6::numeric, $7, $8, $9, $9)",
    )
    .bind(id)
    .bind(batch)
    .bind(employee)
    .bind(&kind)
    .bind(leave_type)
    .bind(value)
    .bind(new.as_of)
    .bind(reason.trim())
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    if let Some(t) = leave_type {
        let old: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, account_id FROM leave.leave_ledger_entry WHERE cause_kind = 'opening_balance_entry' AND cause_id = $1 AND entry_kind = 'opening'",
        )
        .bind(entry)
        .fetch_optional(&mut ***tx)
        .await?;
        if let Some((ledger, account)) = old {
            sqlx::query(
                "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reverses_entry_id, reason, created_by)
                 SELECT app.current_org(), $1, $3, 'reversal', -quantity, 'opening_balance_entry', $2, id, $4, $5 FROM leave.leave_ledger_entry WHERE id = $6",
            )
            .bind(account)
            .bind(entry)
            .bind(new.as_of)
            .bind(reason.trim())
            .bind(actor)
            .bind(ledger)
            .execute(&mut ***tx)
            .await?;
        }
        if value.parse::<f64>().is_ok_and(|v| v != 0.0) {
            post_opening(tx, actor, id, employee, t, value, new.as_of).await?;
        }
    }
    let after = get_entry(tx, id).await?;
    db::audit(tx, actor, "openbal.entry_changed", "opening_balance_entry", Some(id), Some(json!({ "before": before, "after": after, "reason": reason.trim() }))).await?;
    Ok(after)
}

#[cfg(test)]
mod tests {
    use super::value_ok;

    #[test]
    fn values() {
        assert!(value_ok("12.50") && value_ok("-1.00") && value_ok("0.00") && value_ok("99999.99"));
        assert!(!value_ok("12.5") && !value_ok("1") && !value_ok("123456.00") && !value_ok("1.0a") && !value_ok("--1.00"));
    }
}
