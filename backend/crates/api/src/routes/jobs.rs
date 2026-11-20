//! Timed job endpoints (TSD §10), called by Cloud Scheduler with the job secret as a bearer token.
//! Not part of the public contract: mounted under /internal and disabled unless JOB_SECRET is set.

use crate::{ApiError, AppState};
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use domain_attendance::jobs;
use serde_json::{json, Value};

/// Constant-time comparison, so the secret cannot be guessed byte by byte from response times.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// POST /internal/jobs/{name}: nightly-recalculation (J1), approval-timeouts (J3), comp-off-expiry (J6), period-housekeeping (J7), notifications-dispatch (relay), approvals-digest (NT-06) or usage-refresh (J10), for every active organisation.
/// One organisation's failure is reported and the others still run.
// shortcut: a shared secret from Secret Manager; switch to Cloud Scheduler OIDC tokens (audience check) when the deployment adds it.
pub async fn run(State(state): State<AppState>, Path(name): Path<String>, headers: HeaderMap) -> Result<Json<Value>, ApiError> {
    let secret = state.job_secret.as_deref().ok_or(ApiError::NotFound)?;
    let sent = headers.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    if !same(sent.as_bytes(), secret) {
        return Err(ApiError::Unauthenticated);
    }
    if !["nightly-recalculation", "approval-timeouts", "comp-off-expiry", "period-housekeeping", "notifications-dispatch", "approvals-digest", "usage-refresh"].contains(&name.as_str()) {
        return Err(ApiError::NotFound);
    }
    let orgs = jobs::active_organisations(&state.pool).await?;
    let mut results = Vec::with_capacity(orgs.len());
    for org in &orgs {
        let outcome = match name.as_str() {
            "nightly-recalculation" => jobs::nightly_recalculation(&state.pool, *org).await,
            _ => {
                let mut tx = db::begin_org(&state.pool, *org).await?;
                let r = match name.as_str() {
                    "period-housekeeping" => domain_attendance::period::housekeeping(&mut tx).await,
                    "comp-off-expiry" => domain_attendance::comp_off::expire_due(&mut tx).await,
                    "notifications-dispatch" => domain_attendance::notify::dispatch(&mut tx, *org).await,
                    "approvals-digest" => domain_attendance::notify::digest(&mut tx).await,
                    "usage-refresh" => jobs::refresh_usage(&mut tx, *org).await,
                    _ => jobs::approval_timeouts(&mut tx).await,
                };
                match r {
                    Ok(s) => tx.commit().await.map(|_| s).map_err(Into::into),
                    Err(e) => Err(e),
                }
            }
        };
        match outcome {
            Ok(summary) => {
                results.push(json!({ "org_id": org, "ok": true, "summary": summary }));
            }
            Err(e) => {
                eprintln!("job {name} failed for organisation {org}: {e:?}"); // logged; the run goes on
                results.push(json!({ "org_id": org, "ok": false }));
            }
        }
    }
    Ok(Json(json!({ "job": name, "organisations": orgs.len(), "results": results })))
}
