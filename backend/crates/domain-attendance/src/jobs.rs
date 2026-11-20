//! Timed jobs (TSD §10): J1 nightly recalculation and J3 approval timeouts. Work runs in organisation-scoped
//! transactions (RLS), exactly like a request: J1 one per employee, J3 one per organisation with a savepoint per request.

use chrono::{Duration, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Active organisations. Only the vendor role sees every organisation row, so the listing runs as `app_vendor`.
/// That role reads every tenant's platform rows and audit log, so only the job service's login role (`jobs_login`)
/// is a member of it; the internet-facing API's login role is not (DEPLOYMENT.md §3). Nothing else is read here.
pub async fn active_organisations(pool: &PgPool) -> Result<Vec<Uuid>, DomainError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE app_vendor").execute(&mut *tx).await?;
    let ids = sqlx::query_scalar("SELECT id FROM platform.organisation WHERE state = 'active' ORDER BY id").fetch_all(&mut *tx).await?;
    tx.rollback().await?;
    Ok(ids)
}

/// Days J1 looks back: open days whose missed-punch cut-off or DS-10 waiting time may have passed since the last run.
const LOOKBACK_DAYS: i64 = 7;

/// "Today" for an employee: the date now in their master branch's zone (BR-11), so a 03:00 India run covers India's
/// yesterday. A branch with no dated zone is read as India time (as `branch_tz`).
async fn local_today(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<NaiveDate, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT (now() AT TIME ZONE coalesce((SELECT t.tz FROM workforce.employee_assignment a JOIN org.branch_timezone t ON t.branch_id = a.branch_id
                                              WHERE a.employee_id = $1 AND a.kind = 'master_branch' AND a.valid @> current_date AND t.valid @> current_date
                                              LIMIT 1), 'Asia/Kolkata'))::date",
    )
    .bind(employee)
    .fetch_one(&mut ***tx)
    .await?)
}

async fn recalc_employee(tx: &mut OrgTx<'_>, employee: Uuid) -> Result<i64, DomainError> {
    let today = local_today(tx, employee).await?;
    let mut n = 0;
    for back in (1..=LOOKBACK_DAYS).rev() {
        if crate::recalculate(tx, employee, today - Duration::days(back), "j1_nightly", None).await?.is_some() {
            n += 1;
        }
    }
    Ok(n)
}

/// J1: recalculates the last `LOOKBACK_DAYS` days (today excluded) of every employee who is current or left within
/// that window. Unchanged inputs write nothing (BR-02); locked and future days are skipped by `recalculate`.
/// One short transaction per employee, so the run never holds request or balance locks for long, and one
/// employee's failure never stops the others.
// shortcut: one request walks every organisation; fan out one Cloud Task per organisation when runs near the scheduler deadline.
pub async fn nightly_recalculation(pool: &PgPool, org: Uuid) -> Result<Value, DomainError> {
    let mut tx = db::begin_org(pool, org).await?;
    let employees: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM workforce.employee
         WHERE joining_date <= current_date AND (employment_status <> 'exited' OR exit_date >= current_date - $1) ORDER BY id",
    )
    .bind(LOOKBACK_DAYS as i32 + 1)
    .fetch_all(&mut **tx)
    .await?;
    tx.commit().await?;
    let (mut written, mut failed) = (0, 0);
    for e in &employees {
        let mut tx = db::begin_org(pool, org).await?;
        match recalc_employee(&mut tx, *e).await {
            Ok(n) => {
                tx.commit().await?;
                written += n;
            }
            Err(err) => {
                eprintln!("J1: employee {e} failed: {err:?}");
                failed += 1;
            }
        }
    }
    let summary = json!({ "employees": employees.len(), "results_written": written, "failed": failed, "days_back": LOOKBACK_DAYS });
    let mut tx = db::begin_org(pool, org).await?;
    db::audit_system(&mut tx, "job.j1_nightly_recalculation", "organisation", None, Some(summary.clone())).await?;
    tx.commit().await?;
    Ok(summary)
}

/// Runs one J3 item inside a savepoint: a failing request is logged and skipped, the rest of the run stands.
async fn savepoint(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    sqlx::query("SAVEPOINT j3_item").execute(&mut ***tx).await?;
    Ok(())
}

async fn settle<T>(tx: &mut OrgTx<'_>, what: &str, id: Uuid, r: Result<Option<T>, DomainError>) -> Result<Option<T>, DomainError> {
    match r {
        Ok(v) => {
            sqlx::query("RELEASE SAVEPOINT j3_item").execute(&mut ***tx).await?;
            Ok(v)
        }
        Err(e) => {
            sqlx::query("ROLLBACK TO SAVEPOINT j3_item").execute(&mut ***tx).await?;
            eprintln!("J3: {what} {id} skipped: {e:?}");
            Ok(None)
        }
    }
}

/// id, state, timeout days, timeout action, last step.
type LeaveRow = (Uuid, String, Option<i32>, String, chrono::DateTime<Utc>);

/// What J3 did to one request.
#[derive(Debug, PartialEq)]
enum Outcome {
    Escalated,
    AutoApproved,
    AutoRejected,
}

/// J3: requests pending longer than their timeout are escalated (once per level, to the HR pool), auto-approved or
/// auto-rejected (SM-REQ R7/R8/R9; RG-07, LM-13). Days are counted from the request's last step, in calendar days (G-F).
pub async fn approval_timeouts(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let mut counts = json!({ "regularisation": { "escalated": 0, "auto_approved": 0, "auto_rejected": 0 },
                             "leave": { "escalated": 0, "auto_approved": 0, "auto_rejected": 0 } });
    let bump = |c: &mut Value, kind: &str, o: &Outcome| {
        let k = match o {
            Outcome::Escalated => "escalated",
            Outcome::AutoApproved => "auto_approved",
            Outcome::AutoRejected => "auto_rejected",
        };
        c[kind][k] = json!(c[kind][k].as_i64().unwrap_or(0) + 1);
    };

    // Regularisation: timeout from the RG settings in force on the request's date; last step from the audit log.
    let regs: Vec<(Uuid, Uuid, NaiveDate, String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT q.id, q.employee_id, q.attendance_date, q.state,
                coalesce((SELECT max(a.occurred_at) FROM audit.audit_entry a WHERE a.target_type = 'regularisation_request' AND a.target_id = q.id), q.created_at)
         FROM attendance.regularisation_request q WHERE q.state IN ('pending_l1','pending_l2') ORDER BY q.created_at",
    )
    .fetch_all(&mut ***tx)
    .await?;
    for (id, employee, date, state, last) in regs {
        let Some(master) = domain_timecap::punch::master_branch_on(tx, employee, date).await? else { continue };
        let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
        let s = domain_policy::rg::load(tx, resolved.and_then(|r| r.version_id)).await?;
        if Utc::now() - last < Duration::days(s.timeout_days) {
            continue;
        }
        savepoint(tx).await?;
        let r = regularisation_timeout(tx, id, &state, Duration::days(s.timeout_days), &s.timeout_action).await;
        if let Some(o) = settle(tx, "regularisation", id, r).await? {
            bump(&mut counts, "regularisation", &o);
        }
    }

    // Leave: timeout from the type version in force on the request's first day; last step from approval_step.
    let leaves: Vec<LeaveRow> = sqlx::query_as(
        "SELECT q.id, q.state, v.timeout_days, v.timeout_action,
                coalesce((SELECT max(s.acted_at) FROM app.approval_step s WHERE s.request_type = 'leave' AND s.request_id = q.id), q.submitted_at)
         FROM leave.leave_request q
         JOIN LATERAL (SELECT timeout_days, timeout_action FROM leave.leave_type_version v
                       WHERE v.leave_type_id = q.leave_type_id AND v.effective_from <= q.from_date ORDER BY v.effective_from DESC LIMIT 1) v ON true
         WHERE q.state IN ('pending_l1','pending_l2') ORDER BY q.submitted_at",
    )
    .fetch_all(&mut ***tx)
    .await?;
    for (id, state, days, action, last) in leaves {
        let Some(days) = days else { continue }; // no timeout configured
        if Utc::now() - last < Duration::days(days.into()) {
            continue;
        }
        savepoint(tx).await?;
        let r = leave_timeout(tx, id, &state, Duration::days(days.into()), &action).await;
        if let Some(o) = settle(tx, "leave", id, r).await? {
            bump(&mut counts, "leave", &o);
        }
    }
    db::audit_system(tx, "job.j3_approval_timeouts", "organisation", None, Some(counts.clone())).await?;
    Ok(counts)
}

async fn regularisation_timeout(tx: &mut OrgTx<'_>, id: Uuid, listed: &str, timeout: Duration, action: &str) -> Result<Option<Outcome>, DomainError> {
    // Lock, then re-check against the row: a person may have acted (or moved it a level) since the list was read.
    let (state, requester, employee, date, last): (String, Uuid, Uuid, NaiveDate, chrono::DateTime<Utc>) = sqlx::query_as(
        "SELECT q.state, q.requested_by_user_id, q.employee_id, q.attendance_date,
                coalesce((SELECT max(a.occurred_at) FROM audit.audit_entry a WHERE a.target_type = 'regularisation_request' AND a.target_id = q.id), q.created_at)
         FROM attendance.regularisation_request q WHERE q.id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_one(&mut ***tx)
    .await?;
    if state != listed || Utc::now() - last < timeout {
        return Ok(None);
    }
    let level: i16 = if state == "pending_l2" { 2 } else { 1 };
    let step = |action: &str| json!({ "action": action, "level": level, "comment": "approval timeout (RG-07)", "on_behalf_of": null });
    // An automatic approval never lands on a locked day (BR-06): that request is escalated to a person instead.
    let action = if action == "auto_approve" && db::employee_locked(tx, employee, date, date.succ_opt()).await? { "escalate" } else { action };
    match action {
        "auto_approve" | "auto_reject" => {
            let approve = action == "auto_approve";
            sqlx::query(
                "UPDATE attendance.regularisation_request SET state = $2, decided_basis = 'auto_on_timeout', decided_at = now(), current_approver_user_id = NULL,
                   updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .bind(if approve { "approved" } else { "rejected" })
            .execute(&mut ***tx)
            .await?;
            db::audit_system(tx, "regularisation.timeout", "regularisation_request", Some(id), Some(step(if approve { "auto_approved" } else { "auto_rejected" }))).await?;
            crate::approval::announce(tx, "regularisation", id, level, if approve { "auto_approved" } else { "auto_rejected" }, None).await?;
            if approve {
                // The employee's own request stands for the manual punch; the approval reference is the request (BR-01).
                crate::regularisation::apply(tx, requester, id).await?;
            }
            event(tx, if approve { "RequestApproved" } else { "RequestRejected" }, "regularisation", id, employee).await?;
            Ok(Some(if approve { Outcome::AutoApproved } else { Outcome::AutoRejected }))
        }
        _ => {
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM audit.audit_entry WHERE target_type = 'regularisation_request' AND target_id = $1
                                AND after_value->>'action' = 'escalated' AND (after_value->>'level')::int = $2)",
            )
            .bind(id)
            .bind(i32::from(level))
            .fetch_one(&mut ***tx)
            .await?;
            if already {
                return Ok(None); // once per level
            }
            sqlx::query("UPDATE attendance.regularisation_request SET escalated = true, current_approver_user_id = NULL, updated_at = now() WHERE id = $1")
                .bind(id)
                .execute(&mut ***tx)
                .await?;
            db::audit_system(tx, "regularisation.escalate", "regularisation_request", Some(id), Some(step("escalated"))).await?;
            crate::approval::announce(tx, "regularisation", id, level, "escalated", None).await?;
            event(tx, "RequestEscalated", "regularisation", id, employee).await?;
            Ok(Some(Outcome::Escalated))
        }
    }
}

/// state, requester, employee, leave type, first day, last step.
type LeaveLocked = (String, Uuid, Uuid, Uuid, NaiveDate, chrono::DateTime<Utc>);

async fn leave_timeout(tx: &mut OrgTx<'_>, id: Uuid, listed: &str, timeout: Duration, action: &str) -> Result<Option<Outcome>, DomainError> {
    let (state, requester, employee, leave_type, from, last): LeaveLocked = sqlx::query_as(
        "SELECT q.state, q.requested_by, q.employee_id, q.leave_type_id, q.from_date,
                coalesce((SELECT max(s.acted_at) FROM app.approval_step s WHERE s.request_type = 'leave' AND s.request_id = q.id), q.submitted_at)
         FROM leave.leave_request q WHERE q.id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_one(&mut ***tx)
    .await?;
    if state != listed || Utc::now() - last < timeout {
        return Ok(None);
    }
    let level: i16 = if state == "pending_l2" { 2 } else { 1 };
    let mut action = action;
    if action == "auto_approve" {
        // An automatic approval never overdraws the balance or touches a locked day: such a request is escalated instead.
        let v = crate::leave::version_on(tx, leave_type, from).await?;
        let mut blocked = false;
        for d in sqlx::query_scalar::<_, NaiveDate>("SELECT leave_date FROM leave.leave_request_day WHERE leave_request_id = $1").bind(id).fetch_all(&mut ***tx).await? {
            blocked |= db::employee_locked(tx, employee, d, d.succ_opt()).await?;
        }
        let comp_off = crate::comp_off::is_comp_off_type(tx, leave_type).await?;
        if let (true, Some(_), false) = (comp_off, &v, blocked) {
            // D-25: comp-off leave is debited from comp-off credits; short of them, the request is escalated.
            let total = crate::leave::total_pct(tx, id).await?;
            match crate::comp_off::debit(tx, None, employee, id, from, total).await {
                Ok(()) => {}
                Err(DomainError::Refused("LIMIT_EXCEEDED", _)) => blocked = true,
                Err(e) => return Err(e),
            }
        } else if let (Some(v), false) = (&v, blocked) {
            if v.paid {
                let account = crate::leave::lock_account(tx, employee, leave_type).await?;
                let total = crate::leave::total_pct(tx, id).await?;
                if crate::leave::balance_pct(tx, account).await? - total < -v.negative_allowed {
                    blocked = true;
                } else {
                    sqlx::query(
                        "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id)
                         VALUES (app.current_org(), $1, $2, 'debit', -($3::numeric / 100), 'leave_request', $4) ON CONFLICT DO NOTHING",
                    )
                    .bind(account)
                    .bind(from)
                    .bind(total as i32)
                    .bind(id)
                    .execute(&mut ***tx)
                    .await?;
                }
            }
        }
        if blocked || v.is_none() {
            action = "escalate";
        }
    }
    match action {
        "auto_approve" | "auto_reject" => {
            let approve = action == "auto_approve";
            sqlx::query("UPDATE leave.leave_request SET state = $2, decided_basis = 'auto_on_timeout', current_approver_user_id = NULL, updated_at = now() WHERE id = $1")
                .bind(id)
                .bind(if approve { "approved" } else { "rejected" })
                .execute(&mut ***tx)
                .await?;
            leave_step(tx, id, level, requester, if approve { "auto_approved" } else { "auto_rejected" }).await?;
            if approve {
                crate::leave::recalc_days(tx, employee, id, "leave_auto_approved").await?;
            }
            event(tx, if approve { "RequestApproved" } else { "RequestRejected" }, "leave", id, employee).await?;
            Ok(Some(if approve { Outcome::AutoApproved } else { Outcome::AutoRejected }))
        }
        _ => {
            let already: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM app.approval_step WHERE request_type = 'leave' AND request_id = $1 AND action = 'escalated' AND level = $2)",
            )
            .bind(id)
            .bind(level)
            .fetch_one(&mut ***tx)
            .await?;
            if already {
                return Ok(None);
            }
            sqlx::query("UPDATE leave.leave_request SET escalated = true, current_approver_user_id = NULL, updated_at = now() WHERE id = $1").bind(id).execute(&mut ***tx).await?;
            leave_step(tx, id, level, requester, "escalated").await?;
            event(tx, "RequestEscalated", "leave", id, employee).await?;
            Ok(Some(Outcome::Escalated))
        }
    }
}

async fn leave_step(tx: &mut OrgTx<'_>, id: Uuid, level: i16, requester: Uuid, action: &str) -> Result<(), DomainError> {
    let s = crate::approval::Step { kind: "leave", request_id: id, level, action, requester, actor: None, on_behalf_of: None, comment: Some("approval timeout (LM-13)") };
    crate::approval::step(tx, s).await
}

async fn event(tx: &mut OrgTx<'_>, kind: &str, request_type: &str, id: Uuid, employee: Uuid) -> Result<(), DomainError> {
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), $1, $2, $3)")
        .bind(kind)
        .bind(id)
        .bind(json!({ "request_type": request_type, "employee_id": employee, "by": "timeout" }))
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

/// J10: the organisation's usage summary for the vendor console (VC-R3, VC-R8), counted as the organisation and written
/// as the jobs role, whose row policy admits only this organisation's row; the vendor views never read client tables (P12).
pub async fn refresh_usage(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let row: (i64, i64, i64, i64, Option<chrono::DateTime<Utc>>, i64) = sqlx::query_as(
        // Active = not exited, as the VC-01 limit check counts them (future joiners included).
        "SELECT (SELECT count(*) FROM workforce.employee WHERE employment_status <> 'exited'),
                (SELECT count(*) FROM org.branch),
                (SELECT count(*) FROM timecap.device WHERE retired_at IS NULL AND online_state = 'online'),
                (SELECT count(*) FROM timecap.device WHERE retired_at IS NULL),
                (SELECT max(created_at) FROM iam.session),
                (SELECT count(*) FROM period.attendance_period WHERE state IN ('awaiting_lock','unlocked'))",
    )
    .fetch_one(&mut ***tx)
    .await?;
    // The row is the transaction's own organisation (RLS WITH CHECK on app_jobs), never a parameter.
    sqlx::query("SET LOCAL ROLE app_jobs").execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO platform.organisation_usage (org_id, active_employees, branches, devices_online, devices_total, last_sign_in_at, periods_awaiting_lock, refreshed_at)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, now())
         ON CONFLICT (org_id) DO UPDATE SET active_employees = EXCLUDED.active_employees, branches = EXCLUDED.branches, devices_online = EXCLUDED.devices_online,
           devices_total = EXCLUDED.devices_total, last_sign_in_at = EXCLUDED.last_sign_in_at, periods_awaiting_lock = EXCLUDED.periods_awaiting_lock, refreshed_at = now()",
    )
    .bind(row.0 as i32)
    .bind(row.1 as i32)
    .bind(row.2 as i32)
    .bind(row.3 as i32)
    .bind(row.4)
    .bind(row.5 as i32)
    .execute(&mut ***tx)
    .await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut ***tx).await?;
    Ok(json!({ "active_employees": row.0, "devices_total": row.3 }))
}
