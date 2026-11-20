//! reports.yaml, slice 18: report runs (R1-R10, R13, R25-R27) as jobs with a spreadsheet, job polling, and the
//! audit views. Dashboards and upload links follow in slice 18b (D-40).

use crate::routes::punches::visible_employees;
use crate::routes::sched::{parse, reply, start};
use crate::{ApiError, AppState, Auth};
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::Json;
use chrono::{Duration, NaiveDate, Utc};
use domain_attendance::reports::{self, Filters, Table};
use domain_iam::authz::{require, Principal, Scope};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

const LINK_SECS: i64 = 900;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportFilters {
    period_id: Option<Uuid>,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    branch_ids: Option<Vec<Uuid>>,
    department_ids: Option<Vec<Uuid>>,
    employee_ids: Option<Vec<Uuid>>,
    format: Option<String>,
}

/// The audit trail and settings history carry no branch or department: until entries do, only a holder of the
/// grant for the whole organisation reads them (a branch-scoped Auditor would otherwise see every branch).
async fn require_org_wide(tx: &mut db::OrgTx<'_>, p: &Principal, op: &str) -> Result<(), ApiError> {
    match require(tx, p, op).await?.scope {
        Scope::All => Ok(()),
        _ => Err(ApiError::Forbidden),
    }
}

/// The grant each report needs (PERMISSION_MATRIX RPT-01..06).
fn grant(report: &str) -> Option<&'static str> {
    Some(match report {
        "R1" | "R2" | "R3" | "R4" | "R5" => "RPT-01",
        "R6" | "R7" | "R8" | "R9" => "RPT-02",
        "R10" => "RPT-03",
        "R13" => "RPT-04",
        "R25" | "R26" => "RPT-05",
        "R27" => "RPT-06",
        _ => return None,
    })
}

fn xlsx(t: &Table) -> Result<Vec<u8>, rust_xlsxwriter::XlsxError> {
    let mut book = rust_xlsxwriter::Workbook::new();
    let sheet = book.add_worksheet();
    for (c, h) in t.headers.iter().enumerate() {
        sheet.write_string(0, c as u16, *h)?;
    }
    for (r, row) in t.rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            // Strings, never formulas; Excel holds at most 32,767 characters in a cell (long audit values are cut, marked).
            let cell = if cell.chars().count() > 32_000 { format!("{}… [cut; the CSV has it all]", cell.chars().take(32_000).collect::<String>()) } else { cell.clone() };
            sheet.write_string(r as u32 + 1, c as u16, &cell)?;
        }
    }
    book.save_to_buffer()
}

/// R13 from the period's totals (frozen at lock, provisional before): days and minutes only (AC-C13-06.1).
async fn payroll_summary(tx: &mut db::OrgTx<'_>, period: Uuid, employees: &[Uuid]) -> Result<Table, ApiError> {
    let totals = domain_attendance::period::totals(tx, period, &(None, None), None).await?;
    let codes: Vec<(Uuid, String, String)> = sqlx::query_as("SELECT id, employee_code, full_name FROM workforce.employee WHERE id = ANY($1)").bind(employees).fetch_all(&mut ***tx).await?;
    let by_id: std::collections::HashMap<String, &Value> = totals.iter().filter_map(|t| Some((t["employee_id"].as_str()?.to_string(), t))).collect();
    let mut rows = vec![];
    for (id, code, name) in &codes {
        let Some(t) = by_id.get(&id.to_string()) else { continue };
        let text = |k: &str| match &t[k] {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            v => v.to_string(),
        };
        rows.push(vec![code.clone(), name.clone(), text("payable_days"), text("deduction_days"), text("late_minutes"), text("early_minutes"), text("overtime_identified_minutes"), text("basis")]);
    }
    rows.sort();
    Ok(Table { headers: vec!["employee_code", "name", "payable_days", "deduction_days", "late_minutes", "early_minutes", "overtime_identified_minutes", "basis"], rows })
}

async fn run(state: AppState, auth: Auth, report: &str, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    let Auth(p, mut tx) = auth;
    let op = grant(report).ok_or(ApiError::NotFound)?;
    if matches!(report, "R26" | "R27") {
        require_org_wide(&mut tx, &p, op).await?;
    }
    // shortcut: who the caller sees is decided by today's assignments, not those over the report's range; an employee
    // who moved branch since reports with the new branch. Use assignments overlapping [from, to] when scope matters there.
    let visible = visible_employees(&mut tx, &p, op).await?;
    let (key, replay) = start(&mut tx, &headers, &format!("POST /v1/reports/{report}/runs"), &raw).await?;
    if let Some(r) = replay {
        return Ok(r);
    }
    let f: ReportFilters = parse(&raw)?;
    let format = f.format.clone().unwrap_or_else(|| "xlsx".into());
    if !matches!(format.as_str(), "xlsx" | "csv") {
        return Err(ApiError::ValidationFailed("format is xlsx or csv".into()));
    }
    // A period, or from + to (OQ-C13-01); R13 always reads a period.
    let (from, to) = match (f.period_id, f.from, f.to) {
        (Some(id), None, None) => sqlx::query_as::<_, (NaiveDate, NaiveDate)>("SELECT period_start, period_end FROM period.attendance_period WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| ApiError::ValidationFailed("period_id is not a period of the organisation".into()))?,
        (None, Some(a), Some(b)) if report != "R13" && a <= b && (b - a).num_days() <= 366 => (a, b),
        _ => return Err(ApiError::ValidationFailed("give period_id, or from and to (at most a year apart); R13 needs period_id".into())),
    };
    let filters = Filters { visible, employees: f.employee_ids, branches: f.branch_ids, departments: f.department_ids };
    let employees = reports::employees(&mut tx, from, to, &filters).await?;
    let table = match (report, f.period_id) {
        ("R13", Some(period)) => payroll_summary(&mut tx, period, &employees).await?,
        _ => reports::rows(&mut tx, report, &employees, from, to).await?,
    };
    let (bytes, content_type) = if format == "csv" {
        (reports::csv(&table), "text/csv")
    } else {
        (xlsx(&table).map_err(|e| ApiError::Internal(e.into()))?, "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
    };
    let store = state.store.clone().ok_or_else(|| ApiError::Internal(anyhow::anyhow!("EXPORT_BUCKET is not set")))?;
    let org: Uuid = sqlx::query_scalar("SELECT app.current_org()").fetch_one(&mut **tx).await?;
    let job = Uuid::now_v7();
    let path = format!("{org}/reports/{job}.{format}"); // TB-02: the organisation id leads the path
    // shortcut: the report runs inside the request and the job is born finished; queue it when reports outgrow a request.
    store.put(&path, bytes, content_type).await.map_err(ApiError::Internal)?;
    let body: Value = sqlx::query_scalar(
        "INSERT INTO app.job (org_id, id, kind, requested_by, params, state, result_path, finished_at)
         VALUES (app.current_org(), $1, $2, $3, $4, 'succeeded', $5, now())
         RETURNING jsonb_build_object('id', id, 'kind', kind, 'state', state, 'created_at', created_at, 'finished_at', finished_at,
                                      'download_url', null, 'download_expires_at', null, 'error_report_url', null)",
    )
    .bind(job)
    .bind(format!("report:{report}"))
    .bind(p.user_id)
    .bind(json!({ "from": from, "to": to, "rows": table.rows.len(), "format": format }))
    .bind(&path)
    .fetch_one(&mut **tx)
    .await?;
    db::audit(&mut tx, p.user_id, "report.run", "job", Some(job), Some(json!({ "report": report, "from": from, "to": to, "rows": table.rows.len() }))).await?;
    // The stored answer carries no link (links expire and are bearer capabilities); polling the job gives a fresh one.
    crate::idempotency::finish(&mut tx, key, StatusCode::ACCEPTED, &body).await?;
    tx.commit().await?;
    Ok(reply(StatusCode::ACCEPTED, body))
}

macro_rules! report_route {
    ($name:ident, [$($ok:literal),*]) => {
        pub async fn $name(State(state): State<AppState>, auth: Auth, Path(report): Path<String>, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
            if ![$($ok),*].contains(&report.as_str()) {
                return Err(ApiError::NotFound);
            }
            run(state, auth, &report, headers, raw).await
        }
    };
}
report_route!(run_attendance, ["R1", "R2", "R3", "R4", "R5"]);
report_route!(run_leave, ["R6", "R7", "R8", "R9"]);
report_route!(run_control, ["R25", "R26"]);

/// POST /v1/reports/people/runs (R10).
pub async fn run_people(State(state): State<AppState>, auth: Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    run(state, auth, "R10", headers, raw).await
}

/// POST /v1/reports/payroll-input-summary/runs (R13).
pub async fn run_payroll_summary(State(state): State<AppState>, auth: Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    run(state, auth, "R13", headers, raw).await
}

/// POST /v1/reports/audit-trail/runs (R27).
pub async fn run_audit_trail(State(state): State<AppState>, auth: Auth, headers: HeaderMap, raw: Bytes) -> Result<Response, ApiError> {
    run(state, auth, "R27", headers, raw).await
}

/// GET /v1/jobs/{id}: the caller's own job; a finished one carries a fresh short-lived link.
pub async fn get_job(State(state): State<AppState>, Auth(p, mut tx): Auth, Path(id): Path<Uuid>) -> Result<Json<Value>, ApiError> {
    let row: Option<(Value, Option<String>)> = sqlx::query_as(
        "SELECT jsonb_build_object('id', id, 'kind', kind, 'state', state, 'created_at', created_at, 'finished_at', finished_at,
                                   'download_url', null, 'download_expires_at', null, 'error_report_url', null), result_path
         FROM app.job WHERE id = $1 AND requested_by = $2",
    )
    .bind(id)
    .bind(p.user_id)
    .fetch_optional(&mut **tx)
    .await?;
    let (mut body, path) = row.ok_or(ApiError::NotFound)?;
    // A link is a fresh read: the caller must still hold the report's grant.
    if let Some(op) = body["kind"].as_str().and_then(|k| k.strip_prefix("report:")).and_then(grant) {
        require(&mut tx, &p, op).await?;
    }
    if let (Some(path), Some(store)) = (path, state.store.clone()) {
        body["download_url"] = json!(store.signed_url(&path, LINK_SECS as u64).await.map_err(ApiError::Internal)?);
        body["download_expires_at"] = json!(Utc::now() + Duration::seconds(LINK_SECS));
        db::audit(&mut tx, p.user_id, "job.download_link", "job", Some(id), None).await?;
        tx.commit().await?;
    }
    Ok(Json(body))
}

#[derive(Deserialize)]
pub struct AuditQuery {
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    actor_id: Option<Uuid>,
    actor_kind: Option<String>,
    target_type: Option<String>,
    target_id: Option<Uuid>,
    action: Option<String>,
    support_access_only: Option<bool>,
    sort: Option<String>,
    setting_ref: Option<String>,
}

const ENTRY: &str = "jsonb_build_object('id', a.id, 'actor_kind', a.actor_kind, 'actor_id', a.actor_id, 'support_grant_id', a.support_grant_id,
    'action', a.action, 'target_type', a.target_type, 'target_id', a.target_id, 'reason', a.after_value->>'reason',
    'before_value', a.before_value, 'after_value', a.after_value, 'change_ref', a.change_ref, 'correlation_id', a.correlation_id,
    'occurred_at', a.occurred_at)";

// shortcut: the newest 200 matching entries (the spec's page maximum); add the cursor when someone pages past them.
const AUDIT_LIMIT: i64 = 200;

/// GET /v1/audit/entries (ADT-01): read-only; viewing is itself audited (entries carry before / after values).
pub async fn audit_entries(Auth(p, mut tx): Auth, Query(q): Query<AuditQuery>) -> Result<Json<Value>, ApiError> {
    require_org_wide(&mut tx, &p, "ADT-01").await?;
    if q.actor_kind.as_deref().is_some_and(|k| !matches!(k, "user" | "vendor_user" | "system_job" | "device")) {
        return Err(ApiError::ValidationFailed("actor_kind is user, vendor_user, system_job or device".into()));
    }
    let ascending = match q.sort.as_deref() {
        None | Some("-occurred_at") => false,
        Some("occurred_at") => true,
        _ => return Err(ApiError::ValidationFailed("sort is occurred_at or -occurred_at".into())),
    };
    let items: Vec<Value> = sqlx::query_scalar(&format!(
        "SELECT {ENTRY} FROM audit.audit_entry a
         WHERE ($1::date IS NULL OR (a.occurred_at AT TIME ZONE 'Asia/Kolkata')::date >= $1) AND ($2::date IS NULL OR (a.occurred_at AT TIME ZONE 'Asia/Kolkata')::date <= $2)
           AND ($3::uuid IS NULL OR a.actor_id = $3) AND ($4::text IS NULL OR a.actor_kind = $4) AND ($5::text IS NULL OR a.target_type = $5)
           AND ($6::uuid IS NULL OR a.target_id = $6) AND ($7::text IS NULL OR a.action = $7) AND (NOT $8 OR a.support_grant_id IS NOT NULL)
         ORDER BY a.occurred_at {}, a.id LIMIT {AUDIT_LIMIT}",
        if ascending { "ASC" } else { "DESC" }
    ))
    .bind(q.from)
    .bind(q.to)
    .bind(q.actor_id)
    .bind(&q.actor_kind)
    .bind(&q.target_type)
    .bind(q.target_id)
    .bind(&q.action)
    .bind(q.support_access_only.unwrap_or(false))
    .fetch_all(&mut **tx)
    .await?;
    db::audit(&mut tx, p.user_id, "audit.viewed", "audit_entry", None, Some(json!({ "results": items.len() }))).await?;
    tx.commit().await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}

/// GET /v1/audit/settings-changes (ADT-02): the settings and policy changes (G9), optionally for one BRD setting.
pub async fn settings_changes(Auth(p, mut tx): Auth, Query(q): Query<AuditQuery>) -> Result<Json<Value>, ApiError> {
    require_org_wide(&mut tx, &p, "ADT-02").await?;
    let items: Vec<Value> = sqlx::query_scalar(&format!(
        "SELECT {ENTRY} FROM audit.audit_entry a
         WHERE (a.action LIKE 'policy%' OR a.action LIKE '%settings%' OR a.action LIKE 'notification_rules%'
                OR a.target_type IN ('policy_version','policy_assignment','organisation_settings'))
           AND ($1::date IS NULL OR (a.occurred_at AT TIME ZONE 'Asia/Kolkata')::date >= $1) AND ($2::date IS NULL OR (a.occurred_at AT TIME ZONE 'Asia/Kolkata')::date <= $2)
           AND ($3::uuid IS NULL OR a.actor_id = $3)
           AND ($4::text IS NULL OR strpos(coalesce(a.before_value::text, '') || coalesce(a.after_value::text, ''), '\"' || $4 || '\"') > 0)
         ORDER BY a.occurred_at DESC, a.id LIMIT {AUDIT_LIMIT}"
    ))
    .bind(q.from)
    .bind(q.to)
    .bind(q.actor_id)
    .bind(&q.setting_ref)
    .fetch_all(&mut **tx)
    .await?;
    Ok(Json(json!({ "items": items, "next_cursor": null })))
}
