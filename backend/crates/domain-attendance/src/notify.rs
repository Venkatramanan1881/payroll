//! Notifications (C14, NT-05/06/07/10 in this slice): rules per event × channel × recipient, the in-app inbox, push
//! device registration, and the relay that turns outbox events into notifications. Defaults: in-app on, every other
//! channel off (D-34); only in-app is delivered (D-35); the other NT events wait for their open questions (D-36).

use chrono::{DateTime, Utc};
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

pub const CHANNELS: [&str; 5] = ["in_app", "email", "sms", "whatsapp", "push"];
/// (event, recipient kind) pairs this release sends.
pub const RULES: [(&str, &str); 5] = [("NT-05", "employee"), ("NT-05", "approver"), ("NT-06", "approver"), ("NT-07", "employee"), ("NT-10", "employee")];

fn invalid(m: impl Into<String>) -> DomainError {
    DomainError::Validation(m.into())
}

/// event, channel, recipient, enabled, updated_at, updated_by.
type RuleRow = (String, String, String, bool, DateTime<Utc>, Option<Uuid>);

/// The rule set: every sendable (event, channel, recipient) with its switch; stored rows override the defaults.
/// Serialises rule changes of one organisation, so an If-Match check and its write cannot interleave with another's.
pub async fn lock_rules(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('notification_rules:' || app.current_org()::text))").execute(&mut ***tx).await?;
    Ok(())
}

pub async fn rules(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let stored: Vec<RuleRow> =
        sqlx::query_as("SELECT event_code, channel, recipient_kind, enabled, updated_at, updated_by FROM notify.notification_rule").fetch_all(&mut ***tx).await?;
    let mut items = vec![];
    for (event, recipient) in RULES {
        for channel in CHANNELS {
            let row = stored.iter().find(|r| r.0 == event && r.1 == channel && r.2 == recipient);
            items.push(json!({ "event_code": event, "channel": channel, "recipient_kind": recipient,
                               "enabled": row.map_or(channel == "in_app", |r| r.3), "updated_at": row.map(|r| r.4), "updated_by": row.and_then(|r| r.5) }));
        }
    }
    let version = stored.iter().map(|r| r.4).max().map_or("defaults".to_string(), |t| t.to_rfc3339());
    Ok(json!({ "items": items, "updated_at": version }))
}

pub struct Change {
    pub event: String,
    pub channel: String,
    pub recipient: String,
    pub enabled: bool,
}

/// NTF-01: switch rules (G9: who, when, old, new, reason are audited).
pub async fn change_rules(tx: &mut OrgTx<'_>, actor: Uuid, changes: &[Change], reason: &str) -> Result<Value, DomainError> {
    if reason.trim().is_empty() || reason.chars().count() > 1000 {
        return Err(invalid("reason is required (1-1000 characters)"));
    }
    if changes.is_empty() || changes.len() > 500 {
        return Err(invalid("changes lists 1-500 rules"));
    }
    let before = rules(tx).await?;
    for c in changes {
        if !RULES.contains(&(c.event.as_str(), c.recipient.as_str())) || !CHANNELS.contains(&c.channel.as_str()) {
            return Err(invalid(format!("{} / {} / {} is not a rule this release sends", c.event, c.channel, c.recipient)));
        }
        if c.enabled && c.channel != "in_app" {
            return Err(invalid(format!("{}: only in-app is delivered until a provider is chosen (D-35)", c.channel)));
        }
        sqlx::query(
            "INSERT INTO notify.notification_rule (org_id, event_code, channel, recipient_kind, enabled, updated_by) VALUES (app.current_org(), $1, $2, $3, $4, $5)
             ON CONFLICT (org_id, event_code, channel, recipient_kind) DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = clock_timestamp(), updated_by = EXCLUDED.updated_by",
        )
        .bind(&c.event)
        .bind(&c.channel)
        .bind(&c.recipient)
        .bind(c.enabled)
        .bind(actor)
        .execute(&mut ***tx)
        .await?;
    }
    let after = rules(tx).await?;
    db::audit(tx, actor, "notification_rules.change", "notification_rule", None, Some(json!({ "before": before["items"], "after": after["items"], "reason": reason.trim() }))).await?;
    Ok(after)
}

async fn enabled(tx: &mut OrgTx<'_>, event: &str, recipient: &str) -> Result<bool, DomainError> {
    let stored: Option<bool> = sqlx::query_scalar("SELECT enabled FROM notify.notification_rule WHERE event_code = $1 AND channel = 'in_app' AND recipient_kind = $2")
        .bind(event)
        .bind(recipient)
        .fetch_optional(&mut ***tx)
        .await?;
    Ok(stored.unwrap_or(true))
}

/// One in-app notification, once per dedupe key (TB-03). Text is English until translations exist (OQ-C14-08).
#[allow(clippy::too_many_arguments)]
async fn send(tx: &mut OrgTx<'_>, event: &str, recipient_kind: &str, user: Uuid, dedupe: &str, title: &str, body: &str, link: Value) -> Result<bool, DomainError> {
    if !enabled(tx, event, recipient_kind).await? {
        return Ok(false);
    }
    let r = sqlx::query(
        "INSERT INTO notify.notification (org_id, event_code, recipient_user_id, channel, language, payload, state, dedupe_key, sent_at)
         SELECT app.current_org(), $1, u.id, 'in_app', u.language, $3, 'sent', $4, now() FROM iam.user_account u WHERE u.id = $2
         ON CONFLICT (org_id, dedupe_key) DO NOTHING",
    )
    .bind(event)
    .bind(user)
    .bind(json!({ "title": title, "body": body, "link": link }))
    .bind(dedupe)
    .execute(&mut ***tx)
    .await?;
    Ok(r.rows_affected() == 1)
}

/// Request tables whose requests have a routed approver and an employee.
fn request_table(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "leave" => "leave.leave_request",
        "permission" => "perm.permission_request",
        "on_duty" => "perm.on_duty_request",
        "regularisation" => "attendance.regularisation_request",
        "comp_off" => "leave.comp_off_credit",
        "period_unlock" => "period.unlock_request",
        _ => return None,
    })
}

fn words(kind: &str) -> String {
    kind.replace('_', " ")
}

/// The employee's active user account (the oldest when there are several).
const EMPLOYEE_USER: &str = "(SELECT u.id FROM iam.user_account u WHERE u.employee_id = {} AND u.active ORDER BY u.created_at LIMIT 1)";

/// NT-05 from one approval step: the employee whose request it is hears of every decision (not of their own action,
/// nor of their submission); the approver it now waits on hears that it waits.
async fn on_step(tx: &mut OrgTx<'_>, event: Uuid, p: &Value) -> Result<i64, DomainError> {
    let (Some(kind), Some(id), Some(action)) = (p["request_type"].as_str(), p["request_id"].as_str().and_then(|s| s.parse::<Uuid>().ok()), p["action"].as_str()) else {
        return Ok(0);
    };
    let Some(table) = request_table(kind) else { return Ok(0) };
    let actor = p["actor_user_id"].as_str().and_then(|s| s.parse::<Uuid>().ok());
    let level = p["level"].as_i64().unwrap_or(1);
    // The request as it is now: whose it is, whom it waits on, and whether it reached a second level.
    let employee_col = if kind == "period_unlock" { "requested_by".to_string() } else { EMPLOYEE_USER.replace("{}", "q.employee_id") };
    let row: Option<(Option<Uuid>, Option<Uuid>, String, bool)> = sqlx::query_as(&format!(
        "SELECT {employee_col}, q.current_approver_user_id, q.state,
                q.state = 'pending_l2' OR EXISTS (SELECT 1 FROM app.approval_step s WHERE s.request_type = $2 AND s.request_id = q.id AND s.level = 2)
         FROM {table} q WHERE q.id = $1"
    ))
    .bind(id)
    .bind(kind)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((owner, approver, state, two_levels)) = row else { return Ok(0) };
    let link = json!({ "kind": kind, "id": id });
    let mut sent = 0;
    if let (Some(o), true) = (owner, action != "submitted") {
        if actor != Some(o) {
            let what = action.replace('_', " ");
            let text = match (action, level, two_levels) {
                ("approved", 1, true) => format!("Your {} request was approved at level 1 and now waits for level 2.", words(kind)),
                (_, 2, _) => format!("Your {} request was {what} at level 2.", words(kind)),
                _ => format!("Your {} request was {what}.", words(kind)),
            };
            sent += i64::from(send(tx, "NT-05", "employee", o, &format!("NT-05:{event}:employee"), &format!("Request {what}"), &text, link.clone()).await?);
        }
    }
    // shortcut: a request waiting on the HR pool (no named approver) notifies no one until pool recipients are defined (OQ-C14-01).
    if let (Some(a), true) = (approver.filter(|a| Some(*a) != actor), state.starts_with("pending") && matches!(action, "submitted" | "approved" | "escalated")) {
        let text = format!("A {} request is waiting for your decision.", words(kind));
        // One notice per approver and request state, however many events of one batch lead there.
        sent += i64::from(send(tx, "NT-05", "approver", a, &format!("NT-05:{id}:{state}:{a}"), "Approval needed", &text, link).await?);
    }
    Ok(sent)
}

/// penalty id, rule, days, recovered as, the employee's user.
type PenaltyRow = (Uuid, String, Option<String>, Option<String>, Option<Uuid>);

/// NT-07: each penalty recorded by the period's latest lock, to the employee.
async fn on_period_locked(tx: &mut OrgTx<'_>, period: Uuid) -> Result<i64, DomainError> {
    let rows: Vec<PenaltyRow> = sqlx::query_as(
        "SELECT x.id, x.rule_ref, to_char(x.days, 'FM990.00'), x.recovered_as,
                (SELECT u.id FROM iam.user_account u WHERE u.employee_id = x.employee_id AND u.active ORDER BY u.created_at LIMIT 1)
         FROM attendance.period_penalty x JOIN period.attendance_period p ON p.id = x.period_id
         WHERE x.period_id = $1 AND x.created_at >= p.locked_at",
    )
    .bind(period)
    .fetch_all(&mut ***tx)
    .await?;
    let mut sent = 0;
    for (id, rule, days, recovered, user) in rows {
        let Some(user) = user else { continue };
        let text = format!("A period-end penalty ({rule}) of {} day(s) was applied, recovered as {}.", days.unwrap_or_default(), recovered.unwrap_or_default().replace('_', " "));
        sent += i64::from(send(tx, "NT-07", "employee", user, &format!("NT-07:{id}"), "Penalty applied", &text, json!({ "kind": "period", "id": period })).await?);
    }
    Ok(sent)
}

/// NT-10: a posted adjustment, to the employee it concerns.
async fn on_adjustment(tx: &mut OrgTx<'_>, adjustment: Uuid) -> Result<i64, DomainError> {
    let row: Option<(Option<Uuid>, chrono::NaiveDate)> = sqlx::query_as(
        "SELECT (SELECT u.id FROM iam.user_account u WHERE u.employee_id = a.employee_id AND u.active ORDER BY u.created_at LIMIT 1), a.original_date
         FROM period.period_adjustment a WHERE a.id = $1",
    )
    .bind(adjustment)
    .fetch_optional(&mut ***tx)
    .await?;
    let Some((Some(user), date)) = row else { return Ok(0) };
    let text = format!("A correction to {date} was posted to the next open period.");
    Ok(i64::from(send(tx, "NT-10", "employee", user, &format!("NT-10:{adjustment}"), "Adjustment posted", &text, json!({ "kind": "adjustment", "id": adjustment })).await?))
}

const HANDLED: [&str; 3] = ["ApprovalStep", "PeriodLocked", "AdjustmentPosted"];

/// The relay (ADR-002) for this organisation: reads its undispatched events this module handles, sends, and marks
/// them dispatched in the same transaction. Other event types are left for their consumers.
pub async fn dispatch(tx: &mut OrgTx<'_>, org: Uuid) -> Result<Value, DomainError> {
    sqlx::query("SET LOCAL ROLE app_relay").execute(&mut ***tx).await?;
    let events: Vec<(Uuid, String, Uuid, Value)> = sqlx::query_as(
        "SELECT event_id, event_type, aggregate_id, payload FROM app.outbox
         WHERE org_id = $1 AND dispatched_at IS NULL AND event_type = ANY($2) AND attempts < 5 ORDER BY created_at LIMIT 1000 FOR UPDATE SKIP LOCKED",
    )
    .bind(org)
    .bind(HANDLED.to_vec())
    .fetch_all(&mut ***tx)
    .await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut ***tx).await?;
    let (mut sent, mut done, mut failed) = (0, vec![], vec![]);
    for (event, kind, aggregate, payload) in &events {
        // One bad event never stalls the rest: it is rolled back alone, counted, and retried up to five times.
        sqlx::query("SAVEPOINT notify_event").execute(&mut ***tx).await?;
        let r = match kind.as_str() {
            "ApprovalStep" => on_step(tx, *event, payload).await,
            "PeriodLocked" => on_period_locked(tx, *aggregate).await,
            _ => on_adjustment(tx, *aggregate).await,
        };
        match r {
            Ok(n) => {
                sqlx::query("RELEASE SAVEPOINT notify_event").execute(&mut ***tx).await?;
                sent += n;
                done.push(*event);
            }
            Err(e) => {
                sqlx::query("ROLLBACK TO SAVEPOINT notify_event").execute(&mut ***tx).await?;
                eprintln!("notification event {event} failed: {e:?}");
                failed.push(*event);
            }
        }
    }
    sqlx::query("SET LOCAL ROLE app_relay").execute(&mut ***tx).await?;
    sqlx::query("UPDATE app.outbox SET dispatched_at = now(), attempts = attempts + 1 WHERE event_id = ANY($1)").bind(&done).execute(&mut ***tx).await?;
    sqlx::query("UPDATE app.outbox SET attempts = attempts + 1 WHERE event_id = ANY($1)").bind(&failed).execute(&mut ***tx).await?;
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut ***tx).await?;
    Ok(json!({ "events": events.len(), "sent": sent, "failed": failed.len() }))
}

/// NT-06: one daily digest per approver with requests waiting on them.
pub async fn digest(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let today = crate::period::org_today(tx).await?;
    let waiting: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT approver, count(*) FROM (
           SELECT current_approver_user_id AS approver FROM leave.leave_request WHERE state LIKE 'pending%'
           UNION ALL SELECT current_approver_user_id FROM perm.permission_request WHERE state LIKE 'pending%'
           UNION ALL SELECT current_approver_user_id FROM perm.on_duty_request WHERE state LIKE 'pending%'
           UNION ALL SELECT current_approver_user_id FROM attendance.regularisation_request WHERE state LIKE 'pending%'
           UNION ALL SELECT current_approver_user_id FROM leave.comp_off_credit WHERE state = 'pending_approval'
           UNION ALL SELECT current_approver_user_id FROM period.unlock_request WHERE state LIKE 'pending%') w
         WHERE approver IS NOT NULL GROUP BY approver",
    )
    .fetch_all(&mut ***tx)
    .await?;
    let mut sent = 0;
    for (user, n) in waiting {
        let text = format!("{n} request(s) are waiting for your decision.");
        sent += i64::from(send(tx, "NT-06", "approver", user, &format!("NT-06:{user}:{today}"), "Approvals pending", &text, Value::Null).await?);
    }
    Ok(json!({ "sent": sent }))
}

// --- Inbox and push devices (the caller's own) ---

pub async fn inbox(tx: &mut OrgTx<'_>, user: Uuid, unread: Option<bool>, event: Option<&str>) -> Result<Value, DomainError> {
    let items: Vec<Value> = sqlx::query_scalar(
        "SELECT jsonb_build_object('id', id, 'event_code', event_code, 'title', payload->>'title', 'body', payload->>'body',
                                   'link', CASE WHEN jsonb_typeof(payload->'link') = 'object' THEN payload->'link' END, 'created_at', created_at, 'read_at', read_at)
         FROM notify.notification WHERE recipient_user_id = $1 AND channel = 'in_app'
           AND ($2::boolean IS NULL OR (read_at IS NULL) = $2) AND ($3::text IS NULL OR event_code = $3)
         ORDER BY created_at DESC, id LIMIT 200",
    )
    .bind(user)
    .bind(unread)
    .bind(event)
    .fetch_all(&mut ***tx)
    .await?;
    let unread_count: i64 = sqlx::query_scalar("SELECT count(*) FROM notify.notification WHERE recipient_user_id = $1 AND channel = 'in_app' AND read_at IS NULL")
        .bind(user)
        .fetch_one(&mut ***tx)
        .await?;
    // shortcut: the newest 200 only; add the cursor when an inbox outgrows it.
    Ok(json!({ "items": items, "next_cursor": null, "unread_count": unread_count }))
}

/// Marks the caller's own notifications read; ids not theirs are ignored.
pub async fn mark_read(tx: &mut OrgTx<'_>, user: Uuid, ids: Option<&[Uuid]>) -> Result<(), DomainError> {
    sqlx::query("UPDATE notify.notification SET read_at = now(), updated_at = now() WHERE recipient_user_id = $1 AND read_at IS NULL AND ($2::uuid[] IS NULL OR id = ANY($2))")
        .bind(user)
        .bind(ids)
        .execute(&mut ***tx)
        .await?;
    Ok(())
}

/// TSD §14: one live row per token; the same token again refreshes it, for another user it moves.
pub async fn register_device(tx: &mut OrgTx<'_>, user: Uuid, token: &str, platform: &str, app_version: Option<&str>) -> Result<Value, DomainError> {
    if token.is_empty() || token.len() > 4096 || !matches!(platform, "android" | "ios") || app_version.is_some_and(|v| v.chars().count() > 40) {
        return Err(invalid("token (1-4096), platform android or ios, app_version up to 40 characters"));
    }
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('push_token:' || $1))").bind(token).execute(&mut ***tx).await?;
    // shortcut: a token stays live in another organisation the device signed into earlier (RLS hides it); revoke across organisations when push is delivered.
    sqlx::query("UPDATE notify.push_device SET revoked_at = now() WHERE token = $1 AND revoked_at IS NULL").bind(token).execute(&mut ***tx).await?;
    let device: Value = sqlx::query_scalar(
        "INSERT INTO notify.push_device (org_id, user_id, platform, token, app_version) VALUES (app.current_org(), $1, $2, $3, $4)
         RETURNING jsonb_build_object('id', id, 'platform', platform, 'app_version', app_version, 'registered_at', registered_at)",
    )
    .bind(user)
    .bind(platform)
    .bind(token)
    .bind(app_version)
    .fetch_one(&mut ***tx)
    .await?;
    db::audit(tx, user, "push_device.register", "push_device", device["id"].as_str().and_then(|s| s.parse().ok()), Some(json!({ "platform": platform }))).await?;
    Ok(device)
}

pub async fn unregister_device(tx: &mut OrgTx<'_>, user: Uuid, id: Uuid) -> Result<(), DomainError> {
    let r = sqlx::query("UPDATE notify.push_device SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL").bind(id).bind(user).execute(&mut ***tx).await?;
    if r.rows_affected() == 1 {
        db::audit(tx, user, "push_device.unregister", "push_device", Some(id), None).await?;
    }
    Ok(())
}
