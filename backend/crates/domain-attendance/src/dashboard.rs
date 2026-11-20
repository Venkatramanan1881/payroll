//! Dashboards (C16, DB-R1..R8, DB-01, DB-02): widgets per role, the Organisation Admin's choice of widgets per role,
//! and per-user default filters. Every count is within the caller's scope; no money (BRX-02).

use chrono::{Datelike, Duration, NaiveDate, Utc};
use db::{DomainError, OrgTx};
use serde_json::{json, Value};
use uuid::Uuid;

/// The widgets each built-in role is offered (DB-R3..R7). Custom roles and the Auditor get none until OQ-C16-01 /
/// OQ-C16-02 are answered (D-41).
pub fn offered(system_code: Option<&str>) -> &'static [&'static str] {
    match system_code {
        Some("org_admin" | "hr_manager") => &["today_counts_by_branch", "missed_punches", "pending_approvals", "devices_offline", "joiners_leavers"],
        Some("payroll_officer") => &["periods_awaiting_lock", "pre_lock_issues"],
        Some("manager") => &["team_attendance_today", "pending_approvals", "team_leave_calendar", "late_absence_trends"],
        Some("supervisor") => &["who_is_in_now", "roster_today_tomorrow"],
        Some("employee") => &["today_punch_status", "month_attendance", "leave_balance", "permission_balance", "comp_off_balance", "my_pending_requests"],
        _ => &[],
    }
}

/// Which of the caller's roles a dashboard is for when none is named: the broadest built-in one (OQ-C16-01).
const PRIORITY: [&str; 7] = ["org_admin", "hr_manager", "payroll_officer", "manager", "supervisor", "employee", "auditor"];

/// The caller's roles: (id, name, system code).
pub async fn roles_of(tx: &mut OrgTx<'_>, user: Uuid) -> Result<Vec<(Uuid, String, Option<String>)>, DomainError> {
    let mut roles: Vec<(Uuid, String, Option<String>)> =
        sqlx::query_as("SELECT r.id, r.name, r.system_code FROM iam.user_role ur JOIN iam.role r ON r.id = ur.role_id WHERE ur.user_id = $1").bind(user).fetch_all(&mut ***tx).await?;
    roles.sort_by_key(|r| r.2.as_deref().and_then(|c| PRIORITY.iter().position(|p| *p == c)).unwrap_or(PRIORITY.len()));
    Ok(roles)
}

/// Chosen widgets for a role (DB-01), or None while no choice was made (the offered list applies, OQ-C16-06).
async fn chosen(tx: &mut OrgTx<'_>, role: Uuid) -> Result<Option<Vec<String>>, DomainError> {
    Ok(sqlx::query_scalar("SELECT widgets FROM iam.dashboard_widget_choice WHERE role_id = $1").bind(role).fetch_optional(&mut ***tx).await?)
}

pub struct View {
    pub user: Uuid,
    pub employee: Option<Uuid>,
    /// Employees within the caller's dashboard scope (None = everyone), already narrowed by the branch filter.
    pub scope: Option<Vec<Uuid>>,
    pub branch: Option<Uuid>,
    pub today: NaiveDate,
}

async fn count(tx: &mut OrgTx<'_>, sql: &str, v: &View) -> Result<i64, DomainError> {
    Ok(sqlx::query_scalar(sql).bind(&v.scope).bind(v.today).fetch_one(&mut ***tx).await?)
}

/// Today's current results of the employees in scope, counted by status group.
const TODAY_COUNTS: &str = "SELECT jsonb_build_object(
    'employees', count(*),
    'present', count(*) FILTER (WHERE r.day_status IN ('present','half_day','quarter_day','on_duty')),
    'on_leave', count(*) FILTER (WHERE r.day_status IN ('leave','half_day_leave')),
    'absent', count(*) FILTER (WHERE r.day_status = 'absent'),
    'not_yet_in', count(*) FILTER (WHERE r.id IS NULL))
  FROM workforce.employee e
  LEFT JOIN attendance.attendance_day d ON d.employee_id = e.id AND d.attendance_date = $2
  LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
  WHERE e.joining_date <= $2 AND (e.exit_date IS NULL OR e.exit_date >= $2) AND ($1::uuid[] IS NULL OR e.id = ANY($1))";

/// One widget's data.
async fn widget(tx: &mut OrgTx<'_>, code: &str, v: &View) -> Result<Value, DomainError> {
    Ok(match code {
        "today_counts_by_branch" => {
            let rows: Vec<Value> = sqlx::query_scalar(
                "SELECT jsonb_build_object('branch_id', b.id, 'branch', b.name, 'employees', count(e.id),
                         'present', count(e.id) FILTER (WHERE r.day_status IN ('present','half_day','quarter_day','on_duty')),
                         'on_leave', count(e.id) FILTER (WHERE r.day_status IN ('leave','half_day_leave')),
                         'absent', count(e.id) FILTER (WHERE r.day_status = 'absent'), 'late', count(e.id) FILTER (WHERE r.late_mark),
                         'not_yet_in', count(e.id) FILTER (WHERE r.id IS NULL))
                 FROM org.branch b
                 JOIN workforce.employee_assignment a ON a.branch_id = b.id AND a.kind = 'master_branch' AND a.valid @> $2::date
                 JOIN workforce.employee e ON e.id = a.employee_id AND e.joining_date <= $2 AND (e.exit_date IS NULL OR e.exit_date >= $2)
                 LEFT JOIN attendance.attendance_day d ON d.employee_id = e.id AND d.attendance_date = $2
                 LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
                 WHERE ($1::uuid[] IS NULL OR e.id = ANY($1)) GROUP BY b.id, b.name ORDER BY b.name",
            )
            .bind(&v.scope)
            .bind(v.today)
            .fetch_all(&mut ***tx)
            .await?;
            json!({ "branches": rows })
        }
        "team_attendance_today" => sqlx::query_scalar(TODAY_COUNTS).bind(&v.scope).bind(v.today).fetch_one(&mut ***tx).await?,
        // Days of the last week left as exceptions (missed punches and other open questions) in scope.
        "missed_punches" => json!({ "days": count(tx,
            "SELECT count(*) FROM attendance.attendance_day d WHERE d.processing_state = 'exception' AND d.attendance_date BETWEEN $2 - 7 AND $2
               AND ($1::uuid[] IS NULL OR d.employee_id = ANY($1))", v).await?, "since": v.today - Duration::days(7) }),
        "pending_approvals" => {
            let n: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM (
                   SELECT current_approver_user_id a FROM leave.leave_request WHERE state LIKE 'pending%'
                   UNION ALL SELECT current_approver_user_id FROM perm.permission_request WHERE state LIKE 'pending%'
                   UNION ALL SELECT current_approver_user_id FROM perm.on_duty_request WHERE state LIKE 'pending%'
                   UNION ALL SELECT current_approver_user_id FROM attendance.regularisation_request WHERE state LIKE 'pending%'
                   UNION ALL SELECT current_approver_user_id FROM leave.comp_off_credit WHERE state = 'pending_approval'
                   UNION ALL SELECT current_approver_user_id FROM period.unlock_request WHERE state LIKE 'pending%') w WHERE a = $1",
            )
            .bind(v.user)
            .fetch_one(&mut ***tx)
            .await?;
            json!({ "waiting_on_you": n })
        }
        // Only devices of branches whose employees the caller sees (the branch filter already narrowed that set).
        "devices_offline" => json!({ "devices": sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM timecap.device d WHERE d.online_state = 'offline' AND d.retired_at IS NULL AND ($3::uuid IS NULL OR d.branch_id = $3)
               AND ($1::uuid[] IS NULL OR d.branch_id IN (SELECT a.branch_id FROM workforce.employee_assignment a
                                                          WHERE a.kind = 'master_branch' AND a.valid @> $2::date AND a.employee_id = ANY($1)))")
            .bind(&v.scope).bind(v.today).bind(v.branch).fetch_one(&mut ***tx).await? }),
        "joiners_leavers" => {
            let first = v.today.with_day(1).expect("valid");
            let (joined, left): (i64, i64) = sqlx::query_as(
                "SELECT count(*) FILTER (WHERE joining_date BETWEEN $2 AND $3), count(*) FILTER (WHERE exit_date BETWEEN $2 AND $3)
                 FROM workforce.employee WHERE ($1::uuid[] IS NULL OR id = ANY($1))",
            )
            .bind(&v.scope)
            .bind(first)
            .bind(v.today)
            .fetch_one(&mut ***tx)
            .await?;
            json!({ "since": first, "joined": joined, "left": left })
        }
        "periods_awaiting_lock" => json!({ "periods": sqlx::query_scalar::<_, Value>(
            "SELECT jsonb_build_object('id', id, 'period_start', period_start, 'period_end', period_end) FROM period.attendance_period
             WHERE state IN ('awaiting_lock','unlocked') ORDER BY period_start").fetch_all(&mut ***tx).await? }),
        "pre_lock_issues" => {
            // Pending requests and exception days inside periods waiting for their lock (the checklist's main items).
            let (requests, days): (i64, i64) = sqlx::query_as(
                "WITH p AS (SELECT period_start s, period_end e FROM period.attendance_period WHERE state IN ('awaiting_lock','unlocked'))
                 SELECT (SELECT count(*) FROM leave.leave_request q WHERE q.state LIKE 'pending%' AND ($1::uuid[] IS NULL OR q.employee_id = ANY($1))
                           AND EXISTS (SELECT 1 FROM p WHERE q.from_date <= p.e AND q.to_date >= p.s))
                      + (SELECT count(*) FROM attendance.regularisation_request q WHERE q.state LIKE 'pending%' AND ($1::uuid[] IS NULL OR q.employee_id = ANY($1))
                           AND EXISTS (SELECT 1 FROM p WHERE q.attendance_date BETWEEN p.s AND p.e)),
                        (SELECT count(*) FROM attendance.attendance_day d WHERE d.processing_state = 'exception' AND ($1::uuid[] IS NULL OR d.employee_id = ANY($1))
                           AND EXISTS (SELECT 1 FROM p WHERE d.attendance_date BETWEEN p.s AND p.e))",
            )
            .bind(&v.scope)
            .fetch_one(&mut ***tx)
            .await?;
            json!({ "pending_requests": requests, "flagged_days": days })
        }
        "team_leave_calendar" => json!({ "leave": sqlx::query_scalar::<_, Value>(
            "SELECT jsonb_build_object('employee_id', q.employee_id, 'name', e.full_name, 'from_date', q.from_date, 'to_date', q.to_date)
             FROM leave.leave_request q JOIN workforce.employee e ON e.id = q.employee_id
             WHERE q.state = 'approved' AND q.to_date >= $2 AND q.from_date <= $2 + 14 AND ($1::uuid[] IS NULL OR q.employee_id = ANY($1))
             ORDER BY q.from_date LIMIT 100").bind(&v.scope).bind(v.today).fetch_all(&mut ***tx).await? }),
        // shortcut: the last 30 days in one total until OQ-C16-05 says how trends are cut (weeks, periods).
        "late_absence_trends" => {
            let (late, absent): (i64, i64) = sqlx::query_as(
                "SELECT count(*) FILTER (WHERE r.late_mark), count(*) FILTER (WHERE r.day_status = 'absent')
                 FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
                 WHERE d.attendance_date BETWEEN $2 - 30 AND $2 AND ($1::uuid[] IS NULL OR d.employee_id = ANY($1))",
            )
            .bind(&v.scope)
            .bind(v.today)
            .fetch_one(&mut ***tx)
            .await?;
            json!({ "since": v.today - Duration::days(30), "late_marks": late, "absences": absent })
        }
        // shortcut: by local calendar date and raw punches (a night shift past midnight, or a rejected punch, can mislead).
        "who_is_in_now" => json!({ "in": count(tx,
            "SELECT count(*) FROM (SELECT p.employee_id FROM timecap.raw_punch p
               WHERE p.employee_id IS NOT NULL AND (p.event_at AT TIME ZONE p.captured_tz)::date = $2 AND ($1::uuid[] IS NULL OR p.employee_id = ANY($1))
               GROUP BY p.employee_id HAVING count(*) % 2 = 1) x", v).await? }),
        "roster_today_tomorrow" => json!({ "entries": sqlx::query_scalar::<_, Value>(
            "SELECT jsonb_build_object('date', r.roster_date, 'weekly_off', r.is_weekly_off, 'employees', count(*))
             FROM sched.roster_entry r WHERE r.roster_date IN ($2, $2 + 1) AND ($1::uuid[] IS NULL OR r.employee_id = ANY($1))
             GROUP BY r.roster_date, r.is_weekly_off ORDER BY r.roster_date").bind(&v.scope).bind(v.today).fetch_all(&mut ***tx).await? }),
        // The employee's own widgets.
        "today_punch_status" | "month_attendance" | "leave_balance" | "permission_balance" | "comp_off_balance" | "my_pending_requests" => {
            let Some(me) = v.employee else { return Ok(json!({})) };
            match code {
                "today_punch_status" => {
                    let punches: Vec<String> = sqlx::query_scalar(
                        "SELECT to_char(event_at AT TIME ZONE captured_tz, 'HH24:MI') FROM timecap.raw_punch
                         WHERE employee_id = $1 AND (event_at AT TIME ZONE captured_tz)::date = $2 ORDER BY event_at",
                    )
                    .bind(me)
                    .bind(v.today)
                    .fetch_all(&mut ***tx)
                    .await?;
                    let status: Option<String> = sqlx::query_scalar(
                        "SELECT r.day_status FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id WHERE d.employee_id = $1 AND d.attendance_date = $2",
                    )
                    .bind(me)
                    .bind(v.today)
                    .fetch_optional(&mut ***tx)
                    .await?;
                    json!({ "punches": punches, "status": status })
                }
                "month_attendance" => sqlx::query_scalar(
                    "SELECT jsonb_build_object('since', $2::date, 'days_present', to_char(coalesce(sum(r.day_fraction_present), 0), 'FM990.00'),
                            'absent', count(*) FILTER (WHERE r.day_status = 'absent'), 'leave', count(*) FILTER (WHERE r.day_status IN ('leave','half_day_leave')),
                            'late_marks', count(*) FILTER (WHERE r.late_mark), 'early_marks', count(*) FILTER (WHERE r.early_mark))
                     FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
                     WHERE d.employee_id = $1 AND d.attendance_date BETWEEN $2 AND $3",
                )
                .bind(me)
                .bind(v.today.with_day(1).expect("valid"))
                .bind(v.today)
                .fetch_one(&mut ***tx)
                .await?,
                "leave_balance" => json!({ "balances": crate::leave::balances(tx, me, Some(v.today)).await? }),
                "permission_balance" => json!({ "usage": crate::permission::usage(tx, me, v.today).await? }),
                "comp_off_balance" => {
                    let b = crate::comp_off::balance(tx, me).await?;
                    json!({ "balance": b["balance"], "pending_approval_days": b["pending_approval_days"] })
                }
                _ => sqlx::query_scalar(
                    "SELECT jsonb_build_object(
                        'leave', (SELECT count(*) FROM leave.leave_request WHERE employee_id = $1 AND state LIKE 'pending%'),
                        'permission', (SELECT count(*) FROM perm.permission_request WHERE employee_id = $1 AND state LIKE 'pending%'),
                        'on_duty', (SELECT count(*) FROM perm.on_duty_request WHERE employee_id = $1 AND state LIKE 'pending%'),
                        'regularisation', (SELECT count(*) FROM attendance.regularisation_request WHERE employee_id = $1 AND state LIKE 'pending%'))",
                )
                .bind(me)
                .fetch_one(&mut ***tx)
                .await?,
            }
        }
        _ => json!({}),
    })
}

/// DB-R1: the dashboard of one of the caller's roles (the broadest when none is named).
pub async fn dashboard(tx: &mut OrgTx<'_>, v: &View, role: Option<&str>, period: Option<Uuid>) -> Result<Value, DomainError> {
    let roles = roles_of(tx, v.user).await?;
    let picked = match role {
        Some(r) => roles.iter().find(|x| x.2.as_deref() == Some(r) || x.1 == r).ok_or_else(|| DomainError::Validation(format!("{r} is not one of your roles")))?,
        None => match roles.first() {
            Some(r) => r,
            None => return Ok(json!({ "role": "none", "filters": { "branch_id": v.branch, "period_id": period }, "widgets": [] })),
        },
    };
    let offered = offered(picked.2.as_deref());
    let codes: Vec<String> = match chosen(tx, picked.0).await? {
        Some(c) => c.into_iter().filter(|w| offered.contains(&w.as_str())).collect(),
        None => offered.iter().map(|s| s.to_string()).collect(),
    };
    let mut widgets = vec![];
    for code in codes {
        let data = widget(tx, &code, v).await?;
        widgets.push(json!({ "code": code, "as_of": Utc::now(), "data": data }));
    }
    Ok(json!({ "role": picked.2.clone().unwrap_or_else(|| picked.1.clone()), "filters": { "branch_id": v.branch, "period_id": period }, "widgets": widgets }))
}

// --- DB-01 widget choices (DSH-02) ---

/// role id, name, system code, chosen widgets, when chosen.
type ChoiceRow = (Uuid, String, Option<String>, Option<Vec<String>>, Option<chrono::DateTime<Utc>>);

pub async fn choices(tx: &mut OrgTx<'_>) -> Result<Value, DomainError> {
    let rows: Vec<ChoiceRow> = sqlx::query_as(
        "SELECT r.id, r.name, r.system_code, c.widgets, c.updated_at FROM iam.role r LEFT JOIN iam.dashboard_widget_choice c ON c.role_id = r.id ORDER BY r.name",
    )
    .fetch_all(&mut ***tx)
    .await?;
    let version = rows.iter().filter_map(|r| r.4).max().map_or("defaults".to_string(), |t| t.to_rfc3339());
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(id, name, code, widgets, _)| {
            let offered = offered(code.as_deref());
            json!({ "role_id": id, "role_name": name, "offered": offered, "default_applied": widgets.is_none(),
                    "chosen": widgets.unwrap_or_else(|| offered.iter().map(|s| s.to_string()).collect()) })
        })
        .collect();
    Ok(json!({ "items": items, "updated_at": version }))
}

/// Serialises choice changes of one organisation, so the If-Match check and the write cannot interleave.
pub async fn lock_choices(tx: &mut OrgTx<'_>) -> Result<(), DomainError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('widget_choices:' || app.current_org()::text))").execute(&mut ***tx).await?;
    Ok(())
}

/// DB-01: only widgets from the role's own list (AC-C16-08.1..08.3, 08.6); audited before / after (G9).
pub async fn set_choice(tx: &mut OrgTx<'_>, actor: Uuid, role: Uuid, widgets: &[String]) -> Result<Value, DomainError> {
    let code: Option<Option<String>> = sqlx::query_scalar("SELECT system_code FROM iam.role WHERE id = $1").bind(role).fetch_optional(&mut ***tx).await?;
    let code = code.ok_or(DomainError::NotFound)?;
    let offered = offered(code.as_deref());
    if let Some(w) = widgets.iter().find(|w| !offered.contains(&w.as_str())) {
        return Err(DomainError::Validation(format!("{w} is not offered to this role")));
    }
    if widgets.len() > 50 || widgets.iter().enumerate().any(|(i, w)| widgets[..i].contains(w)) {
        return Err(DomainError::Validation("widgets are up to 50, each once".into()));
    }
    let before = chosen(tx, role).await?;
    sqlx::query(
        "INSERT INTO iam.dashboard_widget_choice (org_id, role_id, widgets, updated_by) VALUES (app.current_org(), $1, $2, $3)
         ON CONFLICT (org_id, role_id) DO UPDATE SET widgets = EXCLUDED.widgets, updated_at = clock_timestamp(), updated_by = EXCLUDED.updated_by",
    )
    .bind(role)
    .bind(widgets)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    db::audit(tx, actor, "dashboard.widget_choice", "role", Some(role), Some(json!({ "before": before, "after": widgets }))).await?;
    let all = choices(tx).await?;
    let mut item = all["items"].as_array().and_then(|i| i.iter().find(|x| x["role_id"] == json!(role)).cloned()).unwrap_or_default();
    item["updated_at"] = all["updated_at"].clone();
    Ok(item)
}

// --- DB-02 default filters (DSH-03) ---

pub async fn default_filters(tx: &mut OrgTx<'_>, user: Uuid) -> Result<Value, DomainError> {
    let known: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM iam.user_account WHERE id = $1)").bind(user).fetch_one(&mut ***tx).await?;
    if !known {
        return Err(DomainError::NotFound);
    }
    let row: Option<(Option<Uuid>, Option<String>, chrono::DateTime<Utc>)> =
        sqlx::query_as("SELECT branch_id, period, updated_at FROM iam.dashboard_default_filter WHERE user_id = $1").bind(user).fetch_optional(&mut ***tx).await?;
    Ok(match row {
        Some((b, p, at)) => json!({ "user_id": user, "branch_id": b, "period": p, "updated_at": at.to_rfc3339() }),
        None => json!({ "user_id": user, "branch_id": null, "period": null, "updated_at": "none" }),
    })
}

/// DB-02: the branch must be inside the target user's own scope (AC-C16-08.4).
pub async fn set_default_filters(tx: &mut OrgTx<'_>, actor: Uuid, user: Uuid, branch: Option<Uuid>, period: Option<&str>, user_branches: Option<&[Uuid]>) -> Result<Value, DomainError> {
    if period.is_some_and(|p| !matches!(p, "current" | "previous")) {
        return Err(DomainError::Validation("period is current or previous".into()));
    }
    if let (Some(b), Some(allowed)) = (branch, user_branches) {
        if !allowed.contains(&b) {
            return Err(DomainError::Validation("branch_id is outside the user's scope".into()));
        }
    }
    let before = default_filters(tx, user).await?;
    sqlx::query(
        "INSERT INTO iam.dashboard_default_filter (org_id, user_id, branch_id, period, updated_by) VALUES (app.current_org(), $1, $2, $3, $4)
         ON CONFLICT (org_id, user_id) DO UPDATE SET branch_id = EXCLUDED.branch_id, period = EXCLUDED.period, updated_at = clock_timestamp(), updated_by = EXCLUDED.updated_by",
    )
    .bind(user)
    .bind(branch)
    .bind(period)
    .bind(actor)
    .execute(&mut ***tx)
    .await
    .map_err(|e| match e.as_database_error().and_then(|d| d.code()).as_deref() {
        Some("23503") => DomainError::Validation("branch_id is not a branch of the organisation".into()),
        _ => e.into(),
    })?;
    db::audit(tx, actor, "dashboard.default_filters", "user_account", Some(user), Some(json!({ "before": before, "branch_id": branch, "period": period }))).await?;
    default_filters(tx, user).await
}
