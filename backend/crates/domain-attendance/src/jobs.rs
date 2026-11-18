//! Timed jobs (TSD §10): J1 nightly recalculation and J3 approval timeouts. Each organisation is processed in its own
//! organisation-scoped transaction (RLS), exactly like a request; one system audit entry records each run.

use chrono::{Duration, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Active organisations. Only the vendor role sees every organisation row, so the listing runs as `app_vendor`
/// (the job login role is a member of it; DEPLOYMENT.md). Nothing else is read under that role.
pub async fn active_organisations(pool: &PgPool) -> Result<Vec<Uuid>, DomainError> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL ROLE app_vendor").execute(&mut *tx).await?;
    let ids = sqlx::query_scalar("SELECT id FROM platform.organisation WHERE state = 'active' ORDER BY id").fetch_all(&mut *tx).await?;
    tx.rollback().await?;
    Ok(ids)
}

/// Days J1 looks back: open days whose missed-punch cut-off or DS-10 waiting time may have passed since the last run.
const LOOKBACK_DAYS: i64 = 7;

/// J1: recalculates the last `LOOKBACK_DAYS` days (today excluded) for every current employee of one organisation.
/// Unchanged inputs write nothing (BR-02); locked and future days are skipped by `recalculate` itself.
// shortcut: one transaction per organisation; split per employee into Cloud Tasks when organisations grow large.
pub async fn nightly_recalculation(tx: &mut OrgTx<'_>, today: NaiveDate) -> Result<Value, DomainError> {
    let employees: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM workforce.employee WHERE employment_status <> 'exited' AND joining_date < $1 ORDER BY id",
    )
    .bind(today)
    .fetch_all(&mut ***tx)
    .await?;
    let mut written = 0;
    for e in &employees {
        for back in (1..=LOOKBACK_DAYS).rev() {
            if crate::recalculate(tx, *e, today - Duration::days(back), "j1_nightly", None).await?.is_some() {
                written += 1;
            }
        }
    }
    let summary = json!({ "employees": employees.len(), "results_written": written, "days_back": LOOKBACK_DAYS });
    db::audit_system(tx, "job.j1_nightly_recalculation", "organisation", None, Some(summary.clone())).await?;
    Ok(summary)
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
        let level: i16 = if state == "pending_l2" { 2 } else { 1 };
        if let Some(o) = regularisation_timeout(tx, id, level, &s.timeout_action).await? {
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
        let level: i16 = if state == "pending_l2" { 2 } else { 1 };
        if let Some(o) = leave_timeout(tx, id, level, &action).await? {
            bump(&mut counts, "leave", &o);
        }
    }
    db::audit_system(tx, "job.j3_approval_timeouts", "organisation", None, Some(counts.clone())).await?;
    Ok(counts)
}

async fn regularisation_timeout(tx: &mut OrgTx<'_>, id: Uuid, level: i16, action: &str) -> Result<Option<Outcome>, DomainError> {
    // Lock and re-check: a person may have decided since the list was read.
    let (state, requester, employee): (String, Uuid, Uuid) =
        sqlx::query_as("SELECT state, requested_by_user_id, employee_id FROM attendance.regularisation_request WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut ***tx)
            .await?;
    if !matches!(state.as_str(), "pending_l1" | "pending_l2") {
        return Ok(None);
    }
    let step = |action: &str| json!({ "action": action, "level": level, "comment": "approval timeout (RG-07)", "on_behalf_of": null });
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
            event(tx, "RequestEscalated", "regularisation", id, employee).await?;
            Ok(Some(Outcome::Escalated))
        }
    }
}

async fn leave_timeout(tx: &mut OrgTx<'_>, id: Uuid, level: i16, action: &str) -> Result<Option<Outcome>, DomainError> {
    let (state, requester, employee, leave_type, from): (String, Uuid, Uuid, Uuid, NaiveDate) =
        sqlx::query_as("SELECT state, requested_by, employee_id, leave_type_id, from_date FROM leave.leave_request WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut ***tx)
            .await?;
    if !matches!(state.as_str(), "pending_l1" | "pending_l2") {
        return Ok(None);
    }
    let mut action = action;
    if action == "auto_approve" {
        // An automatic approval never overdraws the balance or touches a locked day: such a request is escalated instead.
        let v = crate::leave::version_on(tx, leave_type, from).await?;
        let mut blocked = false;
        for d in sqlx::query_scalar::<_, NaiveDate>("SELECT leave_date FROM leave.leave_request_day WHERE leave_request_id = $1").bind(id).fetch_all(&mut ***tx).await? {
            blocked |= db::employee_locked(tx, employee, d, d.succ_opt()).await?;
        }
        if let (Some(v), false) = (&v, blocked) {
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
