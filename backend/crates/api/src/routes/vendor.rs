//! vendor.yaml: the vendor console (V01). Vendor tokens only (D-22); organisation tokens get 401. Mounted only where
//! the vendor sign-in is configured; elsewhere every route answers 404.

use crate::idempotency;
use crate::routes::sched::{if_match, parse, reply};
use crate::{unix_now, ApiError, AppState};
use axum::body::Bytes;
use axum::extract::{FromRequestParts, Path, Query, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::NaiveDate;
use db::OrgTx;
use domain_iam::vendor::{self as v, NewOrg, PlanIn, Subscription};
use domain_iam::tokens;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

/// A signed-in, active vendor user and a transaction as `app_vendor`.
pub struct Vendor {
    id: Uuid,
    role: String,
    tx: OrgTx<'static>,
}

impl FromRequestParts<AppState> for Vendor {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if state.vendor_verifier.is_none() {
            return Err(ApiError::NotFound);
        }
        let token = parts.headers.get("authorization").and_then(|h| h.to_str().ok()).and_then(|h| h.strip_prefix("Bearer ")).ok_or(ApiError::Unauthenticated)?;
        let id = tokens::verify_vendor(&state.jwt_secret, token, unix_now()).map_err(|_| ApiError::Unauthenticated)?;
        let mut tx = db::begin_vendor(&state.pool).await?;
        // Re-read per request: a deactivated vendor user stops at once.
        let role = v::vendor_role(&mut tx, id).await?.ok_or(ApiError::Unauthenticated)?;
        Ok(Vendor { id, role, tx })
    }
}

fn need(role: &str, op: &str) -> Result<(), ApiError> {
    if v::allowed(role, op) { Ok(()) } else { Err(ApiError::Forbidden) }
}

fn today() -> NaiveDate {
    (chrono::Utc::now() + chrono::Duration::minutes(330)).date_naive()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoogleSignIn {
    id_token: String,
}

/// POST /v1/vendor/auth/google/sign-in (D-22): a Google ID token of the vendor's domain -> a 1-hour vendor token.
pub async fn google_sign_in(State(state): State<AppState>, raw: Bytes) -> Result<Json<Value>, ApiError> {
    let verifier = state.vendor_verifier.clone().ok_or(ApiError::NotFound)?;
    let b: GoogleSignIn = parse(&raw)?;
    let email = verifier.verify(&b.id_token).await.map_err(|_| ApiError::Unauthenticated)?;
    let mut tx = db::begin_vendor(&state.pool).await?;
    let id = v::vendor_by_email(&mut tx, &email).await?.ok_or(ApiError::Unauthenticated)?;
    sqlx::query("INSERT INTO audit.audit_entry (actor_kind, actor_id, action, target_type, target_id) VALUES ('vendor_user', $1, 'vendor.signed_in', 'vendor_user', $1)")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "access_token": tokens::issue_vendor(&state.jwt_secret, id, unix_now()), "expires_in": tokens::VENDOR_TTL_SECS, "token_type": "Bearer" })))
}

/// Creates: the client key makes a retry return what the first call created (one key, one organisation).
async fn keyed(tx: &mut OrgTx<'static>, headers: &HeaderMap, action: &str) -> Result<(Uuid, Option<Uuid>), ApiError> {
    let key = idempotency::key(headers)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))").bind(key.to_string()).execute(&mut ***tx).await?;
    Ok((key, v::replay(tx, action, key).await?))
}

async fn done(tx: OrgTx<'static>, status: StatusCode, body: Value) -> Result<Response, ApiError> {
    tx.commit().await?;
    Ok(reply(status, body))
}

// ---------------------------------------------------------------------------------------------------------------------
// Organisations

#[derive(Deserialize)]
pub struct OrgQuery {
    state: Option<String>,
    sector_template_id: Option<Uuid>,
    q: Option<String>,
}

pub async fn list_orgs(mut vu: Vendor, Query(q): Query<OrgQuery>) -> Result<Json<Value>, ApiError> {
    need(&vu.role, "PLT-10")?;
    if q.q.as_deref().is_some_and(|s| s.chars().count() > 100) {
        return Err(ApiError::ValidationFailed("q is at most 100 characters".into()));
    }
    Ok(Json(json!({ "items": v::list_orgs(&mut vu.tx, q.state.as_deref(), q.sector_template_id, q.q.as_deref()).await?, "next_cursor": null })))
}

pub async fn get_org(mut vu: Vendor, Path(id): Path<Uuid>) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-10")?;
    Ok(reply(StatusCode::OK, v::get_org(&mut vu.tx, id).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitIn {
    max_active: Option<i32>,
    on_reach: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdminIn {
    email: String,
    mobile: Option<String>,
    language: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscriptionIn {
    start_date: NaiveDate,
    renewal_date: NaiveDate,
    billing_status: String,
    renewal_reminder_days: Option<i32>,
    #[serde(default, deserialize_with = "present")]
    auto_suspend_after_days: Option<Option<i32>>,
    reason: Option<String>,
}

/// Tells "absent" (None) from "null" (Some(None)).
fn present<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Option<i32>>, D::Error> {
    Ok(Some(Option::<i32>::deserialize(d)?))
}

impl SubscriptionIn {
    fn into_domain(self) -> Subscription {
        Subscription {
            start_date: self.start_date,
            renewal_date: self.renewal_date,
            billing_status: self.billing_status,
            renewal_reminder_days: self.renewal_reminder_days,
            auto_suspend_after_days: self.auto_suspend_after_days,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrgIn {
    display_name: String,
    legal_name: String,
    sector_template_id: Uuid,
    plan_id: Uuid,
    modules: Vec<String>,
    employee_limit: LimitIn,
    first_admin: AdminIn,
    subscription: SubscriptionIn,
}

/// POST /v1/vendor/organisations (PLT-01).
pub async fn create_org(mut vu: Vendor, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-01")?;
    let (key, replay) = keyed(&mut vu.tx, &headers, "vendor.organisation_created").await?;
    if let Some(org) = replay {
        let body = v::get_org(&mut vu.tx, org).await?;
        return done(vu.tx, StatusCode::CREATED, body).await;
    }
    let o: OrgIn = parse(&raw)?;
    let mut modules = o.modules;
    modules.sort();
    modules.dedup();
    let new = NewOrg {
        display_name: o.display_name,
        legal_name: o.legal_name,
        sector_template_id: o.sector_template_id,
        plan_id: o.plan_id,
        modules,
        max_active: o.employee_limit.max_active,
        on_reach: o.employee_limit.on_reach,
        admin_email: o.first_admin.email,
        admin_mobile: o.first_admin.mobile,
        admin_language: o.first_admin.language,
        subscription: o.subscription.into_domain(),
    };
    let org = v::create_org(&mut vu.tx, vu.id, new, key).await?;
    let body = v::get_org(&mut vu.tx, org).await?;
    done(vu.tx, StatusCode::CREATED, body).await
}

/// Runs one organisation command under the row lock and If-Match.
async fn org_command(vu: &mut Vendor, id: Uuid, headers: &HeaderMap, op: &str) -> Result<(), ApiError> {
    need(&vu.role, op)?;
    idempotency::key(headers)?;
    v::lock_org(&mut vu.tx, id).await?;
    if_match(headers, &v::get_org(&mut vu.tx, id).await?)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanChangeIn {
    plan_id: Uuid,
    valid_from: Option<NaiveDate>,
    reason: String,
}

pub async fn change_plan(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    org_command(&mut vu, id, &headers, "PLT-03").await?;
    let b: PlanChangeIn = parse(&raw)?;
    let body = v::change_plan(&mut vu.tx, vu.id, id, b.plan_id, b.valid_from.unwrap_or_else(today), &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitChangeIn {
    max_active: Option<i32>,
    valid_from: Option<NaiveDate>,
    reason: String,
}

pub async fn change_limit(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    org_command(&mut vu, id, &headers, "PLT-03").await?;
    let b: LimitChangeIn = parse(&raw)?;
    let body = v::change_limit(&mut vu.tx, vu.id, id, b.max_active, None, b.valid_from.unwrap_or_else(today), &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VendorSettingsIn {
    on_reach: Option<String>,
    renewal_reminder_days: Option<i32>,
    #[serde(default, deserialize_with = "present")]
    auto_suspend_after_days: Option<Option<i32>>,
    reason: String,
}

pub async fn change_vendor_settings(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    org_command(&mut vu, id, &headers, "PLT-08").await?;
    let b: VendorSettingsIn = parse(&raw)?;
    let body = v::change_vendor_settings(&mut vu.tx, vu.id, id, b.on_reach.as_deref(), b.renewal_reminder_days, b.auto_suspend_after_days, today(), &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

pub async fn change_subscription(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    org_command(&mut vu, id, &headers, "PLT-05").await?;
    let b: SubscriptionIn = parse(&raw)?;
    let reason = b.reason.clone().ok_or_else(|| ApiError::ValidationFailed("reason is required".into()))?;
    let body = v::change_subscription(&mut vu.tx, vu.id, id, b.into_domain(), &reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SwitchIn {
    valid_from: Option<NaiveDate>,
    reason: String,
}

async fn switch(mut vu: Vendor, id: Uuid, module: String, headers: HeaderMap, raw: Bytes, on: bool) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-02")?;
    idempotency::key(&headers)?;
    v::lock_org(&mut vu.tx, id).await?;
    let b: SwitchIn = parse(&raw)?;
    let body = v::switch_module(&mut vu.tx, vu.id, id, &module, on, b.valid_from.unwrap_or_else(today), &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

pub async fn module_on(vu: Vendor, Path((id, module)): Path<(Uuid, String)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    switch(vu, id, module, headers, raw, true).await
}

pub async fn module_off(vu: Vendor, Path((id, module)): Path<(Uuid, String)>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    switch(vu, id, module, headers, raw, false).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReasonIn {
    reason: String,
}

async fn state_change(mut vu: Vendor, id: Uuid, headers: HeaderMap, raw: Bytes, suspend: bool) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-06")?;
    idempotency::key(&headers)?;
    v::lock_org(&mut vu.tx, id).await?;
    let b: ReasonIn = parse(&raw)?;
    let body = v::set_state(&mut vu.tx, vu.id, id, suspend, &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

pub async fn suspend(vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    state_change(vu, id, headers, raw, true).await
}

pub async fn reactivate(vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    state_change(vu, id, headers, raw, false).await
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    kind: Option<String>,
}

pub async fn history(mut vu: Vendor, Path(id): Path<Uuid>, Query(q): Query<HistoryQuery>) -> Result<Json<Value>, ApiError> {
    need(&vu.role, "PLT-19")?;
    Ok(Json(json!({ "items": v::history(&mut vu.tx, id, q.kind.as_deref()).await?, "next_cursor": null })))
}

// ---------------------------------------------------------------------------------------------------------------------
// Plans

pub async fn list_plans(mut vu: Vendor) -> Result<Json<Value>, ApiError> {
    need(&vu.role, "PLT-10")?;
    Ok(Json(json!({ "items": v::list_plans(&mut vu.tx).await? })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanBody {
    code: Option<String>,
    name: Option<String>,
    modules: Option<Vec<String>>,
    #[serde(default, deserialize_with = "present")]
    employee_limit: Option<Option<i32>>,
    mobile_app: Option<bool>,
}

impl PlanBody {
    fn into_domain(self) -> PlanIn {
        PlanIn { code: self.code, name: self.name, modules: self.modules, employee_limit: self.employee_limit, mobile_app: self.mobile_app }
    }
}

pub async fn create_plan(mut vu: Vendor, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-04")?;
    let (key, replay) = keyed(&mut vu.tx, &headers, "vendor.plan_created").await?;
    if let Some(id) = replay {
        let body = v::get_plan(&mut vu.tx, id).await?;
        return done(vu.tx, StatusCode::CREATED, body).await;
    }
    let b: PlanBody = parse(&raw)?;
    if b.code.is_none() || b.name.is_none() || b.modules.is_none() || b.mobile_app.is_none() {
        return Err(ApiError::ValidationFailed("code, name, modules and mobile_app are required".into()));
    }
    let body = v::create_plan(&mut vu.tx, vu.id, b.into_domain(), key).await?;
    done(vu.tx, StatusCode::CREATED, body).await
}

pub async fn update_plan(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-04")?;
    let b: PlanBody = parse(&raw)?;
    if b.code.is_some() {
        return Err(ApiError::ValidationFailed("a plan's code does not change".into()));
    }
    sqlx::query("SELECT 1 FROM platform.plan WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **vu.tx).await?;
    if_match(&headers, &v::get_plan(&mut vu.tx, id).await?)?;
    let body = v::update_plan(&mut vu.tx, vu.id, id, b.into_domain()).await?;
    done(vu.tx, StatusCode::OK, body).await
}

// ---------------------------------------------------------------------------------------------------------------------
// Vendor users

pub async fn list_vendor_users(mut vu: Vendor) -> Result<Json<Value>, ApiError> {
    need(&vu.role, "PLT-12")?;
    Ok(Json(json!({ "items": v::list_vendor_users(&mut vu.tx).await?, "next_cursor": null })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VendorUserIn {
    email: String,
    full_name: String,
    vendor_role: String,
}

pub async fn create_vendor_user(mut vu: Vendor, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-12")?;
    let (key, replay) = keyed(&mut vu.tx, &headers, "vendor.vendor_user_created").await?;
    if let Some(id) = replay {
        let body = v::get_vendor_user(&mut vu.tx, id).await?;
        return done(vu.tx, StatusCode::CREATED, body).await;
    }
    let b: VendorUserIn = parse(&raw)?;
    let body = v::create_vendor_user(&mut vu.tx, vu.id, &b.email, &b.full_name, &b.vendor_role, key).await?;
    done(vu.tx, StatusCode::CREATED, body).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoleIn {
    vendor_role: String,
    reason: String,
}

pub async fn change_vendor_role(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-12")?;
    idempotency::key(&headers)?;
    let b: RoleIn = parse(&raw)?;
    sqlx::query("SELECT 1 FROM platform.vendor_user WHERE id = $1 FOR UPDATE").bind(id).execute(&mut **vu.tx).await?;
    if_match(&headers, &v::get_vendor_user(&mut vu.tx, id).await?)?;
    let body = v::change_vendor_role(&mut vu.tx, vu.id, id, &b.vendor_role, &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}

pub async fn deactivate_vendor_user(mut vu: Vendor, Path(id): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    need(&vu.role, "PLT-12")?;
    idempotency::key(&headers)?;
    let b: ReasonIn = parse(&raw)?;
    let body = v::deactivate_vendor_user(&mut vu.tx, vu.id, id, &b.reason).await?;
    done(vu.tx, StatusCode::OK, body).await
}
