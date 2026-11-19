//! Vendor console (V01): organisations, plans, modules, limits, subscription, suspension and vendor users.
//! Runs as `app_vendor`, which sees the platform schema and the audit log only (never employee data, P12).
//! Creating an organisation also seeds its own rows: the transaction switches to `app_api` under the new organisation.

use crate::defaults::{SYSTEM_ROLE_GRANTS, VENDOR_ROLE_GRANTS};
use chrono::NaiveDate;
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

/// Optional modules released in V1 (MD-01); C01–C16 are always on and never switched.
const OPTIONAL_V1: &[&str] = &["M02"];

// ---------------------------------------------------------------------------------------------------------------------
// Vendor users and permissions (matrix VA / VS / VR columns)

/// The active vendor user's role, or None (unknown or deactivated).
pub async fn vendor_role(tx: &mut OrgTx<'_>, vendor_user: Uuid) -> Result<Option<String>, DomainError> {
    Ok(sqlx::query_scalar("SELECT vendor_role FROM platform.vendor_user WHERE id = $1 AND active").bind(vendor_user).fetch_optional(&mut ***tx).await?)
}

pub async fn vendor_by_email(tx: &mut OrgTx<'_>, email: &str) -> Result<Option<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT id FROM platform.vendor_user WHERE lower(email) = lower($1) AND active").bind(email).fetch_optional(&mut ***tx).await?)
}

/// Whether a vendor role may perform a platform operation.
pub fn allowed(role: &str, op: &str) -> bool {
    // shortcut: G-01 leaves every VS cell open; support views like read_only until it is decided.
    let role = if role == "support" { "read_only" } else { role };
    VENDOR_ROLE_GRANTS.iter().any(|(r, o)| *r == role && *o == op)
}

async fn audit(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Option<Uuid>, action: &str, target_type: &str, target: Option<Uuid>, after: Value) -> Result<(), DomainError> {
    audit_keyed(tx, vendor_user, org, action, target_type, target, after, None).await
}

#[allow(clippy::too_many_arguments)]
async fn audit_keyed(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Option<Uuid>, action: &str, target_type: &str, target: Option<Uuid>, after: Value, key: Option<Uuid>) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO audit.audit_entry (org_id, actor_kind, actor_id, action, target_type, target_id, after_value, change_ref)
         VALUES ($1, 'vendor_user', $2, $3, $4, $5, $6, $7)",
    )
    .bind(org)
    .bind(vendor_user)
    .bind(action)
    .bind(target_type)
    .bind(target)
    .bind(after)
    .bind(key.map(|k| k.to_string()))
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// Vendor creates are idempotent by the client's key, kept on their audit entry (`change_ref`): the organisation-scoped
/// idempotency table cannot hold a request that has no organisation yet. Returns what that key already created.
pub async fn replay(tx: &mut OrgTx<'_>, action: &str, key: Uuid) -> Result<Option<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT target_id FROM audit.audit_entry WHERE actor_kind = 'vendor_user' AND action = $1 AND change_ref = $2")
        .bind(action)
        .bind(key.to_string())
        .fetch_optional(&mut ***tx)
        .await?
        .flatten())
}

fn reason_ok(r: &str) -> Result<(), DomainError> {
    if r.trim().is_empty() || r.chars().count() > 1000 {
        return Err(invalid("reason is required (1-1000 characters)"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------------------
// Organisations

const ORG_JSON: &str = "jsonb_build_object('id', o.id, 'display_name', o.display_name, 'legal_name', o.legal_name, 'state', o.state,
    'state_changed_at', o.state_changed_at, 'state_reason', o.state_reason, 'sector_template_id', o.sector_template_id, 'created_at', o.created_at,
    'updated_at', o.updated_at,
    'plan_id', (SELECT p.plan_id FROM platform.organisation_plan p WHERE p.org_id = o.id AND p.valid @> current_date),
    'modules', coalesce((SELECT jsonb_agg(jsonb_build_object('module_code', m.module_code, 'enabled', m.enabled, 'valid_from', lower(m.valid),
                          'valid_to', upper(m.valid), 'reason', m.reason, 'set_by', m.set_by, 'set_at', m.set_at) ORDER BY m.module_code, lower(m.valid))
                        FROM platform.module_entitlement m WHERE m.org_id = o.id), '[]'),
    'employee_limit', (SELECT jsonb_build_object('max_active', l.max_active, 'on_reach', l.on_reach, 'valid_from', lower(l.valid))
                       FROM platform.employee_limit l WHERE l.org_id = o.id AND l.valid @> current_date),
    'subscription', (SELECT jsonb_build_object('start_date', s.start_date, 'renewal_date', s.renewal_date, 'billing_status', s.billing_status,
                       'renewal_reminder_days', s.renewal_reminder_days, 'auto_suspend_after_days', s.auto_suspend_after_days)
                     FROM platform.subscription s WHERE s.org_id = o.id))";

pub async fn get_org(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {ORG_JSON} FROM platform.organisation o WHERE o.id = $1"))
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?
        .ok_or(DomainError::NotFound)
}

pub async fn list_orgs(tx: &mut OrgTx<'_>, state: Option<&str>, template: Option<Uuid>, q: Option<&str>) -> Result<Vec<Value>, DomainError> {
    let q = q.map(|q| q.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"));
    Ok(sqlx::query_scalar(&format!(
        "SELECT {ORG_JSON} FROM platform.organisation o
         WHERE ($1::text IS NULL OR o.state = $1) AND ($2::uuid IS NULL OR o.sector_template_id = $2)
           AND ($3::text IS NULL OR o.display_name ILIKE '%' || $3 || '%' OR o.legal_name ILIKE '%' || $3 || '%')
         ORDER BY o.display_name, o.id"
    ))
    .bind(state)
    .bind(template)
    .bind(q)
    .fetch_all(&mut ***tx)
    .await?)
}

fn check_modules(modules: &[String]) -> Result<(), DomainError> {
    for m in modules {
        if !OPTIONAL_V1.contains(&m.as_str()) {
            return Err(invalid(format!("{m} cannot be switched in V1: C01-C16 are always on and only {} is released", OPTIONAL_V1.join(", "))));
        }
    }
    Ok(())
}

pub struct Subscription {
    pub start_date: NaiveDate,
    pub renewal_date: NaiveDate,
    pub billing_status: String,
    pub renewal_reminder_days: Option<i32>,
    pub auto_suspend_after_days: Option<Option<i32>>,
}

fn check_subscription(s: &Subscription) -> Result<(), DomainError> {
    if s.renewal_date < s.start_date {
        return Err(invalid("renewal_date is before start_date"));
    }
    let b = s.billing_status.trim();
    if b.is_empty() || b.chars().count() > 40 {
        return Err(invalid("billing_status is 1-40 characters"));
    }
    if s.renewal_reminder_days.is_some_and(|d| d < 0) || s.auto_suspend_after_days.flatten().is_some_and(|d| d < 0) {
        return Err(invalid("day counts cannot be negative"));
    }
    Ok(())
}

pub struct NewOrg {
    pub display_name: String,
    pub legal_name: String,
    pub sector_template_id: Uuid,
    pub plan_id: Uuid,
    pub modules: Vec<String>,
    pub max_active: Option<i32>,
    pub on_reach: String,
    pub admin_email: String,
    pub admin_mobile: Option<String>,
    pub admin_language: Option<String>,
    pub subscription: Subscription,
}

fn valid_email(e: &str) -> bool {
    let Some((l, d)) = e.split_once('@') else { return false };
    e.len() <= 254 && !l.is_empty() && !d.contains('@') && d.contains('.') && !d.starts_with('.') && !d.ends_with('.') && !e.contains(char::is_whitespace)
}

fn valid_mobile(m: &str) -> bool {
    m.len() >= 8 && m.len() <= 16 && m.starts_with('+') && m[1..].bytes().all(|b| b.is_ascii_digit()) && !m[1..].starts_with('0')
}

/// SM-ORG O1 (PLT-01): platform rows as `app_vendor`, then the new organisation's own rows as `app_api` under it,
/// all in one transaction: the seven system roles with the matrix's default grants (P9) and the first admin.
pub async fn create_org(tx: &mut OrgTx<'_>, vendor_user: Uuid, o: NewOrg, key: Uuid) -> Result<Uuid, DomainError> {
    let (name, legal) = (o.display_name.trim(), o.legal_name.trim());
    if name.is_empty() || name.chars().count() > 200 || legal.is_empty() || legal.chars().count() > 300 {
        return Err(invalid("display_name (1-200) and legal_name (1-300) are required"));
    }
    check_modules(&o.modules)?;
    check_subscription(&o.subscription)?;
    if o.max_active.is_some_and(|m| m < 1) || !["block", "allow_and_alert"].contains(&o.on_reach.as_str()) {
        return Err(invalid("employee_limit: max_active is at least 1 or null; on_reach is block or allow_and_alert"));
    }
    if !valid_email(&o.admin_email) {
        return Err(invalid("first_admin.email is not a valid email"));
    }
    // shortcut: the first admin signs in by phone OTP until a password invitation exists (the contract carries no password).
    let mobile = o.admin_mobile.as_deref().filter(|m| valid_mobile(m)).ok_or_else(|| invalid("first_admin.mobile (E.164) is required: the first admin signs in by phone until password invitations exist"))?;
    if o.admin_language.as_deref().is_some_and(|l| !["en", "ta", "hi"].contains(&l)) {
        return Err(invalid("first_admin.language must be en, ta or hi"));
    }
    let template: Option<Value> = sqlx::query_scalar("SELECT content FROM platform.sector_template WHERE id = $1").bind(o.sector_template_id).fetch_optional(&mut ***tx).await?;
    let template = template.ok_or_else(|| invalid("sector_template_id is not a sector template"))?;
    let plan: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM platform.plan WHERE id = $1)").bind(o.plan_id).fetch_one(&mut ***tx).await?;
    if !plan {
        return Err(invalid("plan_id is not a plan"));
    }

    let org: Uuid = sqlx::query_scalar("INSERT INTO platform.organisation (legal_name, display_name, sector_template_id, created_by) VALUES ($1, $2, $3, $4) RETURNING id")
        .bind(legal)
        .bind(name)
        .bind(o.sector_template_id)
        .bind(vendor_user)
        .fetch_one(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO platform.organisation_plan (org_id, plan_id, valid, set_by, reason) VALUES ($1, $2, daterange(current_date, NULL), $3, 'organisation created')")
        .bind(org)
        .bind(o.plan_id)
        .bind(vendor_user)
        .execute(&mut ***tx)
        .await?;
    for m in &o.modules {
        sqlx::query("INSERT INTO platform.module_entitlement (org_id, module_code, enabled, valid, set_by, reason) VALUES ($1, $2, true, daterange(current_date, NULL), $3, 'organisation created')")
            .bind(org)
            .bind(m)
            .bind(vendor_user)
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("INSERT INTO platform.employee_limit (org_id, max_active, on_reach, valid, set_by) VALUES ($1, $2, $3, daterange(current_date, NULL), $4)")
        .bind(org)
        .bind(o.max_active)
        .bind(&o.on_reach)
        .bind(vendor_user)
        .execute(&mut ***tx)
        .await?;
    let s = &o.subscription;
    sqlx::query(
        "INSERT INTO platform.subscription (org_id, start_date, renewal_date, billing_status, renewal_reminder_days, auto_suspend_after_days, updated_by)
         VALUES ($1, $2, $3, $4, coalesce($5, 15), $6, $7)",
    )
    .bind(org)
    .bind(s.start_date)
    .bind(s.renewal_date)
    .bind(s.billing_status.trim())
    .bind(s.renewal_reminder_days)
    .bind(s.auto_suspend_after_days.flatten())
    .bind(vendor_user)
    .execute(&mut ***tx)
    .await?;
    let summary = json!({ "template": o.sector_template_id, "plan": o.plan_id, "modules": o.modules, "max_active": o.max_active, "on_reach": o.on_reach });
    audit_keyed(tx, vendor_user, Some(org), "vendor.organisation_created", "organisation", Some(org), summary, Some(key)).await?;

    // The organisation's own rows: as the API role, under the new organisation (RLS applies as for any request).
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut ***tx).await?;
    sqlx::query("SELECT app.set_org($1)").bind(org).execute(&mut ***tx).await?;
    let mut admin_role = None;
    for (code, name) in [
        ("org_admin", "Organisation Admin"),
        ("hr_manager", "HR Manager"),
        ("payroll_officer", "Payroll Officer"),
        ("manager", "Manager"),
        ("supervisor", "Supervisor"),
        ("employee", "Employee"),
        ("auditor", "Auditor"),
    ] {
        let role: Uuid = sqlx::query_scalar("INSERT INTO iam.role (org_id, name, system_code) VALUES ($1, $2, $3) RETURNING id")
            .bind(org)
            .bind(name)
            .bind(code)
            .fetch_one(&mut ***tx)
            .await?;
        for (_, op, kind, setting) in SYSTEM_ROLE_GRANTS.iter().filter(|g| g.0 == code) {
            sqlx::query("INSERT INTO iam.role_grant (org_id, role_id, operation_code, grant_kind, setting_ref) VALUES ($1, $2, $3, $4, $5)")
                .bind(org)
                .bind(role)
                .bind(op)
                .bind(kind)
                .bind(setting)
                .execute(&mut ***tx)
                .await?;
        }
        if code == "org_admin" {
            admin_role = Some(role);
        }
    }
    let admin: Uuid = sqlx::query_scalar("INSERT INTO iam.user_account (org_id, login_kind, mobile, email, language) VALUES ($1, 'mobile_otp', $2, $3, coalesce($4, 'en')) RETURNING id")
        .bind(org)
        .bind(mobile)
        .bind(o.admin_email.trim())
        .bind(&o.admin_language)
        .fetch_one(&mut ***tx)
        .await?;
    sqlx::query("INSERT INTO iam.user_role (org_id, user_id, role_id) VALUES ($1, $2, $3)").bind(org).bind(admin).bind(admin_role).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO iam.user_scope (org_id, user_id, scope_kind) VALUES ($1, $2, 'all')").bind(org).bind(admin).execute(&mut ***tx).await?;
    // OQ-V01-10: only the template's organisation settings are copied for now; policy sets and lists come later.
    if let Some(settings) = template.get("organisation_settings").filter(|s| s.is_object()) {
        sqlx::query("INSERT INTO org.organisation_settings_version (org_id, effective_from, settings, schema_version, reason) VALUES ($1, current_date, $2, 1, 'from the sector template')")
            .bind(org)
            .bind(settings)
            .execute(&mut ***tx)
            .await?;
    }
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES ($1, 'OrganisationCreated', $1, $2)")
        .bind(org)
        .bind(json!({ "first_admin_user_id": admin }))
        .execute(&mut ***tx)
        .await?;
    sqlx::query("SET LOCAL ROLE app_vendor").execute(&mut ***tx).await?;
    Ok(org)
}

/// An event of the organisation's own outbox: written as the API role under that organisation (the vendor role has
/// no rights on organisation tables), then back to the vendor role.
async fn org_event(tx: &mut OrgTx<'_>, org: Uuid, event: &str, payload: Value) -> Result<(), DomainError> {
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut ***tx).await?;
    sqlx::query("SELECT app.set_org($1)").bind(org).execute(&mut ***tx).await?;
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES ($1, $2, $1, $3)").bind(org).bind(event).bind(payload).execute(&mut ***tx).await?;
    sqlx::query("SET LOCAL ROLE app_vendor").execute(&mut ***tx).await?;
    Ok(())
}

/// Locks the organisation row (writers of one organisation are serialised before If-Match).
pub async fn lock_org(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(), DomainError> {
    let found: Option<i32> = sqlx::query_scalar("SELECT 1 FROM platform.organisation WHERE id = $1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    found.map(|_| ()).ok_or(DomainError::NotFound)
}

async fn touch(tx: &mut OrgTx<'_>, org: Uuid, vendor_user: Uuid) -> Result<(), DomainError> {
    sqlx::query("UPDATE platform.organisation SET updated_at = now(), updated_by = $2 WHERE id = $1").bind(org).bind(vendor_user).execute(&mut ***tx).await?;
    Ok(())
}

/// Closes the open dated row of `table` at `from` (never edits history: `from` must be after the open row's start).
async fn close_open_row(tx: &mut OrgTx<'_>, table: &str, org: Uuid, extra: &str, extra_val: Option<&str>, from: NaiveDate) -> Result<(), DomainError> {
    let start: Option<NaiveDate> = sqlx::query_scalar(&format!(
        "SELECT lower(valid) FROM platform.{table} WHERE org_id = $1 {extra} AND upper_inf(valid) FOR UPDATE"
    ))
    .bind(org)
    .bind(extra_val)
    .fetch_optional(&mut ***tx)
    .await?;
    if let Some(s) = start {
        if from <= s {
            return Err(invalid(format!("the current row starts {s}; a change takes effect after it (dated history is never edited)")));
        }
        sqlx::query(&format!("UPDATE platform.{table} SET valid = daterange(lower(valid), $3) WHERE org_id = $1 {extra} AND upper_inf(valid)"))
            .bind(org)
            .bind(extra_val)
            .bind(from)
            .execute(&mut ***tx)
            .await?;
    }
    Ok(())
}

pub async fn change_plan(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, plan: Uuid, from: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM platform.plan WHERE id = $1)").bind(plan).fetch_one(&mut ***tx).await?;
    if !exists {
        return Err(invalid("plan_id is not a plan"));
    }
    close_open_row(tx, "organisation_plan", org, "AND $2::text IS NULL", None, from).await?;
    sqlx::query("INSERT INTO platform.organisation_plan (org_id, plan_id, valid, set_by, reason) VALUES ($1, $2, daterange($3, NULL), $4, $5)")
        .bind(org)
        .bind(plan)
        .bind(from)
        .bind(vendor_user)
        .bind(reason)
        .execute(&mut ***tx)
        .await?;
    touch(tx, org, vendor_user).await?;
    audit(tx, vendor_user, Some(org), "vendor.plan_changed", "organisation", Some(org), json!({ "plan_id": plan, "valid_from": from, "reason": reason })).await?;
    get_org(tx, org).await
}

/// A new dated limit row; `on_reach` None keeps the current one (VC-01).
pub async fn change_limit(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, max_active: Option<i32>, on_reach: Option<&str>, from: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    if max_active.is_some_and(|m| m < 1) || on_reach.is_some_and(|r| !["block", "allow_and_alert"].contains(&r)) {
        return Err(invalid("max_active is at least 1 or null; on_reach is block or allow_and_alert"));
    }
    let current: Option<(Option<i32>, String)> =
        sqlx::query_as("SELECT max_active, on_reach FROM platform.employee_limit WHERE org_id = $1 AND upper_inf(valid)").bind(org).fetch_optional(&mut ***tx).await?;
    let on_reach = on_reach.map(str::to_string).or(current.as_ref().map(|c| c.1.clone())).unwrap_or_else(|| "allow_and_alert".into());
    close_open_row(tx, "employee_limit", org, "AND $2::text IS NULL", None, from).await?;
    sqlx::query("INSERT INTO platform.employee_limit (org_id, max_active, on_reach, valid, set_by) VALUES ($1, $2, $3, daterange($4, NULL), $5)")
        .bind(org)
        .bind(max_active)
        .bind(&on_reach)
        .bind(from)
        .bind(vendor_user)
        .execute(&mut ***tx)
        .await?;
    touch(tx, org, vendor_user).await?;
    let before = current.map(|c| json!({ "max_active": c.0, "on_reach": c.1 }));
    audit(tx, vendor_user, Some(org), "vendor.employee_limit_changed", "organisation", Some(org), json!({ "before": before, "max_active": max_active, "on_reach": on_reach, "valid_from": from, "reason": reason })).await?;
    get_org(tx, org).await
}

/// VC-02 / VC-04 live on the subscription row (updated with old and new values in the audit); VC-01 is a dated limit row.
#[allow(clippy::too_many_arguments)]
pub async fn change_vendor_settings(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, on_reach: Option<&str>, reminder: Option<i32>, auto_suspend: Option<Option<i32>>, from: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    if on_reach.is_none() && reminder.is_none() && auto_suspend.is_none() {
        return Err(invalid("give at least one of on_reach, renewal_reminder_days, auto_suspend_after_days"));
    }
    if reminder.is_some_and(|d| d < 0) || auto_suspend.flatten().is_some_and(|d| d < 0) {
        return Err(invalid("day counts cannot be negative"));
    }
    let before = get_org(tx, org).await?;
    if let Some(r) = on_reach {
        let max: Option<i32> = before["employee_limit"]["max_active"].as_i64().map(|m| m as i32);
        change_limit(tx, vendor_user, org, max, Some(r), from, reason).await?;
    }
    if reminder.is_some() || auto_suspend.is_some() {
        let changed = sqlx::query(
            "UPDATE platform.subscription SET renewal_reminder_days = coalesce($2, renewal_reminder_days),
                    auto_suspend_after_days = CASE WHEN $3 THEN $4 ELSE auto_suspend_after_days END, updated_at = now(), updated_by = $5
             WHERE org_id = $1",
        )
        .bind(org)
        .bind(reminder)
        .bind(auto_suspend.is_some())
        .bind(auto_suspend.flatten())
        .bind(vendor_user)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(invalid("record the subscription first"));
        }
    }
    touch(tx, org, vendor_user).await?;
    let after = get_org(tx, org).await?;
    audit(tx, vendor_user, Some(org), "vendor.settings_changed", "organisation", Some(org),
        json!({ "before": { "employee_limit": before["employee_limit"], "subscription": before["subscription"] },
                "after": { "employee_limit": after["employee_limit"], "subscription": after["subscription"] }, "reason": reason })).await?;
    Ok(after)
}

pub async fn change_subscription(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, s: Subscription, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    check_subscription(&s)?;
    let before = get_org(tx, org).await?["subscription"].clone();
    sqlx::query(
        "INSERT INTO platform.subscription (org_id, start_date, renewal_date, billing_status, renewal_reminder_days, auto_suspend_after_days, updated_by)
         VALUES ($1, $2, $3, $4, coalesce($5, 15), $6, $7)
         ON CONFLICT (org_id) DO UPDATE SET start_date = $2, renewal_date = $3, billing_status = $4,
           renewal_reminder_days = coalesce($5, platform.subscription.renewal_reminder_days),
           auto_suspend_after_days = CASE WHEN $8 THEN $6 ELSE platform.subscription.auto_suspend_after_days END, updated_at = now(), updated_by = $7",
    )
    .bind(org)
    .bind(s.start_date)
    .bind(s.renewal_date)
    .bind(s.billing_status.trim())
    .bind(s.renewal_reminder_days)
    .bind(s.auto_suspend_after_days.flatten())
    .bind(vendor_user)
    .bind(s.auto_suspend_after_days.is_some())
    .execute(&mut ***tx)
    .await?;
    touch(tx, org, vendor_user).await?;
    let after = get_org(tx, org).await?;
    audit(tx, vendor_user, Some(org), "vendor.subscription_changed", "organisation", Some(org), json!({ "before": before, "after": after["subscription"], "reason": reason })).await?;
    Ok(after)
}

/// MD-01: a new dated entitlement row; the same state already in force -> unchanged.
pub async fn switch_module(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, module: &str, on: bool, from: NaiveDate, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    if module.len() == 3 && module.starts_with('C') {
        return Err(invalid("common modules C01-C16 are always on (AC-V01-03.5)"));
    }
    check_modules(&[module.to_string()])?;
    let current: Option<bool> = sqlx::query_scalar("SELECT enabled FROM platform.module_entitlement WHERE org_id = $1 AND module_code = $2 AND upper_inf(valid)")
        .bind(org)
        .bind(module)
        .fetch_optional(&mut ***tx)
        .await?;
    if current.unwrap_or(false) != on {
        close_open_row(tx, "module_entitlement", org, "AND module_code = $2", Some(module), from).await?;
        sqlx::query("INSERT INTO platform.module_entitlement (org_id, module_code, enabled, valid, set_by, reason) VALUES ($1, $2, $3, daterange($4, NULL), $5, $6)")
            .bind(org)
            .bind(module)
            .bind(on)
            .bind(from)
            .bind(vendor_user)
            .bind(reason)
            .execute(&mut ***tx)
            .await?;
        touch(tx, org, vendor_user).await?;
        audit(tx, vendor_user, Some(org), if on { "vendor.module_on" } else { "vendor.module_off" }, "organisation", Some(org), json!({ "module": module, "valid_from": from, "reason": reason })).await?;
    }
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('module_code', module_code, 'enabled', enabled, 'valid_from', lower(valid), 'valid_to', upper(valid), 'reason', reason,
                                   'set_by', set_by, 'set_at', set_at)
         FROM platform.module_entitlement WHERE org_id = $1 AND module_code = $2 ORDER BY lower(valid) DESC LIMIT 1",
    )
    .bind(org)
    .bind(module)
    .fetch_optional(&mut ***tx)
    .await?
    .unwrap_or_else(|| json!({ "module_code": module, "enabled": false, "valid_from": from, "valid_to": null, "reason": null, "set_by": vendor_user, "set_at": chrono::Utc::now() })))
}

/// SM-ORG O2 / O4. The same state again -> unchanged.
pub async fn set_state(tx: &mut OrgTx<'_>, vendor_user: Uuid, org: Uuid, suspend: bool, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    let to = if suspend { "suspended" } else { "active" };
    let changed = sqlx::query("UPDATE platform.organisation SET state = $2, state_changed_at = now(), state_reason = $3, updated_at = now(), updated_by = $4 WHERE id = $1 AND state <> $2")
        .bind(org)
        .bind(to)
        .bind(reason)
        .bind(vendor_user)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    if changed == 1 {
        audit(tx, vendor_user, Some(org), if suspend { "vendor.organisation_suspended" } else { "vendor.organisation_reactivated" }, "organisation", Some(org), json!({ "reason": reason })).await?;
        let event = if suspend { "OrganisationSuspended" } else { "OrganisationReactivated" };
        org_event(tx, org, event, json!({ "reason": reason })).await?;
    }
    get_org(tx, org).await
}

/// VC-R2 history, oldest first: dated rows (plan, module, limit) and audited changes (vendor settings, subscription, state).
pub async fn history(tx: &mut OrgTx<'_>, org: Uuid, kind: Option<&str>) -> Result<Vec<Value>, DomainError> {
    get_org(tx, org).await?;
    Ok(sqlx::query_scalar(
        "SELECT e FROM (
           SELECT set_at AS at, jsonb_build_object('kind', 'plan', 'subject', NULL, 'old_value', NULL, 'new_value', plan_id, 'valid_from', lower(valid),
                  'reason', reason, 'set_by', set_by, 'set_at', set_at) AS e FROM platform.organisation_plan WHERE org_id = $1
           UNION ALL
           SELECT set_at, jsonb_build_object('kind', 'module', 'subject', module_code, 'old_value', NULL, 'new_value', enabled, 'valid_from', lower(valid),
                  'reason', reason, 'set_by', set_by, 'set_at', set_at) FROM platform.module_entitlement WHERE org_id = $1
           UNION ALL
           SELECT set_at, jsonb_build_object('kind', 'employee_limit', 'subject', NULL, 'old_value', NULL,
                  'new_value', jsonb_build_object('max_active', max_active, 'on_reach', on_reach), 'valid_from', lower(valid), 'reason', NULL,
                  'set_by', set_by, 'set_at', set_at) FROM platform.employee_limit WHERE org_id = $1
           UNION ALL
           SELECT occurred_at, jsonb_build_object('kind', CASE action WHEN 'vendor.settings_changed' THEN 'vendor_settings' WHEN 'vendor.subscription_changed' THEN 'subscription' ELSE 'state' END,
                  'subject', NULL, 'old_value', after_value->'before', 'new_value', coalesce(after_value->'after', after_value), 'valid_from', NULL,
                  'reason', after_value->>'reason', 'set_by', actor_id, 'set_at', occurred_at)
             FROM audit.audit_entry WHERE org_id = $1 AND actor_kind = 'vendor_user'
              AND action IN ('vendor.settings_changed','vendor.subscription_changed','vendor.organisation_suspended','vendor.organisation_reactivated')
         ) h WHERE $2::text IS NULL OR e->>'kind' = $2 ORDER BY at",
    )
    .bind(org)
    .bind(kind)
    .fetch_all(&mut ***tx)
    .await?)
}

// ---------------------------------------------------------------------------------------------------------------------
// Plans (MD-02)

const PLAN_JSON: &str = "jsonb_build_object('id', p.id, 'code', p.code, 'name', p.name, 'modules', to_jsonb(p.modules), 'employee_limit', p.employee_limit,
    'mobile_app', p.mobile_app, 'updated_at', p.updated_at)";

pub async fn list_plans(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!("SELECT {PLAN_JSON} FROM platform.plan p ORDER BY p.code")).fetch_all(&mut ***tx).await?)
}

pub async fn get_plan(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {PLAN_JSON} FROM platform.plan p WHERE p.id = $1")).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub struct PlanIn {
    pub code: Option<String>,
    pub name: Option<String>,
    pub modules: Option<Vec<String>>,
    pub employee_limit: Option<Option<i32>>,
    pub mobile_app: Option<bool>,
}

fn check_plan(p: &PlanIn) -> Result<(), DomainError> {
    if p.code.as_deref().is_some_and(|c| c.trim().is_empty() || c.chars().count() > 30) || p.name.as_deref().is_some_and(|n| n.trim().is_empty() || n.chars().count() > 120) {
        return Err(invalid("code is 1-30 and name 1-120 characters"));
    }
    if p.employee_limit.flatten().is_some_and(|l| l < 1) {
        return Err(invalid("employee_limit is at least 1 or null"));
    }
    if let Some(m) = &p.modules {
        check_modules(m)?;
    }
    Ok(())
}

fn conflict(what: &'static str) -> impl FnOnce(sqlx::Error) -> DomainError {
    move |e| {
        if e.as_database_error().and_then(|d| d.code()).as_deref() == Some("23505") {
            DomainError::Refused("ALREADY_EXISTS", format!("{what} is already in use"))
        } else {
            e.into()
        }
    }
}

pub async fn create_plan(tx: &mut OrgTx<'_>, vendor_user: Uuid, p: PlanIn, key: Uuid) -> Result<Value, DomainError> {
    check_plan(&p)?;
    let id: Uuid = sqlx::query_scalar("INSERT INTO platform.plan (code, name, modules, employee_limit, mobile_app) VALUES ($1, $2, $3, $4, $5) RETURNING id")
        .bind(p.code.as_deref().map(str::trim))
        .bind(p.name.as_deref().map(str::trim))
        .bind(p.modules.clone().unwrap_or_default())
        .bind(p.employee_limit.flatten())
        .bind(p.mobile_app.unwrap_or(false))
        .fetch_one(&mut ***tx)
        .await
        .map_err(conflict("the plan code"))?;
    let plan = get_plan(tx, id).await?;
    audit_keyed(tx, vendor_user, None, "vendor.plan_created", "plan", Some(id), plan.clone(), Some(key)).await?;
    Ok(plan)
}

pub async fn update_plan(tx: &mut OrgTx<'_>, vendor_user: Uuid, id: Uuid, p: PlanIn) -> Result<Value, DomainError> {
    check_plan(&p)?;
    let before = get_plan(tx, id).await?;
    sqlx::query(
        "UPDATE platform.plan SET name = coalesce($2, name), modules = coalesce($3, modules),
                employee_limit = CASE WHEN $4 THEN $5 ELSE employee_limit END, mobile_app = coalesce($6, mobile_app), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(p.name.as_deref().map(str::trim))
    .bind(p.modules)
    .bind(p.employee_limit.is_some())
    .bind(p.employee_limit.flatten())
    .bind(p.mobile_app)
    .execute(&mut ***tx)
    .await?;
    let after = get_plan(tx, id).await?;
    audit(tx, vendor_user, None, "vendor.plan_changed", "plan", Some(id), json!({ "before": before, "after": after })).await?;
    Ok(after)
}

// ---------------------------------------------------------------------------------------------------------------------
// Vendor users (VC-05)

const VENDOR_USER_JSON: &str = "jsonb_build_object('id', v.id, 'email', v.email, 'full_name', v.full_name, 'vendor_role', v.vendor_role, 'active', v.active, 'updated_at', v.updated_at)";

pub async fn list_vendor_users(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(&format!("SELECT {VENDOR_USER_JSON} FROM platform.vendor_user v ORDER BY v.email")).fetch_all(&mut ***tx).await?)
}

pub async fn get_vendor_user(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(&format!("SELECT {VENDOR_USER_JSON} FROM platform.vendor_user v WHERE v.id = $1")).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

fn check_role(r: &str) -> Result<(), DomainError> {
    if !["full_admin", "support", "read_only"].contains(&r) {
        return Err(invalid("vendor_role is full_admin, support or read_only"));
    }
    Ok(())
}

pub async fn create_vendor_user(tx: &mut OrgTx<'_>, actor: Uuid, email: &str, full_name: &str, role: &str, key: Uuid) -> Result<Value, DomainError> {
    check_role(role)?;
    let name = full_name.trim();
    if !valid_email(email) || name.is_empty() || name.chars().count() > 200 {
        return Err(invalid("a valid email and a full_name of 1-200 characters are required"));
    }
    let id: Uuid = sqlx::query_scalar("INSERT INTO platform.vendor_user (email, full_name, vendor_role, created_by, updated_by) VALUES (lower($1), $2, $3, $4, $4) RETURNING id")
        .bind(email.trim())
        .bind(name)
        .bind(role)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await
        .map_err(conflict("the email"))?;
    let u = get_vendor_user(tx, id).await?;
    audit_keyed(tx, actor, None, "vendor.vendor_user_created", "vendor_user", Some(id), u.clone(), Some(key)).await?;
    Ok(u)
}

/// At least one active full admin always remains; nobody changes or switches off their own vendor access.
async fn full_admin_remains(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM platform.vendor_user WHERE active AND vendor_role = 'full_admin'").fetch_one(&mut ***tx).await?;
    if n == 0 {
        return Err(DomainError::IllegalTransition("the last active full admin cannot be removed".into()));
    }
    Ok(())
}

pub async fn change_vendor_role(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, role: &str, reason: &str) -> Result<Value, DomainError> {
    check_role(role)?;
    reason_ok(reason)?;
    if actor == id {
        return Err(DomainError::Forbidden);
    }
    let before = get_vendor_user(tx, id).await?;
    if before["vendor_role"] != role {
        sqlx::query("UPDATE platform.vendor_user SET vendor_role = $2, updated_at = now(), updated_by = $3 WHERE id = $1").bind(id).bind(role).bind(actor).execute(&mut ***tx).await?;
        full_admin_remains(tx).await?;
        audit(tx, actor, None, "vendor.vendor_user_role_changed", "vendor_user", Some(id), json!({ "before": before["vendor_role"], "after": role, "reason": reason })).await?;
    }
    get_vendor_user(tx, id).await
}

/// Deactivating ends console access at once: every request re-reads `active`.
pub async fn deactivate_vendor_user(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, reason: &str) -> Result<Value, DomainError> {
    reason_ok(reason)?;
    if actor == id {
        return Err(DomainError::Forbidden);
    }
    let changed = sqlx::query("UPDATE platform.vendor_user SET active = false, updated_at = now(), updated_by = $2 WHERE id = $1 AND active").bind(id).bind(actor).execute(&mut ***tx).await?.rows_affected();
    let u = get_vendor_user(tx, id).await?;
    if changed == 1 {
        full_admin_remains(tx).await?;
        audit(tx, actor, None, "vendor.vendor_user_deactivated", "vendor_user", Some(id), json!({ "reason": reason })).await?;
    }
    Ok(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_roles_follow_the_matrix() {
        assert!(allowed("full_admin", "PLT-01") && allowed("full_admin", "PLT-12"));
        assert!(allowed("read_only", "PLT-10") && !allowed("read_only", "PLT-01"));
        assert!(allowed("support", "PLT-19") && !allowed("support", "PLT-06"), "support views like read_only (G-01)");
        assert!(!allowed("full_admin", "PLT-14"), "approving support access is the organisation's");
    }

    #[test]
    fn modules_v1() {
        assert!(check_modules(&["M02".into()]).is_ok());
        assert!(check_modules(&["C01".into()]).is_err() && check_modules(&["M03".into()]).is_err());
    }
}
