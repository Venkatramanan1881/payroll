//! identity.yaml: Firebase sign-in, organisation choice, refresh, sign-out.

use crate::{unix_now, ApiError, AppState};
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
    let (cands, password) = tokens::verify_choice(&state.jwt_secret, &req.choice_token, unix_now()).map_err(|_| ApiError::Unauthenticated)?;
    let (org_id, user_id) = req.candidate_ref.parse::<usize>().ok().and_then(|i| cands.get(i).copied()).ok_or(ApiError::Unauthenticated)?;
    if password {
        return after_password(&state, org_id, user_id, None).await; // the chosen organisation's AC-08 still applies
    }
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

/// Not treated as a write for VC-R4: a suspended organisation's users must still be able to end their session.
pub async fn sign_out(State(state): State<AppState>, headers: axum::http::HeaderMap) -> Result<StatusCode, ApiError> {
    let (p, mut tx) = crate::authenticate(&state, &headers, false).await?;
    if let Some(sid) = p.session_id {
        signin::revoke_session(&mut tx, p.user_id, sid).await?;
        tx.commit().await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------------------------------------------------
// Admin password sign-in and the AC-08 second step (UM-R3, D-23)

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasswordSignIn {
    email: String,
    password: String,
    device_label: Option<String>,
}

/// After a correct password: tokens when AC-08 is off; otherwise SECOND_FACTOR_REQUIRED with a challenge (and, on the
/// first sign-in, a new secret to enrol, sealed inside the challenge until its first code is right).
async fn after_password(state: &AppState, org_id: Uuid, user_id: Uuid, device: Option<&str>) -> Result<Json<Value>, ApiError> {
    let (on, stored) = domain_iam::password::second_step(&state.pool, org_id, user_id).await?;
    if !on {
        return open(state, org_id, user_id, device).await;
    }
    let key = state.mfa_key.ok_or_else(|| ApiError::Internal(anyhow::anyhow!("MFA_ENCRYPTION_KEY is not set: AC-08 sign-in cannot proceed")))?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let (pending, uri) = match stored {
        Some(_) => (None, None),
        None => {
            let secret = domain_iam::totp::new_secret();
            let email: Option<String> = {
                let mut tx = db::begin_org(&state.pool, org_id).await?;
                sqlx::query_scalar("SELECT email FROM iam.user_account WHERE id = $1").bind(user_id).fetch_one(&mut **tx).await?
            };
            let uri = domain_iam::totp::otpauth_uri(&secret, email.as_deref().unwrap_or("admin"));
            (Some(b64.encode(domain_iam::totp::encrypt(&key, &secret, &domain_iam::password::secret_aad(org_id, user_id)))), Some(uri))
        }
    };
    let challenge = tokens::issue_challenge(&state.jwt_secret, user_id, org_id, pending, unix_now());
    Err(ApiError::SecondFactorRequired(challenge, uri))
}

pub async fn password_sign_in(State(state): State<AppState>, req: Result<Json<PasswordSignIn>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let req = body(req)?;
    if req.email.len() > 254 || req.password.is_empty() || req.password.chars().count() > 200 {
        return Err(ApiError::ValidationFailed("email (up to 254) and password (1-200 characters) are required".into()));
    }
    // A wrong password and an unknown email give the same answer.
    let cands = domain_iam::password::candidates(&state.pool, req.email.trim(), &req.password).await?;
    match cands.as_slice() {
        [] => Err(ApiError::Unauthenticated),
        [only] => after_password(&state, only.org_id, only.user_id, req.device_label.as_deref()).await,
        many => {
            let choice = tokens::issue_password_choice(&state.jwt_secret, many.iter().map(|c| (c.org_id, c.user_id)).collect(), unix_now());
            let refs = many.iter().enumerate().map(|(i, c)| (i.to_string(), c.display_name.clone())).collect();
            Err(ApiError::OrganisationChoiceRequired(choice, refs))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecondFactor {
    challenge_token: String,
    code: String,
}

pub async fn verify_second_factor(State(state): State<AppState>, req: Result<Json<SecondFactor>, JsonRejection>) -> Result<Json<Value>, ApiError> {
    let req = body(req)?;
    if !(6..=8).contains(&req.code.len()) || !req.code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ApiError::ValidationFailed("code is 6-8 digits".into()));
    }
    let (user_id, org_id, pending) = tokens::verify_challenge(&state.jwt_secret, &req.challenge_token, unix_now()).map_err(|_| ApiError::Unauthenticated)?;
    let key = state.mfa_key.ok_or_else(|| ApiError::Internal(anyhow::anyhow!("MFA_ENCRYPTION_KEY is not set")))?;
    // Guessing is capped per user across every challenge and instance: 5 wrong codes in 15 minutes stop it.
    if domain_iam::password::failed_codes(&state.pool, org_id, user_id).await? >= domain_iam::password::MAX_FAILED_CODES {
        return Err(ApiError::RateLimited);
    }
    let (_, stored) = domain_iam::password::second_step(&state.pool, org_id, user_id).await?;
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    // A stored secret always wins; a pending one (first sign-in) counts only while nothing is stored yet.
    let (sealed, pending_used) = match (stored, pending) {
        (Some(s), _) => (s, false),
        (None, Some(p)) => (b64.decode(p).map_err(|_| ApiError::Unauthenticated)?, true),
        (None, None) => return Err(ApiError::Unauthenticated),
    };
    let secret = domain_iam::totp::decrypt(&key, &sealed, &domain_iam::password::secret_aad(org_id, user_id)).ok_or(ApiError::Unauthenticated)?;
    let Some(step) = domain_iam::totp::verify_step(&secret, &req.code, unix_now()) else {
        domain_iam::password::record_failed_code(&state.pool, org_id, user_id).await?;
        return Err(ApiError::OtpInvalid);
    };
    // Single use: a code (or an older one) already accepted is refused, so an overheard code cannot sign in again.
    if !domain_iam::password::claim_step(&state.pool, org_id, user_id, step).await? {
        domain_iam::password::record_failed_code(&state.pool, org_id, user_id).await?;
        return Err(ApiError::OtpInvalid);
    }
    // A first enrolment that lost a race to another one does not sign in: only the stored secret counts.
    if pending_used && !domain_iam::password::store_secret(&state.pool, org_id, user_id, &sealed).await? {
        return Err(ApiError::Unauthenticated);
    }
    open(&state, org_id, user_id, None).await
}
