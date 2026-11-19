//! Identity and access administration (C03): user accounts, roles and grants, approver rules, delegations, access
//! settings and the caller's own profile. Every query runs under the caller's organisation (RLS); grants and scope
//! are read per request (authz.rs), so each change applies to the user's next request.

use chrono::{DateTime, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use serde_json::{json, Map, Value};
use uuid::Uuid;

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

// ---------------------------------------------------------------------------------------------------------------------
// Operation catalogue (PERMISSION_MATRIX §3.2–§3.17): the codes an organisation role may hold.

const GRANTABLE: &[&str] = &[
    "IAM-01", "IAM-02", "IAM-03", "IAM-04", "IAM-05", "IAM-06", "IAM-07", "IAM-08", "IAM-09", "IAM-10", "IAM-11", "ORG-01", "ORG-02", "ORG-03",
    "ORG-04", "ORG-05", "ORG-06", "ORG-07", "WRK-01", "WRK-02", "WRK-03", "WRK-04", "WRK-05", "WRK-06", "WRK-07", "WRK-08", "TCP-01", "TCP-02",
    "TCP-03", "TCP-04", "TCP-05", "TCP-06", "TCP-07", "TCP-08", "TCP-09", "TCP-10", "TCP-11", "SCH-01", "SCH-02", "SCH-03", "SCH-04", "SCH-05",
    "SCH-06", "SCH-07", "SCH-08", "POL-01", "POL-02", "POL-03", "POL-04", "POL-05", "POL-06", "POL-07", "ATT-01", "ATT-02", "ATT-03", "ATT-04",
    "ATT-05", "ATT-06", "ATT-07", "ATT-08", "ATT-09", "ATT-10", "ATT-11", "ATT-12", "ATT-13", "LVE-01", "LVE-02", "LVE-03", "LVE-04", "LVE-05",
    "LVE-06", "LVE-07", "LVE-08", "PRM-01", "PRM-02", "PRM-03", "PRM-04", "PRM-05", "PRM-06", "PRM-07", "PRM-08", "PRM-09", "PRM-10", "PRM-11",
    "PRM-12", "PRM-13", "CMP-01", "CMP-02", "CMP-03", "CMP-04", "CMP-05", "CMP-06", "CMP-07", "PER-01", "PER-02", "PER-03", "PER-04", "PER-05",
    "PER-06", "PER-07", "PER-08", "OBL-01", "OBL-02", "OBL-03", "OBL-04", "NTF-01", "RPT-01", "RPT-02", "RPT-03", "RPT-04", "RPT-05", "RPT-06",
    "DSH-01", "DSH-02", "DSH-03", "ADT-01", "ADT-02",
];
/// P10 fixed denials inside the catalogue: no role may hold them.
const FIXED_DENIALS: &[&str] = &["ADT-03", "ADT-04"];

/// Optional modules and the operation prefixes they gate (MD-03); common modules C01–C16 are always on.
const GATED: &[(&str, &str)] = &[("CMP-", "M02")];

/// Optional modules switched on today (vendor entitlements, MD-01).
async fn enabled_modules(tx: &mut OrgTx<'_>) -> Result<Vec<String>, DomainError> {
    Ok(sqlx::query_scalar("SELECT module_code FROM platform.module_entitlement WHERE org_id = app.current_org() AND enabled AND valid @> (now() AT TIME ZONE 'Asia/Kolkata')::date")
        .fetch_all(&mut ***tx)
        .await?)
}

fn module_on(code: &str, enabled: &[String]) -> bool {
    GATED.iter().find(|(prefix, _)| code.starts_with(prefix)).is_none_or(|(_, m)| enabled.iter().any(|e| e == m))
}

// ---------------------------------------------------------------------------------------------------------------------
// Access settings (AC-03..06, AC-08), stored in the organisation-settings versions under their own keys

fn access_defaults() -> Map<String, Value> {
    json!({
        "delegation_enabled": true, "settings_change_mode": "hr_only", "custom_roles_enabled": true, "admin_second_step": true,
        // OQ-C03-13 open: employees see their own data until the organisation switches an item off.
        "employee_visibility": { "own_punches": true, "day_status": true, "counters": true, "permission_balance": true, "deduction_reason": true },
    })
    .as_object()
    .cloned()
    .expect("object")
}

/// Settings in force today; `updated_at` (the ETag) is the version's creation time.
pub async fn access_settings(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let row: Option<(Value, String)> = sqlx::query_as(
        "SELECT settings, to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') FROM org.organisation_settings_version
         WHERE effective_from <= (now() AT TIME ZONE 'Asia/Kolkata')::date ORDER BY effective_from DESC LIMIT 1",
    )
    .fetch_optional(&mut ***tx)
    .await?;
    let mut s = access_defaults();
    if let Some((Value::Object(stored), _)) = &row {
        for k in ["delegation_enabled", "settings_change_mode", "custom_roles_enabled", "admin_second_step"] {
            if let Some(v) = stored.get(k) {
                s.insert(k.into(), v.clone());
            }
        }
        if let (Some(Value::Object(stored_vis)), Some(Value::Object(vis))) = (stored.get("employee_visibility"), s.get_mut("employee_visibility")) {
            vis.extend(stored_vis.clone());
        }
    }
    s.insert("updated_at".into(), row.map_or(Value::Null, |r| r.1.into()));
    Ok(Value::Object(s))
}

/// A new organisation-settings version from `from` carrying every other key (append-only versions, G9).
pub async fn change_access_settings(tx: &mut OrgTx<'_>, actor: Uuid, changes: Map<String, Value>, from: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    for (k, v) in &changes {
        let ok = match k.as_str() {
            "delegation_enabled" | "custom_roles_enabled" | "admin_second_step" => v.is_boolean(),
            "settings_change_mode" => v == "hr_only" || v == "hr_with_org_admin_approval",
            "employee_visibility" => v.as_object().is_some_and(|o| {
                let keys = ["own_punches", "day_status", "counters", "permission_balance", "deduction_reason"];
                o.len() == keys.len() && keys.iter().all(|k| o.get(*k).is_some_and(Value::is_boolean))
            }),
            _ => false,
        };
        if !ok {
            return Err(invalid(format!("{k} has a value that is not allowed")));
        }
    }
    if changes.get("settings_change_mode").is_some_and(|v| v == "hr_with_org_admin_approval") {
        return Err(invalid("AC-04 with Organisation Admin approval needs the settings-change approval workflow, which is not built yet"));
    }
    // shortcut: India's date for every organisation; read the organisation's zone when one outside India signs up.
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Kolkata')::date").fetch_one(&mut ***tx).await?;
    if from < today {
        return Err(invalid("effective_from cannot be in the past"));
    }
    let prev: Option<(NaiveDate, Value, i32)> = sqlx::query_as(
        "SELECT effective_from, settings, schema_version FROM org.organisation_settings_version WHERE effective_from <= $1 ORDER BY effective_from DESC LIMIT 1",
    )
    .bind(from)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((prev_from, mut doc, schema_version)) = prev else {
        return Err(invalid("set up the organisation settings first (POST /v1/organisation-settings/versions)"));
    };
    if prev_from == from {
        return Err(DomainError::VersionFrozen);
    }
    let later: Option<NaiveDate> = sqlx::query_scalar("SELECT min(effective_from) FROM org.organisation_settings_version WHERE effective_from > $1")
        .bind(from)
        .fetch_one(&mut ***tx)
        .await?;
    if let Some(d) = later {
        return Err(DomainError::IllegalTransition(format!("an organisation-settings version from {d} already exists and would undo this change")));
    }
    let before = access_settings(tx).await?;
    let obj = doc.as_object_mut().ok_or_else(|| invalid("stored settings are not an object"))?;
    obj.extend(changes.clone());
    sqlx::query(
        "INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5)",
    )
    .bind(from)
    .bind(&doc)
    .bind(schema_version)
    .bind(reason)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "iam.access_settings_changed", "organisation_settings_version", None, Some(json!({ "before": before, "changes": changes, "effective_from": from, "reason": reason })))
        .await?;
    access_settings(tx).await
}

async fn setting_on(tx: &mut OrgTx<'_>, key: &str) -> Result<bool, DomainError> {
    Ok(access_settings(tx).await?[key].as_bool().unwrap_or(true))
}

/// Somebody active must keep full access administration (IAM-01, IAM-02 and IAM-03 over the whole organisation);
/// a change that would leave nobody able to manage users, roles and grants is refused.
async fn admin_remains(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    let ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM iam.user_account u WHERE u.active
           AND EXISTS (SELECT 1 FROM iam.user_scope s WHERE s.user_id = u.id AND s.scope_kind = 'all')
           AND (SELECT count(DISTINCT g.operation_code) FROM iam.role_grant g JOIN iam.user_role ur ON ur.role_id = g.role_id
                 WHERE ur.user_id = u.id AND g.operation_code IN ('IAM-01','IAM-02','IAM-03') AND g.grant_kind IN ('all','scoped')) = 3)",
    )
    .fetch_one(&mut ***tx)
    .await?;
    if !ok {
        return Err(DomainError::IllegalTransition("this would leave nobody able to manage users, roles and grants".into()));
    }
    Ok(())
}

/// Serialises writers of one row (or of the organisation's settings) before their If-Match comparison.
pub async fn lock_user(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(), DomainError> {
    sqlx::query("SELECT 1 FROM iam.user_account WHERE id = $1 FOR UPDATE").bind(id).execute(&mut ***tx).await?;
    Ok(())
}

pub async fn lock_role(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(), DomainError> {
    sqlx::query("SELECT 1 FROM iam.role WHERE id = $1 FOR UPDATE").bind(id).execute(&mut ***tx).await?;
    Ok(())
}

/// Settings versions and approver rules are append-only or replaced as a set: one writer per organisation at a time.
pub async fn lock_settings(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('iam-settings:' || app.current_org()::text))").execute(&mut ***tx).await?;
    Ok(())
}

/// A unique index hit (email, mobile, employee link, role name) is a conflict with an existing row (409).
fn duplicate(what: &'static str) -> impl FnOnce(sqlx::Error) -> DomainError {
    move |e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
            DomainError::Refused("ALREADY_EXISTS", format!("{what} is already in use in the organisation"))
        } else {
            e.into()
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Users

const USER_JSON: &str = "jsonb_build_object('id', u.id, 'login_kind', u.login_kind, 'mobile', u.mobile, 'email', u.email, 'employee_id', u.employee_id,
    'language', u.language, 'active', u.active, 'last_sign_in_at', u.last_sign_in_at, 'updated_at', u.updated_at,
    'roles', coalesce((SELECT jsonb_agg(jsonb_build_object('id', r.id, 'name', r.name, 'system_code', r.system_code) ORDER BY r.name)
                       FROM iam.user_role ur JOIN iam.role r ON r.id = ur.role_id WHERE ur.user_id = u.id), '[]'),
    'scopes', coalesce((SELECT jsonb_agg(jsonb_build_object('scope_kind', s.scope_kind, 'branch_id', s.branch_id, 'department_id', s.department_id,
                                                            'department_kind', s.department_kind) ORDER BY s.scope_kind, s.branch_id, s.department_id)
                        FROM iam.user_scope s WHERE s.user_id = u.id), '[]'))";

pub async fn get_user(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {USER_JSON} FROM iam.user_account u WHERE u.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_users(tx: &mut OrgTx<'_>, role: Option<Uuid>, active: Option<bool>, q: Option<&str>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {USER_JSON} FROM iam.user_account u LEFT JOIN workforce.employee e ON e.id = u.employee_id
         WHERE ($1::uuid IS NULL OR EXISTS (SELECT 1 FROM iam.user_role ur WHERE ur.user_id = u.id AND ur.role_id = $1))
           AND ($2::boolean IS NULL OR u.active = $2)
           AND ($3::text IS NULL OR u.email ILIKE '%' || $3 || '%' OR u.mobile LIKE '%' || $3 || '%' OR e.full_name ILIKE '%' || $3 || '%')
         -- $3 arrives with LIKE wildcards escaped
         ORDER BY coalesce(u.email, u.mobile), u.id"
    ))
    .bind(role)
    .bind(active)
    .bind(q.map(|q| q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")))
    .fetch_all(&mut ***tx)
    .await?)
}

pub struct ScopeIn {
    pub kind: String,
    pub branch: Option<Uuid>,
    pub department: Option<Uuid>,
}

/// AC-07: one `all`, or branches only, or departments only (combinations: OQ-C03-04).
fn check_scopes(scopes: &[ScopeIn]) -> Result<(), DomainError> {
    if scopes.is_empty() {
        return Err(invalid("at least one scope is required"));
    }
    for s in scopes {
        match (s.kind.as_str(), s.branch, s.department) {
            ("all", None, None) | ("branch", Some(_), None) | ("department", None, Some(_)) => {}
            _ => return Err(invalid("scope all has no ids; branch needs branch_id; department needs department_id")),
        }
    }
    let kinds: std::collections::BTreeSet<&str> = scopes.iter().map(|s| s.kind.as_str()).collect();
    if kinds.len() > 1 || (kinds.contains("all") && scopes.len() > 1) {
        return Err(invalid("scope is all, or branches only, or departments only (combinations: OQ-C03-04)"));
    }
    Ok(())
}

async fn set_access(tx: &mut OrgTx<'_>, actor: Uuid, user: Uuid, roles: &[Uuid], scopes: &[ScopeIn]) -> Result<(), DomainError> {
    check_scopes(scopes)?;
    let known: i64 = sqlx::query_scalar("SELECT count(*) FROM iam.role WHERE id = ANY($1)").bind(roles).fetch_one(&mut ***tx).await?;
    if roles.is_empty() || known != roles.len() as i64 {
        return Err(invalid("role_ids must name existing roles of the organisation, without repeats"));
    }
    sqlx::query("DELETE FROM iam.user_role WHERE user_id = $1").bind(user).execute(&mut ***tx).await?;
    sqlx::query("DELETE FROM iam.user_scope WHERE user_id = $1").bind(user).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id, created_by) SELECT app.current_org(), $1, r, $3 FROM unnest($2::uuid[]) r")
        .bind(user)
        .bind(roles)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    for s in scopes {
        sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind, branch_id, department_id, created_by) VALUES (app.current_org(), $1, $2, $3, $4, $5)")
            .bind(user)
            .bind(&s.kind)
            .bind(s.branch)
            .bind(s.department)
            .bind(actor)
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("UPDATE iam.user_account SET updated_at = now(), updated_by = $2 WHERE id = $1").bind(user).bind(actor).execute(&mut ***tx).await?;
    Ok(())
}

pub struct NewUser {
    pub login_kind: String,
    pub mobile: Option<String>,
    pub email: Option<String>,
    pub initial_password: Option<String>,
    pub employee_id: Option<Uuid>,
    pub language: Option<String>,
    pub roles: Vec<Uuid>,
    pub scopes: Vec<ScopeIn>,
}

/// Argon2id with a random salt (TSD §2); the password itself is never stored or logged.
fn hash_password(password: &str) -> Result<String, DomainError> {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default().hash_password(password.as_bytes(), &salt).map(|h| h.to_string()).map_err(|_| invalid("the password cannot be hashed"))
}

pub async fn create_user(tx: &mut OrgTx<'_>, actor: Uuid, u: NewUser) -> Result<Value, DomainError> {
    let hash = match (u.login_kind.as_str(), &u.mobile, &u.email, &u.initial_password) {
        ("mobile_otp", Some(m), _, None) if is_mobile(m) => None,
        ("mobile_otp", ..) => return Err(invalid("mobile_otp needs mobile in E.164 form (+91...) and no password")),
        ("password", m, Some(e), Some(p)) if is_email(e) && m.as_deref().is_none_or(is_mobile) && (8..=200).contains(&p.chars().count()) => Some(hash_password(p)?),
        ("password", ..) => return Err(invalid("password needs a valid email, an initial_password of 8-200 characters and, if given, a mobile in E.164 form")),
        _ => return Err(invalid("login_kind must be mobile_otp or password")),
    };
    if u.language.as_deref().is_some_and(|l| !["en", "ta", "hi"].contains(&l)) {
        return Err(invalid("language must be en, ta or hi"));
    }
    if let Some(e) = u.employee_id {
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM workforce.employee WHERE id = $1)").bind(e).fetch_one(&mut ***tx).await?;
        if !exists {
            return Err(invalid("employee_id is not an employee of the organisation"));
        }
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO iam.user_account (org_id, employee_id, login_kind, mobile, email, password_hash, language, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, coalesce($6, 'en'), $7, $7) RETURNING id",
    )
    .bind(u.employee_id)
    .bind(&u.login_kind)
    .bind(&u.mobile)
    .bind(&u.email)
    .bind(hash)
    .bind(&u.language)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await
    .map_err(duplicate("the mobile, email or employee link"))?;
    set_access(tx, actor, id, &u.roles, &u.scopes).await?;
    let created = get_user(tx, id).await?;
    db::audit(tx, actor, "iam.user_created", "user_account", Some(id), Some(created.clone())).await?;
    Ok(created)
}

fn is_email(e: &str) -> bool {
    let Some((local, domain)) = e.split_once('@') else { return false };
    e.len() <= 254 && !local.is_empty() && !domain.contains('@') && domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.') && !e.contains(char::is_whitespace)
}

fn is_mobile(m: &str) -> bool {
    m.len() >= 8 && m.len() <= 16 && m.starts_with('+') && m[1..].bytes().all(|b| b.is_ascii_digit()) && !m[1..].starts_with('0')
}

/// Nobody changes or switches off their own access (no self-lockout, no self-promotion).
fn not_self(actor: Uuid, user: Uuid) -> Result<(), DomainError> {
    if actor == user {
        return Err(DomainError::Forbidden);
    }
    Ok(())
}

pub async fn change_access(tx: &mut OrgTx<'_>, actor: Uuid, user: Uuid, roles: &[Uuid], scopes: &[ScopeIn], reason: &str) -> Result<Value, DomainError> {
    not_self(actor, user)?;
    let before = get_user(tx, user).await?;
    set_access(tx, actor, user, roles, scopes).await?;
    admin_remains(tx).await?;
    let after = get_user(tx, user).await?;
    let change = json!({ "before": { "roles": before["roles"], "scopes": before["scopes"] }, "after": { "roles": after["roles"], "scopes": after["scopes"] }, "reason": reason });
    db::audit(tx, actor, "iam.user_access_changed", "user_account", Some(user), Some(change)).await?;
    Ok(after)
}

/// Deactivate (every session revoked; sign-in and tokens refused from now, AC-C03-03.7) or reactivate. Idempotent.
pub async fn set_active(tx: &mut OrgTx<'_>, actor: Uuid, user: Uuid, active: bool, reason: &str) -> Result<Value, DomainError> {
    not_self(actor, user)?;
    let changed = sqlx::query("UPDATE iam.user_account SET active = $2, updated_by = $3 WHERE id = $1 AND active <> $2")
        .bind(user)
        .bind(active)
        .bind(actor)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    let user_json = get_user(tx, user).await?;
    if changed == 1 {
        if !active {
            sqlx::query("UPDATE iam.session SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL").bind(user).execute(&mut ***tx).await?;
            admin_remains(tx).await?;
        }
        let action = if active { "iam.user_reactivated" } else { "iam.user_deactivated" };
        db::audit(tx, actor, action, "user_account", Some(user), Some(json!({ "reason": reason }))).await?;
    }
    Ok(user_json)
}

// ---------------------------------------------------------------------------------------------------------------------
// Me

pub async fn me(tx: &mut OrgTx<'_>, user: Uuid) -> Result<Value, DomainError> {
    let mut me = get_user(tx, user).await?;
    let enabled = enabled_modules(tx).await?;
    // Strongest grant per operation across the user's roles (authz.rs uses the same order).
    let grants: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT DISTINCT ON (g.operation_code) g.operation_code, g.grant_kind, g.setting_ref
         FROM iam.role_grant g JOIN iam.user_role ur ON ur.role_id = g.role_id WHERE ur.user_id = $1
         ORDER BY g.operation_code, array_position(ARRAY['all','scoped','self','approve'], g.grant_kind)",
    )
    .bind(user)
    .fetch_all(&mut ***tx)
    .await?;
    me["grants"] = grants
        .into_iter()
        .filter(|g| module_on(&g.0, &enabled))
        .map(|(c, k, s)| json!({ "operation_code": c, "grant_kind": k, "setting_ref": s }))
        .collect();
    let langs: Option<Value> = sqlx::query_scalar(
        "SELECT settings->'languages' FROM org.organisation_settings_version WHERE effective_from <= (now() AT TIME ZONE 'Asia/Kolkata')::date
         ORDER BY effective_from DESC LIMIT 1",
    )
    .fetch_optional(&mut ***tx)
    .await?
    .flatten();
    me["offered_languages"] = langs.filter(Value::is_array).unwrap_or(json!(["en"]));
    Ok(me)
}

pub async fn change_language(tx: &mut OrgTx<'_>, user: Uuid, language: &str) -> Result<Value, DomainError> {
    let offered = me(tx, user).await?["offered_languages"].clone();
    if !offered.as_array().is_some_and(|l| l.iter().any(|x| x == language)) {
        return Err(invalid(format!("{language} is not offered by the organisation (OR-03)")));
    }
    sqlx::query("UPDATE iam.user_account SET language = $2, updated_by = $1 WHERE id = $1").bind(user).bind(language).execute(&mut ***tx).await?;
    db::audit(tx, user, "iam.language_changed", "user_account", Some(user), Some(json!({ "language": language }))).await?;
    get_user(tx, user).await
}

pub async fn my_modules(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let enabled = enabled_modules(tx).await?;
    let mut items: Vec<Value> = (1..=16).map(|n| json!({ "module_code": format!("C{n:02}"), "enabled": true })).collect();
    items.extend(["M01", "M02", "M03", "M04"].iter().map(|m| json!({ "module_code": m, "enabled": enabled.iter().any(|e| e == m) })));
    let mobile: Option<bool> = sqlx::query_scalar(
        "SELECT p.mobile_app FROM platform.organisation_plan op JOIN platform.plan p ON p.id = op.plan_id WHERE op.org_id = app.current_org() AND op.valid @> (now() AT TIME ZONE 'Asia/Kolkata')::date",
    )
    .fetch_optional(&mut ***tx)
    .await?;
    Ok(json!({ "items": items, "mobile_app": mobile.unwrap_or(false) }))
}

// ---------------------------------------------------------------------------------------------------------------------
// Roles

async fn role_json(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let enabled = enabled_modules(tx).await?;
    let mut role: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', r.id, 'name', r.name, 'system_code', r.system_code, 'updated_at', r.updated_at,
           'grants', coalesce((SELECT jsonb_agg(jsonb_build_object('operation_code', g.operation_code, 'grant_kind', g.grant_kind, 'setting_ref', g.setting_ref)
                                ORDER BY g.operation_code) FROM iam.role_grant g WHERE g.role_id = r.id), '[]'))
         FROM iam.role r WHERE r.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?
    .ok_or(DomainError::NotFound)?;
    if let Some(g) = role["grants"].as_array_mut() {
        g.retain(|x| module_on(x["operation_code"].as_str().unwrap_or(""), &enabled)); // AC-C03-01.5
    }
    Ok(role)
}

pub async fn get_role(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    role_json(tx, id).await
}

pub async fn list_roles(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM iam.role ORDER BY system_code IS NULL, name").fetch_all(&mut ***tx).await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(role_json(tx, id).await?);
    }
    Ok(out)
}

fn check_name(name: &str) -> Result<&str, DomainError> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 80 {
        return Err(invalid("name must be 1-80 characters"));
    }
    Ok(n)
}

/// AC-06: custom roles only when switched on; a new role starts with no grants (P9).
pub async fn create_role(tx: &mut OrgTx<'_>, actor: Uuid, name: &str) -> Result<Value, DomainError> {
    if !setting_on(tx, "custom_roles_enabled").await? {
        return Err(DomainError::Forbidden); // AC-C03-02.2: the action is not available
    }
    let id: Uuid = sqlx::query_scalar("INSERT INTO iam.role (org_id, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $2) RETURNING id")
        .bind(check_name(name)?)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await
        .map_err(duplicate("the role name"))?;
    db::audit(tx, actor, "iam.role_created", "role", Some(id), Some(json!({ "name": name }))).await?;
    role_json(tx, id).await
}

/// Custom roles only (renaming system roles: OQ-C03-01).
pub async fn rename_role(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, name: &str) -> Result<Value, DomainError> {
    let system: Option<Option<String>> = sqlx::query_scalar("SELECT system_code FROM iam.role WHERE id = $1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    if system.ok_or(DomainError::NotFound)?.is_some() {
        return Err(DomainError::IllegalTransition("system roles keep their names (OQ-C03-01)".into()));
    }
    sqlx::query("UPDATE iam.role SET name = $2, updated_by = $3 WHERE id = $1")
        .bind(id)
        .bind(check_name(name)?)
        .bind(actor)
        .execute(&mut ***tx)
        .await
        .map_err(duplicate("the role name"))?;
    db::audit(tx, actor, "iam.role_renamed", "role", Some(id), Some(json!({ "name": name }))).await?;
    role_json(tx, id).await
}

pub struct GrantIn {
    pub operation_code: String,
    pub grant_kind: String,
    pub setting_ref: Option<String>,
}

/// AC-01: replaces a role's grants. Only the organisation operations of the matrix; P10 fixed denials refused.
pub async fn change_grants(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, grants: &[GrantIn], reason: &str) -> Result<Value, DomainError> {
    let mut seen = std::collections::HashSet::new();
    for g in grants {
        if FIXED_DENIALS.contains(&g.operation_code.as_str()) {
            return Err(invalid(format!("{} is a fixed denial no role may hold (P10)", g.operation_code)));
        }
        if !GRANTABLE.contains(&g.operation_code.as_str()) {
            return Err(invalid(format!("{} is not an organisation operation of the permission matrix", g.operation_code)));
        }
        if !["all", "scoped", "self", "approve"].contains(&g.grant_kind.as_str()) {
            return Err(invalid("grant_kind must be all, scoped, self or approve"));
        }
        if g.setting_ref.as_deref().is_some_and(|s| s.is_empty() || s.len() > 20) {
            return Err(invalid("setting_ref is at most 20 characters"));
        }
        if !seen.insert(g.operation_code.as_str()) {
            return Err(invalid(format!("{} appears twice", g.operation_code)));
        }
    }
    let before = role_json(tx, id).await?;
    if !before["system_code"].is_null() {
        return Err(DomainError::IllegalTransition("system roles keep the matrix's grants until G-09 / OQ-C03-01 is decided".into()));
    }
    let holds: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_role WHERE user_id = $1 AND role_id = $2)").bind(actor).bind(id).fetch_one(&mut ***tx).await?;
    if holds {
        return Err(DomainError::Forbidden); // nobody changes the grants of a role they hold (no self-promotion)
    }
    sqlx::query("DELETE FROM iam.role_grant WHERE role_id = $1").bind(id).execute(&mut ***tx).await?;
    for g in grants {
        sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind, setting_ref, created_by) VALUES (app.current_org(), $1, $2, $3, $4, $5)")
            .bind(id)
            .bind(&g.operation_code)
            .bind(&g.grant_kind)
            .bind(&g.setting_ref)
            .bind(actor)
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("UPDATE iam.role SET updated_by = $2 WHERE id = $1").bind(id).bind(actor).execute(&mut ***tx).await?;
    admin_remains(tx).await?;
    let after = role_json(tx, id).await?;
    db::audit(tx, actor, "iam.role_grants_changed", "role", Some(id), Some(json!({ "before": before["grants"], "after": after["grants"], "reason": reason }))).await?;
    Ok(after)
}

// ---------------------------------------------------------------------------------------------------------------------
// Approver rules (AC-02)

const REQUEST_TYPES: &[&str] = &["leave", "permission", "on_duty", "regularisation", "comp_off", "shift_change", "punch_flag", "period_unlock", "settings_change", "opening_balance"];

pub async fn approver_rules(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let items: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('request_type', request_type, 'level', level, 'source', source, 'named_user_id', named_user_id, 'updated_at', updated_at)
         FROM iam.approver_rule ORDER BY request_type, level",
    )
    .fetch_all(&mut ***tx)
    .await?;
    let latest: Option<String> = sqlx::query_scalar("SELECT to_char(max(updated_at) AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS.US\"Z\"') FROM iam.approver_rule")
        .fetch_one(&mut ***tx)
        .await?;
    Ok(json!({ "items": items, "updated_at": latest.unwrap_or_else(|| "none".into()) }))
}

pub struct LevelIn {
    pub level: i16,
    pub source: String,
    pub named_user_id: Option<Uuid>,
}

/// Replaces the rules of one request type; a level left out returns to its default (L1 reporting manager, L2 HR).
pub async fn change_approver_rules(tx: &mut OrgTx<'_>, actor: Uuid, request_type: &str, levels: &[LevelIn], reason: &str) -> Result<Value, DomainError> {
    if !REQUEST_TYPES.contains(&request_type) {
        return Err(invalid("unknown request_type"));
    }
    let mut seen = std::collections::HashSet::new();
    for l in levels {
        if !(1..=2).contains(&l.level) || !seen.insert(l.level) {
            return Err(invalid("levels are 1 and/or 2, each once"));
        }
        if !["reporting_manager", "named_user", "department_head", "hr"].contains(&l.source.as_str()) {
            return Err(invalid("source must be reporting_manager, named_user, department_head or hr"));
        }
        match (l.source == "named_user", l.named_user_id) {
            (true, Some(u)) => {
                let active: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE id = $1 AND active)").bind(u).fetch_one(&mut ***tx).await?;
                if !active {
                    return Err(invalid("named_user_id must be an active user of the organisation"));
                }
            }
            (false, None) => {}
            _ => return Err(invalid("named_user_id is set exactly when source = named_user")),
        }
    }
    let before = approver_rules(tx).await?;
    sqlx::query("DELETE FROM iam.approver_rule WHERE request_type = $1").bind(request_type).execute(&mut ***tx).await?;
    for l in levels {
        sqlx::query("INSERT INTO iam.approver_rule (org_id, request_type, level, source, named_user_id, updated_by) VALUES (app.current_org(), $1, $2, $3, $4, $5)")
            .bind(request_type)
            .bind(l.level)
            .bind(&l.source)
            .bind(l.named_user_id)
            .bind(actor)
            .execute(&mut ***tx)
            .await?;
    }
    let after = approver_rules(tx).await?;
    db::audit(tx, actor, "iam.approver_rules_changed", "approver_rule", None, Some(json!({ "request_type": request_type, "before": before["items"], "after": after["items"], "reason": reason })))
        .await?;
    Ok(after)
}

// ---------------------------------------------------------------------------------------------------------------------
// Delegations (AC-03)

const DELEGATION_JSON: &str = "jsonb_build_object('id', d.id, 'approver_user_id', d.approver_user_id, 'delegate_user_id', d.delegate_user_id,
    'valid_from', lower(d.valid), 'valid_to', upper(d.valid), 'created_by', d.created_by)";

pub async fn list_delegations(tx: &mut OrgTx<'_>, approver: Option<Uuid>, delegate: Option<Uuid>, active_on: Option<DateTime<Utc>>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!(
        "SELECT {DELEGATION_JSON} FROM iam.delegation d
         WHERE ($1::uuid IS NULL OR d.approver_user_id = $1) AND ($2::uuid IS NULL OR d.delegate_user_id = $2) AND ($3::timestamptz IS NULL OR d.valid @> $3)
         ORDER BY lower(d.valid) DESC"
    ))
    .bind(approver)
    .bind(delegate)
    .bind(active_on)
    .fetch_all(&mut ***tx)
    .await?)
}

pub async fn create_delegation(tx: &mut OrgTx<'_>, actor: Uuid, approver: Uuid, delegate: Uuid, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Value, DomainError> {
    if !setting_on(tx, "delegation_enabled").await? {
        return Err(DomainError::Forbidden); // AC-C03-08.3
    }
    if approver == delegate || from >= to || to <= Utc::now() {
        return Err(invalid("the delegate differs from the approver, valid_from is before valid_to, and the window has not ended"));
    }
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM iam.user_account WHERE id IN ($1, $2) AND active").bind(approver).bind(delegate).fetch_one(&mut ***tx).await?;
    if active != 2 {
        return Err(invalid("approver and delegate must be active users of the organisation"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO iam.delegation (org_id, approver_user_id, delegate_user_id, valid, created_by) VALUES (app.current_org(), $1, $2, tstzrange($3, $4), $5) RETURNING id",
    )
    .bind(approver)
    .bind(delegate)
    .bind(from)
    .bind(to)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await
    .map_err(|e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23P01") {
            DomainError::Refused("CONFLICTING_SETTINGS", "the approver already has a delegation in that window".into())
        } else {
            e.into()
        }
    })?;
    db::audit(tx, actor, "iam.delegation_created", "delegation", Some(id), Some(json!({ "approver": approver, "delegate": delegate, "from": from, "to": to }))).await?;
    get_delegation(tx, id).await
}

async fn get_delegation(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {DELEGATION_JSON} FROM iam.delegation d WHERE d.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

/// Shortens the window to now; a delegation not yet started is removed. Ended already -> unchanged.
pub async fn end_delegation(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid) -> Result<Value, DomainError> {
    let changed = sqlx::query(
        "UPDATE iam.delegation SET valid = tstzrange(least(lower(valid), now()), now()) WHERE id = $1 AND upper(valid) > now() AND lower(valid) < now()",
    )
    .bind(id)
    .execute(&mut ***tx)
    .await?
    .rows_affected();
    // Not started yet: nothing happened under it, so it is removed (the table forbids an empty window) and audited.
    let mut d = get_delegation(tx, id).await?;
    let future = sqlx::query("DELETE FROM iam.delegation WHERE id = $1 AND lower(valid) >= now()").bind(id).execute(&mut ***tx).await?.rows_affected();
    if future > 0 {
        d["valid_to"] = d["valid_from"].clone();
    } else {
        d = get_delegation(tx, id).await?;
    }
    if changed + future > 0 {
        db::audit(tx, actor, "iam.delegation_ended", "delegation", Some(id), None).await?;
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(kind: &str, b: bool, d: bool) -> ScopeIn {
        ScopeIn { kind: kind.into(), branch: b.then(Uuid::new_v4), department: d.then(Uuid::new_v4) }
    }

    #[test]
    fn scopes_are_all_or_branches_or_departments() {
        assert!(check_scopes(&[s("all", false, false)]).is_ok());
        assert!(check_scopes(&[s("branch", true, false), s("branch", true, false)]).is_ok());
        assert!(check_scopes(&[s("department", false, true)]).is_ok());
        assert!(check_scopes(&[]).is_err());
        assert!(check_scopes(&[s("branch", true, false), s("department", false, true)]).is_err(), "OQ-C03-04");
        assert!(check_scopes(&[s("all", false, false), s("branch", true, false)]).is_err());
        assert!(check_scopes(&[s("branch", false, false)]).is_err());
    }

    #[test]
    fn catalogue_and_module_gating() {
        assert_eq!(GRANTABLE.len(), 117, "119 organisation rows minus the two fixed denials");
        assert!(!GRANTABLE.iter().any(|c| c.starts_with("PLT-")));
        assert!(module_on("LVE-01", &[]) && !module_on("CMP-01", &[]) && module_on("CMP-01", &["M02".into()]));
        assert!(is_mobile("+919876543210") && !is_mobile("9876543210") && !is_mobile("+0123456789"));
        assert!(is_email("hr@example.in") && !is_email("hr@") && !is_email("@x.in") && !is_email("a@b") && !is_email("a b@x.in"));
    }
}
