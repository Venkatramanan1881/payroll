//! policy.yaml: policy sets, versions, rule groups, publish (assignments in Task 5).

use crate::routes::employees::today;
use crate::routes::sched::{done, if_match, parse, reply, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::NaiveDate;
use domain_iam::authz::require;
use domain_policy::version;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewSet {
    name: String,
    effective_from: NaiveDate,
    copy_from_version_id: Option<Uuid>,
}

pub async fn create_set(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-01").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/policy-sets", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewSet = parse(&raw)?;
    let id = version::create_set(&mut tx, p.user_id, &r.name, r.effective_from, r.copy_from_version_id).await?;
    let body = version::get_set(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

pub async fn list_sets(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "POL-07").await?;
    Ok(Json(json!({ "items": version::list_sets(&mut tx).await?, "next_cursor": null })))
}

pub async fn get_set(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "POL-07").await?;
    Ok(Json(version::get_set(&mut tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewDraft {
    effective_from: NaiveDate,
    copy_from_version_id: Option<Uuid>,
}

pub async fn create_draft(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-01").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/policy-sets/{id}/versions"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewDraft = parse(&raw)?;
    let v = version::create_draft(&mut tx, p.user_id, id, r.effective_from, r.copy_from_version_id).await?;
    let body = version::get_version(&mut tx, v).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

pub async fn get_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-07").await?;
    Ok(reply(StatusCode::OK, version::get_version(&mut tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPatch {
    effective_from: Option<NaiveDate>,
    reason: Option<String>,
}

pub async fn update_version(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-01").await?;
    sqlx::query("SELECT 1 FROM policy.policy_version WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &version::get_version(&mut tx, id).await?)?;
    let r: DraftPatch = parse(&raw)?;
    version::update_draft(&mut tx, p.user_id, id, r.effective_from, r.reason).await?;
    let body = version::get_version(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

pub async fn get_rule_group(Auth(p, mut tx): Auth, Path((id, group)): Path<(Uuid, String)>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "POL-07").await?;
    Ok(Json(version::get_document(&mut tx, id, &group).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleGroupPut {
    schema_version: i32,
    settings: Value,
}

/// PUT a rule group (POL-01; PC also needs TCP-11, HW also needs CMP-01). If-Match carries the version's ETag.
pub async fn put_rule_group(Auth(p, mut tx): Auth, Path((id, group)): Path<(Uuid, String)>, headers: HeaderMap, raw: Bytes) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "POL-01").await?;
    match group.as_str() {
        "PC" => drop(require(&mut tx, &p, "TCP-11").await?),
        "HW" => drop(require(&mut tx, &p, "CMP-01").await?),
        _ => {}
    }
    sqlx::query("SELECT 1 FROM policy.policy_version WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &version::get_version(&mut tx, id).await?)?;
    let r: RuleGroupPut = parse(&raw)?;
    let (document, warnings) = version::put_document(&mut tx, p.user_id, id, &group, r.schema_version, r.settings).await?;
    tx.commit().await?;
    Ok(Json(json!({ "document": document, "warnings": warnings })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Switch {
    enabled: bool,
}

pub async fn switch_rule_group(Auth(p, mut tx): Auth, Path((id, group)): Path<(Uuid, String)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-06").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/policy-versions/{id}/rule-groups/{group}/switch"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: Switch = parse(&raw)?;
    let body = version::switch(&mut tx, p.user_id, id, &group, r.enabled).await?;
    done(tx, key, StatusCode::OK, body).await
}

pub async fn publish(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-02").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/policy-versions/{id}/publish"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    version::publish(&mut tx, p.user_id, id, today()).await?;
    let body = version::get_version(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

use axum::extract::Query;
use domain_policy::assignment;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAssignment {
    policy_set_id: Uuid,
    level: String,
    branch_id: Option<Uuid>,
    list_entry_id: Option<Uuid>,
    shift_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    valid_from: NaiveDate,
    valid_to: Option<NaiveDate>,
    reason: String,
}

pub async fn create_assignment(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-04").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/policy-assignments", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewAssignment = parse(&raw)?;
    let targets: Vec<Uuid> = [r.branch_id, r.list_entry_id, r.shift_id, r.employee_id].into_iter().flatten().collect();
    if targets.len() > 1 {
        return Err(ApiError::ValidationFailed("give at most one target id".into()));
    }
    let new = assignment::NewAssignment { policy_set_id: r.policy_set_id, level: r.level, target: targets.first().copied(), valid_from: r.valid_from, valid_to: r.valid_to, reason: r.reason };
    let id = assignment::create_assignment(&mut tx, p.user_id, new).await?;
    let body = assignment::get_assignment(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
pub struct AssignmentFilter {
    policy_set_id: Option<Uuid>,
    level: Option<String>,
    target_id: Option<Uuid>,
    on_date: Option<NaiveDate>,
}

pub async fn list_assignments(Auth(p, mut tx): Auth, Query(f): Query<AssignmentFilter>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "POL-07").await?;
    let items = assignment::list_assignments(&mut tx, f.policy_set_id, f.level.as_deref(), f.target_id, f.on_date).await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndRequest {
    valid_to: NaiveDate,
    reason: String,
}

pub async fn end_assignment(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "POL-04").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/policy-assignments/{id}/end"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: EndRequest = parse(&raw)?;
    assignment::end_assignment(&mut tx, p.user_id, id, r.valid_to, &r.reason).await?;
    let body = assignment::get_assignment(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}
