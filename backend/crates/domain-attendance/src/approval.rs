//! Shared approval flow pieces (DOMAIN_MODEL §6, SM-REQ): AC-02 routing, AC-03 delegation, who may decide, and the
//! append-only `app.approval_step` rows (BR-07 is also enforced by that table's CHECKs).

use chrono::{NaiveDate, Utc};
use db::{DomainError, OrgTx};
use domain_iam::authz::{employee_in_scope, require, AuthzError, GrantKind, Principal};
use uuid::Uuid;

/// AC-02 approver for a request type and level: a user, or None for the HR pool (anyone holding the level's grant with
/// the employee in scope). Defaults: level 1 the reporting manager, level 2 HR.
// shortcut: department_head routes to the HR pool until departments carry heads (OQ-C01).
pub async fn route(tx: &mut OrgTx<'_>, kind: &str, employee: Uuid, level: i16, today: NaiveDate) -> Result<Option<Uuid>, DomainError> {
    let rule: Option<(String, Option<Uuid>)> = sqlx::query_as("SELECT source, named_user_id FROM iam.approver_rule WHERE request_type = $1 AND level = $2")
        .bind(kind)
        .bind(level)
        .fetch_optional(&mut ***tx)
        .await?;
    let source = rule.as_ref().map_or(if level == 1 { "reporting_manager" } else { "hr" }, |r| r.0.as_str());
    let routed = match source {
        "named_user" => rule.and_then(|r| r.1),
        "reporting_manager" => sqlx::query_scalar(
            "SELECT u.id FROM workforce.employee_assignment a JOIN iam.user_account u ON u.employee_id = a.manager_employee_id AND u.active
             WHERE a.employee_id = $1 AND a.kind = 'reporting_manager' AND a.valid @> $2::date ORDER BY u.created_at LIMIT 1",
        )
        .bind(employee)
        .bind(today)
        .fetch_optional(&mut ***tx)
        .await?,
        _ => None,
    };
    // BR-07: the employee's own user never approves their request; it goes to the HR pool instead.
    Ok(match routed {
        Some(u) => {
            let own: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE id = $1 AND employee_id = $2)").bind(u).bind(employee).fetch_one(&mut ***tx).await?;
            if own { None } else { Some(u) }
        }
        None => None,
    })
}

/// Whether `p` may decide with grant `op`: Ok(Some(on_behalf_of)) as a delegate (AC-03), Ok(None) as the approver or
/// from the HR pool. NotFound when not routed to the caller or out of scope, so a request's existence never leaks.
pub async fn may_decide(tx: &mut OrgTx<'_>, p: &Principal, employee: Uuid, op: &str, approver: Option<Uuid>) -> Result<Option<Uuid>, DomainError> {
    let grant = match require(tx, p, op).await {
        Ok(g) => g,
        Err(AuthzError::Forbidden) => return Err(DomainError::NotFound),
        Err(AuthzError::Db(e)) => return Err(e.into()),
    };
    match approver {
        Some(a) if a == p.user_id => Ok(None),
        Some(a) => {
            // AC-03 off (organisation setting) means delegations are not honoured; the BRD gives no default, so on.
            let delegated: bool = sqlx::query_scalar(
                "SELECT coalesce((SELECT (settings->>'delegation_enabled')::boolean FROM org.organisation_settings_version WHERE effective_from <= current_date
                                  ORDER BY effective_from DESC LIMIT 1), true)
                    AND EXISTS (SELECT 1 FROM iam.delegation WHERE approver_user_id = $1 AND delegate_user_id = $2 AND valid @> now())",
            )
            .bind(a)
            .bind(p.user_id)
            .fetch_one(&mut ***tx)
            .await?;
            // A delegate never acts for the employee's own user (BR-07 through delegation).
            let for_employee: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE id = $1 AND employee_id = $2)").bind(a).bind(employee).fetch_one(&mut ***tx).await?;
            if delegated && !for_employee { Ok(Some(a)) } else { Err(DomainError::NotFound) }
        }
        None => {
            let today = Utc::now().date_naive();
            let job: Vec<(String, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
                "SELECT kind, branch_id, list_entry_id FROM workforce.employee_assignment WHERE employee_id = $1 AND valid @> $2::date AND kind IN ('master_branch','department')",
            )
            .bind(employee)
            .bind(today)
            .fetch_all(&mut ***tx)
            .await?;
            let master = job.iter().find(|j| j.0 == "master_branch").and_then(|j| j.1);
            let dept = job.iter().find(|j| j.0 == "department").and_then(|j| j.2);
            let in_scope = matches!(grant.kind, GrantKind::All | GrantKind::Scoped) && master.is_some_and(|m| employee_in_scope(&grant.scope, m, dept));
            if in_scope { Ok(None) } else { Err(DomainError::NotFound) }
        }
    }
}

pub struct Step<'a> {
    pub kind: &'a str,
    pub request_id: Uuid,
    pub level: i16,
    pub action: &'a str,
    pub requester: Uuid,
    /// None = the system (an automatic decision).
    pub actor: Option<Uuid>,
    pub on_behalf_of: Option<Uuid>,
    pub comment: Option<&'a str>,
}

/// One approval step row. clock_timestamp keeps steps written in one transaction in order.
pub async fn step(tx: &mut OrgTx<'_>, s: Step<'_>) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO app.approval_step (org_id, request_type, request_id, level, action, requester_user_id, actor_user_id, on_behalf_of_user_id, comment, acted_at)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, clock_timestamp())",
    )
    .bind(s.kind)
    .bind(s.request_id)
    .bind(s.level)
    .bind(s.action)
    .bind(s.requester)
    .bind(s.actor)
    .bind(s.on_behalf_of)
    .bind(s.comment)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// SQL for the ApprovalStep list of a request (`id_sql` names the request id column).
pub fn steps_sql(kind: &str, id_sql: &str) -> String {
    format!(
        "coalesce((SELECT jsonb_agg(jsonb_build_object('level', s.level, 'action', s.action, 'actor_user_id', s.actor_user_id,
            'on_behalf_of_user_id', s.on_behalf_of_user_id, 'comment', s.comment, 'acted_at', s.acted_at) ORDER BY s.acted_at, s.id)
          FROM app.approval_step s WHERE s.request_type = '{kind}' AND s.request_id = {id_sql}), '[]')"
    )
}
