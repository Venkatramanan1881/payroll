//! Authentication: bearer token -> organisation-scoped transaction + Principal (API_CONVENTIONS §4 steps 1-2).

use crate::{ApiError, AppState};
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, HeaderMap, Method};
use db::OrgTx;
use domain_iam::authz::Principal;
use domain_iam::tokens::verify_access;
use uuid::Uuid;

pub fn unix_now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("clock after 1970").as_secs() as i64
}

pub async fn authenticate(state: &AppState, headers: &HeaderMap, is_write: bool) -> Result<(Principal, OrgTx<'static>), ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthenticated)?;
    let claims = verify_access(&state.jwt_secret, token, unix_now()).map_err(|_| ApiError::Unauthenticated)?;
    let mut tx = db::begin_org(&state.pool, claims.org).await?;
    // RLS hides users of other organisations, so a token pairing a user with the wrong organisation finds nothing.
    let user: Option<(bool, Option<Uuid>)> = sqlx::query_as("SELECT active, employee_id FROM iam.user_account WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(&mut **tx)
        .await?;
    let Some((true, employee_id)) = user else { return Err(ApiError::Unauthenticated) };
    let org_state: Option<String> = sqlx::query_scalar("SELECT state FROM platform.organisation WHERE id = $1")
        .bind(claims.org)
        .fetch_optional(&mut **tx)
        .await?;
    match org_state.as_deref() {
        None => return Err(ApiError::Unauthenticated),
        Some("suspended") if is_write => return Err(ApiError::OrgSuspended), // VC-R4: view and export only
        _ => {}
    }
    Ok((Principal { user_id: claims.sub, org_id: claims.org, employee_id }, tx))
}

/// Axum extractor for authenticated routes. Writes are anything other than GET/HEAD.
pub struct Auth(pub Principal, pub OrgTx<'static>);

impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let is_write = !matches!(parts.method, Method::GET | Method::HEAD);
        let (p, tx) = authenticate(state, &parts.headers, is_write).await?;
        Ok(Auth(p, tx))
    }
}
