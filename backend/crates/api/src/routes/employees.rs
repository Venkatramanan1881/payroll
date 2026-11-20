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
    if let Some((status, body)) = idempotency::begin(&mut tx, key, "POST /v1/employees", &raw).await? {
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

use axum::extract::{Path, Query};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use db::OrgTx;
use domain_iam::authz::Principal;
use domain_workforce::assignment;
use uuid::Uuid;

/// The employee as the caller may see it: outside the caller's organisation or scope it is NOT_FOUND, never FORBIDDEN.
pub(crate) async fn load_in_scope(tx: &mut OrgTx<'_>, p: &Principal, id: Uuid, op: &str, lock: bool) -> Result<Value, ApiError> {
    let grant = require(tx, p, op).await?;
    if lock {
        // Writes hold the row from the ETag check to commit, so two edits of one version cannot both apply.
        sqlx::query("SELECT 1 FROM workforce.employee WHERE id = $1 FOR UPDATE").bind(id).execute(&mut ***tx).await?;
    }
    let emp = employee::load_employee(tx, id, today()).await?.ok_or(ApiError::NotFound)?;
    let uuid_at = |k: &str| emp["current"][k].as_str().and_then(|s| Uuid::parse_str(s).ok());
    let visible = match grant.kind {
        GrantKind::All | GrantKind::Scoped => uuid_at("master_branch_id").is_some_and(|b| employee_in_scope(&grant.scope, b, uuid_at("department_id"))),
        GrantKind::SelfOnly => p.employee_id == Some(id),
        GrantKind::Approve => false,
    };
    if visible { Ok(emp) } else { Err(ApiError::NotFound) }
}

fn if_match(headers: &HeaderMap, current: &Value) -> Result<(), ApiError> {
    let sent = headers.get(header::IF_MATCH).ok_or(ApiError::PreconditionRequired)?;
    if sent.to_str().ok() == Some(employee::etag(current).as_str()) { Ok(()) } else { Err(ApiError::PreconditionFailed) }
}

fn with_etag(status: StatusCode, body: Value, etag: &str) -> Response {
    (status, [(header::ETAG, etag.to_string())], Json(body)).into_response()
}

/// GET /v1/employees/{employee_id} (WRK-08).
pub async fn get(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    let emp = load_in_scope(&mut tx, &p, id, "WRK-08", false).await?;
    let tag = employee::etag(&emp);
    Ok(with_etag(StatusCode::OK, emp, &tag))
}

/// PATCH /v1/employees/{employee_id} (WRK-02).
pub async fn patch(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let emp = load_in_scope(&mut tx, &p, id, "WRK-02", true).await?;
    if_match(&headers, &emp)?;
    let fields: serde_json::Map<String, Value> = serde_json::from_slice(&raw).map_err(|e| ApiError::ValidationFailed(e.to_string()))?;
    employee::update_employee(&mut tx, p.user_id, id, fields, today()).await?;
    let emp = employee::load_employee(&mut tx, id, today()).await?.ok_or(ApiError::NotFound)?;
    tx.commit().await?;
    let tag = employee::etag(&emp);
    Ok(with_etag(StatusCode::OK, emp, &tag))
}

#[derive(serde::Deserialize)]
pub struct ListQuery {
    limit: Option<String>,
    cursor: Option<String>,
    branch_id: Option<String>,
    department_id: Option<String>,
    contractor_id: Option<String>,
    employment_status: Option<String>,
    q: Option<String>,
    sort: Option<String>,
}

/// GET /v1/employees (WRK-08): the employees the caller's scope covers today; Self grants list only their own record.
pub async fn list(Auth(p, mut tx): Auth, Query(q): Query<ListQuery>) -> Result<Json<Value>, ApiError> {
    use domain_iam::authz::Scope;
    use employee::Visible;
    let bad = |m: &str| ApiError::ValidationFailed(m.to_string());
    let grant = require(&mut tx, &p, "WRK-08").await?;
    let id = |v: &Option<String>, name: &str| v.as_deref().map(Uuid::parse_str).transpose().map_err(|_| bad(&format!("{name} is not an id")));
    let limit: i64 = match q.limit.as_deref() {
        None => 50,
        Some(s) => s.parse().ok().filter(|n| (1..=200).contains(n)).ok_or_else(|| bad("limit is 1-200"))?,
    };
    // The cursor is the offset of the next page, opaque to clients.
    let offset: i64 = match q.cursor.as_deref() {
        None => 0,
        Some(c) => c.parse().ok().filter(|n| *n >= 0).ok_or_else(|| bad("cursor is not valid"))?,
    };
    let order = employee::order_by(q.sort.as_deref().unwrap_or("employee_code")).ok_or_else(|| bad("sort is not offered"))?;
    if q.employment_status.as_deref().is_some_and(|s| !["on_probation", "active", "on_notice", "exited"].contains(&s)) {
        return Err(bad("employment_status is not valid"));
    }
    if q.q.as_deref().is_some_and(|s| s.chars().count() > 100) {
        return Err(bad("q is at most 100 characters"));
    }
    let filter = employee::ListFilter {
        q: q.q.filter(|s| !s.trim().is_empty()),
        status: q.employment_status,
        branch: id(&q.branch_id, "branch_id")?,
        department: id(&q.department_id, "department_id")?,
        contractor: id(&q.contractor_id, "contractor_id")?,
    };
    let visible = match (&grant.kind, &grant.scope) {
        (GrantKind::All | GrantKind::Scoped, Scope::All) => Visible::All,
        (GrantKind::All | GrantKind::Scoped, Scope::Branches(b)) => Visible::Branches(b),
        (GrantKind::All | GrantKind::Scoped, Scope::Departments(d)) => Visible::Departments(d),
        (GrantKind::SelfOnly, _) if let Some(me) = p.employee_id => Visible::Only(me),
        _ => return Ok(Json(serde_json::json!({ "items": [], "next_cursor": null }))),
    };
    let mut items = employee::list_employees(&mut tx, today(), visible, &filter, order, limit, offset).await?;
    let more = items.len() as i64 > limit;
    items.truncate(limit as usize);
    Ok(Json(serde_json::json!({ "items": items, "next_cursor": more.then(|| (offset + limit).to_string()) })))
}

#[derive(serde::Deserialize)]
pub struct AssignmentFilter {
    kind: Option<String>,
    on: Option<chrono::NaiveDate>,
}

/// GET /v1/employees/{employee_id}/assignments (WRK-08).
pub async fn list_assignments(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, Query(f): Query<AssignmentFilter>) -> Result<Json<Value>, ApiError> {
    load_in_scope(&mut tx, &p, id, "WRK-08", false).await?;
    let items = assignment::list_assignments(&mut tx, id, f.kind.as_deref(), f.on).await?;
    Ok(Json(serde_json::json!({ "items": items })))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeAssignment {
    kind: String,
    value_id: Option<Uuid>,
    valid_from: chrono::NaiveDate,
    reason: String,
}

/// POST /v1/employees/{employee_id}/change-assignment (WRK-03). Idempotency-Key and If-Match (employee ETag).
pub async fn change_assignment(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let emp = load_in_scope(&mut tx, &p, id, "WRK-03", true).await?;
    let key = idempotency::key(&headers)?;
    // A retry replays the stored answer even though the ETag has since moved on.
    if let Some((status, body)) = idempotency::begin(&mut tx, key, &format!("POST /v1/employees/{id}/change-assignment"), &raw).await? {
        let tag = employee::etag(&employee::load_employee(&mut tx, id, today()).await?.ok_or(ApiError::NotFound)?);
        return Ok(with_etag(status, body, &tag));
    }
    if_match(&headers, &emp)?;
    let req: ChangeAssignment = serde_json::from_slice(&raw).map_err(|e| ApiError::ValidationFailed(e.to_string()))?;
    let after = assignment::change_assignment(&mut tx, p.user_id, id, &req.kind, req.value_id, req.valid_from, &req.reason).await?;
    let body = serde_json::json!({ "assignment": after, "adjustment_posted": false });
    idempotency::finish(&mut tx, key, StatusCode::OK, &body).await?;
    let tag = employee::etag(&employee::load_employee(&mut tx, id, today()).await?.ok_or(ApiError::NotFound)?);
    tx.commit().await?;
    Ok(with_etag(StatusCode::OK, body, &tag))
}
