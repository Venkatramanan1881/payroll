//! workforce.yaml: employee master.

use crate::{idempotency, ApiError, Auth};
use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use domain_iam::authz::{employee_in_scope, require, GrantKind};
use domain_workforce::employee::{self, EmployeeCreate};
use serde_json::Value;

// shortcut: "today" is the UTC date; per-branch local dates (BR-11) matter once dates near midnight drive rules.
pub(crate) fn today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

/// POST /v1/employees (WRK-01, scope: the new employee's master branch / department).
pub async fn create(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<(StatusCode, Json<Value>), ApiError> {
    let grant = require(&mut tx, &p, "WRK-01").await?;
    let key = idempotency::key(&headers)?;
    if let Some((status, body)) = idempotency::begin(&mut tx, key, &raw).await? {
        return Ok((status, Json(body)));
    }
    let input: EmployeeCreate = serde_json::from_slice(&raw).map_err(|e| ApiError::ValidationFailed(e.to_string()))?;
    let in_scope = matches!(grant.kind, GrantKind::All | GrantKind::Scoped)
        && employee_in_scope(&grant.scope, input.master_branch_id, input.department_id);
    if !in_scope {
        return Err(ApiError::Forbidden);
    }
    let id = employee::create_employee(&mut tx, p.user_id, input, today()).await?;
    let body = employee::load_employee(&mut tx, id, today()).await?.ok_or_else(|| ApiError::Internal(anyhow::anyhow!("created employee not visible")))?;
    idempotency::finish(&mut tx, key, StatusCode::CREATED, &body).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(body)))
}
