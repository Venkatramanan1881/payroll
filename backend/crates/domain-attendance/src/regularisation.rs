//! Regularisation requests (RG-01..11) on the shared approval flow (SM-REQ R1-R6, R10; AC-02, AC-03, BR-07).
//! Approval steps are read back from the audit log; an approval adds manual punches (BR-01) and recalculates the day.

use chrono::{DateTime, NaiveDate, Utc};
use db::{invalid, DomainError, OrgTx};
use domain_iam::authz::Principal;
use domain_policy::rg::{self, RgSettings};
use domain_timecap::punch::{branch_tz, master_branch_on};
use serde_json::{json, Value};
use uuid::Uuid;

const KIND: &str = "regularisation";

/// employee, date, type, proposed in, proposed out, target punch, reason.
type RequestRow = (Uuid, NaiveDate, String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<Uuid>, String);
type Span = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

pub struct NewRequest {
    pub employee: Uuid,
    pub date: NaiveDate,
    pub request_type: String,
    pub proposed_in: Option<DateTime<Utc>>,
    pub proposed_out: Option<DateTime<Utc>>,
    pub target: Option<Uuid>,
    pub reason: String,
    pub attachment: Option<String>,
}

fn refused(code: &'static str, detail: impl Into<String>) -> DomainError {
    DomainError::Refused(code, detail.into())
}

async fn settings_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, master: Uuid) -> Result<RgSettings, DomainError> {
    let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
    rg::load(tx, resolved.and_then(|r| r.version_id)).await
}

/// Today in the employee's branch zone (BR-11).
async fn local_today(tx: &mut OrgTx<'_>, branch: Uuid) -> Result<NaiveDate, DomainError> {
    let tz = branch_tz(tx, branch, Utc::now()).await?;
    Ok(sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(tz).fetch_one(&mut ***tx).await?)
}

/// Shape checks per type, and the proposed times must sit within a day either side of the date.
async fn check_shape(tx: &mut OrgTx<'_>, r: &NewRequest, master: Uuid) -> Result<(), DomainError> {
    if r.reason.trim().is_empty() {
        return Err(invalid("reason is required (RG-09)"));
    }
    if !rg::TYPES.contains(&r.request_type.as_str()) {
        return Err(invalid("request_type"));
    }
    let (i, o) = (r.proposed_in.is_some(), r.proposed_out.is_some());
    let ok = match r.request_type.as_str() {
        "missed_punch" => i || o,
        "late_arrival" => i,
        "early_checkout" => o,
        "whole_day" => i && o,
        _ => r.target.is_some(), // wrong_punch
    };
    if !ok {
        return Err(invalid(format!("{} needs its proposed time(s){}", r.request_type, if r.request_type == "wrong_punch" { " and target_raw_punch_id" } else { "" })));
    }
    if let (Some(a), Some(b)) = (r.proposed_in, r.proposed_out) {
        if b <= a {
            return Err(invalid("proposed_out_at must be after proposed_in_at"));
        }
    }
    let tz = branch_tz(tx, master, Utc::now()).await?;
    for t in [r.proposed_in, r.proposed_out].into_iter().flatten() {
        let near: bool = sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE $3)::date BETWEEN $2::date - 1 AND $2::date + 1")
            .bind(t)
            .bind(r.date)
            .bind(&tz)
            .fetch_one(&mut ***tx)
            .await?;
        if !near {
            return Err(invalid("proposed times must fall within a day of attendance_date"));
        }
    }
    if let Some(target) = r.target {
        // The disputed punch must be this employee's and belong to this day (or its edges), so a request can
        // never reach into an old or locked day (RG-04).
        let own: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM timecap.raw_punch WHERE id = $1 AND employee_id = $2
                            AND (event_at AT TIME ZONE $4)::date BETWEEN $3::date - 1 AND $3::date + 1)",
        )
        .bind(target)
        .bind(r.employee)
        .bind(r.date)
        .bind(&tz)
        .fetch_one(&mut ***tx)
        .await?;
        if !own {
            return Err(invalid("target_raw_punch_id is not a punch of this employee on attendance_date"));
        }
    }
    if let Some(path) = &r.attachment {
        let ours: bool = sqlx::query_scalar("SELECT starts_with($1, app.current_org()::text || '/') AND position('..' in $1) = 0").bind(path).fetch_one(&mut ***tx).await?;
        if !ours {
            return Err(invalid("attachment.path must be a file of this organisation"));
        }
    }
    Ok(())
}

async fn route(tx: &mut OrgTx<'_>, employee: Uuid, level: i16, today: NaiveDate) -> Result<Option<Uuid>, DomainError> {
    crate::approval::route(tx, KIND, employee, level, today).await
}

fn audit_step(action: &str, level: i16, comment: Option<&str>, on_behalf_of: Option<Uuid>) -> Value {
    json!({ "action": action, "level": level, "comment": comment, "on_behalf_of": on_behalf_of })
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

async fn insert(tx: &mut OrgTx<'_>, user: Uuid, r: &NewRequest, origin: &str, state: &str, approver: Option<Uuid>, basis: Option<&str>) -> Result<Uuid, DomainError> {
    let decided = basis.is_some();
    Ok(sqlx::query_scalar(
        "INSERT INTO attendance.regularisation_request (org_id, employee_id, attendance_date, request_type, origin, state, decided_basis,
           proposed_in_at, proposed_out_at, target_raw_punch_id, reason, attachment_path, requested_by_user_id, current_approver_user_id,
           decided_by_user_id, decided_at, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13,
           CASE WHEN $14 AND $4 = 'hr_direct' THEN $12 END, CASE WHEN $14 THEN now() END, $12, $12) RETURNING id",
    )
    .bind(r.employee)
    .bind(r.date)
    .bind(&r.request_type)
    .bind(origin)
    .bind(state)
    .bind(basis)
    .bind(r.proposed_in)
    .bind(r.proposed_out)
    .bind(r.target)
    .bind(r.reason.trim())
    .bind(&r.attachment)
    .bind(user)
    .bind(approver)
    .bind(decided)
    .fetch_one(&mut ***tx)
    .await?)
}

/// SM-REQ R1/R2/R3: the employee's own request. Refused submissions create nothing.
pub async fn submit(tx: &mut OrgTx<'_>, user: Uuid, r: NewRequest) -> Result<Uuid, DomainError> {
    let master = master_branch_on(tx, r.employee, r.date).await?.ok_or_else(|| invalid("not employed on attendance_date"))?;
    let s = settings_on(tx, r.employee, r.date, master).await?;
    if !s.enabled || !s.types_on.get(&r.request_type).copied().unwrap_or(false) {
        return Err(refused("RULE_GROUP_DISABLED", format!("{} requests are switched off (RG-01/RG-02)", r.request_type)));
    }
    check_shape(tx, &r, master).await?;
    if r.attachment.is_none() && s.attachment_types.contains(&r.request_type) {
        return Err(invalid("an attachment is required for this request type (RG-09)"));
    }
    let today = local_today(tx, master).await?;
    if r.date > today || (today - r.date).num_days() > s.window_days || db::employee_locked(tx, r.employee, r.date, r.date.succ_opt()).await? {
        return Err(refused("OUTSIDE_REQUEST_WINDOW", format!("requests are allowed for the last {} days and before the period is locked (RG-04)", s.window_days)));
    }
    // RG-03 per calendar month of the date. shortcut: calendar month, not the attendance cycle; switch when cycles are configurable.
    // Two submissions at once must not both slip under the limit.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text || to_char($2::date, 'YYYY-MM'), 0))")
        .bind(r.employee)
        .bind(r.date)
        .execute(&mut ***tx)
        .await?;
    let (overall, of_type): (i64, i64) = sqlx::query_as(
        "SELECT count(*), count(*) FILTER (WHERE request_type = $3) FROM attendance.regularisation_request
         WHERE employee_id = $1 AND origin = 'employee' AND state NOT IN ('rejected','cancelled')
           AND date_trunc('month', attendance_date) = date_trunc('month', $2::date)",
    )
    .bind(r.employee)
    .bind(r.date)
    .bind(&r.request_type)
    .fetch_one(&mut ***tx)
    .await?;
    // A limit of 0 means no limit (RG-01 and RG-02 switch requests off).
    let over = (s.limit_overall > 0 && overall >= s.limit_overall) || s.limit_per_type.get(&r.request_type).is_some_and(|l| *l > 0 && of_type >= *l);
    if over && s.over_limit_blocks {
        return Err(refused("LIMIT_EXCEEDED", "the regularisation limit for this period is reached (RG-03)"));
    }

    let id = if !over && s.auto_types.contains(&r.request_type) {
        // R2: auto-approved on submit (RG-06); the rule decides, not the requester.
        let id = insert(tx, user, &r, "employee", "approved", None, Some("auto_on_submit")).await?;
        db::audit(tx, user, "regularisation.submit", "regularisation_request", Some(id), Some(audit_step("submitted", 1, None, None))).await?;
        db::audit(tx, user, "regularisation.auto_approve", "regularisation_request", Some(id), Some(audit_step("auto_approved", 1, None, None))).await?;
        apply(tx, user, id).await?;
        event(tx, "RequestApproved", id, r.employee).await?;
        id
    } else {
        // R3: over the limit with RG-08 = HR approval goes straight to the HR level.
        let (state, level) = if over { ("pending_l2", 2) } else { ("pending_l1", 1) };
        let approver = route(tx, r.employee, level, today).await?.filter(|a| *a != user);
        let id = insert(tx, user, &r, "employee", state, approver, None).await?;
        db::audit(tx, user, "regularisation.submit", "regularisation_request", Some(id), Some(json!({ "action": "submitted", "level": level, "over_limit": over }))).await?;
        id
    };
    event(tx, "RequestSubmitted", id, r.employee).await?;
    Ok(id)
}

/// RG-10: HR's direct correction, approved at once (DP-08 open: no second person) and applied.
pub async fn hr_correct(tx: &mut OrgTx<'_>, p: &Principal, r: NewRequest) -> Result<Uuid, DomainError> {
    if p.employee_id == Some(r.employee) {
        return Err(invalid("you cannot correct your own attendance (G-17)"));
    }
    let master = master_branch_on(tx, r.employee, r.date).await?.ok_or_else(|| invalid("not employed on attendance_date"))?;
    if !settings_on(tx, r.employee, r.date, master).await?.hr_direct {
        return Err(refused("RULE_GROUP_DISABLED", "direct correction by HR is switched off (RG-10)"));
    }
    check_shape(tx, &r, master).await?;
    if db::employee_locked(tx, r.employee, r.date, r.date.succ_opt()).await? {
        return Err(DomainError::PeriodLocked);
    }
    let id = insert(tx, p.user_id, &r, "hr_direct", "approved", None, Some("approver")).await?;
    db::audit(tx, p.user_id, "regularisation.hr_correct", "regularisation_request", Some(id), Some(audit_step("approved", 1, Some(r.reason.trim()), None))).await?;
    apply(tx, p.user_id, id).await?;
    event(tx, "RequestApproved", id, r.employee).await?;
    Ok(id)
}

/// The effect of an approved request: manual punches referencing it (BR-01), the corrected punch chosen, the day
/// recalculated (BR-03). Keys derive from the request, so the effect can never be applied twice.
pub(crate) async fn apply(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid) -> Result<(), DomainError> {
    let (employee, date, kind, pin, pout, target, reason): RequestRow =
        sqlx::query_as(
            "SELECT employee_id, attendance_date, request_type, proposed_in_at, proposed_out_at, target_raw_punch_id, reason
             FROM attendance.regularisation_request WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&mut ***tx)
        .await?;
    // Late arrival replaces the day's first in; early check-out its last out (as the current result read them).
    let replaced = match (kind.as_str(), target) {
        (_, Some(t)) => Some(t),
        ("late_arrival" | "early_checkout", None) => {
            let dir = if kind == "late_arrival" { "in" } else { "out" };
            sqlx::query_scalar(
                "SELECT u.raw_punch_id FROM attendance.attendance_day d JOIN attendance.day_result_punch u ON u.result_id = d.current_result_id
                 JOIN timecap.raw_punch p ON p.id = u.raw_punch_id
                 WHERE d.employee_id = $1 AND d.attendance_date = $2 AND u.assigned_direction = $3 AND NOT u.excluded
                 ORDER BY CASE WHEN $3 = 'in' THEN p.event_at END, p.event_at DESC LIMIT 1",
            )
            .bind(employee)
            .bind(date)
            .bind(dir)
            .fetch_optional(&mut ***tx)
            .await?
        }
        _ => None,
    };
    if replaced.is_some() && target.is_none() {
        sqlx::query("UPDATE attendance.regularisation_request SET target_raw_punch_id = $2, updated_at = now() WHERE id = $1").bind(id).bind(replaced).execute(&mut ***tx).await?;
    }
    let master = master_branch_on(tx, employee, date).await?.ok_or_else(|| invalid("not employed on attendance_date"))?;
    let tz = branch_tz(tx, master, Utc::now()).await?;
    for (n, at) in [(1, pin), (2, pout)] {
        let Some(at) = at else { continue };
        sqlx::query(
            "INSERT INTO timecap.raw_punch (org_id, employee_id, source, event_at, captured_tz, idempotency_key, branch_id, manual_reason,
               manual_entered_by, manual_approval_ref, created_by)
             VALUES (app.current_org(), $1, 'manual', $2, $3, $4, $5, $6, $7, $8, $7) ON CONFLICT (org_id, idempotency_key) DO NOTHING",
        )
        .bind(employee)
        .bind(at)
        .bind(&tz)
        .bind(format!("reg:{id}:{n}"))
        .bind(master)
        .bind(&reason)
        .bind(actor)
        .bind(id)
        .execute(&mut ***tx)
        .await?;
    }
    crate::recalculate(tx, employee, date, "regularisation", Some(id)).await?;
    if let Some(t) = replaced {
        // The replaced punch may sit on the day's edge (a night shift): its own day is recalculated too.
        let t_at: DateTime<Utc> = sqlx::query_scalar("SELECT event_at FROM timecap.raw_punch WHERE id = $1").bind(t).fetch_one(&mut ***tx).await?;
        crate::recalculate_around(tx, employee, t_at, "regularisation", Some(id)).await?;
    }
    Ok(())
}

struct Pending {
    employee: Uuid,
    state: String,
    requester: Uuid,
    approver: Option<Uuid>,
}

async fn lock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Pending, DomainError> {
    let row: Option<(Uuid, String, Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT employee_id, state, requested_by_user_id, current_approver_user_id FROM attendance.regularisation_request WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let (employee, state, requester, approver) = row.ok_or(DomainError::NotFound)?;
    Ok(Pending { employee, state, requester, approver })
}

/// Whether `p` may decide at `level`: Ok(Some(on_behalf_of)) when acting as a delegate (AC-03), Ok(None) as approver.
/// Err(NotFound) out of scope, Err(Forbidden) in scope but not the routed approver.
async fn may_decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, employee: Uuid, level: i16, approver: Option<Uuid>) -> Result<Option<Uuid>, DomainError> {
    // Level 2 reached over the limit (RG-08) is ATT-05's; a normal second level (RG-05) is ATT-04's.
    let over_limit: bool = sqlx::query_scalar(
        "SELECT coalesce(bool_or((after_value->>'over_limit')::boolean), false) FROM audit.audit_entry
         WHERE target_type = 'regularisation_request' AND target_id = $1 AND action = 'regularisation.submit'",
    )
    .bind(id)
    .fetch_one(&mut ***tx)
    .await?;
    let op = match (level, over_limit) {
        (1, _) => "ATT-03",
        (_, true) => "ATT-05",
        _ => "ATT-04",
    };
    crate::approval::may_decide(tx, p, employee, op, approver).await
}

/// SM-REQ R4/R5 (approve) and R6 (reject). The same decision again changes nothing.
pub async fn decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, approve: bool, note: Option<&str>) -> Result<(), DomainError> {
    let r = lock(tx, id).await?;
    if p.user_id == r.requester || p.employee_id == Some(r.employee) {
        return Err(DomainError::SelfApproval); // BR-07, delegates included
    }
    let done = if approve { "approved" } else { "rejected" };
    let level: i16 = match r.state.as_str() {
        "pending_l1" => 1,
        "pending_l2" => 2,
        // A closed request answers only to whoever decided it (or, if cancelled, the approver it was waiting on).
        s => {
            let decider: Option<Uuid> = sqlx::query_scalar("SELECT coalesce(decided_by_user_id, current_approver_user_id) FROM attendance.regularisation_request WHERE id = $1").bind(id).fetch_one(&mut ***tx).await?;
            if decider != Some(p.user_id) {
                return Err(DomainError::NotFound);
            }
            return if s == done { Ok(()) } else { Err(DomainError::IllegalTransition(format!("request is {s}"))) };
        }
    };
    let on_behalf_of = may_decide(tx, p, id, r.employee, level, r.approver).await?;
    if level == 2 {
        // SM-REQ: nobody approves a level twice; the second level is a second person.
        let decided_l1: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM audit.audit_entry WHERE target_type = 'regularisation_request' AND target_id = $1
                            AND action = 'regularisation.approve' AND actor_id = $2)",
        )
        .bind(id)
        .bind(p.user_id)
        .fetch_one(&mut ***tx)
        .await?;
        if decided_l1 {
            return Err(DomainError::IllegalTransition("the first-level approver cannot also decide the second level".into()));
        }
    }
    let note = note.map(str::trim).filter(|n| !n.is_empty());
    if !approve && note.is_none() {
        return Err(invalid("reason is required to reject"));
    }
    let date: NaiveDate = sqlx::query_scalar("SELECT attendance_date FROM attendance.regularisation_request WHERE id = $1").bind(id).fetch_one(&mut ***tx).await?;
    let master = master_branch_on(tx, r.employee, date).await?.ok_or_else(|| invalid("not employed on attendance_date"))?;
    let two_levels = settings_on(tx, r.employee, date, master).await?.two_levels;
    let action = if approve { "approved" } else { "rejected" };
    db::audit(tx, p.user_id, &format!("regularisation.{}", if approve { "approve" } else { "reject" }), "regularisation_request", Some(id), Some(audit_step(action, level, note, on_behalf_of))).await?;
    if approve && level == 1 && two_levels {
        // R4: manager approved; HR decides next (RG-05).
        let today = local_today(tx, master).await?;
        let next = route(tx, r.employee, 2, today).await?.filter(|a| *a != r.requester);
        sqlx::query("UPDATE attendance.regularisation_request SET state = 'pending_l2', current_approver_user_id = $2, updated_at = now(), updated_by = $3 WHERE id = $1")
            .bind(id)
            .bind(next)
            .bind(p.user_id)
            .execute(&mut ***tx)
            .await?;
        return Ok(());
    }
    sqlx::query(
        "UPDATE attendance.regularisation_request SET state = $2, decided_basis = 'approver', decided_by_user_id = $3, decided_at = now(),
           current_approver_user_id = NULL, updated_at = now(), updated_by = $3 WHERE id = $1",
    )
    .bind(id)
    .bind(done)
    .bind(p.user_id)
    .execute(&mut ***tx)
    .await?;
    if approve {
        apply(tx, p.user_id, id).await?;
        event(tx, "RequestApproved", id, r.employee).await?;
    }
    Ok(())
}

/// SM-REQ R10: the requester cancels a pending request (GAP-SM-09: nothing else yet).
pub async fn cancel(tx: &mut OrgTx<'_>, user: Uuid, id: Uuid, reason: Option<&str>) -> Result<(), DomainError> {
    let r = lock(tx, id).await?;
    if r.requester != user {
        return Err(DomainError::NotFound);
    }
    match r.state.as_str() {
        "cancelled" => return Ok(()),
        "pending_l1" | "pending_l2" => {}
        s => return Err(DomainError::IllegalTransition(format!("request is {s}"))),
    }
    // The approver it was waiting on is kept: they may still ask about it (ILLEGAL_TRANSITION, not NOT_FOUND).
    sqlx::query("UPDATE attendance.regularisation_request SET state = 'cancelled', updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(id)
        .bind(user)
        .execute(&mut ***tx)
        .await?;
    let level = if r.state == "pending_l2" { 2 } else { 1 };
    db::audit(tx, user, "regularisation.cancel", "regularisation_request", Some(id), Some(audit_step("cancelled", level, reason, None))).await?;
    event(tx, "RequestCancelled", id, r.employee).await?;
    Ok(())
}

/// RG-11: an hr_direct whole-day correction at the shift times for each listed employee whose day has no completed pair.
/// Returns (corrected, skipped: employees with no timed shift that day).
pub async fn bulk_outage(tx: &mut OrgTx<'_>, p: &Principal, employees: &[Uuid], date: NaiveDate, reason: &str) -> Result<(usize, usize), DomainError> {
    let (mut corrected, mut skipped) = (0, 0);
    for e in employees {
        if p.employee_id == Some(*e) {
            continue; // G-17: nobody corrects their own attendance
        }
        let Some(master) = master_branch_on(tx, *e, date).await? else { continue };
        if !settings_on(tx, *e, date, master).await?.bulk {
            return Err(refused("RULE_GROUP_DISABLED", "bulk regularisation is switched off (RG-11)"));
        }
        let complete: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
                            WHERE d.employee_id = $1 AND d.attendance_date = $2 AND EXISTS (SELECT 1 FROM jsonb_array_elements(r.sessions) s WHERE s->>'out_at' IS NOT NULL))",
        )
        .bind(e)
        .bind(date)
        .fetch_one(&mut ***tx)
        .await?;
        if complete {
            continue;
        }
        let day = domain_sched::schedule::expected_day(tx, *e, date, master).await?;
        let tz = branch_tz(tx, master, Utc::now()).await?;
        let times: Option<Span> = match (day.weekly_off || day.holiday.is_some(), day.shift_version_id) {
            (false, Some(v)) => sqlx::query_as(
                "SELECT (($2::date + start_time) AT TIME ZONE $3),
                        (($2::date + end_time + CASE WHEN crosses_midnight THEN interval '1 day' ELSE interval '0' END) AT TIME ZONE $3)
                 FROM sched.shift_version WHERE id = $1",
            )
            .bind(v)
            .bind(date)
            .bind(&tz)
            .fetch_optional(&mut ***tx)
            .await?,
            _ => None,
        };
        let Some((Some(start), Some(end))) = times else {
            skipped += 1;
            continue;
        };
        let r = NewRequest { employee: *e, date, request_type: "whole_day".into(), proposed_in: Some(start), proposed_out: Some(end), target: None, reason: reason.to_string(), attachment: None };
        if db::employee_locked(tx, *e, date, date.succ_opt()).await? {
            skipped += 1;
            continue;
        }
        let id = insert(tx, p.user_id, &r, "hr_direct", "approved", None, Some("approver")).await?;
        db::audit(tx, p.user_id, "regularisation.bulk_outage", "regularisation_request", Some(id), Some(audit_step("approved", 1, Some(reason), None))).await?;
        apply(tx, p.user_id, id).await?;
        event(tx, "RequestApproved", id, *e).await?;
        corrected += 1;
    }
    Ok((corrected, skipped))
}

const REQUEST_JSON: &str = "jsonb_build_object('id', q.id, 'employee_id', q.employee_id, 'attendance_date', q.attendance_date,
    'request_type', q.request_type, 'origin', q.origin, 'state', q.state, 'escalated', q.escalated,
    'proposed_in_at', q.proposed_in_at, 'proposed_out_at', q.proposed_out_at, 'target_raw_punch_id', q.target_raw_punch_id,
    'reason', q.reason, 'attachment', CASE WHEN q.attachment_path IS NOT NULL THEN jsonb_build_object('path', q.attachment_path) END,
    'requested_by_user_id', q.requested_by_user_id, 'decided_by_user_id', q.decided_by_user_id, 'decided_at', q.decided_at,
    'decided_basis', q.decided_basis, 'current_approver_user_id', q.current_approver_user_id, 'submitted_at', q.created_at,
    'approval_steps', coalesce((SELECT jsonb_agg(jsonb_build_object('level', coalesce((a.after_value->>'level')::int, 1), 'action', a.after_value->>'action',
        'actor_user_id', CASE WHEN a.after_value->>'action' LIKE 'auto_%' THEN NULL ELSE a.actor_id END,
        'on_behalf_of_user_id', a.after_value->'on_behalf_of', 'comment', a.after_value->>'comment', 'acted_at', a.occurred_at)
        ORDER BY a.occurred_at, CASE a.after_value->>'action' WHEN 'submitted' THEN 0 ELSE 1 END, a.id) FROM audit.audit_entry a WHERE a.target_type = 'regularisation_request' AND a.target_id = q.id), '[]'))";

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {REQUEST_JSON} FROM attendance.regularisation_request q WHERE q.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub struct Filter {
    pub employees: Option<Vec<Uuid>>,
    pub state: Option<String>,
    pub origin: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

// shortcut: no cursor paging, newest 500.
pub async fn list(tx: &mut OrgTx<'_>, f: &Filter) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {REQUEST_JSON} FROM attendance.regularisation_request q
         WHERE ($1::uuid[] IS NULL OR q.employee_id = ANY($1)) AND ($2::text IS NULL OR q.state = $2) AND ($3::text IS NULL OR q.origin = $3)
           AND ($4::date IS NULL OR q.attendance_date >= $4) AND ($5::date IS NULL OR q.attendance_date <= $5)
         ORDER BY q.created_at DESC, q.id LIMIT 500"
    );
    Ok(sqlx::query_scalar(&sql).bind(&f.employees).bind(&f.state).bind(&f.origin).bind(f.from).bind(f.to).fetch_all(&mut ***tx).await?)
}

/// Whether the caller may decide this pending request now.
pub async fn can_decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid) -> Result<bool, DomainError> {
    let row: Option<(Uuid, String, Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT employee_id, state, requested_by_user_id, current_approver_user_id FROM attendance.regularisation_request
         WHERE id = $1 AND state IN ('pending_l1','pending_l2')",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((employee, state, requester, approver)) = row else { return Ok(false) };
    if requester == p.user_id || p.employee_id == Some(employee) {
        return Ok(false);
    }
    let level = if state == "pending_l1" { 1 } else { 2 };
    match may_decide(tx, p, id, employee, level, approver).await {
        Ok(_) => Ok(true),
        Err(DomainError::NotFound | DomainError::Forbidden) => Ok(false),
        Err(e) => Err(e),
    }
}

// shortcut: the HR-pool rows are checked one by one for scope; index by branch when queues grow.
/// Pending requests the caller may decide now (never their own, BR-07): routed to them, to someone who delegated
/// to them, or to the HR pool.
pub async fn pending_for(tx: &mut OrgTx<'_>, p: &Principal, level: Option<i16>) -> Result<Vec<Value>, DomainError> {
    let rows: Vec<(Uuid, Uuid, String, Uuid, Option<Uuid>)> = sqlx::query_as(
        "SELECT id, employee_id, state, requested_by_user_id, current_approver_user_id FROM attendance.regularisation_request
         WHERE state IN ('pending_l1','pending_l2') AND requested_by_user_id <> $1
           AND (current_approver_user_id IS NULL OR current_approver_user_id = $1
                OR current_approver_user_id IN (SELECT approver_user_id FROM iam.delegation WHERE delegate_user_id = $1 AND valid @> now()))
         ORDER BY created_at, id",
    )
    .bind(p.user_id)
    .fetch_all(&mut ***tx)
    .await?;
    let mut out = Vec::new();
    for (id, employee, state, requester, approver) in rows {
        let l = if state == "pending_l1" { 1 } else { 2 };
        if level.is_some_and(|w| w != l) || requester == p.user_id || p.employee_id == Some(employee) {
            continue;
        }
        match may_decide(tx, p, id, employee, l, approver).await {
            Ok(_) => out.push(get(tx, id).await?),
            Err(DomainError::NotFound | DomainError::Forbidden) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(out)
}
