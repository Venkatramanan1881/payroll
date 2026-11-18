//! Punch flag decisions (SM-FLAG F2 accept / F3 reject). The punch itself is never changed (BR-01).

use crate::punch::outbox;
use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

const FLAG_JSON: &str = "jsonb_build_object('id', f.id, 'punch_id', f.punch_id, 'employee_id', p.employee_id, 'reason', f.reason, 'state', f.state,
    'counts_before_decision', f.counts_before_decision, 'evidence', f.evidence, 'created_at', f.created_at,
    'decision', (SELECT jsonb_build_object('decision', d.decision, 'decided_by', d.decided_by, 'on_behalf_of_user_id', NULL,
        'reason', d.reason, 'decided_at', d.decided_at) FROM timecap.punch_flag_decision d WHERE d.flag_id = f.id ORDER BY d.decided_at DESC LIMIT 1))";

pub async fn get_flag(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {FLAG_JSON} FROM timecap.punch_flag f JOIN timecap.raw_punch p ON p.id = f.punch_id WHERE f.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// The employee whose punch carries the flag (for scope and BR-07) and the punch time (for recalculation).
pub async fn flag_punch(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(Uuid, chrono::DateTime<chrono::Utc>), DomainError> {
    let row: Option<(Option<Uuid>, chrono::DateTime<chrono::Utc>)> =
        sqlx::query_as("SELECT p.employee_id, p.event_at FROM timecap.punch_flag f JOIN timecap.raw_punch p ON p.id = f.punch_id WHERE f.id = $1")
            .bind(id)
            .fetch_optional(&mut ***tx)
            .await?;
    match row {
        Some((Some(e), at)) => Ok((e, at)),
        _ => Err(DomainError::NotFound),
    }
}

pub struct FlagFilter {
    pub employees: Option<Vec<Uuid>>,
    pub state: String,
    pub reason: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

// shortcut: no cursor paging, oldest 500 only; add keyset paging on (created_at, id) when queues grow.
pub async fn list_flags(tx: &mut OrgTx<'_>, f: &FlagFilter) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {FLAG_JSON} FROM timecap.punch_flag f JOIN timecap.raw_punch p ON p.id = f.punch_id
         WHERE ($1::uuid[] IS NULL OR p.employee_id = ANY($1)) AND f.state = $2 AND ($3::text IS NULL OR f.reason = $3)
           AND ($4::date IS NULL OR (p.event_at AT TIME ZONE p.captured_tz)::date >= $4) AND ($5::date IS NULL OR (p.event_at AT TIME ZONE p.captured_tz)::date <= $5)
         ORDER BY f.created_at, f.id LIMIT 500"
    );
    Ok(sqlx::query_scalar(&sql).bind(&f.employees).bind(&f.state).bind(&f.reason).bind(f.from).bind(f.to).fetch_all(&mut ***tx).await?)
}

/// Accepts or rejects a raised flag. Repeating the same decision changes nothing; the opposite decision after one is
/// ILLEGAL_TRANSITION (reversal is GAP-SM-06). Returns true when a decision was recorded.
pub async fn decide(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, accept: bool, reason: Option<&str>) -> Result<bool, DomainError> {
    let reason = reason.map(str::trim).filter(|r| !r.is_empty());
    if !accept && reason.is_none() {
        return Err(invalid("reason is required to reject"));
    }
    let state: String = sqlx::query_scalar("SELECT state FROM timecap.punch_flag WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)?;
    let target = if accept { "accepted" } else { "rejected" };
    if state == target {
        return Ok(false);
    }
    if state != "raised" {
        return Err(DomainError::IllegalTransition(format!("flag is already {state}")));
    }
    sqlx::query("INSERT INTO timecap.punch_flag_decision (org_id, flag_id, decision, decided_by, reason, created_by) VALUES (app.current_org(), $1, $2, $3, $4, $3)")
        .bind(id)
        .bind(target)
        .bind(actor)
        .bind(reason)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("UPDATE timecap.punch_flag SET state = $2, updated_at = now(), updated_by = $3 WHERE id = $1").bind(id).bind(target).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, &format!("punch_flag.{}", if accept { "accept" } else { "reject" }), "punch_flag", Some(id), Some(json!({ "reason": reason }))).await?;
    outbox(tx, "PunchFlagDecided", id, json!({ "decision": target })).await?;
    Ok(true)
}
