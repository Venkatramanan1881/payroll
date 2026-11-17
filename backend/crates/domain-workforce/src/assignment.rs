//! Dated assignment history (EM-R6): list and change. A change ends the current range and starts the next one.

use crate::employee::{insert_assignment, invalid, outbox, target_column, WorkforceError};
use chrono::NaiveDate;
use db::OrgTx;
use serde_json::{json, Value};
use uuid::Uuid;

const ASSIGNMENT_JSON: &str = "jsonb_build_object(
    'id', a.id, 'kind', a.kind, 'branch_id', a.branch_id, 'list_entry_id', a.list_entry_id, 'list_kind', a.list_kind,
    'manager_employee_id', a.manager_employee_id, 'contractor_id', a.contractor_id,
    'valid_from', lower(a.valid), 'valid_to', upper(a.valid), 'reason', a.reason, 'created_at', a.created_at)";

pub async fn list_assignments(tx: &mut OrgTx<'_>, employee: Uuid, kind: Option<&str>, on: Option<NaiveDate>) -> Result<Vec<Value>, WorkforceError> {
    let sql = format!(
        "SELECT {ASSIGNMENT_JSON} FROM workforce.employee_assignment a
         WHERE a.employee_id = $1 AND ($2::text IS NULL OR a.kind = $2) AND ($3::date IS NULL OR a.valid @> $3::date)
         ORDER BY a.kind, lower(a.valid)"
    );
    Ok(sqlx::query_scalar(&sql).bind(employee).bind(kind).bind(on).fetch_all(&mut ***tx).await?)
}

/// change-assignment (WRK-03). `value` None ends the assignment (contractor only). Returns the affected assignment.
pub async fn change_assignment(
    tx: &mut OrgTx<'_>,
    actor: Uuid,
    employee: Uuid,
    kind: &str,
    value: Option<Uuid>,
    valid_from: NaiveDate,
    reason: &str,
) -> Result<Value, WorkforceError> {
    if kind == "master_branch" || target_column(kind).is_none() {
        return Err(invalid("kind (master branch uses transfer-master-branch)"));
    }
    if value.is_none() && kind != "contractor" {
        return Err(invalid("value_id can be null only for contractor"));
    }
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(invalid("reason"));
    }
    if let (Some(v), "department" | "designation" | "category" | "employee_type") = (value, kind) {
        let usable: Option<bool> = sqlx::query_scalar("SELECT retired_at IS NULL FROM org.list_entry WHERE id = $1 AND kind = $2")
            .bind(v)
            .bind(kind)
            .fetch_optional(&mut ***tx)
            .await?;
        if usable != Some(true) {
            return Err(invalid("value_id"));
        }
    }
    let joining: NaiveDate = sqlx::query_scalar("SELECT joining_date FROM workforce.employee WHERE id = $1")
        .bind(employee)
        .fetch_one(&mut ***tx)
        .await?;
    if valid_from < joining {
        return Err(invalid("valid_from is before the joining date"));
    }
    let locked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.lock_coverage WHERE employee_id = $1 AND dates @> $2::date)")
        .bind(employee)
        .bind(valid_from)
        .fetch_one(&mut ***tx)
        .await?;
    if locked {
        return Err(WorkforceError::PeriodLocked); // OQ-C02-08: refused, not back-posted
    }

    let current: Option<(Uuid, NaiveDate, Value)> = sqlx::query_as(&format!(
        "SELECT a.id, lower(a.valid), {ASSIGNMENT_JSON} FROM workforce.employee_assignment a
         WHERE a.employee_id = $1 AND a.kind = $2 AND a.valid @> $3::date FOR UPDATE"
    ))
    .bind(employee)
    .bind(kind)
    .bind(valid_from)
    .fetch_optional(&mut ***tx)
    .await?;
    if let Some((id, start, _)) = &current {
        if *start >= valid_from {
            return Err(invalid("valid_from must be after the current assignment's start"));
        }
        sqlx::query("UPDATE workforce.employee_assignment SET valid = daterange(lower(valid), $2) WHERE id = $1")
            .bind(id)
            .bind(valid_from)
            .execute(&mut ***tx)
            .await?;
    } else if value.is_none() {
        return Err(invalid("no current assignment to end"));
    }

    let affected = match value {
        Some(v) => {
            let id = insert_assignment(tx, employee, kind, v, valid_from, actor).await?;
            sqlx::query("UPDATE workforce.employee_assignment SET reason = $2 WHERE id = $1").bind(id).bind(reason).execute(&mut ***tx).await?;
            id
        }
        None => current.as_ref().map(|c| c.0).expect("checked above"),
    };
    let after: Value = sqlx::query_scalar(&format!("SELECT {ASSIGNMENT_JSON} FROM workforce.employee_assignment a WHERE a.id = $1"))
        .bind(affected)
        .fetch_one(&mut ***tx)
        .await?;
    // Touch the employee so its ETag changes with its assignments.
    sqlx::query("UPDATE workforce.employee SET updated_by = $2 WHERE id = $1").bind(employee).bind(actor).execute(&mut ***tx).await?;
    let change = json!({ "kind": kind, "valid_from": valid_from, "reason": reason, "before": current.map(|c| c.2), "after": after });
    db::audit(tx, actor, "employee.assignment_change", "employee", Some(employee), Some(change)).await?;
    outbox(tx, "EmployeeAssignmentChanged", employee).await?;
    Ok(after)
}
