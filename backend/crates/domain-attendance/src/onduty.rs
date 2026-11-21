//! On-duty (C09, PM-R5/PM-R6, OD-01..OD-12): official work away from the workplace for hours, a half day, a full day
//! or several days, on the shared approval flow (SM-REQ). Approved on-duty counts as worked with no late or early
//! mark for the time it covers; it never uses the permission limit (OD-12).

use crate::approval::{self, Step};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use db::{DomainError, OrgTx};
use domain_iam::authz::Principal;
use domain_policy::od::{self, OdSettings};
use domain_timecap::punch::master_branch_on;
use serde_json::{json, Value};
use uuid::Uuid;

const KIND: &str = "on_duty";

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

fn refused(code: &'static str, detail: impl Into<String>) -> DomainError {
    DomainError::Refused(code, detail.into())
}

async fn settings_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<OdSettings, DomainError> {
    let master = master_branch_on(tx, employee, date).await?.ok_or_else(|| invalid("not employed on that date"))?;
    let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
    od::load(tx, resolved.and_then(|r| r.version_id)).await
}

async fn lock_employee(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('on_duty:' || $1::text))").bind(employee).execute(&mut ***tx).await?;
    Ok(())
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

pub struct NewOnDuty {
    pub kind: String,
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub from_time: Option<NaiveTime>,
    pub to_time: Option<NaiveTime>,
    pub purpose: String,
    pub place: String,
}

/// PM-R5: an employee's own application; refusals create nothing.
pub async fn submit(tx: &mut OrgTx<'_>, user: Uuid, employee: Uuid, r: NewOnDuty) -> Result<Uuid, DomainError> {
    lock_employee(tx, employee).await?;
    let s = settings_on(tx, employee, r.from).await?;
    if !s.enabled {
        return Err(refused("RULE_GROUP_DISABLED", "on-duty is switched off (OD-01)"));
    }
    if !od::KINDS.contains(&r.kind.as_str()) || !s.kind_on(&r.kind) {
        return Err(invalid(format!("duration {} is not allowed (OD-02)", r.kind)));
    }
    let (purpose, place) = (r.purpose.trim(), r.place.trim());
    if purpose.is_empty() || purpose.chars().count() > 500 || place.is_empty() || place.chars().count() > 300 {
        return Err(invalid("purpose (1-500) and place (1-300) are required (OD-06)"));
    }
    if r.to < r.from || (r.kind != "several_days" && r.to != r.from) || (r.kind == "several_days" && (r.to - r.from).num_days() > 60) {
        return Err(invalid("to_date equals from_date unless several_days (at most 60 days)"));
    }
    let hours = r.kind == "hours";
    if hours != (r.from_time.is_some() && r.to_time.is_some()) || r.from_time.is_some() != r.to_time.is_some() {
        return Err(invalid("from_time and to_time are given exactly when duration_kind = hours"));
    }
    if hours && r.from_time == r.to_time {
        return Err(invalid("from_time and to_time differ"));
    }
    // Hours and half days are parts of a shift: the day needs one with set times, and hours fall inside it.
    if matches!(r.kind.as_str(), "hours" | "half_day") {
        let Some((start, end)) = crate::shift_times(tx, employee, r.from).await? else {
            return Err(invalid("hours and half days need a shift with set times on that date"));
        };
        if let (Some(f), Some(t)) = (r.from_time, r.to_time) {
            let (a, b) = crate::permission::instants(tx, employee, r.from, f, t).await?;
            if a < start || b > end {
                return Err(invalid("on-duty hours fall inside the shift"));
            }
        }
    }
    // OD-04: before the event only, or before / after within N days (the employee's own date).
    let tz = crate::employee_tz(tx, employee, r.from).await?;
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE $1)::date").bind(&tz).fetch_one(&mut ***tx).await?;
    let window_ok = if s.before_only { today < r.from } else { today <= r.from + Duration::days(s.after_days) };
    if !window_ok {
        return Err(refused("OUTSIDE_REQUEST_WINDOW", "the on-duty is outside its application window (OD-04)"));
    }
    let mut d = r.from;
    while d <= r.to {
        if db::employee_locked(tx, employee, d, d.succ_opt()).await? {
            return Err(DomainError::PeriodLocked);
        }
        d = d.succ_opt().expect("valid");
    }
    leave_clash(tx, employee, &r.kind, r.from, r.to).await?;
    let overlapping: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM perm.on_duty_request WHERE employee_id = $1 AND state IN ('pending_l1','pending_l2','approved')
                        AND daterange(from_date, to_date, '[]') && daterange($2, $3, '[]'))",
    )
    .bind(employee)
    .bind(r.from)
    .bind(r.to)
    .fetch_one(&mut ***tx)
    .await?;
    if overlapping {
        return Err(refused("OVERLAPPING_REQUEST", "another on-duty covers those dates"));
    }
    let approver = approval::route(tx, KIND, employee, 1, Utc::now().date_naive()).await?.filter(|a| *a != user);
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO perm.on_duty_request (org_id, employee_id, duration_kind, from_date, to_date, from_time, to_time, purpose, place, state,
           current_approver_user_id, requested_by, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, 'pending_l1', $9, $10, $10, $10) RETURNING id",
    )
    .bind(employee)
    .bind(&r.kind)
    .bind(r.from)
    .bind(r.to)
    .bind(r.from_time)
    .bind(r.to_time)
    .bind(purpose)
    .bind(place)
    .bind(approver)
    .bind(user)
    .fetch_one(&mut ***tx)
    .await?;
    approval::step(tx, Step { kind: KIND, request_id: id, level: 1, action: "submitted", requester: user, actor: Some(user), on_behalf_of: None, comment: None }).await?;
    db::audit(tx, user, "on_duty_request.submit", "on_duty_request", Some(id), Some(json!({ "kind": r.kind, "from": r.from, "to": r.to }))).await?;
    event(tx, "RequestSubmitted", id, employee).await?;
    Ok(id)
}

/// Approved leave on a covered day: a full-day leave leaves nothing to be on duty for; a half-day leave still
/// allows hours or the other half.
async fn leave_clash(tx: &mut OrgTx<'_>, employee: Uuid, kind: &str, from: NaiveDate, to: NaiveDate) -> Result<(), DomainError> {
    let mut d = from;
    while d <= to {
        let leave = crate::leave::leave_on(tx, employee, d).await?;
        if matches!(leave, Some(None)) || (leave.is_some() && matches!(kind, "full_day" | "several_days")) {
            return Err(refused("OVERLAPPING_REQUEST", format!("leave is taken on {d}")));
        }
        d = d.succ_opt().expect("valid");
    }
    Ok(())
}

struct Row {
    employee: Uuid,
    kind: String,
    state: String,
    requester: Uuid,
    approver: Option<Uuid>,
    from: NaiveDate,
    to: NaiveDate,
}

type RowTuple = (Uuid, String, String, Uuid, Option<Uuid>, NaiveDate, NaiveDate);

async fn lock(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Row, DomainError> {
    let r: Option<RowTuple> = sqlx::query_as(
        "SELECT employee_id, duration_kind, state, requested_by, current_approver_user_id, from_date, to_date FROM perm.on_duty_request WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?;
    let (employee, kind, state, requester, approver, from, to) = r.ok_or(DomainError::NotFound)?;
    Ok(Row { employee, kind, state, requester, approver, from, to })
}

async fn recalc(tx: &mut OrgTx<'_>, employee: Uuid, from: NaiveDate, to: NaiveDate, trigger: &str, id: Uuid) -> Result<(), DomainError> {
    let mut d = from;
    while d <= to {
        crate::recalculate(tx, employee, d, trigger, Some(id)).await?;
        d = d.succ_opt().expect("valid");
    }
    Ok(())
}

async fn any_locked(tx: &mut OrgTx<'_>, employee: Uuid, from: NaiveDate, to: NaiveDate) -> Result<bool, DomainError> {
    Ok(db::employee_locked(tx, employee, from, to.succ_opt()).await?)
}

/// SM-REQ R4/R5/R6 (PRM-09 level 1, PRM-10 level 2). Final approval recalculates the covered days.
pub async fn decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, approve: bool, note: Option<&str>) -> Result<(), DomainError> {
    let employee = employee_of(tx, id).await?;
    lock_employee(tx, employee).await?;
    let r = lock(tx, id).await?;
    if p.user_id == r.requester || p.employee_id == Some(r.employee) {
        return Err(DomainError::SelfApproval); // BR-07
    }
    let done = if approve { "approved" } else { "rejected" };
    let level: i16 = match r.state.as_str() {
        "pending_l1" => 1,
        "pending_l2" => 2,
        s => {
            let decider: Option<Uuid> = sqlx::query_scalar(
                "SELECT coalesce((SELECT actor_user_id FROM app.approval_step WHERE request_type = 'on_duty' AND request_id = $1 AND action IN ('approved','rejected')
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
    let on_behalf_of = approval::may_decide(tx, p, r.employee, if level == 1 { "PRM-09" } else { "PRM-10" }, r.approver).await?;
    if level == 2 {
        let twice: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM app.approval_step WHERE request_type = 'on_duty' AND request_id = $1 AND action = 'approved' AND actor_user_id = $2)")
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
    // Settings only matter for a level-1 approval (so a request of someone who has since left can still be decided).
    if approve && level == 1 && settings_on(tx, r.employee, r.from).await?.two_levels {
        approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
        let next = approval::route(tx, KIND, r.employee, 2, Utc::now().date_naive()).await?.filter(|a| *a != r.requester);
        sqlx::query("UPDATE perm.on_duty_request SET state = 'pending_l2', current_approver_user_id = $2, updated_at = now(), updated_by = $3 WHERE id = $1")
            .bind(id)
            .bind(next)
            .bind(p.user_id)
            .execute(&mut ***tx)
            .await?;
        db::audit(tx, p.user_id, "on_duty_request.approve_level_1", "on_duty_request", Some(id), Some(json!({ "comment": note }))).await?;
        event(tx, "RequestApprovedAtLevel", id, r.employee).await?;
        return Ok(());
    }
    if approve && any_locked(tx, r.employee, r.from, r.to).await? {
        return Err(DomainError::PeriodLocked);
    }
    if approve {
        leave_clash(tx, r.employee, &r.kind, r.from, r.to).await?; // leave approved since the submission
    }
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: done, requester: r.requester, actor: Some(p.user_id), on_behalf_of, comment: note }).await?;
    sqlx::query("UPDATE perm.on_duty_request SET state = $2, decided_basis = 'approver', current_approver_user_id = NULL, updated_at = now(), updated_by = $3 WHERE id = $1")
        .bind(id)
        .bind(done)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, p.user_id, &format!("on_duty_request.{}", if approve { "approve" } else { "reject" }), "on_duty_request", Some(id), Some(json!({ "comment": note }))).await?;
    if approve {
        recalc(tx, r.employee, r.from, r.to, "on_duty_approved", id).await?;
        event(tx, "RequestApproved", id, r.employee).await?;
    }
    Ok(())
}

/// SM-REQ R10/R11 (PRM-11; G-27 open, so only whoever holds it). An approved on-duty's days are recalculated
/// from punches (AC-C09-11.6).
pub async fn cancel(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, reason: &str) -> Result<(), DomainError> {
    let employee = employee_of(tx, id).await?;
    lock_employee(tx, employee).await?;
    let r = lock(tx, id).await?;
    match r.state.as_str() {
        "cancelled" => return Ok(()),
        "rejected" => return Err(DomainError::IllegalTransition("request is rejected".into())),
        _ => {}
    }
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(invalid("reason is required"));
    }
    if r.state == "approved" && any_locked(tx, r.employee, r.from, r.to).await? {
        return Err(DomainError::PeriodLocked);
    }
    let level = if r.state == "pending_l2" { 2 } else { 1 };
    sqlx::query("UPDATE perm.on_duty_request SET state = 'cancelled', cancelled_by = $2, cancelled_at = now(), updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(id)
        .bind(p.user_id)
        .execute(&mut ***tx)
        .await?;
    approval::step(tx, Step { kind: KIND, request_id: id, level, action: "cancelled", requester: r.requester, actor: Some(p.user_id), on_behalf_of: None, comment: Some(reason.trim()) }).await?;
    if r.state == "approved" {
        recalc(tx, r.employee, r.from, r.to, "on_duty_cancelled", id).await?;
    }
    db::audit(tx, p.user_id, "on_duty_request.cancel", "on_duty_request", Some(id), Some(json!({ "reason": reason.trim(), "was": r.state }))).await?;
    event(tx, "RequestCancelled", id, r.employee).await?;
    Ok(())
}

fn request_json() -> String {
    format!(
        "jsonb_build_object('id', q.id, 'employee_id', q.employee_id, 'duration_kind', q.duration_kind, 'from_date', q.from_date, 'to_date', q.to_date,
           'employee_code', (SELECT e.employee_code FROM workforce.employee e WHERE e.id = q.employee_id),
           'employee_name', (SELECT e.full_name FROM workforce.employee e WHERE e.id = q.employee_id),
           'from_time', to_char(q.from_time, 'HH24:MI'), 'to_time', to_char(q.to_time, 'HH24:MI'), 'purpose', q.purpose, 'place', q.place,
           'proof_path', q.proof_path, 'requested_by', q.requested_by, 'state', q.state, 'escalated', q.escalated, 'decided_basis', q.decided_basis,
           'current_approver_user_id', q.current_approver_user_id, 'submitted_at', q.submitted_at, 'cancelled_at', q.cancelled_at,
           'cancelled_by', q.cancelled_by, 'steps', {})",
        approval::steps_sql(KIND, "q.id")
    )
}

pub async fn get(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {} FROM perm.on_duty_request q WHERE q.id = $1", request_json()))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn employee_of(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Uuid, DomainError> {
    sqlx::query_scalar("SELECT employee_id FROM perm.on_duty_request WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub struct Filter {
    pub employees: Option<Vec<Uuid>>,
    pub approver: Uuid,
    pub employee: Option<Uuid>,
    pub state: Option<String>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// Whether `decide` would accept the caller now (PRM-09 level 1, PRM-10 level 2).
pub async fn can_decide(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid) -> Result<bool, DomainError> {
    approval::can_decide_two_level(tx, p, "on_duty", "perm.on_duty_request", id, ("PRM-09", "PRM-10")).await
}

/// The requests waiting for the caller's decision: exactly those `can_decide` accepts, newest first.
pub async fn awaiting(tx: &mut OrgTx<'_>, p: &Principal) -> Result<Vec<Value>, DomainError> {
    let mut out = Vec::new();
    for id in approval::awaiting_candidates(tx, p, "perm.on_duty_request", "q.submitted_at", &["pending_l1", "pending_l2"]).await? {
        if can_decide(tx, p, id).await? {
            out.push(get(tx, id).await?);
        }
    }
    Ok(out)
}

pub async fn list(tx: &mut OrgTx<'_>, f: &Filter) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {} FROM perm.on_duty_request q
         WHERE ($1::uuid[] IS NULL OR q.employee_id = ANY($1) OR q.current_approver_user_id = $6
                OR EXISTS (SELECT 1 FROM app.approval_step st WHERE st.request_type = 'on_duty' AND st.request_id = q.id AND st.actor_user_id = $6))
           AND ($2::uuid IS NULL OR q.employee_id = $2) AND ($3::text IS NULL OR q.state = $3)
           AND ($4::date IS NULL OR q.to_date >= $4) AND ($5::date IS NULL OR q.from_date <= $5)
         ORDER BY q.from_date DESC",
        request_json()
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

/// What an approved on-duty covers on a day, for the engine.
pub enum Cover {
    Day,
    Half,
    Hours(DateTime<Utc>, DateTime<Utc>),
}

pub async fn approved_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Vec<Cover>, DomainError> {
    let rows: Vec<(String, Option<NaiveTime>, Option<NaiveTime>)> = sqlx::query_as(
        "SELECT duration_kind, from_time, to_time FROM perm.on_duty_request WHERE employee_id = $1 AND state = 'approved' AND $2 BETWEEN from_date AND to_date",
    )
    .bind(employee)
    .bind(date)
    .fetch_all(&mut ***tx)
    .await?;
    let mut out = vec![];
    for (kind, from, to) in rows {
        out.push(match (kind.as_str(), from, to) {
            ("hours", Some(f), Some(t)) => {
                let (a, b) = crate::permission::instants(tx, employee, date, f, t).await?;
                Cover::Hours(a, b)
            }
            ("half_day", ..) => Cover::Half,
            _ => Cover::Day,
        });
    }
    Ok(out)
}
