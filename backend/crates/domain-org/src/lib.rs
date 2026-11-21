//! Organisation setup (C01): organisation, branches and time zones, geo-fences, IP allow lists, lists, contractors and
//! organisation settings versions. Every query runs under the caller's organisation (RLS).

use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Map, Value};
use uuid::Uuid;

// ---------------------------------------------------------------------------------------------------------------------
// Organisation

pub async fn organisation(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    sqlx::query_scalar(
        "SELECT jsonb_build_object('display_name', display_name, 'legal_name', legal_name, 'state', state, 'state_changed_at', state_changed_at)
         FROM platform.organisation WHERE id = app.current_org()",
    )
    .fetch_optional(&mut ***tx)
    .await?
    .ok_or(DomainError::NotFound)
}

// ---------------------------------------------------------------------------------------------------------------------
// Branches

const BRANCH_JSON: &str = "jsonb_build_object('id', b.id, 'name', b.name, 'code', b.code, 'address', b.address,
    'tz', (SELECT t.tz FROM org.branch_timezone t WHERE t.branch_id = b.id AND t.valid @> current_date),
    'time_zones', coalesce((SELECT jsonb_agg(jsonb_build_object('tz', t.tz, 'valid_from', lower(t.valid), 'valid_to', upper(t.valid)) ORDER BY lower(t.valid))
                            FROM org.branch_timezone t WHERE t.branch_id = b.id), '[]'),
    'holiday_calendar_id', b.holiday_calendar_id, 'closed_on', b.closed_on, 'created_at', b.created_at, 'updated_at', b.updated_at)";

pub async fn get_branch(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {BRANCH_JSON} FROM org.branch b WHERE b.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

/// `only`: the caller's branch scope (None = every branch).
pub async fn list_branches(tx: &mut OrgTx<'_>, only: Option<&[Uuid]>) -> Result<Vec<Value>, DomainError> {
    let sql = format!("SELECT {BRANCH_JSON} FROM org.branch b WHERE ($1::uuid[] IS NULL OR b.id = ANY($1)) ORDER BY b.code");
    Ok(sqlx::query_scalar(&sql).bind(only).fetch_all(&mut ***tx).await?)
}

fn text(v: &str, field: &str, max: usize) -> Result<String, DomainError> {
    let t = v.trim();
    if t.is_empty() || t.chars().count() > max {
        return Err(invalid(format!("{field} must be 1 to {max} characters")));
    }
    Ok(t.to_string())
}

async fn valid_tz(tx: &mut OrgTx<'_>, tz: &str) -> Result<(), DomainError> {
    let ok: bool = sqlx::query_scalar("SELECT app.is_valid_timezone($1)").bind(tz).fetch_one(&mut ***tx).await?;
    if ok { Ok(()) } else { Err(invalid("tz is not a known time zone")) }
}

/// ORG-01: a branch with its first time zone (BR-11).
pub async fn create_branch(tx: &mut OrgTx<'_>, actor: Uuid, code: &str, name: &str, address: Option<&str>, tz: &str, tz_from: NaiveDate) -> Result<Uuid, DomainError> {
    let (code, name) = (text(code, "code", 20)?, text(name, "name", 120)?);
    valid_tz(tx, tz).await?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO org.branch (org_id, code, name, address, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $4, $4) RETURNING id")
        .bind(&code)
        .bind(&name)
        .bind(address.map(str::trim).filter(|a| !a.is_empty()))
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO org.branch_timezone (org_id, branch_id, tz, valid, created_by) VALUES (app.current_org(), $1, $2, daterange($3, NULL), $4)")
        .bind(id)
        .bind(tz)
        .bind(tz_from)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "branch.create", "branch", Some(id), Some(json!({ "code": code, "name": name, "tz": tz }))).await?;
    Ok(id)
}

/// `address`: None leaves it, Some(None) or blank clears it (null in the PATCH), Some(Some(a)) sets it.
pub async fn update_branch(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, code: Option<&str>, name: Option<&str>, address: Option<Option<&str>>) -> Result<(), DomainError> {
    if code.is_none() && name.is_none() && address.is_none() {
        return Err(invalid("at least one field"));
    }
    let code = code.map(|c| text(c, "code", 20)).transpose()?;
    let name = name.map(|n| text(n, "name", 120)).transpose()?;
    let before = get_branch(tx, id).await?;
    sqlx::query("UPDATE org.branch SET code = coalesce($2, code), name = coalesce($3, name), address = CASE WHEN $6 THEN $4 ELSE address END, updated_by = $5 WHERE id = $1")
        .bind(id)
        .bind(&code)
        .bind(&name)
        .bind(address.flatten().map(str::trim).filter(|a| !a.is_empty()))
        .bind(actor)
        .bind(address.is_some())
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "branch.update", "branch", Some(id), Some(json!({ "before": before }))).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Geo-fences (PC-03, PC-R14)

const FENCE_JSON: &str = "jsonb_build_object('id', f.id, 'branch_id', f.branch_id, 'name', f.name, 'latitude', f.latitude, 'longitude', f.longitude,
    'radius_m', f.radius_m, 'enabled', f.enabled, 'tested_at', f.tested_at, 'tested_by', f.tested_by, 'updated_at', f.updated_at)";

pub async fn get_fence(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {FENCE_JSON} FROM org.geo_fence f WHERE f.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn fence_branch(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Uuid, DomainError> {
    sqlx::query_scalar("SELECT branch_id FROM org.geo_fence WHERE id = $1").bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn list_fences(tx: &mut OrgTx<'_>, branch: Uuid) -> Result<Vec<Value>, DomainError> {
    get_branch(tx, branch).await?;
    let sql = format!("SELECT {FENCE_JSON} FROM org.geo_fence f WHERE f.branch_id = $1 ORDER BY f.created_at");
    Ok(sqlx::query_scalar(&sql).bind(branch).fetch_all(&mut ***tx).await?)
}

pub struct FenceIn {
    pub name: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: i32,
}

fn check_fence(f: &FenceIn) -> Result<(), DomainError> {
    if !(-90.0..=90.0).contains(&f.lat) || !(-180.0..=180.0).contains(&f.lon) || f.radius_m < 1 || f.radius_m > 100_000 {
        return Err(invalid("latitude, longitude or radius_m is out of range"));
    }
    if f.name.as_ref().is_some_and(|n| n.chars().count() > 120) {
        return Err(invalid("name is longer than 120 characters"));
    }
    Ok(())
}

/// A new fence starts switched off; HR tests it from the branch and switches it on (PC-R14).
pub async fn create_fence(tx: &mut OrgTx<'_>, actor: Uuid, branch: Uuid, f: FenceIn) -> Result<Uuid, DomainError> {
    check_fence(&f)?;
    get_branch(tx, branch).await?;
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO org.geo_fence (org_id, branch_id, name, latitude, longitude, radius_m, enabled, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, false, $6, $6) RETURNING id",
    )
    .bind(branch)
    .bind(&f.name)
    .bind(f.lat)
    .bind(f.lon)
    .bind(f.radius_m)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "geo_fence.create", "geo_fence", Some(id), None).await?;
    Ok(id)
}

/// A moved or resized fence must be tested again before it counts: it is switched off and its test cleared.
pub async fn update_fence(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, f: FenceIn) -> Result<(), DomainError> {
    check_fence(&f)?;
    let before = get_fence(tx, id).await?;
    sqlx::query(
        "UPDATE org.geo_fence SET name = $2, latitude = $3, longitude = $4, radius_m = $5, enabled = false, tested_at = NULL, tested_by = NULL, updated_by = $6
         WHERE id = $1",
    )
    .bind(id)
    .bind(&f.name)
    .bind(f.lat)
    .bind(f.lon)
    .bind(f.radius_m)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "geo_fence.update", "geo_fence", Some(id), Some(json!({ "before": before }))).await?;
    Ok(())
}

/// PC-R14: measures a point against the fence and records the test.
pub async fn test_fence(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, lat: f64, lon: f64) -> Result<Value, DomainError> {
    if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
        return Err(invalid("latitude or longitude is out of range"));
    }
    let (clat, clon, radius): (f64, f64, i32) = sqlx::query_as("SELECT latitude::float8, longitude::float8, radius_m FROM org.geo_fence WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)?;
    let distance = domain_timecap::evaluate::distance_m(lat, lon, clat, clon);
    let tested_at: chrono::DateTime<chrono::Utc> = sqlx::query_scalar("UPDATE org.geo_fence SET tested_at = now(), tested_by = $2, updated_by = $2 WHERE id = $1 RETURNING tested_at")
        .bind(id)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    let result = json!({ "inside": distance <= f64::from(radius), "distance_m": (distance * 10.0).round() / 10.0, "radius_m": radius, "tested_at": tested_at });
    db::audit(tx, actor, "geo_fence.test", "geo_fence", Some(id), Some(result.clone())).await?;
    Ok(result)
}

/// PC-R14 says a fence "can" be tested first; the approved contract does not refuse an untested fence.
pub async fn switch_fence(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, on: bool) -> Result<(), DomainError> {
    let found: Option<bool> = sqlx::query_scalar("SELECT true FROM org.geo_fence WHERE id = $1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    found.ok_or(DomainError::NotFound)?;
    sqlx::query("UPDATE org.geo_fence SET enabled = $2, updated_by = $3 WHERE id = $1").bind(id).bind(on).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, if on { "geo_fence.switch_on" } else { "geo_fence.switch_off" }, "geo_fence", Some(id), None).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// IP allow list (PC-05)

pub async fn ip_list(tx: &mut OrgTx<'_>, branch: Uuid) -> Result<Value, DomainError> {
    get_branch(tx, branch).await?;
    let networks: Vec<String> = sqlx::query_scalar("SELECT network::text FROM org.branch_ip_allow WHERE branch_id = $1 ORDER BY network")
        .bind(branch)
        .fetch_all(&mut ***tx)
        .await?;
    Ok(json!({ "branch_id": branch, "networks": networks }))
}

/// Replaces the whole list; each entry must be a network in CIDR form (cast checked by the database).
pub async fn replace_ip_list(tx: &mut OrgTx<'_>, actor: Uuid, branch: Uuid, networks: &[String], reason: &str) -> Result<(), DomainError> {
    if reason.trim().is_empty() {
        return Err(invalid("reason is required"));
    }
    if networks.len() > 200 || networks.iter().any(|n| n.len() > 43) {
        return Err(invalid("networks holds at most 200 CIDR entries"));
    }
    let before = ip_list(tx, branch).await?;
    sqlx::query("DELETE FROM org.branch_ip_allow WHERE branch_id = $1").bind(branch).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO org.branch_ip_allow (org_id, branch_id, network, created_by) SELECT DISTINCT app.current_org(), $1, n::cidr, $3 FROM unnest($2::text[]) n")
        .bind(branch)
        .bind(networks)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    sqlx::query("UPDATE org.branch SET updated_by = $2 WHERE id = $1").bind(branch).bind(actor).execute(&mut ***tx).await?; // new ETag for the list
    db::audit(tx, actor, "branch_ip_allow.replace", "branch", Some(branch), Some(json!({ "before": before["networks"], "after": networks, "reason": reason.trim() }))).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Lists (OR-R2, OR-R3, OR-R5) and contractors (OR-R4)

pub const LIST_KINDS: [&str; 4] = ["department", "designation", "category", "employee_type"];

fn kind_ok(kind: &str) -> Result<(), DomainError> {
    if LIST_KINDS.contains(&kind) { Ok(()) } else { Err(DomainError::NotFound) }
}

const ENTRY_JSON: &str = "jsonb_build_object('id', e.id, 'kind', e.kind, 'name', e.name, 'code', e.code, 'retired_at', e.retired_at, 'updated_at', e.updated_at)";

pub async fn get_entry(tx: &mut OrgTx<'_>, kind: &str, id: Uuid) -> Result<Value, DomainError> {
    kind_ok(kind)?;
    let sql = format!("SELECT {ENTRY_JSON} FROM org.list_entry e WHERE e.id = $1 AND e.kind = $2");
    sqlx::query_scalar(&sql).bind(id).bind(kind).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn list_entries(tx: &mut OrgTx<'_>, kind: &str, include_retired: bool) -> Result<Vec<Value>, DomainError> {
    kind_ok(kind)?;
    let sql = format!("SELECT {ENTRY_JSON} FROM org.list_entry e WHERE e.kind = $1 AND ($2 OR e.retired_at IS NULL) ORDER BY e.name");
    Ok(sqlx::query_scalar(&sql).bind(kind).bind(include_retired).fetch_all(&mut ***tx).await?)
}

pub async fn create_entry(tx: &mut OrgTx<'_>, actor: Uuid, kind: &str, name: &str, code: Option<&str>) -> Result<Uuid, DomainError> {
    kind_ok(kind)?;
    let name = text(name, "name", 120)?;
    let code = code.map(|c| text(c, "code", 20)).transpose()?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO org.list_entry (org_id, kind, name, code, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $4, $4) RETURNING id")
        .bind(kind)
        .bind(&name)
        .bind(&code)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    db::audit(tx, actor, "list_entry.create", "list_entry", Some(id), Some(json!({ "kind": kind, "name": name }))).await?;
    Ok(id)
}

/// `code`: None leaves it, Some(None) clears it, Some(Some(c)) sets it.
pub async fn update_entry(tx: &mut OrgTx<'_>, actor: Uuid, kind: &str, id: Uuid, name: Option<&str>, code: Option<Option<&str>>) -> Result<(), DomainError> {
    let before = get_entry(tx, kind, id).await?;
    if name.is_none() && code.is_none() {
        return Err(invalid("at least one field"));
    }
    let name = name.map(|n| text(n, "name", 120)).transpose()?;
    let new_code = code.flatten().map(|c| text(c, "code", 20)).transpose()?;
    sqlx::query("UPDATE org.list_entry SET name = coalesce($2, name), code = CASE WHEN $5 THEN $3 ELSE code END, updated_by = $4 WHERE id = $1")
        .bind(id)
        .bind(&name)
        .bind(&new_code)
        .bind(actor)
        .bind(code.is_some())
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "list_entry.update", "list_entry", Some(id), Some(json!({ "before": before }))).await?;
    Ok(())
}

/// Retired, never deleted once used (OQ-C01-04). Retiring twice changes nothing.
pub async fn retire_entry(tx: &mut OrgTx<'_>, actor: Uuid, kind: &str, id: Uuid) -> Result<(), DomainError> {
    get_entry(tx, kind, id).await?;
    sqlx::query("UPDATE org.list_entry SET retired_at = coalesce(retired_at, now()), updated_by = $2 WHERE id = $1").bind(id).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, "list_entry.retire", "list_entry", Some(id), None).await?;
    Ok(())
}

const CONTRACTOR_JSON: &str = "jsonb_build_object('id', c.id, 'name', c.name, 'contact', c.contact, 'retired_at', c.retired_at, 'updated_at', c.updated_at)";

pub async fn get_contractor(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {CONTRACTOR_JSON} FROM org.contractor c WHERE c.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn list_contractors(tx: &mut OrgTx<'_>, include_retired: bool) -> Result<Vec<Value>, DomainError> {
    let sql = format!("SELECT {CONTRACTOR_JSON} FROM org.contractor c WHERE $1 OR c.retired_at IS NULL ORDER BY c.name");
    Ok(sqlx::query_scalar(&sql).bind(include_retired).fetch_all(&mut ***tx).await?)
}

pub async fn create_contractor(tx: &mut OrgTx<'_>, actor: Uuid, name: &str, contact: Option<&str>) -> Result<Uuid, DomainError> {
    let name = text(name, "name", 200)?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO org.contractor (org_id, name, contact, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $3) RETURNING id")
        .bind(&name)
        .bind(contact.map(str::trim).filter(|c| !c.is_empty()))
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    db::audit(tx, actor, "contractor.create", "contractor", Some(id), None).await?;
    Ok(id)
}

/// `contact`: None leaves it, Some(None) or blank clears it, Some(Some(c)) sets it.
pub async fn update_contractor(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, name: Option<&str>, contact: Option<Option<&str>>) -> Result<(), DomainError> {
    let before = get_contractor(tx, id).await?;
    if name.is_none() && contact.is_none() {
        return Err(invalid("at least one field"));
    }
    let name = name.map(|n| text(n, "name", 200)).transpose()?;
    sqlx::query("UPDATE org.contractor SET name = coalesce($2, name), contact = CASE WHEN $5 THEN $3 ELSE contact END, updated_by = $4 WHERE id = $1")
        .bind(id)
        .bind(&name)
        .bind(contact.flatten().map(str::trim).filter(|c| !c.is_empty()))
        .bind(actor)
        .bind(contact.is_some())
        .execute(&mut ***tx)
        .await?;
    db::audit(tx, actor, "contractor.update", "contractor", Some(id), Some(json!({ "before": before }))).await?;
    Ok(())
}

pub async fn retire_contractor(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid) -> Result<(), DomainError> {
    get_contractor(tx, id).await?;
    sqlx::query("UPDATE org.contractor SET retired_at = coalesce(retired_at, now()), updated_by = $2 WHERE id = $1").bind(id).bind(actor).execute(&mut ***tx).await?;
    db::audit(tx, actor, "contractor.retire", "contractor", Some(id), None).await?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Organisation settings (OR-01..04, EM-01..08), append-only versions (G5, G9)

/// Access settings (AC-03..08) and period settings (`period`, PL-03..07) live in the same document under their own keys
/// (identity.yaml AccessSettings, period.yaml PeriodSettings); an organisation-settings version carries them forward unchanged.
const ACCESS_KEYS: [&str; 7] = ["delegation_enabled", "settings_change_mode", "custom_roles_enabled", "admin_second_step", "employee_visibility", "period", "opening_balances"];

fn one_of(v: &Value, key: &str, allowed: &[&str]) -> Result<(), String> {
    match v.get(key).and_then(Value::as_str) {
        Some(s) if allowed.contains(&s) => Ok(()),
        _ => Err(format!("{key} must be one of {}", allowed.join(", "))),
    }
}

/// OrganisationSettings (organisation.yaml): complete document, known keys only.
pub fn check_settings(s: &Value) -> Result<(), String> {
    let o = s.as_object().ok_or("settings must be an object")?;
    const KNOWN: [&str; 15] = [
        "financial_year", "week_start", "languages", "currency", "number_format", "employee_code_mode", "branch_code_prefixes", "mandatory_fields",
        "contact_self_edit", "probation_months", "confirmation_reminder_days", "rejoin_code", "allowed_branches_mode", "non_allowed_branch_punch",
        "other_branch_costing",
    ];
    if let Some(k) = o.keys().find(|k| !KNOWN.contains(&k.as_str())) {
        return Err(format!("{k} is not an organisation setting"));
    }
    let fy = o.get("financial_year").and_then(Value::as_object).ok_or("financial_year is required")?;
    if let Some(k) = fy.keys().find(|k| !["kind", "custom_start_month"].contains(&k.as_str())) {
        return Err(format!("financial_year.{k} is not a known field"));
    }
    one_of(&Value::Object(fy.clone()), "kind", &["calendar_year", "april_to_march", "custom"])?;
    let month = fy.get("custom_start_month");
    if (fy["kind"] == "custom") != month.is_some() || month.is_some_and(|m| !m.as_i64().is_some_and(|m| (1..=12).contains(&m))) {
        return Err("financial_year.custom_start_month (1-12) is set exactly when kind = custom".into());
    }
    one_of(s, "week_start", &["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"])?;
    let langs = o.get("languages").and_then(Value::as_array).ok_or("languages is required")?;
    let mut seen = std::collections::HashSet::new();
    if langs.is_empty() || !langs.iter().all(|l| l.as_str().is_some_and(|l| ["en", "ta", "hi"].contains(&l) && seen.insert(l))) {
        return Err("languages must list en, ta or hi at least once and without repeats".into());
    }
    if !o.get("currency").and_then(Value::as_str).is_some_and(|c| c.len() == 3 && c.bytes().all(|b| b.is_ascii_uppercase())) {
        return Err("currency must be an ISO 4217 code such as INR".into());
    }
    if o.get("number_format").is_some_and(|n| !n.as_str().is_some_and(|n| n.chars().count() <= 30)) {
        return Err("number_format must be text of up to 30 characters".into());
    }
    one_of(s, "employee_code_mode", &["automatic", "manual"])?;
    if let Some(p) = o.get("branch_code_prefixes") {
        let p = p.as_object().ok_or("branch_code_prefixes must be an object")?;
        if p.iter().any(|(k, v)| k.parse::<Uuid>().is_err() || !v.as_str().is_some_and(|v| v.chars().count() <= 10)) {
            return Err("branch_code_prefixes maps branch ids to prefixes of up to 10 characters".into());
        }
    }
    let fields = o.get("mandatory_fields").and_then(Value::as_array).ok_or("mandatory_fields is required")?;
    let mut seen = std::collections::HashSet::new();
    if !fields.iter().all(|f| f.as_str().is_some_and(|f| f.chars().count() <= 80 && seen.insert(f))) {
        return Err("mandatory_fields must be distinct names of up to 80 characters".into());
    }
    one_of(s, "contact_self_edit", &["off", "on_with_approval", "on_without_approval"])?;
    if !o.get("probation_months").and_then(Value::as_i64).is_some_and(|m| (0..=60).contains(&m)) {
        return Err("probation_months must be 0 to 60".into());
    }
    if o.get("confirmation_reminder_days").is_some_and(|d| !d.as_i64().is_some_and(|d| d >= 0)) {
        return Err("confirmation_reminder_days must be 0 or more".into());
    }
    one_of(s, "rejoin_code", &["new_code", "reuse_old_code"])?;
    one_of(s, "allowed_branches_mode", &["master_only", "chosen_branches", "all_branches"])?;
    one_of(s, "non_allowed_branch_punch", &["block", "record_and_flag"])?;
    one_of(s, "other_branch_costing", &["branch_worked", "master_branch"])?;
    Ok(())
}

fn version_json(row_alias: &str) -> String {
    let a = row_alias;
    format!(
        "jsonb_build_object('id', {a}.id, 'effective_from', {a}.effective_from, 'schema_version', {a}.schema_version,
           'settings', {a}.settings - ARRAY['delegation_enabled','settings_change_mode','custom_roles_enabled','admin_second_step','employee_visibility','period','opening_balances'],
           'reason', {a}.reason, 'created_at', {a}.created_at, 'created_by', {a}.created_by)"
    )
}

pub async fn settings_on(tx: &mut OrgTx<'_>, on: NaiveDate) -> Result<Value, DomainError> {
    let sql = format!("SELECT {} FROM org.organisation_settings_version v WHERE v.effective_from <= $1 ORDER BY v.effective_from DESC LIMIT 1", version_json("v"));
    sqlx::query_scalar(&sql).bind(on).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

// shortcut: no cursor paging; settings versions are few per organisation.
pub async fn settings_versions(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    let sql = format!("SELECT {} FROM org.organisation_settings_version v ORDER BY v.effective_from DESC", version_json("v"));
    Ok(sqlx::query_scalar(&sql).fetch_all(&mut ***tx).await?)
}

/// A new version from `effective_from` (append-only; AC-04 "HR Manager only": saved directly).
pub async fn create_settings_version(tx: &mut OrgTx<'_>, actor: Uuid, from: NaiveDate, reason: &str, schema_version: i32, settings: Value) -> Result<Uuid, DomainError> {
    check_settings(&settings).map_err(invalid)?;
    if reason.trim().is_empty() || schema_version < 1 {
        return Err(invalid("reason and a schema_version of 1 or more are required"));
    }
    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM org.organisation_settings_version WHERE effective_from = $1)").bind(from).fetch_one(&mut ***tx).await?;
    if taken {
        return Err(DomainError::VersionFrozen); // versions are never edited (G5)
    }
    if db::any_locked(tx, from, None).await? {
        return Err(DomainError::PeriodLocked); // DP-10: a locked period keeps its settings
    }
    // Carry the access settings (AC-03..08) of the version this one follows.
    let previous: Option<Value> = sqlx::query_scalar("SELECT settings FROM org.organisation_settings_version WHERE effective_from < $1 ORDER BY effective_from DESC LIMIT 1")
        .bind(from)
        .fetch_optional(&mut ***tx)
        .await?;
    let mut doc: Map<String, Value> = settings.as_object().cloned().unwrap_or_default();
    if let Some(Value::Object(prev)) = previous {
        for k in ACCESS_KEYS {
            if let Some(v) = prev.get(k) {
                doc.insert(k.to_string(), v.clone());
            }
        }
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(from)
    .bind(Value::Object(doc))
    .bind(schema_version)
    .bind(reason.trim())
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, actor, "organisation_settings.create_version", "organisation_settings_version", Some(id), Some(json!({ "effective_from": from, "settings": settings, "reason": reason.trim() }))).await?;
    Ok(id)
}

pub async fn get_settings_version(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {} FROM org.organisation_settings_version v WHERE v.id = $1", version_json("v"));
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> Value {
        json!({ "financial_year": { "kind": "april_to_march" }, "week_start": "monday", "languages": ["en", "ta"], "currency": "INR",
                "employee_code_mode": "automatic", "mandatory_fields": [], "contact_self_edit": "on_with_approval", "probation_months": 6,
                "rejoin_code": "new_code", "allowed_branches_mode": "master_only", "non_allowed_branch_punch": "record_and_flag",
                "other_branch_costing": "branch_worked" })
    }

    #[test]
    fn complete_document_passes_and_each_error_is_caught() {
        assert_eq!(check_settings(&valid()), Ok(()));
        let breakers: [(&str, Value); 8] = [
            ("week_start", json!("someday")),
            ("languages", json!([])),
            ("languages", json!(["en", "en"])),
            ("currency", json!("inr")),
            ("probation_months", json!(61)),
            ("financial_year", json!({ "kind": "custom" })),
            ("allowed_branches_mode", json!("everywhere")),
            ("delegation_enabled", json!(true)),
        ];
        for (k, v) in breakers {
            let mut s = valid();
            s[k] = v.clone();
            assert!(check_settings(&s).is_err(), "{k} = {v}");
        }
        let mut s = valid();
        s.as_object_mut().unwrap().remove("employee_code_mode");
        assert!(check_settings(&s).is_err(), "a missing required key");
    }
}
