//! Dated policy assignments (G3) and the PolicyResolver (BR-05, decision D-03).

use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// Most specific first (decision D-03).
pub const PRECEDENCE: [&str; 6] = ["employee", "shift", "department", "category", "branch", "organisation"];

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedPolicy {
    pub policy_set_id: Uuid,
    /// The set's published version in force on the date by effective date; None if it has none yet.
    pub version_id: Option<Uuid>,
    pub level: String,
}

const ASSIGNMENT_JSON: &str = "jsonb_build_object('id', a.id, 'policy_set_id', a.policy_set_id, 'level', a.level, 'branch_id', a.branch_id,
    'list_entry_id', a.list_entry_id, 'shift_id', a.shift_id, 'employee_id', a.employee_id, 'valid_from', lower(a.valid),
    'valid_to', upper(a.valid), 'reason', a.reason, 'created_at', a.created_at, 'created_by', a.created_by)";

pub struct NewAssignment {
    pub policy_set_id: Uuid,
    pub level: String,
    pub target: Option<Uuid>,
    pub valid_from: NaiveDate,
    pub valid_to: Option<NaiveDate>,
    pub reason: String,
}

/// The target column for a level (fixed list; never input text).
fn target_column(level: &str) -> Result<Option<&'static str>, DomainError> {
    Ok(match level {
        "organisation" => None,
        "branch" => Some("branch_id"),
        "department" | "category" => Some("list_entry_id"),
        "shift" => Some("shift_id"),
        "employee" => Some("employee_id"),
        other => return Err(invalid(format!("unknown level {other}"))),
    })
}

pub async fn get_assignment(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {ASSIGNMENT_JSON} FROM policy.policy_assignment a WHERE a.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// Overlap with another assignment at the same level and target is refused by the exclusion constraint (422).
pub async fn create_assignment(tx: &mut OrgTx<'_>, actor: Uuid, a: NewAssignment) -> Result<Uuid, DomainError> {
    if a.reason.trim().is_empty() {
        return Err(invalid("reason"));
    }
    let column = target_column(&a.level)?;
    if column.is_some() != a.target.is_some() {
        return Err(invalid(format!("level {} needs exactly its own target id", a.level)));
    }
    if a.valid_to.is_some_and(|to| to <= a.valid_from) {
        return Err(invalid("valid_to must be after valid_from"));
    }
    if db::any_locked(tx, a.valid_from, a.valid_to).await? {
        return Err(DomainError::PeriodLocked);
    }
    let (col_sql, val_sql) = column.map(|c| (format!(", {c}"), ", $6")).unwrap_or_default();
    let sql = format!(
        "INSERT INTO policy.policy_assignment (org_id, policy_set_id, level, valid, reason, created_by{col_sql})
         VALUES (app.current_org(), $1, $2, daterange($3, $4), $5, $7{val_sql}) RETURNING id"
    );
    let id: Uuid = sqlx::query_scalar(&sql)
        .bind(a.policy_set_id)
        .bind(&a.level)
        .bind(a.valid_from)
        .bind(a.valid_to)
        .bind(a.reason.trim())
        .bind(a.target)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    db::audit(tx, actor, "policy_assignment.create", "policy_assignment", Some(id), None).await?;
    Ok(id)
}

pub async fn list_assignments(tx: &mut OrgTx<'_>, set: Option<Uuid>, level: Option<&str>, target: Option<Uuid>, on: Option<NaiveDate>) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {ASSIGNMENT_JSON} FROM policy.policy_assignment a
         WHERE ($1::uuid IS NULL OR a.policy_set_id = $1) AND ($2::text IS NULL OR a.level = $2)
           AND ($3::uuid IS NULL OR $3 IN (a.branch_id, a.list_entry_id, a.shift_id, a.employee_id)) AND ($4::date IS NULL OR a.valid @> $4::date)
         ORDER BY a.level, lower(a.valid)"
    );
    Ok(sqlx::query_scalar(&sql).bind(set).bind(level).bind(target).bind(on).fetch_all(&mut ***tx).await?)
}

/// Ends an assignment on `to` (exclusive). It must still be open past `to` and start before it.
pub async fn end_assignment(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, to: NaiveDate, reason: &str) -> Result<(), DomainError> {
    if reason.trim().is_empty() {
        return Err(invalid("reason"));
    }
    let row: Option<(NaiveDate, Option<NaiveDate>)> = sqlx::query_as("SELECT lower(valid), upper(valid) FROM policy.policy_assignment WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?;
    let (from, current_to) = row.ok_or(DomainError::NotFound)?;
    if to <= from || current_to.is_some_and(|c| to > c) {
        return Err(invalid("valid_to must fall inside the assignment"));
    }
    // Ending changes which policy applies on [to, old end): refuse if that touches locked days.
    if db::any_locked(tx, to, current_to).await? {
        return Err(DomainError::PeriodLocked);
    }
    sqlx::query("UPDATE policy.policy_assignment SET valid = daterange(lower(valid), $2) WHERE id = $1").bind(id).bind(to).execute(&mut ***tx).await?;
    db::audit(tx, actor, "policy_assignment.end", "policy_assignment", Some(id), Some(json!({ "valid_to": to, "reason": reason }))).await?;
    Ok(())
}

/// BR-05 / D-03: the most specific assignment valid on `date` wins; then that set's published version by effective date.
/// `branch` is only for the expected day's shift (holiday calendar, D-04); the branch level uses the master branch (EM-R9).
pub async fn resolve_policy(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate, branch: Uuid) -> Result<Option<ResolvedPolicy>, DomainError> {
    let shift = domain_sched::schedule::expected_day(tx, employee, date, branch).await?.shift_id;
    let job: Vec<(String, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "SELECT kind, branch_id, list_entry_id FROM workforce.employee_assignment
         WHERE employee_id = $1 AND valid @> $2::date AND kind IN ('master_branch','department','category')",
    )
    .bind(employee)
    .bind(date)
    .fetch_all(&mut ***tx)
    .await?;
    let of = |k: &str| job.iter().find(|j| j.0 == k).and_then(|j| j.1.or(j.2));
    for level in PRECEDENCE {
        let target = match level {
            "employee" => Some(employee),
            "shift" => shift,
            "department" => of("department"),
            "category" => of("category"),
            "branch" => of("master_branch"),
            _ => None,
        };
        if level != "organisation" && target.is_none() {
            continue;
        }
        let set: Option<Uuid> = sqlx::query_scalar(
            "SELECT policy_set_id FROM policy.policy_assignment WHERE level = $1 AND valid @> $2::date AND ($3::uuid IS NULL OR target_key = $3) LIMIT 1",
        )
        .bind(level)
        .bind(date)
        .bind(target)
        .fetch_optional(&mut ***tx)
        .await?;
        if let Some(set) = set {
            let version: Option<Uuid> = sqlx::query_scalar(
                "SELECT id FROM policy.policy_version WHERE policy_set_id = $1 AND state IN ('scheduled','in_force','superseded') AND effective_from <= $2
                 ORDER BY effective_from DESC, created_at DESC LIMIT 1",
            )
            .bind(set)
            .bind(date)
            .fetch_optional(&mut ***tx)
            .await?;
            return Ok(Some(ResolvedPolicy { policy_set_id: set, version_id: version, level: level.to_string() }));
        }
    }
    Ok(None)
}
