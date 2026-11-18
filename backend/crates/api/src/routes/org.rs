//! organisation.yaml: organisation, branches, geo-fences, IP allow lists, lists, contractors, organisation settings.

use crate::routes::employees::today;
use crate::routes::sched::{done, if_match, parse, reply, start};
use crate::{ApiError, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::NaiveDate;
use db::OrgTx;
use domain_iam::authz::{require, Principal, Scope};
use domain_org as org;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// Branches the caller may act on with `op`: None = all; department scopes reach no branch settings.
async fn branch_scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str) -> Result<Option<Vec<Uuid>>, ApiError> {
    Ok(match require(tx, p, op).await?.scope {
        Scope::All => None,
        Scope::Branches(b) => Some(b),
        Scope::Departments(_) | Scope::Nothing => Some(vec![]),
    })
}

/// The branch must be in the caller's scope for `op`; otherwise it does not exist for them (404).
async fn branch_in_scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str, branch: Uuid) -> Result<(), ApiError> {
    match branch_scope(tx, p, op).await? {
        Some(b) if !b.contains(&branch) => Err(ApiError::NotFound),
        _ => Ok(()),
    }
}

/// GET /v1/organisation (ORG-07).
pub async fn organisation(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(Json(org::organisation(&mut tx).await?))
}

/// GET /v1/branches (ORG-07, within scope).
pub async fn list_branches(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    let scope = branch_scope(&mut tx, &p, "ORG-07").await?;
    Ok(Json(json!({ "items": org::list_branches(&mut tx, scope.as_deref()).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewBranch {
    name: String,
    code: String,
    address: Option<String>,
    tz: String,
    time_zone_valid_from: NaiveDate,
}

/// POST /v1/branches (ORG-01; a branch-scoped caller cannot create branches outside their scope, so only All may).
pub async fn create_branch(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    if branch_scope(&mut tx, &p, "ORG-01").await?.is_some() {
        return Err(ApiError::Forbidden);
    }
    let (key, replay) = start(&mut tx, &headers, "POST /v1/branches", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: NewBranch = parse(&raw)?;
    let id = org::create_branch(&mut tx, p.user_id, &r.code, &r.name, r.address.as_deref(), &r.tz, r.time_zone_valid_from).await?;
    let body = org::get_branch(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// GET /v1/branches/{id} (ORG-07).
pub async fn get_branch(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-07", id).await?;
    Ok(reply(StatusCode::OK, org::get_branch(&mut tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchPatch {
    name: Option<String>,
    code: Option<String>,
    address: Option<String>,
}

/// PATCH /v1/branches/{id} (ORG-01, If-Match).
pub async fn update_branch(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-01", id).await?;
    sqlx::query("SELECT 1 FROM org.branch WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &org::get_branch(&mut tx, id).await?)?;
    let r: BranchPatch = parse(&raw)?;
    org::update_branch(&mut tx, p.user_id, id, r.code.as_deref(), r.name.as_deref(), r.address.as_deref()).await?;
    let body = org::get_branch(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

/// GET /v1/branches/{id}/geo-fences (ORG-07).
pub async fn list_fences(Auth(p, mut tx): Auth, Path(branch): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-07", branch).await?;
    Ok(Json(json!({ "items": org::list_fences(&mut tx, branch).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FenceIn {
    name: Option<String>,
    latitude: f64,
    longitude: f64,
    radius_m: i32,
}

impl FenceIn {
    fn into_domain(self) -> org::FenceIn {
        org::FenceIn { name: self.name, lat: self.latitude, lon: self.longitude, radius_m: self.radius_m }
    }
}

/// POST /v1/branches/{id}/geo-fences (ORG-02).
pub async fn create_fence(Auth(p, mut tx): Auth, Path(branch): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-02", branch).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/branches/{branch}/geo-fences"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: FenceIn = parse(&raw)?;
    let id = org::create_fence(&mut tx, p.user_id, branch, r.into_domain()).await?;
    let body = org::get_fence(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

async fn fence_in_scope(tx: &mut OrgTx<'_>, p: &Principal, op: &str, id: Uuid) -> Result<(), ApiError> {
    let branch = org::fence_branch(tx, id).await?;
    branch_in_scope(tx, p, op, branch).await
}

/// PATCH /v1/geo-fences/{id} (ORG-02, If-Match). A changed fence is switched off until tested again.
pub async fn update_fence(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    fence_in_scope(&mut tx, &p, "ORG-02", id).await?;
    sqlx::query("SELECT 1 FROM org.geo_fence WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &org::get_fence(&mut tx, id).await?)?;
    let r: FenceIn = parse(&raw)?;
    org::update_fence(&mut tx, p.user_id, id, r.into_domain()).await?;
    let body = org::get_fence(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestIn {
    latitude: f64,
    longitude: f64,
    #[allow(dead_code)] // reported by the device; the test compares the point itself
    accuracy_m: Option<f64>,
}

/// POST /v1/geo-fences/{id}/test (ORG-02, PC-R14).
pub async fn test_fence(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    fence_in_scope(&mut tx, &p, "ORG-02", id).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/geo-fences/{id}/test"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: TestIn = parse(&raw)?;
    let body = org::test_fence(&mut tx, p.user_id, id, r.latitude, r.longitude).await?;
    done(tx, key, StatusCode::OK, body).await
}

async fn switch(p: Principal, mut tx: OrgTx<'static>, id: Uuid, headers: HeaderMap, raw: Bytes, on: bool) -> Result<Response, ApiError> {
    fence_in_scope(&mut tx, &p, "ORG-02", id).await?;
    let op = format!("POST /v1/geo-fences/{id}/switch-{}", if on { "on" } else { "off" });
    let (key, replay) = start(&mut tx, &headers, &op, &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    org::switch_fence(&mut tx, p.user_id, id, on).await?;
    let body = org::get_fence(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// POST /v1/geo-fences/{id}/switch-on (ORG-02; only after a test).
pub async fn switch_on(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    switch(p, tx, id, headers, raw, true).await
}

/// POST /v1/geo-fences/{id}/switch-off (ORG-02).
pub async fn switch_off(Auth(p, tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    switch(p, tx, id, headers, raw, false).await
}

/// GET /v1/branches/{id}/ip-allow-list (ORG-07).
pub async fn ip_list(Auth(p, mut tx): Auth, Path(branch): Path<Uuid>) -> Result<Response, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-07", branch).await?;
    let mut body = org::ip_list(&mut tx, branch).await?;
    body["updated_at"] = org::get_branch(&mut tx, branch).await?["updated_at"].clone(); // the list's ETag is the branch's
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IpChange {
    networks: Vec<String>,
    reason: String,
}

/// POST /v1/branches/{id}/ip-allow-list/change (ORG-03, If-Match = the branch ETag). AC-04 approvals are not built: applied directly.
pub async fn change_ip_list(Auth(p, mut tx): Auth, Path(branch): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    branch_in_scope(&mut tx, &p, "ORG-03", branch).await?;
    sqlx::query("SELECT 1 FROM org.branch WHERE id = $1 FOR UPDATE").bind(branch).execute(&mut **tx).await?;
    if_match(&headers, &org::get_branch(&mut tx, branch).await?)?;
    let r: IpChange = parse(&raw)?;
    org::replace_ip_list(&mut tx, p.user_id, branch, &r.networks, &r.reason).await?;
    let mut body = org::ip_list(&mut tx, branch).await?;
    body["updated_at"] = org::get_branch(&mut tx, branch).await?["updated_at"].clone();
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
pub struct Retired {
    include_retired: Option<bool>,
}

/// GET /v1/lists/{kind} (ORG-07).
pub async fn list_entries(Auth(p, mut tx): Auth, Path(kind): Path<String>, Query(q): Query<Retired>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(Json(json!({ "items": org::list_entries(&mut tx, &kind, q.include_retired.unwrap_or(false)).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryIn {
    name: Option<String>,
    code: Option<String>,
}

/// POST /v1/lists/{kind} (ORG-04).
pub async fn create_entry(Auth(p, mut tx): Auth, Path(kind): Path<String>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-04").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/lists/{kind}"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: EntryIn = parse(&raw)?;
    let id = org::create_entry(&mut tx, p.user_id, &kind, r.name.as_deref().unwrap_or(""), r.code.as_deref()).await?;
    let body = org::get_entry(&mut tx, &kind, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// PATCH /v1/lists/{kind}/{id} (ORG-04, If-Match).
pub async fn update_entry(Auth(p, mut tx): Auth, Path((kind, id)): Path<(String, Uuid)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-04").await?;
    sqlx::query("SELECT 1 FROM org.list_entry WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &org::get_entry(&mut tx, &kind, id).await?)?;
    let r: EntryIn = parse(&raw)?;
    org::update_entry(&mut tx, p.user_id, &kind, id, r.name.as_deref(), r.code.as_deref()).await?;
    let body = org::get_entry(&mut tx, &kind, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

/// POST /v1/lists/{kind}/{id}/retire (ORG-04).
pub async fn retire_entry(Auth(p, mut tx): Auth, Path((kind, id)): Path<(String, Uuid)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-04").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/lists/{kind}/{id}/retire"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    org::retire_entry(&mut tx, p.user_id, &kind, id).await?;
    let body = org::get_entry(&mut tx, &kind, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

/// GET /v1/contractors (ORG-07).
pub async fn list_contractors(Auth(p, mut tx): Auth, Query(q): Query<Retired>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(Json(json!({ "items": org::list_contractors(&mut tx, q.include_retired.unwrap_or(false)).await?, "next_cursor": null })))
}

/// GET /v1/contractors/{id} (ORG-07).
pub async fn get_contractor(Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(reply(StatusCode::OK, org::get_contractor(&mut tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContractorIn {
    name: Option<String>,
    contact: Option<String>,
}

/// POST /v1/contractors (ORG-05).
pub async fn create_contractor(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-05").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/contractors", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: ContractorIn = parse(&raw)?;
    let id = org::create_contractor(&mut tx, p.user_id, r.name.as_deref().unwrap_or(""), r.contact.as_deref()).await?;
    let body = org::get_contractor(&mut tx, id).await?;
    done(tx, key, StatusCode::CREATED, body).await
}

/// PATCH /v1/contractors/{id} (ORG-05, If-Match).
pub async fn update_contractor(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-05").await?;
    sqlx::query("SELECT 1 FROM org.contractor WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **tx).await?;
    if_match(&headers, &org::get_contractor(&mut tx, id).await?)?;
    let r: ContractorIn = parse(&raw)?;
    org::update_contractor(&mut tx, p.user_id, id, r.name.as_deref(), r.contact.as_deref()).await?;
    let body = org::get_contractor(&mut tx, id).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

/// POST /v1/contractors/{id}/retire (ORG-05).
pub async fn retire_contractor(Auth(p, mut tx): Auth, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-05").await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/contractors/{id}/retire"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    org::retire_contractor(&mut tx, p.user_id, id).await?;
    let body = org::get_contractor(&mut tx, id).await?;
    done(tx, key, StatusCode::OK, body).await
}

#[derive(Deserialize)]
pub struct OnQuery {
    on: Option<NaiveDate>,
}

/// GET /v1/organisation-settings (ORG-07): the version in force on `on` (default today).
pub async fn settings(Auth(p, mut tx): Auth, Query(q): Query<OnQuery>) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(Json(org::settings_on(&mut tx, q.on.unwrap_or_else(today)).await?))
}

/// GET /v1/organisation-settings/versions (ORG-07).
pub async fn settings_versions(Auth(p, mut tx): Auth) -> Result<Json<Value>, ApiError> {
    require(&mut tx, &p, "ORG-07").await?;
    Ok(Json(json!({ "items": org::settings_versions(&mut tx).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettingsIn {
    effective_from: NaiveDate,
    reason: String,
    schema_version: i32,
    settings: Value,
}

/// POST /v1/organisation-settings/versions (ORG-06; AC-04 "HR Manager only": saved directly, 201).
pub async fn create_settings_version(Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "ORG-06").await?;
    let (key, replay) = start(&mut tx, &headers, "POST /v1/organisation-settings/versions", &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let r: SettingsIn = parse(&raw)?;
    let id = org::create_settings_version(&mut tx, p.user_id, r.effective_from, &r.reason, r.schema_version, r.settings).await?;
    let mut body = org::get_settings_version(&mut tx, id).await?;
    body["warnings"] = json!([]);
    done(tx, key, StatusCode::CREATED, body).await
}
