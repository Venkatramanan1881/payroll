//! Compensatory off (M02, SM-COMPOFF, CO-R1..CO-R4, HW-01..HW-06). A credit is earned by work on a weekly off or
//! holiday (or an approved on-duty there, OD-10), approved on SM-REQ (one level, CMP-05) unless HW-02 is off, taken as
//! leave of the reserved type COMP_OFF (D-25) earliest-expiring first, and lapses at expiry (D-26, J6).

use crate::approval::{self, Step};
use chrono::{Duration, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use domain_iam::authz::Principal;
use domain_policy::hw::{self, HwSettings};
use engine::day::DayOutput;
use serde_json::{json, Value};
use uuid::Uuid;

const KIND: &str = "comp_off";
/// The leave type code that takes comp-off (D-25).
pub const LEAVE_CODE: &str = "COMP_OFF";

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

/// One employee's comp-off writes (earning, decisions, debits, reversals, expiry) run one at a time, so every ledger
/// sum read after this lock is current.
async fn lock_employee(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('comp_off:' || $1::text))").bind(employee).execute(&mut ***tx).await?;
    Ok(())
}

async fn employee_user(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<Option<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT id FROM iam.user_account WHERE employee_id = $1 ORDER BY created_at LIMIT 1").bind(employee).fetch_optional(&mut ***tx).await?)
}

/// The HW settings in force for the employee on a date.
pub async fn settings_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<HwSettings, DomainError> {
    let Some(master) = domain_timecap::punch::master_branch_on(tx, employee, date).await? else { return Ok(HwSettings::default()) };
    let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
    hw::load(tx, resolved.and_then(|r| r.version_id)).await
}

async fn post_credit(tx: &mut OrgTx<'_>, credit: Uuid, date: NaiveDate, pct: i64, actor: Option<Uuid>) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO leave.comp_off_ledger_entry (org_id, credit_id, entry_date, entry_kind, quantity, cause_kind, cause_id, created_by)
         VALUES (app.current_org(), $1, $2, 'credit', $3::numeric / 100, 'comp_off_credit', $1, $4) ON CONFLICT DO NOTHING",
    )
    .bind(credit)
    .bind(date)
    .bind(pct as i32)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// SM-COMPOFF C1/C3/C5 after a day result: a weekly off or holiday worked for HW-04 hours (or covered by approved
/// on-duty with OD-10) earns a credit. A pending credit follows later recalculations; a credited one is never changed
/// by them (OQ-M02-04). Only days already over earn.
pub async fn on_day(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, policy_version: Uuid, out: &DayOutput) -> Result<(), DomainError> {
    let hw = hw::load(tx, Some(policy_version)).await?;
    let off = matches!(out.status, "weekly_off" | "holiday");
    let on_duty = out.flags.iter().any(|f| f == "on_duty") && domain_policy::od::load(tx, Some(policy_version)).await?.comp_off_on_off_days;
    let pct = match (off && hw.comp_off, on_duty) {
        (false, _) => 0,
        (true, true) => 100,
        (true, false) => hw.earned_pct(out.net_min),
    };
    lock_employee(tx, employee).await?;
    // SM-COMPOFF: a claim an approver rejected is final; only the system's own withdrawal lets the day earn again.
    let rejected: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.comp_off_credit WHERE employee_id = $1 AND worked_date = $2 AND state = 'not_credited' AND decided_by IS NOT NULL)")
        .bind(employee)
        .bind(date)
        .fetch_one(&mut ***tx)
        .await?;
    if rejected {
        return Ok(());
    }
    let existing: Option<(Uuid, String, i64)> = sqlx::query_as(
        "SELECT id, state, (fraction * 100)::int8 FROM leave.comp_off_credit WHERE employee_id = $1 AND worked_date = $2 AND state <> 'not_credited' FOR UPDATE",
    )
    .bind(employee)
    .bind(date)
    .fetch_optional(&mut ***tx)
    .await?;
    match existing {
        None if pct == 0 => Ok(()),
        None => {
            let requester = employee_user(tx, employee).await?;
            let (state, approver) = if hw.approval {
                ("pending_approval", approval::route(tx, KIND, employee, 1, Utc::now().date_naive()).await?)
            } else {
                ("credited", None)
            };
            let expires = (state == "credited").then(|| date + Duration::days(hw.expiry_days));
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO leave.comp_off_credit (org_id, employee_id, worked_date, fraction, earned_basis, state, expires_on, current_approver_user_id, decided_at)
                 VALUES (app.current_org(), $1, $2, $3::numeric / 100, 'policy', $4, $5, $6, CASE WHEN $4 = 'credited' THEN now() END) RETURNING id",
            )
            .bind(employee)
            .bind(date)
            .bind(pct as i32)
            .bind(state)
            .bind(expires)
            .bind(approver)
            .fetch_one(&mut ***tx)
            .await?;
            // shortcut: an employee with no user account gets no approval steps (the step needs a requester); the audit entry stays.
            if let Some(user) = requester {
                let action = if hw.approval { "submitted" } else { "auto_approved" };
                approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action, requester: user, actor: None, on_behalf_of: None, comment: None }).await?;
            }
            if !hw.approval {
                post_credit(tx, id, date, pct, None).await?;
            }
            db::audit_system(tx, &format!("comp_off_credit.{}", if hw.approval { "earn" } else { "credit_by_system" }), "comp_off_credit", Some(id), Some(json!({ "worked_date": date, "fraction_pct": pct }))).await?;
            event(tx, if hw.approval { "RequestSubmitted" } else { "CompOffCredited" }, id, employee).await
        }
        Some((id, state, was)) if state == "pending_approval" && pct != was => {
            let requester = employee_user(tx, employee).await?;
            if pct == 0 {
                sqlx::query("UPDATE leave.comp_off_credit SET state = 'not_credited', current_approver_user_id = NULL, decided_at = now(), updated_at = now() WHERE id = $1")
                    .bind(id)
                    .execute(&mut ***tx)
                    .await?;
                if let Some(user) = requester {
                    approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action: "auto_rejected", requester: user, actor: None, on_behalf_of: None, comment: Some("the day no longer earns comp-off") }).await?;
                }
            } else {
                sqlx::query("UPDATE leave.comp_off_credit SET fraction = $2::numeric / 100, updated_at = now() WHERE id = $1").bind(id).bind(pct as i32).execute(&mut ***tx).await?;
            }
            db::audit_system(tx, "comp_off_credit.recalculated", "comp_off_credit", Some(id), Some(json!({ "fraction_pct": pct, "was": was }))).await?;
            Ok(())
        }
        Some(_) => Ok(()),
    }
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

/// employee, worked date, state, fraction (hundredths), approver, decider.
type CreditRow = (Uuid, NaiveDate, String, i64, Option<Uuid>, Option<Uuid>);

/// SM-COMPOFF C2 / C4 (CMP-05, one level). Credited answers approve with 200 and no second entry; not_credited
/// answers reject with 200; the opposite decision is ILLEGAL_TRANSITION.
pub async fn decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, approve: bool, note: Option<&str>) -> Result<(), DomainError> {
    let employee: Uuid = sqlx::query_scalar("SELECT employee_id FROM leave.comp_off_credit WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)?;
    lock_employee(tx, employee).await?;
    let row: Option<CreditRow> = sqlx::query_as(
        "SELECT employee_id, worked_date, state, (fraction * 100)::int8, current_approver_user_id, decided_by FROM leave.comp_off_credit WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let (employee, worked, state, pct, approver, decided_by) = row.ok_or(DomainError::NotFound)?;
    if p.employee_id == Some(employee) {
        return Err(DomainError::SelfApproval); // BR-07
    }
    let on_behalf_of = approval::may_decide(tx, p, employee, "CMP-05", approver).await?;
    if state != "pending_approval" && decided_by.is_some_and(|d| d != p.user_id) {
        return Err(DomainError::NotFound); // a closed credit answers only to whoever decided it
    }
    match (state.as_str(), approve) {
        ("pending_approval", _) => {}
        ("not_credited", false) => return Ok(()),
        ("credited" | "partially_used" | "used" | "expired_lapsed", true) => return Ok(()),
        (s, _) => return Err(DomainError::IllegalTransition(format!("credit is {s}"))),
    }
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    if !approve && note.is_none() {
        return Err(invalid("reason is required to reject"));
    }
    let requester = employee_user(tx, employee).await?;
    if approve {
        let hw = settings_on(tx, employee, worked).await?;
        sqlx::query(
            "UPDATE leave.comp_off_credit SET state = 'credited', expires_on = $2, decided_by = $3, decided_at = now(), current_approver_user_id = NULL,
               updated_at = now(), updated_by = $3 WHERE id = $1",
        )
        .bind(id)
        .bind(worked + Duration::days(hw.expiry_days)) // D-27: calendar days from the worked date
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
        let today = crate::period::org_today(tx).await?;
        post_credit(tx, id, today, pct, Some(p.user_id)).await?;
    } else {
        sqlx::query("UPDATE leave.comp_off_credit SET state = 'not_credited', decided_by = $2, decided_at = now(), current_approver_user_id = NULL, updated_at = now(), updated_by = $2 WHERE id = $1")
            .bind(id)
            .bind(p.user_id)
            .execute(&mut ***tx)
            .await?;
    }
    if let Some(user) = requester {
        let action = if approve { "approved" } else { "rejected" };
        approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action, requester: user, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
    }
    db::audit(tx, p.user_id, &format!("comp_off_credit.{}", if approve { "approve" } else { "reject" }), "comp_off_credit", Some(id), Some(json!({ "comment": note }))).await?;
    if approve {
        event(tx, "RequestApproved", id, employee).await?;
    }
    Ok(())
}

const CREDIT_JSON: &str = "jsonb_build_object('id', c.id, 'employee_id', c.employee_id, 'worked_date', c.worked_date, 'fraction', to_char(c.fraction, 'FM0.00'),
    'employee_code', (SELECT e.employee_code FROM workforce.employee e WHERE e.id = c.employee_id),
    'employee_name', (SELECT e.full_name FROM workforce.employee e WHERE e.id = c.employee_id),
    'earned_basis', c.earned_basis, 'state', c.state, 'expires_on', c.expires_on, 'escalated', c.escalated,
    'current_approver_user_id', c.current_approver_user_id, 'decided_at', c.decided_at, 'decided_by', c.decided_by,
    'remaining', to_char(coalesce((SELECT sum(e.quantity) FROM leave.comp_off_ledger_entry e WHERE e.credit_id = c.id), 0), 'FM990.00'),
    'steps', ";

fn credit_json() -> String {
    format!("{CREDIT_JSON}{})", approval::steps_sql(KIND, "c.id"))
}

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {} FROM leave.comp_off_credit c WHERE c.id = $1", credit_json()))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub struct Filter {
    pub employees: Option<Vec<Uuid>>,
    pub approver: Uuid,
    pub employee: Option<Uuid>,
    pub state: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// Whether `decide` would accept the caller now: a pending credit, never one's own, CMP-05 with routing or scope.
pub async fn can_decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid) -> Result<bool, DomainError> {
    let r: Option<(Uuid, Option<Uuid>)> =
        sqlx::query_as("SELECT employee_id, current_approver_user_id FROM leave.comp_off_credit WHERE id = $1 AND state = 'pending_approval'").bind(id).fetch_optional(&mut ***tx).await?;
    let Some((employee, approver)) = r else { return Ok(false) };
    if p.employee_id == Some(employee) {
        return Ok(false);
    }
    match approval::may_decide(tx, p, employee, "CMP-05", approver).await {
        Ok(_) => Ok(true),
        Err(DomainError::NotFound | DomainError::Forbidden) => Ok(false),
        Err(e) => Err(e),
    }
}

/// The credits waiting for the caller's decision: exactly those `can_decide` accepts, newest first.
pub async fn awaiting(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Vec<Value>, DomainError> {
    let mut out = Vec::new();
    for id in approval::awaiting_candidates(tx, p, "leave.comp_off_credit", "q.created_at", &["pending_approval"]).await? {
        if can_decide(tx, p, id).await? {
            out.push(get(tx, id).await?);
        }
    }
    Ok(out)
}

pub async fn list(tx: &mut OrgTx<'_>, f: &Filter) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {} FROM leave.comp_off_credit c
         WHERE ($1::uuid[] IS NULL OR c.employee_id = ANY($1) OR c.current_approver_user_id = $6 OR c.decided_by = $6)
           AND ($2::uuid IS NULL OR c.employee_id = $2) AND ($3::text IS NULL OR c.state = $3)
           AND ($4::date IS NULL OR c.worked_date >= $4) AND ($5::date IS NULL OR c.worked_date <= $5)
         ORDER BY c.worked_date DESC",
        credit_json()
    ))
    .bind(&f.employees)
    .bind(f.employee)
    .bind(&f.state)
    .bind(f.from)
    .bind(f.to)
    .bind(f.approver)
    .fetch_all(&mut ***tx)
    .await?)
}

/// CO-R2 / LM-R8: balance = the sum of the employee's comp-off ledger, with open credits and pending days.
pub async fn balance(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<Value, DomainError> {
    let (balance, pending): (String, String) = sqlx::query_as(
        "SELECT to_char(coalesce((SELECT sum(e.quantity) FROM leave.comp_off_ledger_entry e JOIN leave.comp_off_credit c ON c.id = e.credit_id WHERE c.employee_id = $1), 0), 'FM990.00'),
                to_char(coalesce((SELECT sum(fraction) FROM leave.comp_off_credit WHERE employee_id = $1 AND state = 'pending_approval'), 0), 'FM990.00')",
    )
    .bind(employee)
    .fetch_one(&mut ***tx)
    .await?;
    let open: Vec<Value> = sqlx::query_scalar(&format!(
        "SELECT {} FROM leave.comp_off_credit c WHERE c.employee_id = $1 AND c.state IN ('credited','partially_used') ORDER BY c.expires_on NULLS LAST, c.worked_date",
        credit_json()
    ))
    .bind(employee)
    .fetch_all(&mut ***tx)
    .await?;
    Ok(json!({ "employee_id": employee, "balance": balance, "pending_approval_days": pending, "open_credits": open }))
}

pub async fn ledger(tx: &mut OrgTx<'_>, employee: Uuid, from: Option<NaiveDate>, to: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('id', e.id, 'credit_id', e.credit_id, 'entry_date', e.entry_date, 'entry_kind', e.entry_kind,
                 'quantity', to_char(e.quantity, 'FM990.00'), 'cause_kind', e.cause_kind, 'cause_id', e.cause_id, 'reason', e.reason,
                 'reverses_entry_id', e.reverses_entry_id, 'created_at', e.created_at)
         FROM leave.comp_off_ledger_entry e JOIN leave.comp_off_credit c ON c.id = e.credit_id
         WHERE c.employee_id = $1 AND ($2::date IS NULL OR e.entry_date >= $2) AND ($3::date IS NULL OR e.entry_date <= $3)
         ORDER BY e.created_at DESC, e.id",
    )
    .bind(employee)
    .bind(from)
    .bind(to)
    .fetch_all(&mut ***tx)
    .await?)
}

// --- Taking comp-off as leave (CO-R2, SM-COMPOFF C6..C8, D-25) ---

pub async fn is_comp_off_type(tx: &mut OrgTx<'_>, leave_type: Uuid) -> Result<bool, DomainError> {
    Ok(sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM leave.leave_type WHERE id = $1 AND code = $2)").bind(leave_type).bind(LEAVE_CODE).fetch_one(&mut ***tx).await?)
}

/// Open credits usable for leave starting on `from` (not expired by then), earliest-expiring first (OQ-M02-09),
/// with what is left of each, in hundredths of a day. Only credits earned before the leave are usable. Callers hold
/// the employee lock.
async fn usable(tx: &mut OrgTx<'_>, employee: Uuid, from: NaiveDate) -> Result<Vec<(Uuid, i64, i64)>, DomainError> {
    Ok(sqlx::query_as(
        "SELECT c.id, (c.fraction * 100)::int8, coalesce((SELECT (sum(e.quantity) * 100)::int8 FROM leave.comp_off_ledger_entry e WHERE e.credit_id = c.id), 0)
         FROM leave.comp_off_credit c WHERE c.employee_id = $1 AND c.state IN ('credited','partially_used') AND c.worked_date < $2
           AND (c.expires_on IS NULL OR c.expires_on >= $2)
         ORDER BY c.expires_on NULLS LAST, c.worked_date",
    )
    .bind(employee)
    .bind(from)
    .fetch_all(&mut ***tx)
    .await?)
}

/// Comp-off available (hundredths of a day) for leave starting on `from`.
pub async fn available_pct(tx: &mut OrgTx<'_>, employee: Uuid, from: NaiveDate) -> Result<i64, DomainError> {
    lock_employee(tx, employee).await?;
    Ok(usable(tx, employee, from).await?.iter().map(|c| c.2).sum())
}

async fn set_state(tx: &mut OrgTx<'_>, credit: Uuid) -> Result<(), DomainError> {
    sqlx::query(
        "UPDATE leave.comp_off_credit c SET state = CASE WHEN r.rest <= 0 THEN 'used' WHEN r.rest < c.fraction THEN 'partially_used' ELSE 'credited' END, updated_at = now()
         FROM (SELECT coalesce(sum(quantity), 0) AS rest FROM leave.comp_off_ledger_entry WHERE credit_id = $1) r WHERE c.id = $1",
    )
    .bind(credit)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// C6/C7: debit an approved comp-off leave from the earliest-expiring credits (shortcut: usability is judged on the
/// leave's first day, not per day; split per day when OQ-M02-09 is answered).
pub async fn debit(tx: &mut OrgTx<'_>, actor: Option<Uuid>, employee: Uuid, request: Uuid, from: NaiveDate, mut pct: i64) -> Result<(), DomainError> {
    lock_employee(tx, employee).await?;
    let credits = usable(tx, employee, from).await?;
    if credits.iter().map(|c| c.2).sum::<i64>() < pct {
        return Err(DomainError::Refused("LIMIT_EXCEEDED", "not enough comp-off balance (CO-R2)".into()));
    }
    for (credit, _, left) in credits {
        if pct == 0 {
            break;
        }
        let take = left.min(pct);
        if take <= 0 {
            continue;
        }
        sqlx::query(
            "INSERT INTO leave.comp_off_ledger_entry (org_id, credit_id, entry_date, entry_kind, quantity, cause_kind, cause_id, created_by)
             VALUES (app.current_org(), $1, $2, 'debit', -($3::numeric / 100), 'leave_request', $4, $5)",
        )
        .bind(credit)
        .bind(from)
        .bind(take as i32)
        .bind(request)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
        set_state(tx, credit).await?;
        pct -= take;
    }
    Ok(())
}

/// C8: a cancelled comp-off leave gives its days back. A credit that has lapsed meanwhile stays lapsed (GAP-SM-15).
pub async fn reverse(tx: &mut OrgTx<'_>, actor: Uuid, employee: Uuid, request: Uuid, reason: &str) -> Result<(), DomainError> {
    lock_employee(tx, employee).await?;
    let credits: Vec<Uuid> = sqlx::query_scalar(
        "INSERT INTO leave.comp_off_ledger_entry (org_id, credit_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reverses_entry_id, reason, created_by)
         SELECT app.current_org(), e.credit_id, current_date, 'reversal', -e.quantity, 'leave_request', $1, e.id, $2, $3
         FROM leave.comp_off_ledger_entry e JOIN leave.comp_off_credit c ON c.id = e.credit_id
         WHERE e.cause_kind = 'leave_request' AND e.cause_id = $1 AND e.entry_kind = 'debit' AND c.state IN ('credited','partially_used','used')
         ON CONFLICT DO NOTHING RETURNING credit_id",
    )
    .bind(request)
    .bind(reason)
    .bind(actor)
    .fetch_all(&mut ***tx)
    .await?;
    for c in credits {
        set_state(tx, c).await?;
    }
    Ok(())
}

/// J6 (C9, D-26): credits past their expiry lapse with an expiry entry for what is left.
pub async fn expire_due(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let today = crate::period::org_today(tx).await?;
    let due: Vec<(Uuid, Uuid, NaiveDate)> = sqlx::query_as(
        "SELECT id, employee_id, expires_on FROM leave.comp_off_credit
         WHERE state IN ('credited','partially_used') AND expires_on < $1 ORDER BY expires_on",
    )
    .bind(today)
    .fetch_all(&mut ***tx)
    .await?;
    let mut lapsed = 0;
    for (id, employee, expires) in &due {
        lock_employee(tx, *employee).await?;
        let still: bool = sqlx::query_scalar("SELECT state IN ('credited','partially_used') FROM leave.comp_off_credit WHERE id = $1").bind(id).fetch_one(&mut ***tx).await?;
        if !still {
            continue;
        }
        lapsed += 1;
        sqlx::query(
            "INSERT INTO leave.comp_off_ledger_entry (org_id, credit_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reason)
             SELECT app.current_org(), $1, $2 + 1, 'expiry', -rest, 'expiry_run', $1, 'comp-off expired unused (HW-05, HW-06 lapse)'
             FROM (SELECT sum(quantity) AS rest FROM leave.comp_off_ledger_entry WHERE credit_id = $1) r WHERE rest > 0
             ON CONFLICT DO NOTHING",
        )
        .bind(id)
        .bind(expires)
        .execute(&mut ***tx)
        .await?;
        sqlx::query("UPDATE leave.comp_off_credit SET state = 'expired_lapsed', updated_at = now() WHERE id = $1").bind(id).execute(&mut ***tx).await?;
        db::audit_system(tx, "comp_off_credit.lapse", "comp_off_credit", Some(*id), Some(json!({ "expires_on": expires }))).await?;
        event(tx, "CompOffExpired", *id, *employee).await?;
    }
    Ok(json!({ "lapsed": lapsed }))
}
