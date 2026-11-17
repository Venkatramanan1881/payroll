//! Operation grants (iam.role_grant) and data scope (iam.user_scope), read per request so revocation is immediate.

use db::OrgTx;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Principal {
    pub user_id: Uuid,
    pub org_id: Uuid,
    pub employee_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GrantKind {
    All,
    Scoped,
    SelfOnly,
    Approve,
}

/// AC-07 data scope. `Nothing` = no scope rows, or rows mixing branches and departments
/// (how those combine is OQ-C03-04, so access is refused until it is decided).
#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    All,
    Branches(Vec<Uuid>),
    Departments(Vec<Uuid>),
    Nothing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Grant {
    pub kind: GrantKind,
    pub scope: Scope,
}

#[derive(Debug)]
pub enum AuthzError {
    Forbidden,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AuthzError {
    fn from(e: sqlx::Error) -> Self {
        AuthzError::Db(e)
    }
}

pub async fn require(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Grant, AuthzError> {
    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT g.grant_kind FROM iam.role_grant g JOIN iam.user_role r ON r.org_id = g.org_id AND r.role_id = g.role_id
         WHERE r.user_id = $1 AND g.operation_code = $2",
    )
    .bind(p.user_id)
    .bind(op)
    .fetch_all(&mut ***tx)
    .await?;
    // Strongest grant wins when several roles grant the same operation.
    let kind = ["all", "scoped", "self", "approve"]
        .into_iter()
        .find(|k| kinds.iter().any(|g| g == k))
        .map(|k| match k {
            "all" => GrantKind::All,
            "scoped" => GrantKind::Scoped,
            "self" => GrantKind::SelfOnly,
            _ => GrantKind::Approve,
        })
        .ok_or(AuthzError::Forbidden)?;
    let rows: Vec<(String, Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("SELECT scope_kind, branch_id, department_id FROM iam.user_scope WHERE user_id = $1")
            .bind(p.user_id)
            .fetch_all(&mut ***tx)
            .await?;
    let scope = if rows.iter().any(|r| r.0 == "all") {
        Scope::All
    } else {
        let branches: Vec<Uuid> = rows.iter().filter_map(|r| r.1).collect();
        let departments: Vec<Uuid> = rows.iter().filter_map(|r| r.2).collect();
        match (branches.is_empty(), departments.is_empty()) {
            (false, true) => Scope::Branches(branches),
            (true, false) => Scope::Departments(departments),
            _ => Scope::Nothing,
        }
    };
    Ok(Grant { kind, scope })
}

pub fn employee_in_scope(scope: &Scope, master_branch: Uuid, department: Option<Uuid>) -> bool {
    match scope {
        Scope::All => true,
        Scope::Branches(b) => b.contains(&master_branch),
        Scope::Departments(d) => department.is_some_and(|dep| d.contains(&dep)),
        Scope::Nothing => false,
    }
}
