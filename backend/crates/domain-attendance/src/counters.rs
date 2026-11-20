//! Period counters (PeriodCounterSet, LA-18 / EC-17) and period-end penalties (BR-10, LA-06..LA-12, EC-05..EC-11).
//! Counters are derived from the current day results plus confirmed opening counters (OB-R2). Penalties are a preview
//! until the lock, where they are recorded and recovered (D-31): from leave types in the LA-12 / EC-11 order (D-29),
//! else loss of pay; the larger of the count and minutes penalties applies once (D-30). Counters reset every
//! attendance period (D-32).

use chrono::{Datelike, NaiveDate};
use db::{DomainError, OrgTx};
use engine::penalty::PenaltyRule;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Default, Clone)]
pub struct Counter {
    pub late_marks: i64,
    pub early_marks: i64,
    pub late_minutes: i64,
    pub early_minutes: i64,
    pub opening_late_marks: i64,
    pub opening_early_marks: i64,
}

/// The attendance period containing `date`, else the calendar month.
pub async fn period_of(tx: &mut OrgTx<'_>, date: NaiveDate) -> Result<(Option<Uuid>, NaiveDate, NaiveDate), DomainError> {
    let p: Option<(Uuid, NaiveDate, NaiveDate)> =
        sqlx::query_as("SELECT id, period_start, period_end FROM period.attendance_period WHERE $1 BETWEEN period_start AND period_end ORDER BY period_start LIMIT 1")
            .bind(date)
            .fetch_optional(&mut ***tx)
            .await?;
    Ok(match p {
        Some((id, f, t)) => (Some(id), f, t),
        None => {
            let first = date.with_day(1).expect("valid");
            let next = if date.month() == 12 { NaiveDate::from_ymd_opt(date.year() + 1, 1, 1) } else { NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1) };
            (None, first, next.expect("valid").pred_opt().expect("valid"))
        }
    })
}

/// Marks and minutes from the current day results in [from, to], plus confirmed opening counters dated in it
/// (D-33: an opening counter is a number of marks). Minutes of a day a slab or an hours shortfall already cut are left out (BR-04: one
/// shortfall, one effect).
pub async fn counter(tx: &mut OrgTx<'_>, employee: Uuid, from: NaiveDate, to: NaiveDate) -> Result<Counter, DomainError> {
    let (lm, em, lmin, emin): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER (WHERE r.late_mark), count(*) FILTER (WHERE r.early_mark),
                coalesce(sum(r.late_minutes) FILTER (WHERE NOT EXISTS (SELECT 1 FROM attendance.day_effect x WHERE x.result_id = r.id AND x.effect_kind IN ('late_slab', 'hours_shortfall'))), 0)::int8,
                coalesce(sum(r.early_minutes) FILTER (WHERE NOT EXISTS (SELECT 1 FROM attendance.day_effect x WHERE x.result_id = r.id AND x.effect_kind IN ('early_slab', 'hours_shortfall'))), 0)::int8
         FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id
         WHERE d.employee_id = $1 AND d.attendance_date BETWEEN $2 AND $3",
    )
    .bind(employee)
    .bind(from)
    .bind(to)
    .fetch_one(&mut ***tx)
    .await?;
    let (ol, oe): (i64, i64) = sqlx::query_as(
        "SELECT coalesce(sum(e.value) FILTER (WHERE e.balance_kind = 'late_counter'), 0)::int8, coalesce(sum(e.value) FILTER (WHERE e.balance_kind = 'early_counter'), 0)::int8
         FROM openbal.opening_balance_entry e JOIN openbal.opening_balance_batch b ON b.id = e.batch_id
         WHERE e.employee_id = $1 AND b.state = 'confirmed' AND e.superseded_by_entry_id IS NULL AND e.as_of_date BETWEEN $2 AND $3",
    )
    .bind(employee)
    .bind(from)
    .bind(to)
    .fetch_one(&mut ***tx)
    .await?;
    Ok(Counter { late_marks: lm + ol, early_marks: em + oe, late_minutes: lmin, early_minutes: emin, opening_late_marks: ol, opening_early_marks: oe })
}

/// One penalty: id when recorded at lock.
#[derive(Debug, Clone)]
pub struct Penalty {
    pub id: Option<Uuid>,
    pub rule_ref: &'static str,
    pub days_pct: i64,
    pub minutes: Option<i64>,
    pub recovered_as: String,
    pub lop: bool,
    pub explanation: Value,
}

impl Penalty {
    pub fn json(&self, period: Option<Uuid>) -> Value {
        json!({ "id": self.id, "period_id": period, "rule_ref": self.rule_ref, "days": format!("{}.{:02}", self.days_pct / 100, self.days_pct % 100),
                "minutes": self.minutes, "recovered_as": self.recovered_as, "provisional": self.id.is_none(), "explanation": [self.explanation] })
    }
}

/// Balance on `on`, leaving out this period's own earlier penalty debits not yet reversed (they are reversed before
/// a re-lock records anew).
async fn leave_balance_pct(tx: &mut OrgTx<'_>, employee: Uuid, leave_type: Uuid, on: NaiveDate, period: Option<Uuid>) -> Result<i64, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT coalesce((sum(e.quantity) * 100)::int8, 0) FROM leave.leave_ledger_entry e JOIN leave.leave_account a ON a.id = e.account_id
         WHERE a.employee_id = $1 AND a.leave_type_id = $2 AND e.entry_date <= $3
           AND NOT (e.entry_kind = 'penalty_debit' AND e.cause_id IN (SELECT id FROM attendance.period_penalty WHERE period_id = $4)
                    AND NOT EXISTS (SELECT 1 FROM leave.leave_ledger_entry r WHERE r.reverses_entry_id = e.id))",
    )
    .bind(employee)
    .bind(leave_type)
    .bind(on)
    .bind(period)
    .fetch_one(&mut ***tx)
    .await?)
}

/// LA-12 / EC-11: the first listed leave type (paid, not comp-off) with enough balance, else loss of pay or report only.
#[allow(clippy::too_many_arguments)]
async fn recover(tx: &mut OrgTx<'_>, employee: Uuid, rule: &PenaltyRule, days: i64, on: NaiveDate, period: Option<Uuid>, recording: bool, used: &mut HashMap<Uuid, i64>) -> Result<(String, Option<Uuid>, bool), DomainError> {
    for code in &rule.recovery {
        if code == crate::comp_off::LEAVE_CODE {
            continue;
        }
        let Some(t): Option<Uuid> = sqlx::query_scalar("SELECT id FROM leave.leave_type WHERE code = $1 AND retired_at IS NULL").bind(code).fetch_optional(&mut ***tx).await? else { continue };
        if !crate::leave::version_on(tx, t, on).await?.is_some_and(|v| v.paid) {
            continue;
        }
        if recording {
            crate::leave::lock_account(tx, employee, t).await?; // no leave approval moves the balance under us
        }
        let left = leave_balance_pct(tx, employee, t, on, period).await? - used.get(&t).copied().unwrap_or(0);
        if left >= days {
            *used.entry(t).or_default() += days;
            return Ok((code.clone(), Some(t), false));
        }
    }
    Ok(if rule.then_lop { ("loss_of_pay".into(), None, true) } else { ("none".into(), None, false) })
}

/// Settings per policy version, loaded once per lock or preview (most employees share one).
pub type SettingsCache = HashMap<Uuid, engine::settings::AttendanceSettings>;

/// The penalties for one employee over [from, to] (their employed part of the period). With `record` =
/// (run, actor) they are written and leave recoveries posted; otherwise a preview.
// shortcut: a few queries per employee (policy, counters, recovery); batch them if a lock of thousands gets slow.
pub async fn penalties(tx: &mut OrgTx<'_>, employee: Uuid, period: Option<Uuid>, from: NaiveDate, to: NaiveDate, record: Option<(Uuid, Uuid)>, cache: &mut SettingsCache) -> Result<Vec<Penalty>, DomainError> {
    let Some(master) = domain_timecap::punch::master_branch_on(tx, employee, to).await? else { return Ok(vec![]) };
    let Some(version) = domain_policy::assignment::resolve_policy(tx, employee, to, master).await?.and_then(|r| r.version_id) else { return Ok(vec![]) };
    if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(version) {
        e.insert(crate::load_settings(tx, version).await?);
    }
    let s = cache[&version].clone();
    if !(s.penalties.late.count_on || s.penalties.late.minutes_on || s.penalties.early.count_on || s.penalties.early.minutes_on) {
        return Ok(vec![]);
    }
    let c = counter(tx, employee, from, to).await?;
    // shortcut: minutes convert to days on the DS-02 full day, else 8 hours; use each day's required minutes if the BRD asks.
    let day_min = s.full_day_min.unwrap_or(480);
    let mut used = HashMap::new();
    let mut out = vec![];
    for (rule, count_ref, minutes_ref, marks, minutes) in [
        (&s.penalties.late, "LA-06", "LA-10", c.late_marks, c.late_minutes),
        (&s.penalties.early, "EC-05", "EC-09", c.early_marks, c.early_minutes),
    ] {
        let Some((rule_ref, days, mins, why)) = rule.apply(count_ref, minutes_ref, marks, minutes, day_min) else { continue };
        let days = days.min(((to - from).num_days() + 1) * 100); // never more than the days employed in the period
        let (recovered_as, leave_type, lop) = recover(tx, employee, rule, days, to, period, record.is_some(), &mut used).await?;
        let mut p = Penalty { id: None, rule_ref, days_pct: days, minutes: mins, recovered_as, lop, explanation: json!({ "step": 1, "rule_ref": rule_ref, "inputs": why, "outcome": "" }) };
        p.explanation["outcome"] = json!(format!("{}.{:02} day(s) recovered as {}", days / 100, days % 100, p.recovered_as));
        if let (Some((run, actor)), Some(period)) = (record, period) {
            let id: Uuid = sqlx::query_scalar(
                "INSERT INTO attendance.period_penalty (org_id, run_id, employee_id, period_id, counter_period, rule_ref, days, minutes, recovered_as, explanation, created_by)
                 VALUES (app.current_org(), $1, $2, $3, daterange($4, $5, '[]'), $6, $7::numeric / 100, $8, $9, $10, $11) RETURNING id",
            )
            .bind(run)
            .bind(employee)
            .bind(period)
            .bind(from)
            .bind(to)
            .bind(rule_ref)
            .bind(days as i32)
            .bind(mins.map(|m| m as i32))
            .bind(&p.recovered_as)
            .bind(json!([p.explanation]))
            .bind(actor)
            .fetch_one(&mut ***tx)
            .await?;
            if let Some(t) = leave_type {
                let account = crate::leave::lock_account(tx, employee, t).await?;
                sqlx::query(
                    "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reason, created_by)
                     VALUES (app.current_org(), $1, $2, 'penalty_debit', -($3::numeric / 100), 'period_penalty', $4, $5, $6)",
                )
                .bind(account)
                .bind(to)
                .bind(days as i32)
                .bind(id)
                .bind(format!("period penalty {rule_ref} (BR-10)"))
                .bind(actor)
                .execute(&mut ***tx)
                .await?;
            }
            p.id = Some(id);
        }
        out.push(p);
    }
    Ok(out)
}

/// At lock: earlier runs' leave recoveries for this period are reversed (a re-lock after an unlock records anew), and
/// each employee's counter set is rebuilt and kept.
pub async fn before_lock(tx: &mut OrgTx<'_>, actor: Uuid, period: Uuid) -> Result<(), DomainError> {
    sqlx::query(
        "INSERT INTO leave.leave_ledger_entry (org_id, account_id, entry_date, entry_kind, quantity, cause_kind, cause_id, reverses_entry_id, reason, created_by)
         SELECT app.current_org(), e.account_id, e.entry_date, 'reversal', -e.quantity, 'period_penalty', e.cause_id, e.id, 'period locked again: penalties recorded anew', $2
         FROM leave.leave_ledger_entry e JOIN attendance.period_penalty p ON p.id = e.cause_id
         WHERE e.cause_kind = 'period_penalty' AND e.entry_kind = 'penalty_debit' AND p.period_id = $1
           AND NOT EXISTS (SELECT 1 FROM leave.leave_ledger_entry r WHERE r.reverses_entry_id = e.id)",
    )
    .bind(period)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// Keeps the rebuilt counter set for the employee's part of the period (derived, replaced on every rebuild).
pub async fn keep_counter(tx: &mut OrgTx<'_>, actor: Uuid, employee: Uuid, from: NaiveDate, to: NaiveDate) -> Result<(), DomainError> {
    let c = counter(tx, employee, from, to).await?;
    sqlx::query("DELETE FROM attendance.period_counter WHERE employee_id = $1 AND counter_period && daterange($2, $3, '[]')").bind(employee).bind(from).bind(to).execute(&mut ***tx).await?;
    sqlx::query(
        "INSERT INTO attendance.period_counter (org_id, employee_id, counter_period, late_marks, early_marks, late_minutes, early_minutes, opening_late_marks, opening_early_marks, created_by, updated_by)
         VALUES (app.current_org(), $1, daterange($2, $3, '[]'), $4, $5, $6, $7, $8, $9, $10, $10)",
    )
    .bind(employee)
    .bind(from)
    .bind(to)
    .bind((c.late_marks - c.opening_late_marks) as i32)
    .bind((c.early_marks - c.opening_early_marks) as i32)
    .bind(c.late_minutes as i32)
    .bind(c.early_minutes as i32)
    .bind(c.opening_late_marks as i32)
    .bind(c.opening_early_marks as i32)
    .bind(actor)
    .execute(&mut ***tx)
    .await?;
    Ok(())
}

/// GET counters: the counter set over the employee's period containing `date`, with the recorded penalties of the
/// latest lock, or a preview while the period is open.
pub async fn view(tx: &mut OrgTx<'_>, employee: Uuid, date: NaiveDate) -> Result<Value, DomainError> {
    let known: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM workforce.employee WHERE id = $1)").bind(employee).fetch_one(&mut ***tx).await?;
    if !known {
        return Err(DomainError::NotFound); // TB-01: another organisation's employee is not found, never an empty set
    }
    let (period, from, to) = period_of(tx, date).await?;
    // The employee's employed part of the period, as the totals and the lock use it.
    let (from, to): (NaiveDate, NaiveDate) = sqlx::query_as("SELECT greatest(joining_date, $2), least(coalesce(exit_date, $3), $3) FROM workforce.employee WHERE id = $1")
        .bind(employee)
        .bind(from)
        .bind(to)
        .fetch_one(&mut ***tx)
        .await?;
    let c = counter(tx, employee, from, to).await?;
    let recorded: Vec<Value> = match period {
        Some(p) => sqlx::query_scalar(
            "SELECT jsonb_build_object('id', x.id, 'period_id', x.period_id, 'rule_ref', x.rule_ref, 'days', to_char(x.days, 'FM990.00'), 'minutes', x.minutes,
                                       'recovered_as', x.recovered_as, 'provisional', false, 'explanation', x.explanation)
             FROM attendance.period_penalty x
             WHERE x.employee_id = $1 AND x.period_id = $2
               AND x.created_at >= (SELECT locked_at FROM period.attendance_period WHERE id = $2) -- recorded by the latest lock only
             ORDER BY x.rule_ref",
        )
        .bind(employee)
        .bind(p)
        .fetch_all(&mut ***tx)
        .await?,
        None => vec![],
    };
    let locked: bool = match period {
        Some(p) => sqlx::query_scalar("SELECT state = 'locked' FROM period.attendance_period WHERE id = $1").bind(p).fetch_one(&mut ***tx).await?,
        None => false,
    };
    let penalties = if locked || from > to { recorded } else { penalties(tx, employee, period, from, to, None, &mut SettingsCache::new()).await?.iter().map(|p| p.json(period)).collect() };
    Ok(json!({
        "employee_id": employee, "counter_period_start": from, "counter_period_end": to,
        "late_marks": c.late_marks, "early_marks": c.early_marks, "late_minutes": c.late_minutes, "early_minutes": c.early_minutes,
        "grace_uses_late": 0, "grace_uses_early": 0, "opening_late_marks": c.opening_late_marks, "opening_early_marks": c.opening_early_marks,
        "opening_late_minutes": 0, "opening_early_minutes": 0, "rebuilt_at": chrono::Utc::now(), "penalties": penalties,
    }))
}
