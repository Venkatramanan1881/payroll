//! reports.yaml, slice 18b: the caller's dashboard (DSH-01), widget choices per role (DSH-02), per-user default
//! filters (DSH-03), and signed upload links (TB-02).

use crate::routes::punches::visible_employees;
use crate::routes::sched::{if_match, parse, reply};
use crate::{ApiError, AppState, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use chrono::{Duration, Utc};
use domain_attendance::dashboard::{self, View};
use domain_iam::authz::require;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DashboardQuery {
    role: Option<String>,
    branch_id: Option<Uuid>,
    period_id: Option<Uuid>,
}

/// GET /v1/dashboards/me (DSH-01): the branch is the query's, else the caller's DB-02 default; it only narrows scope.
pub async fn me(Auth(p, mut tx): Auth, Query(q): Query<DashboardQuery>) -> Result<Response, ApiError> {
    let visible = visible_employees(&mut tx, &p, "DSH-01").await?;
    let default_branch: Option<Uuid> = sqlx::query_scalar("SELECT branch_id FROM iam.dashboard_default_filter WHERE user_id = $1").bind(p.user_id).fetch_optional(&mut **tx).await?.flatten();
    let branch = q.branch_id.or(default_branch);
    let today = domain_attendance::period::org_today(&mut tx).await?;
    // A branch filter keeps only the employees of that branch the caller already sees (AC-C16-08.4).
    let scope: Option<Vec<Uuid>> = match branch {
        None => visible,
        Some(b) => Some(
            sqlx::query_scalar(
                "SELECT employee_id FROM workforce.employee_assignment WHERE kind = 'master_branch' AND branch_id = $1 AND valid @> $2::date
                   AND ($3::uuid[] IS NULL OR employee_id = ANY($3))",
            )
            .bind(b)
            .bind(today)
            .bind(&visible)
            .fetch_all(&mut **tx)
            .await?,
        ),
    };
    let view = View { user: p.user_id, employee: p.employee_id, scope, branch, today };
    Ok(reply(StatusCode::OK, dashboard::dashboard(&mut tx, &view, q.role.as_deref(), q.period_id).await?))
}

/// GET /v1/dashboards/widget-choices (DSH-02), with the choices' ETag.
pub async fn choices(Auth(p, mut tx): Auth) -> Result<Response, ApiError> {
    require(&mut tx, &p, "DSH-02").await?;
    Ok(reply(StatusCode::OK, dashboard::choices(&mut tx).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChoiceIn {
    widgets: Vec<String>,
}

/// PUT /v1/dashboards/widget-choices/{role_id} (DSH-02; If-Match from the last read of the choices).
pub async fn set_choice(Auth(p, mut tx): Auth, Path(role): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "DSH-02").await?;
    dashboard::lock_choices(&mut tx).await?;
    if_match(&headers, &dashboard::choices(&mut tx).await?)?;
    let c: ChoiceIn = parse(&raw)?;
    let body = dashboard::set_choice(&mut tx, p.user_id, role, &c.widgets).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

/// GET /v1/dashboards/default-filters/{user_id} (DSH-03).
pub async fn default_filters(Auth(p, mut tx): Auth, Path(user): Path<Uuid>) -> Result<Response, ApiError> {
    require(&mut tx, &p, "DSH-03").await?;
    Ok(reply(StatusCode::OK, dashboard::default_filters(&mut tx, user).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FiltersIn {
    #[serde(default)]
    branch_id: Option<Uuid>,
    #[serde(default)]
    period: Option<String>,
}

/// PUT /v1/dashboards/default-filters/{user_id} (DSH-03; If-Match): the branch must be in the target user's scope.
pub async fn set_default_filters(Auth(p, mut tx): Auth, Path(user): Path<Uuid>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    require(&mut tx, &p, "DSH-03").await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('default_filter:' || $1::text))").bind(user).execute(&mut **tx).await?;
    if_match(&headers, &dashboard::default_filters(&mut tx, user).await?)?;
    let f: FiltersIn = parse(&raw)?;
    // The target user's branches: None when they see every branch; departments-only scopes allow no branch filter.
    let scopes: Vec<(String, Option<Uuid>)> = sqlx::query_as("SELECT scope_kind, branch_id FROM iam.user_scope WHERE user_id = $1").bind(user).fetch_all(&mut **tx).await?;
    let branches: Option<Vec<Uuid>> = if scopes.iter().any(|s| s.0 == "all") { None } else { Some(scopes.iter().filter_map(|s| s.1).collect()) };
    let body = dashboard::set_default_filters(&mut tx, p.user_id, user, f.branch_id, f.period.as_deref(), branches.as_deref()).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::OK, body))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadIn {
    purpose: String,
    file_name: String,
    content_type: String,
    size_bytes: i64,
}

const MB: i64 = 1024 * 1024;
const LINK_SECS: i64 = 900;

/// What each purpose accepts (DC-06): content types and the largest size.
fn allowed(purpose: &str) -> Option<(&'static [&'static str], i64)> {
    const SHEETS: &[&str] = &["text/csv", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"];
    Some(match purpose {
        "opening_balances" | "employee_import" | "roster_import" => (SHEETS, 10 * MB),
        "request_attachment" => (&["application/pdf", "image/jpeg", "image/png"], 5 * MB),
        _ => return None,
    })
}

/// POST /v1/files/upload-links: a short-lived signed PUT link under the organisation's own path (TB-02). The
/// idempotency key names the object, so a retried request signs the same path again.
pub async fn upload_link(State(state): State<AppState>, Auth(p, mut tx): Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let key = crate::idempotency::key(&headers)?;
    let u: UploadIn = parse(&raw)?;
    let (types, max) = allowed(&u.purpose).ok_or_else(|| ApiError::ValidationFailed("purpose is opening_balances, employee_import, roster_import or request_attachment".into()))?;
    if u.file_name.trim().is_empty() || u.file_name.chars().count() > 255 {
        return Err(ApiError::ValidationFailed("file_name is 1-255 characters".into()));
    }
    if !types.contains(&u.content_type.as_str()) || u.size_bytes < 1 || u.size_bytes > max {
        return Err(ApiError::Refused("FILE_TYPE_NOT_ALLOWED", format!("{} accepts {} up to {} MB", u.purpose, types.join(", "), max / MB)));
    }
    // The stored name keeps only safe characters: no path separators or traversal can reach the object path.
    let safe: String = u.file_name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }).collect();
    let mut safe = safe.trim_start_matches('.').to_string();
    while safe.contains("..") {
        safe = safe.replace("..", ".");
    }
    let org: Uuid = sqlx::query_scalar("SELECT app.current_org()").fetch_one(&mut **tx).await?;
    let path = format!("{org}/uploads/{}/{key}/{}", u.purpose, if safe.is_empty() { "file".into() } else { safe });
    let store = state.store.clone().ok_or_else(|| ApiError::Internal(anyhow::anyhow!("EXPORT_BUCKET is not set")))?;
    let url = store.signed_put(&path, LINK_SECS as u64).await.map_err(ApiError::Internal)?;
    db::audit(&mut tx, p.user_id, "file.upload_link", "file", None, Some(json!({ "purpose": u.purpose, "path": path, "size_bytes": u.size_bytes }))).await?;
    tx.commit().await?;
    let body: Value = json!({ "upload_url": url, "method": "PUT", "headers": { "content-type": u.content_type },
                              "expires_at": Utc::now() + Duration::seconds(LINK_SECS), "file": { "path": path } });
    Ok(reply(StatusCode::CREATED, body))
}
