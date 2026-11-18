//! Policy sets, versions and rule-group documents (G2, G3, G5; SM-POLICYVER).

use chrono::NaiveDate;
use db::{invalid, DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// The rule groups a policy version can hold (= DB domain policy.rule_group_code).
pub const RULE_GROUPS: [&str; 15] = ["PR", "BK", "LA", "EC", "EA", "MP", "DS", "RD", "RG", "LM", "LV", "PM", "OD", "HW", "PC"];

const VERSION_JSON: &str = "(to_jsonb(v) - 'org_id') || jsonb_build_object('rule_groups', coalesce((
    SELECT jsonb_agg(jsonb_build_object('rule_group', g.rule_group, 'enabled', g.enabled, 'schema_version', g.schema_version,
                                       'settings', g.settings, 'updated_at', g.updated_at, 'updated_by', g.updated_by) ORDER BY g.rule_group)
    FROM policy.policy_rule_group g WHERE g.policy_version_id = v.id), '[]'))";

/// Structural check only (typed per-setting validation arrives with the slice that evaluates each group):
/// a known group, an object, and every key a setting ID of that group ("LA-03").
pub fn check_document(group: &str, settings: &Value) -> Result<(), DomainError> {
    if !RULE_GROUPS.contains(&group) {
        return Err(invalid(format!("unknown rule group {group}")));
    }
    let obj = settings.as_object().ok_or_else(|| invalid("settings must be an object"))?;
    for key in obj.keys() {
        let ok = key
            .strip_prefix(group)
            .and_then(|rest| rest.strip_prefix('-'))
            .is_some_and(|n| n.len() == 2 && n.bytes().all(|b| b.is_ascii_digit()));
        if !ok {
            return Err(invalid(format!("{key} is not a {group} setting")));
        }
    }
    Ok(())
}

pub async fn get_version(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    let sql = format!("SELECT {VERSION_JSON} FROM policy.policy_version v WHERE v.id = $1");
    sqlx::query_scalar(&sql).bind(id).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub async fn get_set(tx: &mut OrgTx<'_>, id: Uuid) -> Result<Value, DomainError> {
    sqlx::query_scalar(
        "SELECT (to_jsonb(s) - 'org_id') || jsonb_build_object('versions', coalesce((
           SELECT jsonb_agg(to_jsonb(v) - 'org_id' ORDER BY v.effective_from) FROM policy.policy_version v WHERE v.policy_set_id = s.id), '[]'))
         FROM policy.policy_set s WHERE s.id = $1",
    )
    .bind(id)
    .fetch_optional(&mut ***tx)
    .await?
    .ok_or(DomainError::NotFound)
}

// shortcut: no cursor paging (next_cursor is always null); add one when organisations hold many policy sets.
pub async fn list_sets(tx: &mut OrgTx<'_>) -> Result<Vec<Value>, DomainError> {
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM policy.policy_set ORDER BY name").fetch_all(&mut ***tx).await?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        out.push(get_set(tx, id).await?);
    }
    Ok(out)
}

pub async fn create_set(tx: &mut OrgTx<'_>, actor: Uuid, name: &str, from: NaiveDate, copy_from: Option<Uuid>) -> Result<Uuid, DomainError> {
    if name.trim().is_empty() {
        return Err(invalid("name"));
    }
    let set: Uuid = sqlx::query_scalar("INSERT INTO policy.policy_set (org_id, name, created_by, updated_by) VALUES (app.current_org(), $1, $2, $2) RETURNING id")
        .bind(name.trim())
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
    create_draft(tx, actor, set, from, copy_from).await?;
    db::audit(tx, actor, "policy_set.create", "policy_set", Some(set), None).await?;
    Ok(set)
}

/// A new draft version, optionally copying another version's rule groups (any set of the organisation).
pub async fn create_draft(tx: &mut OrgTx<'_>, actor: Uuid, set: Uuid, from: NaiveDate, copy_from: Option<Uuid>) -> Result<Uuid, DomainError> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM policy.policy_set WHERE id = $1)").bind(set).fetch_one(&mut ***tx).await?;
    if !exists {
        return Err(DomainError::NotFound);
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO policy.policy_version (org_id, policy_set_id, effective_from, created_by, updated_by) VALUES (app.current_org(), $1, $2, $3, $3) RETURNING id",
    )
    .bind(set)
    .bind(from)
    .bind(actor)
    .fetch_one(&mut ***tx)
    .await?;
    if let Some(src) = copy_from {
        get_version(tx, src).await?; // NotFound for another organisation's version
        sqlx::query(
            "INSERT INTO policy.policy_rule_group (org_id, policy_version_id, rule_group, enabled, settings, schema_version, created_by, updated_by)
             SELECT org_id, $1, rule_group, enabled, settings, schema_version, $3, $3 FROM policy.policy_rule_group WHERE policy_version_id = $2",
        )
        .bind(id)
        .bind(src)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    }
    db::audit(tx, actor, "policy_version.create", "policy_version", Some(id), Some(json!({ "copy_from": copy_from }))).await?;
    Ok(id)
}

/// Only drafts can change (G5): published versions (scheduled, in force or superseded) are frozen.
async fn require_draft(tx: &mut OrgTx<'_>, id: Uuid) -> Result<(), DomainError> {
    let state: Option<String> = sqlx::query_scalar("SELECT state FROM policy.policy_version WHERE id = $1 FOR UPDATE").bind(id).fetch_optional(&mut ***tx).await?;
    match state.as_deref() {
        None => Err(DomainError::NotFound),
        Some("draft") => Ok(()),
        Some(_) => Err(DomainError::VersionFrozen),
    }
}

/// PATCH a draft (effective_from, reason).
pub async fn update_draft(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, from: Option<NaiveDate>, reason: Option<String>) -> Result<(), DomainError> {
    if from.is_none() && reason.is_none() {
        return Err(invalid("at least one field"));
    }
    require_draft(tx, id).await?;
    let n = sqlx::query("UPDATE policy.policy_version SET effective_from = coalesce($2, effective_from), reason = coalesce($3, reason), updated_at = now(), updated_by = $4 WHERE id = $1")
        .bind(id)
        .bind(from)
        .bind(reason)
        .bind(actor)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    if n == 0 {
        return Err(DomainError::NotFound);
    }
    db::audit(tx, actor, "policy_version.update", "policy_version", Some(id), None).await?;
    Ok(())
}

async fn touch(tx: &mut OrgTx<'_>, actor: Uuid, version: Uuid) -> Result<(), DomainError> {
    require_draft(tx, version).await?;
    // Moves the version's ETag, so a stale If-Match on the next rule-group edit is refused.
    let n = sqlx::query("UPDATE policy.policy_version SET updated_at = now(), updated_by = $2 WHERE id = $1")
        .bind(version)
        .bind(actor)
        .execute(&mut ***tx)
        .await?
        .rows_affected();
    if n == 0 { Err(DomainError::NotFound) } else { Ok(()) }
}

pub async fn get_document(tx: &mut OrgTx<'_>, version: Uuid, group: &str) -> Result<Value, DomainError> {
    sqlx::query_scalar(
        "SELECT jsonb_build_object('rule_group', rule_group, 'enabled', enabled, 'schema_version', schema_version, 'settings', settings,
           'updated_at', updated_at, 'updated_by', updated_by)
         FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = $2",
    )
    .bind(version)
    .bind(group)
    .fetch_optional(&mut ***tx)
    .await?
    .ok_or(DomainError::NotFound)
}

/// Replaces one rule group's settings in a draft. A new group starts switched on.
/// Returns the stored document and warnings for keys stored but not evaluated yet.
pub async fn put_document(tx: &mut OrgTx<'_>, actor: Uuid, version: Uuid, group: &str, schema_version: i32, settings: Value) -> Result<(Value, Vec<String>), DomainError> {
    check_document(group, &settings)?;
    if group == "PC" {
        crate::pc::PcSettings::parse(&settings).map_err(invalid)?;
    }
    if group == "RG" {
        crate::rg::RgSettings::parse(&settings).map_err(invalid)?;
    }
    let warnings = if engine::settings::GROUPS.contains(&group) {
        engine::settings::AttendanceSettings::default().apply(group, &settings).map_err(invalid)?
    } else {
        vec![]
    };
    if schema_version < 1 {
        return Err(invalid("schema_version"));
    }
    touch(tx, actor, version).await?;
    sqlx::query(
        "INSERT INTO policy.policy_rule_group (org_id, policy_version_id, rule_group, enabled, settings, schema_version, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, true, $3, $4, $5, $5)
         ON CONFLICT (org_id, policy_version_id, rule_group) DO UPDATE
           SET settings = EXCLUDED.settings, schema_version = EXCLUDED.schema_version, updated_at = now(), updated_by = EXCLUDED.updated_by",
    )
    .bind(version)
    .bind(group)
    .bind(&settings)
    .bind(schema_version)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "policy_rule_group.put", "policy_version", Some(version), Some(json!({ "rule_group": group, "settings": settings }))).await?;
    Ok((get_document(tx, version, group).await?, warnings))
}

/// G2 master switch for one rule group of a draft.
pub async fn switch(tx: &mut OrgTx<'_>, actor: Uuid, version: Uuid, group: &str, enabled: bool) -> Result<Value, DomainError> {
    if !RULE_GROUPS.contains(&group) {
        return Err(invalid(format!("unknown rule group {group}")));
    }
    touch(tx, actor, version).await?;
    sqlx::query(
        "INSERT INTO policy.policy_rule_group (org_id, policy_version_id, rule_group, enabled, settings, schema_version, created_by, updated_by)
         VALUES (app.current_org(), $1, $2, $3, '{}', 1, $4, $4)
         ON CONFLICT (org_id, policy_version_id, rule_group) DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = now(), updated_by = EXCLUDED.updated_by",
    )
    .bind(version)
    .bind(group)
    .bind(enabled)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "policy_rule_group.switch", "policy_version", Some(version), Some(json!({ "rule_group": group, "enabled": enabled }))).await?;
    get_document(tx, version, group).await
}

/// Draft -> in_force (effective_from <= today) or scheduled; earlier in_force versions of the set become superseded.
/// Refused when effective_from falls in a locked period (GAP-SM-16, kept neutral: no back-posting).
pub async fn publish(tx: &mut OrgTx<'_>, actor: Uuid, id: Uuid, today: NaiveDate) -> Result<(), DomainError> {
    let row: Option<(Uuid, String, NaiveDate)> = sqlx::query_as("SELECT policy_set_id, state, effective_from FROM policy.policy_version WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut ***tx)
        .await?;
    let (set, state, from) = row.ok_or(DomainError::NotFound)?;
    match state.as_str() {
        "draft" => {}
        "scheduled" | "in_force" | "superseded" => return Ok(()),
        other => return Err(DomainError::IllegalTransition(format!("cannot publish a {other} version"))),
    }
    // The version governs [from, next published version's start): refuse if that touches locked days (BR-06).
    let next: Option<NaiveDate> = sqlx::query_scalar(
        "SELECT min(effective_from) FROM policy.policy_version WHERE policy_set_id = $1 AND state IN ('scheduled','in_force','superseded') AND effective_from > $2",
    )
    .bind(set)
    .bind(from)
    .fetch_one(&mut ***tx)
    .await?;
    if db::any_locked(tx, from, next).await? {
        return Err(DomainError::PeriodLocked);
    }
    let new_state = if from <= today { "in_force" } else { "scheduled" };
    sqlx::query("UPDATE policy.policy_version SET state = $2, submitted_by = $3, updated_by = $3 WHERE id = $1").bind(id).bind(new_state).bind(actor).execute(&mut ***tx).await?;
    if new_state == "in_force" {
        sqlx::query("UPDATE policy.policy_version SET state = 'superseded' WHERE policy_set_id = $1 AND id <> $2 AND state = 'in_force' AND effective_from < $3")
            .bind(set)
            .bind(id)
            .bind(from)
            .execute(&mut ***tx)
            .await?;
    }
    db::audit(tx, actor, "policy_version.publish", "policy_version", Some(id), Some(json!({ "state": new_state }))).await?;
    Ok(())
}
