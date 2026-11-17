//! Recording app and web punches (PC-R5, PC-R11, PC-R13; TSD §9.2, §9.3). Raw punches are insert-only (BR-01).

use crate::evaluate::{evaluate, locate, Fence, Loc, PunchContext, PunchInput};
use chrono::{DateTime, NaiveDate, Utc};
use db::{invalid, DomainError, OrgTx};
use domain_policy::pc;
use serde_json::{json, Value};
use uuid::Uuid;

pub struct Capture {
    pub idempotency_key: Uuid,
    /// "app" or "web".
    pub source: String,
    /// Original event time (BR-11): server receipt for a live punch, phone clock for an offline one.
    pub event_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    pub captured_tz: String,
    pub location: Option<Loc>,
    pub location_unavailable_reason: Option<String>,
    pub selfie_path: Option<String>,
    pub client_ref: Option<String>,
    pub client_ip: Option<String>,
    pub device_uptime_ms: Option<i64>,
    pub clock_skew_ms: Option<i64>,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Stored { punch_id: Uuid, flags: Vec<String>, duplicate: bool },
    Refused { attempt_id: Uuid, rule_ref: &'static str, message: String },
}

/// The punch already stored under this key, if any. Another employee's key is a client error, never a replay.
async fn existing(tx: &mut OrgTx<'_>, employee: Uuid, key: Uuid) -> Result<Option<Outcome>, DomainError> {
    let row: Option<(Uuid, Option<Uuid>)> = sqlx::query_as("SELECT id, employee_id FROM timecap.raw_punch WHERE idempotency_key = $1")
        .bind(key.to_string())
        .fetch_optional(&mut ***tx)
        .await?;
    let Some((id, owner)) = row else { return Ok(None) };
    if owner != Some(employee) {
        return Err(invalid("idempotency_key is already used"));
    }
    let flags: Vec<String> = sqlx::query_scalar("SELECT reason FROM timecap.punch_flag WHERE punch_id = $1 ORDER BY created_at, reason")
        .bind(id)
        .fetch_all(&mut ***tx)
        .await?;
    Ok(Some(Outcome::Stored { punch_id: id, flags, duplicate: true }))
}

async fn org_setting(tx: &mut OrgTx<'_>, key: &str, date: NaiveDate) -> Result<Option<String>, DomainError> {
    Ok(sqlx::query_scalar("SELECT settings->>$1 FROM org.organisation_settings_version WHERE effective_from <= $2 ORDER BY effective_from DESC LIMIT 1")
        .bind(key)
        .bind(date)
        .fetch_optional(&mut ***tx)
        .await?
        .flatten())
}

/// The local date of an instant in a time zone (BR-11); refuses an unknown zone.
pub async fn local_date(tx: &mut OrgTx<'_>, at: DateTime<Utc>, tz: &str) -> Result<NaiveDate, DomainError> {
    let valid: bool = sqlx::query_scalar("SELECT app.is_valid_timezone($1)").bind(tz).fetch_one(&mut ***tx).await?;
    if !valid {
        return Err(invalid("captured_tz is not a known time zone"));
    }
    Ok(sqlx::query_scalar("SELECT ($1::timestamptz AT TIME ZONE $2)::date").bind(at).bind(tz).fetch_one(&mut ***tx).await?)
}

pub async fn master_branch_on(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Option<Uuid>, DomainError> {
    Ok(sqlx::query_scalar("SELECT branch_id FROM workforce.employee_assignment WHERE employee_id = $1 AND kind = 'master_branch' AND valid @> $2::date")
        .bind(employee)
        .bind(date)
        .fetch_optional(&mut ***tx)
        .await?
        .flatten())
}

/// Evaluates and stores one punch for `employee`, or records the refused attempt (E18).
/// The caller commits in both cases so a refused attempt survives the 422.
pub async fn record(tx: &mut OrgTx<'_>, actor: Uuid, employee: Uuid, c: &Capture) -> Result<Outcome, DomainError> {
    if !matches!(c.source.as_str(), "app" | "web") {
        return Err(invalid("source must be app or web"));
    }
    if let Some(o) = existing(tx, employee, c.idempotency_key).await? {
        return Ok(o);
    }
    if let Some(path) = &c.selfie_path {
        let ours: bool = sqlx::query_scalar("SELECT starts_with($1, app.current_org()::text || '/')").bind(path).fetch_one(&mut ***tx).await?;
        if !ours {
            return Err(invalid("selfie.path must be a file of this organisation")); // TB-02
        }
    }
    let date = local_date(tx, c.event_at, &c.captured_tz).await?;
    let master = master_branch_on(tx, employee, date).await?.ok_or_else(|| invalid("not employed on the punch date"))?;

    // Settings in force on the punch date (BR-05); the policy is chosen by the master branch (EM-R9).
    let resolved = domain_policy::assignment::resolve_policy(tx, employee, date, master).await?;
    let settings = pc::load(tx, resolved.and_then(|r| r.version_id)).await?;
    let allowed_branches: Vec<Uuid> = match org_setting(tx, "EM-06", date).await?.as_deref() {
        Some("all_branches") => sqlx::query_scalar("SELECT id FROM org.branch WHERE id <> $1").bind(master).fetch_all(&mut ***tx).await?,
        Some("chosen_branches") => {
            sqlx::query_scalar("SELECT branch_id FROM workforce.employee_allowed_branch WHERE employee_id = $1 AND valid @> $2::date AND branch_id <> $3")
                .bind(employee)
                .bind(date)
                .bind(master)
                .fetch_all(&mut ***tx)
                .await?
        }
        _ => vec![], // EM-06 default: master branch only
    };
    let branch_not_allowed_blocks = org_setting(tx, "EM-07", date).await?.as_deref() == Some("block");
    let fences: Vec<(Uuid, f64, f64, i32)> =
        sqlx::query_as("SELECT branch_id, latitude::float8, longitude::float8, radius_m FROM org.geo_fence WHERE enabled").fetch_all(&mut ***tx).await?;
    let mut ctx = PunchContext {
        settings,
        master_branch: master,
        allowed_branches,
        branch_not_allowed_blocks,
        fences: fences.into_iter().map(|(branch_id, lat, lon, r)| Fence { branch_id, lat, lon, radius_m: r.into() }).collect(),
        ip_ok: None,
        off_day: false,
    };
    if c.source == "web" && ctx.settings.as_ref().is_some_and(|s| s.web_ip_restricted) {
        let ok = match &c.client_ip {
            Some(ip) => sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM org.branch_ip_allow WHERE branch_id = ANY($1) AND $2::inet <<= network)")
                .bind(ctx.applicable_branches())
                .bind(ip)
                .fetch_one(&mut ***tx)
                .await?,
            None => false,
        };
        ctx.ip_ok = Some(ok);
    }
    // D-04: the holiday calendar of the branch punched at, else the master branch.
    let located = locate(c.location, &ctx);
    let day = domain_sched::schedule::expected_day(tx, employee, date, located.branch_id.unwrap_or(master)).await?;
    ctx.off_day = day.weekly_off || day.holiday.is_some() || day.shift_id.is_none();

    let input = PunchInput {
        source: c.source.clone(),
        location: c.location,
        has_selfie: c.selfie_path.is_some(),
        event_at: c.event_at,
        received_at: c.received_at,
        clock_skew_ms: c.clock_skew_ms,
    };
    let d = evaluate(&input, &ctx);
    let (lat, lon, acc) = c.location.map(|l| (Some(l.lat), Some(l.lon), Some(l.accuracy_m as i32))).unwrap_or_default();

    if let Some((rule_ref, message)) = d.refused {
        let attempt_id: Uuid = sqlx::query_scalar(
            "INSERT INTO timecap.punch_attempt_refused (org_id, employee_id, source, attempted_at, rule_ref, message, latitude, longitude, accuracy_m, client_ip, created_by)
             VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, $9::inet, $10) RETURNING id",
        )
        .bind(employee)
        .bind(&c.source)
        .bind(c.received_at)
        .bind(rule_ref)
        .bind(&message)
        .bind(lat)
        .bind(lon)
        .bind(acc)
        .bind(&c.client_ip)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
        return Ok(Outcome::Refused { attempt_id, rule_ref, message });
    }

    // A concurrent request with the same key waits here, then finds the stored punch.
    let id: Option<Uuid> = sqlx::query_scalar(
        "INSERT INTO timecap.raw_punch (org_id, employee_id, source, client_ref, event_at, captured_tz, received_at, idempotency_key, branch_id,
           latitude, longitude, accuracy_m, fence_distance_m, inside_fence, selfie_file_path, client_ip, device_uptime_ms, created_by)
         VALUES (app.current_org(), $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15::inet, $16, $17)
         ON CONFLICT (org_id, idempotency_key) DO NOTHING RETURNING id",
    )
    .bind(employee)
    .bind(&c.source)
    .bind(&c.client_ref)
    .bind(c.event_at)
    .bind(&c.captured_tz)
    .bind(c.received_at)
    .bind(c.idempotency_key.to_string())
    .bind(d.located.branch_id)
    .bind(lat)
    .bind(lon)
    .bind(acc)
    .bind(d.located.fence_distance_m)
    .bind(d.located.inside_fence)
    .bind(&c.selfie_path)
    .bind(&c.client_ip)
    .bind(c.device_uptime_ms)
    .bind(actor)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some(id) = id else {
        return existing(tx, employee, c.idempotency_key).await?.ok_or_else(|| invalid("idempotency_key is already used"));
    };
    for f in &d.flags {
        let mut evidence = f.evidence.clone();
        if f.reason == "location_unavailable" {
            evidence["reason"] = json!(c.location_unavailable_reason);
        }
        sqlx::query(
            "INSERT INTO timecap.punch_flag (org_id, punch_id, reason, counts_before_decision, evidence, created_by, updated_by)
             VALUES (app.current_org(), $1, $2, $3, $4, $5, $5)",
        )
        .bind(id)
        .bind(f.reason)
        .bind(f.counts_before_decision)
        .bind(evidence)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    }
    let flags: Vec<String> = d.flags.iter().map(|f| f.reason.to_string()).collect();
    outbox(tx, "PunchRecorded", id, json!({ "employee_id": employee, "local_date": date })).await?;
    if !flags.is_empty() {
        outbox(tx, "PunchFlagged", id, json!({ "employee_id": employee, "reasons": flags })).await?;
    }
    db::audit(tx, actor, "punch.record", "raw_punch", Some(id), Some(json!({ "source": c.source, "flags": flags }))).await?;
    Ok(Outcome::Stored { punch_id: id, flags, duplicate: false })
}

pub(crate) async fn outbox(tx: &mut OrgTx<'_>, event: &str, aggregate: Uuid, payload: Value) -> Result<(), DomainError> {
    sqlx::query("INSERT INTO app.outbox (org_id, event_type, aggregate_id, payload) VALUES (app.current_org(), $1, $2, $3)")
        .bind(event)
        .bind(aggregate)
        .bind(payload)
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

const PUNCH_JSON: &str = "jsonb_build_object('id', p.id, 'employee_id', p.employee_id, 'source', p.source, 'event_at', p.event_at,
    'captured_tz', p.captured_tz, 'received_at', p.received_at, 'branch_id', p.branch_id, 'device_id', p.device_id, 'enrolment_id', p.enrolment_id,
    'client_ref', p.client_ref, 'direction_tag', p.direction_tag, 'device_uptime_ms', p.device_uptime_ms, 'is_manual', p.source = 'manual',
    'manual_reason', p.manual_reason, 'manual_entered_by', p.manual_entered_by, 'manual_approval_ref', p.manual_approval_ref,
    'has_selfie', p.selfie_file_path IS NOT NULL,
    'location', CASE WHEN $2 AND p.latitude IS NOT NULL THEN jsonb_build_object('latitude', p.latitude, 'longitude', p.longitude,
        'accuracy_m', p.accuracy_m, 'fence_distance_m', p.fence_distance_m, 'inside_fence', p.inside_fence) END,
    'flags', coalesce((SELECT jsonb_agg(jsonb_build_object('id', f.id, 'reason', f.reason, 'state', f.state,
        'counts_before_decision', f.counts_before_decision) ORDER BY f.created_at, f.reason) FROM timecap.punch_flag f WHERE f.punch_id = p.id), '[]'))";

/// PunchRecord; location only when the caller may see it (TCP-08, PERMISSION_MATRIX §4).
pub async fn get_punch(tx: &mut OrgTx<'_>, id: Uuid, with_location: bool) -> Result<Value, DomainError> {
    let sql = format!("SELECT {PUNCH_JSON} FROM timecap.raw_punch p WHERE p.id = $1");
    sqlx::query_scalar(&sql).bind(id).bind(with_location).fetch_optional(&mut ***tx).await?.ok_or(DomainError::NotFound)
}

pub struct PunchFilter {
    pub employees: Option<Vec<Uuid>>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub source: Option<String>,
    /// Has a flag still waiting for a decision.
    pub has_open_flag: Option<bool>,
}

// shortcut: no cursor paging, newest 500 only; add keyset paging on (event_at, id) when lists grow.
pub async fn list_punches(tx: &mut OrgTx<'_>, f: &PunchFilter, with_location: bool) -> Result<Vec<Value>, DomainError> {
    let sql = format!(
        "SELECT {PUNCH_JSON} FROM timecap.raw_punch p
         WHERE ($1::uuid[] IS NULL OR p.employee_id = ANY($1)) AND ($3::date IS NULL OR (p.event_at AT TIME ZONE p.captured_tz)::date >= $3)
           AND ($4::date IS NULL OR (p.event_at AT TIME ZONE p.captured_tz)::date <= $4) AND ($5::text IS NULL OR p.source = $5)
           AND ($6::bool IS NULL OR EXISTS (SELECT 1 FROM timecap.punch_flag f WHERE f.punch_id = p.id AND f.state = 'raised') = $6)
         ORDER BY p.event_at DESC, p.id LIMIT 500"
    );
    Ok(sqlx::query_scalar(&sql)
        .bind(&f.employees)
        .bind(with_location)
        .bind(f.from)
        .bind(f.to)
        .bind(&f.source)
        .bind(f.has_open_flag)
        .fetch_all(&mut ***tx)
        .await?)
}

/// PC-R9: manual punches for a device failure. Each entry is a new RawPunch (source manual, reason, entered by); nothing is
/// edited. Locked dates are stored too; the difference posts to the next open period (SM-ADJ Q1).
/// `request_key` makes the per-entry idempotency keys, so a replayed request never stores twice.
pub async fn record_manual(
    tx: &mut OrgTx<'_>,
    actor: Uuid,
    request_key: Uuid,
    branch: Uuid,
    device: Option<Uuid>,
    reason: &str,
    entries: &[(Uuid, DateTime<Utc>)],
) -> Result<Vec<Uuid>, DomainError> {
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(invalid("reason"));
    }
    let mut ids = Vec::with_capacity(entries.len());
    for (i, (employee, at)) in entries.iter().enumerate() {
        // shortcut: a branch with no dated time zone is read as India time (V1 market); require one when branches abroad appear.
        let tz: String = sqlx::query_scalar(
            "SELECT coalesce((SELECT tz FROM org.branch_timezone WHERE branch_id = $1 AND valid @> ($2::timestamptz AT TIME ZONE 'UTC')::date), 'Asia/Kolkata')",
        )
        .bind(branch)
        .bind(at)
        .fetch_one(&mut ***tx)
        .await?;
        let date = local_date(tx, *at, &tz).await?;
        if master_branch_on(tx, *employee, date).await?.is_none() {
            return Err(invalid(format!("entries[{i}].employee_id is not employed on that date")));
        }
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO timecap.raw_punch (org_id, employee_id, source, device_id, event_at, captured_tz, idempotency_key, branch_id, manual_reason, manual_entered_by, created_by)
             VALUES (app.current_org(), $1, 'manual', $2, $3, $4, $5, $6, $7, $8, $8) RETURNING id",
        )
        .bind(employee)
        .bind(device)
        .bind(at)
        .bind(&tz)
        .bind(format!("manual:{request_key}:{i}"))
        .bind(branch)
        .bind(reason)
        .bind(actor)
        .fetch_one(&mut ***tx)
        .await?;
        outbox(tx, "PunchRecorded", id, json!({ "employee_id": employee, "local_date": date, "manual": true })).await?;
        db::audit(tx, actor, "punch.manual", "raw_punch", Some(id), Some(json!({ "reason": reason, "branch_id": branch }))).await?;
        ids.push(id);
    }
    Ok(ids)
}

pub struct AttemptFilter {
    pub employees: Option<Vec<Uuid>>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub rule_ref: Option<String>,
}

// shortcut: no cursor paging, newest 500 only.
pub async fn list_attempts(tx: &mut OrgTx<'_>, f: &AttemptFilter, with_location: bool) -> Result<Vec<Value>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT jsonb_build_object('id', a.id, 'employee_id', a.employee_id, 'source', a.source, 'device_id', a.device_id, 'attempted_at', a.attempted_at,
           'rule_ref', a.rule_ref, 'message', a.message,
           'location', CASE WHEN $2 AND a.latitude IS NOT NULL THEN jsonb_build_object('latitude', a.latitude, 'longitude', a.longitude, 'accuracy_m', a.accuracy_m) END)
         FROM timecap.punch_attempt_refused a
         WHERE ($1::uuid[] IS NULL OR a.employee_id = ANY($1)) AND ($3::date IS NULL OR a.attempted_at::date >= $3)
           AND ($4::date IS NULL OR a.attempted_at::date <= $4) AND ($5::text IS NULL OR a.rule_ref = $5)
         ORDER BY a.attempted_at DESC, a.id LIMIT 500",
    )
    .bind(&f.employees)
    .bind(with_location)
    .bind(f.from)
    .bind(f.to)
    .bind(&f.rule_ref)
    .fetch_all(&mut ***tx)
    .await?)
}
