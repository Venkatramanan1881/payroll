//! identity.yaml: Firebase sign-in, organisation choice, refresh, sign-out.

use crate::{unix_now, ApiError, AppState, Auth};
use axum::extract::{rejection::JsonRejection, State};
use axum::http::StatusCode;
use axum::Json;
use domain_iam::{signin, tokens};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

pub(crate) fn body<T>(r: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    r.map(|Json(t)| t).map_err(|e| ApiError::ValidationFailed(e.body_text()))
}

fn token_pair(state: &AppState, user_id: Uuid, org_id: Uuid, sid: Uuid, refresh: String) -> Json<Value> {
    let access = tokens::issue_access(&state.jwt_secret, user_id, org_id, Some(sid), unix_now());
    Json(json!({ "access_token": access, "refresh_token": refresh, "expires_in": tokens::ACCESS_TTL_SECS, "token_type": "Bearer" }))
}

async fn open(state: &AppState, org_id: Uuid, user_id: Uuid, device: Option<&str>) -> Result<Json<Value>, ApiError> {
    let (sid, refresh) = signin::open_session(&state.pool, org_id, user_id, device).await?.ok_or(ApiError::Unauthenticated)?;
    Ok(token_pair(state, user_id, org_id, sid, refresh))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FirebaseSignIn {
    firebase_id_token: String,
    device_label: Option<String>,
}

pub async fn firebase_sign_in(State(state): State<AppState>, req: Result<Json<FirebaseSignIn>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let req = body(req)?;
    // Bad token and unknown number give the same answer, so the API never confirms a number exists.
    let phone = state.phone_verifier.verify(&req.firebase_id_token).await.map_err(|_| ApiError::Unauthenticated)?;
    let cands = signin::candidates(&state.pool, &phone).await?;
    match cands.as_slice() {
        [] => Err(ApiError::Unauthenticated),
        [only] => open(&state, only.org_id, only.user_id, req.device_label.as_deref()).await,
        many => {
            let choice = tokens::issue_choice(&state.jwt_secret, many.iter().map(|c| (c.org_id, c.user_id)).collect(), unix_now());
            let refs = many.iter().enumerate().map(|(i, c)| (i.to_string(), c.display_name.clone())).collect();
            Err(ApiError::OrganisationChoiceRequired(choice, refs))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChooseOrganisation {
    choice_token: String,
    candidate_ref: String,
}

pub async fn choose_organisation(State(state): State<AppState>, req: Result<Json<ChooseOrganisation>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let req = body(req)?;
    let cands = tokens::verify_choice(&state.jwt_secret, &req.choice_token, unix_now()).map_err(|_| ApiError::Unauthenticated)?;
    let (org_id, user_id) = req.candidate_ref.parse::<usize>().ok().and_then(|i| cands.get(i).copied()).ok_or(ApiError::Unauthenticated)?;
    open(&state, org_id, user_id, None).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refresh {
    refresh_token: String,
}

pub async fn refresh(State(state): State<AppState>, req: Result<Json<Refresh>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let req = body(req)?;
    let (org_id, user_id, sid, refresh) = signin::rotate_session(&state.pool, &req.refresh_token).await?.ok_or(ApiError::Unauthenticated)?;
    Ok(token_pair(&state, user_id, org_id, sid, refresh))
}

pub async fn sign_out(Auth(p, mut tx): Auth) -> Result<StatusCode, ApiError> {
    if let Some(sid) = p.session_id {
        signin::revoke_session(&mut tx, p.user_id, sid).await?;
        tx.commit().await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
