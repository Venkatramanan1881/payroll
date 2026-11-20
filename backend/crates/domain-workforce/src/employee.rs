//! Employee master (C02): create and read. Writes run inside the caller's organisation-scoped transaction.

use chrono::NaiveDate;
use db::OrgTx;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug)]
pub enum WorkforceError {
    Validation(String),
    LimitReached,
    PeriodLocked,
    Db(sqlx::Error),
}

impl From<sqlx::Error> for WorkforceError {
    /// Constraint violations (unknown reference, overlapping range, bad value) come from the caller's input, so 422.
    fn from(e: sqlx::Error) -> Self {
        if let Some(db) = e.as_database_error() {
            if matches!(db.code().as_deref(), Some("23503" | "23505" | "23514" | "23P01")) {
                return WorkforceError::Validation(db.constraint().unwrap_or("constraint").to_string());
            }
        }
        WorkforceError::Db(e)
    }
}

/// workforce.yaml EmployeeCreate. Unknown fields (including custom_fields, not yet supported) are refused.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmployeeCreate {
    pub employee_code: Option<String>,
    pub full_name: String,
    pub date_of_birth: Option<NaiveDate>,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub emergency_contact: Option<Value>,
    pub joining_date: NaiveDate,
    pub employment_status: String,
    pub probation_end_date: Option<NaiveDate>,
    pub master_branch_id: Uuid,
    #[serde(default)]
    pub allowed_branch_ids: Vec<Uuid>,
    pub department_id: Option<Uuid>,
    pub designation_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub employee_type_id: Option<Uuid>,
    pub reporting_manager_employee_id: Option<Uuid>,
    pub contractor_id: Option<Uuid>,
}

pub(crate) fn invalid(msg: &str) -> WorkforceError {
    WorkforceError::Validation(msg.to_string())
}

fn validate(input: &EmployeeCreate) -> Result<(), WorkforceError> {
    if input.full_name.trim().is_empty() || input.full_name.chars().count() > 200 {
        return Err(invalid("full_name"));
    }
    if !matches!(input.employment_status.as_str(), "on_probation" | "active") {
        return Err(invalid("employment_status"));
    }
    if let Some(m) = &input.mobile {
        let digits = m.strip_prefix('+').unwrap_or("");
        if !(7..=15).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_digit()) || digits.starts_with('0') {
            return Err(invalid("mobile"));
        }
    }
    if input.employee_code.as_deref().is_some_and(|c| c.is_empty() || c.chars().count() > 30) {
        return Err(invalid("employee_code"));
    }
    Ok(())
}

/// Creates an employee with its first dated assignments. Returns the new id.
pub async fn create_employee(tx: &mut OrgTx<'_>, actor: Uuid, input: EmployeeCreate, today: NaiveDate) -> Result<Uuid, WorkforceError> {
    validate(&input)?;
    // One create at a time per organisation: code numbering and the employee limit (VC-01) cannot race.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(app.current_org()::text, 0))").execute(&mut ***tx).await?;

    let limit: Option<(Option<i32>, String)> =
        sqlx::query_as("SELECT max_active, on_reach FROM platform.employee_limit WHERE org_id = app.current_org() AND valid @> $1::date")
            .bind(today)
            .fetch_optional(&mut ***tx)
            .await?;
    let mut over_limit = false;
    if let Some((Some(max), on_reach)) = &limit {
        let active: i64 = sqlx::query_scalar("SELECT count(*) FROM workforce.employee WHERE employment_status <> 'exited'")
            .fetch_one(&mut ***tx)
            .await?;
        if active >= i64::from(*max) {
            if on_reach == "block" {
                return Err(WorkforceError::LimitReached);
            }
            over_limit = true;
        }
    }

    let code_mode: Option<String> = sqlx::query_scalar(
        "SELECT settings->>'employee_code_mode' FROM org.organisation_settings_version WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(today)
    .fetch_optional(&mut ***tx)
    .await?
    .flatten();
    let code = match (code_mode.as_deref().unwrap_or("automatic"), &input.employee_code) {
        ("manual", Some(c)) => c.clone(),
        ("manual", None) => return Err(invalid("employee_code is required (EM-01 = manual)")),
        (_, Some(_)) => return Err(invalid("employee_code is generated (EM-01 = automatic)")),
        (_, None) => next_code(tx, input.master_branch_id).await?,
    };

    // BR-06: a joining date inside (or before) a locked period would put new days into frozen attendance.
    let locked: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM period.attendance_period WHERE state = 'locked' AND period_end >= $1)")
        .bind(input.joining_date)
        .fetch_one(&mut ***tx)
        .await?;
    if locked {
        return Err(WorkforceError::PeriodLocked);
    }

    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO workforce.employee (org_id, id, employee_code, full_name, date_of_birth, mobile, email, address,
           emergency_contact, joining_date, probation_end_date, employment_status, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)",
    )
    .bind(id)
    .bind(&code)
    .bind(input.full_name.trim())
    .bind(input.date_of_birth)
    .bind(&input.mobile)
    .bind(&input.email)
    .bind(&input.address)
    .bind(&input.emergency_contact)
    .bind(input.joining_date)
    .bind(input.probation_end_date)
    .bind(&input.employment_status)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    sqlx::query(
        "INSERT INTO workforce.employment_status_history (org_id, employee_id, to_status, effective_on, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4)",
    )
    .bind(id)
    .bind(&input.employment_status)
    .bind(input.joining_date)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;

    insert_assignment(tx, id, "master_branch", input.master_branch_id, input.joining_date, actor).await?;
    let optional = [
        ("department", input.department_id),
        ("designation", input.designation_id),
        ("category", input.category_id),
        ("employee_type", input.employee_type_id),
        ("reporting_manager", input.reporting_manager_employee_id),
        ("contractor", input.contractor_id),
    ];
    for (kind, target) in optional {
        if let Some(t) = target {
            insert_assignment(tx, id, kind, t, input.joining_date, actor).await?;
        }
    }
    for b in &input.allowed_branch_ids {
        sqlx::query(
            "INSERT INTO workforce.employee_allowed_branch (org_id, employee_id, branch_id, valid, created_by)
             VALUES (app.current_org(), $1, $2, daterange($3, NULL), $4)",
        )
        .bind(id)
        .bind(b)
        .bind(input.joining_date)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    }

    db::audit(tx, actor, "employee.create", "employee", Some(id), None).await?;
    outbox(tx, "EmployeeCreated", id).await?;
    if over_limit {
        outbox(tx, "EmployeeLimitExceeded", id).await?;
    }
    Ok(id)
}

/// The column that holds an assignment's target, by kind (workforce.employee_assignment CHECK).
pub(crate) fn target_column(kind: &str) -> Option<&'static str> {
    match kind {
        "master_branch" => Some("branch_id"),
        "department" | "designation" | "category" | "employee_type" => Some("list_entry_id"),
        "reporting_manager" => Some("manager_employee_id"),
        "contractor" => Some("contractor_id"),
        _ => None,
    }
}

pub(crate) async fn insert_assignment(
    tx: &mut OrgTx<'_>,
    employee: Uuid,
    kind: &str,
    target: Uuid,
    from: NaiveDate,
    actor: Uuid,
) -> Result<Uuid, WorkforceError> {
    let column = target_column(kind).ok_or_else(|| invalid("kind"))?; // fixed list, never input text
    let sql = format!(
        "INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, valid, {column}, created_by)
         VALUES (app.current_org(), $1, $2, daterange($3, NULL), $4, $5) RETURNING id"
    );
    Ok(sqlx::query_scalar(&sql).bind(employee).bind(kind).bind(from).bind(target).bind(actor).fetch_one(&mut ***tx).await?)
}

pub(crate) async fn outbox(tx: &mut OrgTx<'_>, event: &str, aggregate: Uuid) -> Result<(), WorkforceError> {
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id) VALUES (app.current_org(), $1, $2)")
        .bind(event)
        .bind(aggregate)
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

/// EM-01 automatic: "<branch code>-<n:04>", n one more than the highest number already used with that prefix.
async fn next_code(tx: &mut OrgTx<'_>, branch: Uuid) -> Result<String, WorkforceError> {
    let prefix: String = sqlx::query_scalar("SELECT code FROM org.branch WHERE id = $1")
        .bind(branch)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or_else(|| invalid("master_branch_id"))?;
    let codes: Vec<String> = sqlx::query_scalar("SELECT employee_code FROM workforce.employee WHERE starts_with(employee_code, $1)")
        .bind(format!("{prefix}-"))
        .fetch_all(&mut ***tx)
        .await?;
    let n = codes.iter().filter_map(|c| c[prefix.len() + 1..].parse::<u32>().ok()).max().unwrap_or(0) + 1;
    Ok(format!("{prefix}-{n:04}"))
}

/// workforce.yaml Employee with `current` = assignments valid on `today`, or on the joining date for a future joiner
/// (so a new hire entered ahead of time stays visible and in scope). None if not visible in the caller's organisation.
pub async fn load_employee(tx: &mut OrgTx<'_>, id: Uuid, today: NaiveDate) -> Result<Option<Value>, WorkforceError> {
    let row: Option<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object(
           'id', e.id, 'employee_code', e.employee_code, 'full_name', e.full_name, 'date_of_birth', e.date_of_birth,
           'mobile', e.mobile, 'email', e.email, 'address', e.address, 'emergency_contact', e.emergency_contact,
           'joining_date', e.joining_date, 'probation_end_date', e.probation_end_date, 'exit_date', e.exit_date,
           'employment_status', e.employment_status, 'rejoin_of_employee_id', e.rejoin_of_employee_id,
           'current', (SELECT jsonb_build_object(
               'master_branch_id', max(a.branch_id::text) FILTER (WHERE a.kind = 'master_branch'),
               'department_id', max(a.list_entry_id::text) FILTER (WHERE a.kind = 'department'),
               'designation_id', max(a.list_entry_id::text) FILTER (WHERE a.kind = 'designation'),
               'category_id', max(a.list_entry_id::text) FILTER (WHERE a.kind = 'category'),
               'employee_type_id', max(a.list_entry_id::text) FILTER (WHERE a.kind = 'employee_type'),
               'reporting_manager_employee_id', max(a.manager_employee_id::text) FILTER (WHERE a.kind = 'reporting_manager'),
               'contractor_id', max(a.contractor_id::text) FILTER (WHERE a.kind = 'contractor'))
             FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.valid @> greatest($2::date, e.joining_date)),
           'created_at', e.created_at, 'updated_at', e.updated_at)
         FROM workforce.employee e WHERE e.id = $1",
    )
    .bind(id)
    .bind(today)
    .fetch_optional(&mut ***tx)
    .await?;
    Ok(row)
}

/// Who the caller may list: everyone, the employees whose current master branch or department is listed, or one employee.
pub enum Visible<'a> {
    All,
    Branches(&'a [Uuid]),
    Departments(&'a [Uuid]),
    Only(Uuid),
}

#[derive(Default)]
pub struct ListFilter {
    pub q: Option<String>,
    pub status: Option<String>,
    pub branch: Option<Uuid>,
    pub department: Option<Uuid>,
    pub contractor: Option<Uuid>,
}

/// Sort keys offered by workforce.yaml; the id breaks ties so pages never overlap.
pub fn order_by(sort: &str) -> Option<&'static str> {
    Some(match sort {
        "employee_code" => "e.employee_code, e.id",
        "-employee_code" => "e.employee_code DESC, e.id DESC",
        "full_name" => "e.full_name, e.id",
        "-full_name" => "e.full_name DESC, e.id DESC",
        "joining_date" => "e.joining_date, e.id",
        "-joining_date" => "e.joining_date DESC, e.id DESC",
        _ => return None,
    })
}

/// workforce.yaml EmployeeSummary rows, using the assignments current today (or on the joining date for a future joiner,
/// as `load_employee`). Returns up to `limit + 1` rows so the caller knows whether another page exists.
// shortcut: offset paging; a row added or removed between pages can shift one row. Move to keyset if lists get busy.
pub async fn list_employees(tx: &mut OrgTx<'_>, today: NaiveDate, visible: Visible<'_>, f: &ListFilter, order: &str, limit: i64, offset: i64) -> Result<Vec<Value>, WorkforceError> {
    let (branches, departments, only) = match visible {
        Visible::All => (None, None, None),
        Visible::Branches(b) => (Some(b), None, None),
        Visible::Departments(d) => (None, Some(d), None),
        Visible::Only(id) => (None, None, Some(id)),
    };
    let like = f.q.as_deref().map(|q| format!("%{}%", q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")));
    Ok(sqlx::query_scalar(&format!(
        "SELECT jsonb_build_object('id', e.id, 'employee_code', e.employee_code, 'full_name', e.full_name,
                'employment_status', e.employment_status, 'master_branch_id', c.b, 'department_id', c.d, 'designation_id', c.g)
         FROM workforce.employee e
         CROSS JOIN LATERAL (
           SELECT max(a.branch_id::text) FILTER (WHERE a.kind = 'master_branch')::uuid AS b,
                  max(a.list_entry_id::text) FILTER (WHERE a.kind = 'department')::uuid AS d,
                  max(a.list_entry_id::text) FILTER (WHERE a.kind = 'designation')::uuid AS g,
                  max(a.contractor_id::text) FILTER (WHERE a.kind = 'contractor')::uuid AS k
           FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.valid @> greatest($1::date, e.joining_date)) c
         WHERE c.b IS NOT NULL AND ($2::uuid[] IS NULL OR c.b = ANY($2)) AND ($3::uuid[] IS NULL OR c.d = ANY($3)) AND ($4::uuid IS NULL OR e.id = $4)
           AND ($5::text IS NULL OR e.full_name ILIKE $5 OR e.employee_code ILIKE $5 OR e.mobile ILIKE $5)
           AND ($6::text IS NULL OR e.employment_status = $6) AND ($7::uuid IS NULL OR c.b = $7)
           AND ($8::uuid IS NULL OR c.d = $8) AND ($9::uuid IS NULL OR c.k = $9)
         ORDER BY {order} LIMIT $10 OFFSET $11"
    ))
    .bind(today)
    .bind(branches)
    .bind(departments)
    .bind(only)
    .bind(like)
    .bind(&f.status)
    .bind(f.branch)
    .bind(f.department)
    .bind(f.contractor)
    .bind(limit + 1)
    .bind(offset)
    .fetch_all(&mut ***tx)
    .await?)
}

/// Weak ETag of an employee record (workforce.yaml): changes whenever the row or its assignments change.
pub fn etag(employee: &Value) -> String {
    format!("W/\"{}\"", employee["updated_at"].as_str().unwrap_or_default())
}

/// PATCH fields (workforce.yaml): personal and contact details. Absent = unchanged, null = cleared where allowed.
const PATCHABLE: [&str; 7] = ["full_name", "date_of_birth", "mobile", "email", "address", "emergency_contact", "employee_code"];

pub async fn update_employee(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, fields: serde_json::Map<String, Value>, today: NaiveDate) -> Result<(), WorkforceError> {
    if fields.is_empty() {
        return Err(invalid("at least one field"));
    }
    if let Some(k) = fields.keys().find(|k| !PATCHABLE.contains(&k.as_str())) {
        return Err(invalid(k)); // includes custom_fields, not supported yet
    }
    let mut fields = fields;
    if let Some(v) = fields.get_mut("full_name") {
        let name = v.as_str().map(str::trim).filter(|n| !n.is_empty() && n.chars().count() <= 200).ok_or_else(|| invalid("full_name"))?;
        *v = Value::String(name.to_string());
    }
    let text_ok = |k: &str, max: usize| fields.get(k).is_none_or(|v| v.is_null() || v.as_str().is_some_and(|s| s.chars().count() <= max));
    if !text_ok("email", 254) {
        return Err(invalid("email"));
    }
    if !text_ok("address", 500) {
        return Err(invalid("address"));
    }
    if fields.get("date_of_birth").is_some_and(|v| !v.is_null() && v.as_str().and_then(|d| d.parse::<NaiveDate>().ok()).is_none()) {
        return Err(invalid("date_of_birth"));
    }
    if fields.get("emergency_contact").is_some_and(|v| !v.is_null() && !v.is_object()) {
        return Err(invalid("emergency_contact"));
    }
    if let Some(v) = fields.get("mobile").filter(|v| !v.is_null()) {
        let digits = v.as_str().and_then(|m| m.strip_prefix('+')).unwrap_or("");
        if !(7..=15).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_digit()) || digits.starts_with('0') {
            return Err(invalid("mobile"));
        }
    }
    if let Some(v) = fields.get("employee_code") {
        let mode: Option<String> = sqlx::query_scalar(
            "SELECT settings->>'employee_code_mode' FROM org.organisation_settings_version WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
        )
        .bind(today)
        .fetch_optional(&mut ***tx)
        .await?
        .flatten();
        let ok = mode.as_deref() == Some("manual") && v.as_str().is_some_and(|c| !c.is_empty() && c.chars().count() <= 30);
        if !ok {
            return Err(invalid("employee_code (only when EM-01 = manual)"));
        }
    }
    // Column names come from PATCHABLE above, never from input text; values are bound from one jsonb parameter.
    let sets: Vec<String> = fields
        .keys()
        .map(|k| match k.as_str() {
            "date_of_birth" => format!("{k} = ($2->>'{k}')::date"),
            "emergency_contact" => format!("{k} = $2->'{k}'"),
            _ => format!("{k} = $2->>'{k}'"),
        })
        .collect();
    let sql = format!("UPDATE workforce.employee SET {}, updated_by = $3 WHERE id = $1", sets.join(", "));
    let before: Option<Value> = sqlx::query_scalar("SELECT to_jsonb(e) FROM workforce.employee e WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?;
    sqlx::query(&sql).bind(id).bind(Value::Object(fields.clone())).bind(actor).execute(&mut ***tx).await?;
    let change = serde_json::json!({ "before": before, "changed": fields });
    db::audit(tx, actor, "employee.update", "employee", Some(id), Some(change)).await?;
    Ok(())
}
