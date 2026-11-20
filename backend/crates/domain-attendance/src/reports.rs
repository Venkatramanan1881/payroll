//! Reports (C13, §14): R1-R10, R13, R25-R27 as rows for a spreadsheet. Read models only, no money (BRX-02).
//! Every report reads the employees the caller may see, then the filters (AC-C13-01.1, 01.2).

use chrono::NaiveDate;
use db::{DomainError, OrgTx};
use uuid::Uuid;

/// Report rows: headers and text cells.
pub struct Table {
    pub headers: Vec<&'static str>,
    pub rows: Vec<Vec<String>>,
}

// shortcut: a report holds at most this many rows (a year of a few hundred employees fits); page or stream past it.
const MAX_ROWS: i64 = 200_000;

pub struct Filters {
    /// The employees the caller may see for this report (None = everyone).
    pub visible: Option<Vec<Uuid>>,
    pub employees: Option<Vec<Uuid>>,
    pub branches: Option<Vec<Uuid>>,
    pub departments: Option<Vec<Uuid>>,
}

/// The employees employed in [from, to] that the caller sees and the filters keep, by code.
pub async fn employees(tx: &mut OrgTx<'_>, from: NaiveDate, to: NaiveDate, f: &Filters) -> Result<Vec<Uuid>, DomainError> {
    Ok(sqlx::query_scalar(
        "SELECT e.id FROM workforce.employee e
         WHERE e.joining_date <= $2 AND (e.exit_date IS NULL OR e.exit_date >= $1)
           AND ($3::uuid[] IS NULL OR e.id = ANY($3)) AND ($4::uuid[] IS NULL OR e.id = ANY($4))
           AND ($5::uuid[] IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'master_branch'
                                              AND a.valid @> least(coalesce(e.exit_date, $2), $2) AND a.branch_id = ANY($5)))
           AND ($6::uuid[] IS NULL OR EXISTS (SELECT 1 FROM workforce.employee_assignment a WHERE a.employee_id = e.id AND a.kind = 'department'
                                              AND a.valid @> least(coalesce(e.exit_date, $2), $2) AND a.list_entry_id = ANY($6)))
         ORDER BY e.employee_code",
    )
    .bind(from)
    .bind(to)
    .bind(&f.visible)
    .bind(&f.employees)
    .bind(&f.branches)
    .bind(&f.departments)
    .fetch_all(&mut ***tx)
    .await?)
}

/// The SQL of each report: $1 the employees (uuid[]), $2 from, $3 to; each row one text[].
fn sql(report: &str) -> Option<(Vec<&'static str>, &'static str)> {
    Some(match report {
        // R1 daily attendance (AC-C13-02.1).
        "R1" => (
            vec!["date", "employee_code", "name", "status", "first_in", "last_out", "net_hours", "late_minutes", "early_minutes", "flags"],
            "SELECT ARRAY[d.attendance_date::text, e.employee_code, e.full_name, r.day_status,
                          to_char((r.sessions->0->>'in_at')::timestamptz AT TIME ZONE r.schedule_tz, 'HH24:MI'),
                          to_char((r.sessions->-1->>'out_at')::timestamptz AT TIME ZONE r.schedule_tz, 'HH24:MI'),
                          to_char(r.net_minutes / 60.0, 'FM990.00'), r.late_minutes::text, r.early_minutes::text, array_to_string(r.flags, ' ')]
             FROM attendance.attendance_day d JOIN attendance.day_result r ON r.id = d.current_result_id JOIN workforce.employee e ON e.id = d.employee_id
             WHERE d.employee_id = ANY($1) AND d.attendance_date BETWEEN $2 AND $3 ORDER BY d.attendance_date, e.employee_code",
        ),
        // R2 monthly register (AC-C13-02.2): day counts per employee.
        "R2" => (
            vec!["employee_code", "name", "days_present", "present", "half_day", "absent", "leave", "on_duty", "weekly_off", "holiday", "late_marks", "early_marks"],
            "SELECT ARRAY[e.employee_code, e.full_name, to_char(coalesce(sum(r.day_fraction_present), 0), 'FM9990.00'),
                          count(*) FILTER (WHERE r.day_status = 'present')::text, count(*) FILTER (WHERE r.day_status IN ('half_day','half_day_leave'))::text,
                          count(*) FILTER (WHERE r.day_status = 'absent')::text, count(*) FILTER (WHERE r.day_status = 'leave')::text,
                          count(*) FILTER (WHERE r.day_status = 'on_duty')::text, count(*) FILTER (WHERE r.day_status = 'weekly_off')::text,
                          count(*) FILTER (WHERE r.day_status = 'holiday')::text, count(*) FILTER (WHERE r.late_mark)::text, count(*) FILTER (WHERE r.early_mark)::text]
             FROM workforce.employee e
             LEFT JOIN attendance.attendance_day d ON d.employee_id = e.id AND d.attendance_date BETWEEN $2 AND $3
             LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
             WHERE e.id = ANY($1) GROUP BY e.id, e.employee_code, e.full_name ORDER BY e.employee_code",
        ),
        // R3 late / early marks, minutes and recorded penalties (AC-C13-02.3..02.5).
        "R3" => (
            vec!["employee_code", "name", "late_marks", "late_minutes", "early_marks", "early_minutes", "penalties"],
            "SELECT ARRAY[e.employee_code, e.full_name, count(r.id) FILTER (WHERE r.late_mark)::text, coalesce(sum(r.late_minutes), 0)::text,
                          count(r.id) FILTER (WHERE r.early_mark)::text, coalesce(sum(r.early_minutes), 0)::text,
                          (SELECT string_agg(p.rule_ref || ' ' || coalesce(to_char(p.days, 'FM990.00'), '') || ' ' || coalesce(p.recovered_as, ''), '; ' ORDER BY p.rule_ref)
                           FROM attendance.period_penalty p JOIN period.attendance_period ap ON ap.id = p.period_id
                           WHERE p.employee_id = e.id AND ap.period_start <= $3 AND ap.period_end >= $2 AND p.created_at >= ap.locked_at)]
             FROM workforce.employee e
             LEFT JOIN attendance.attendance_day d ON d.employee_id = e.id AND d.attendance_date BETWEEN $2 AND $3
             LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
             WHERE e.id = ANY($1) GROUP BY e.id, e.employee_code, e.full_name ORDER BY e.employee_code",
        ),
        // R4 regularisations with status (AC-C13-02.6).
        "R4" => (
            vec!["date", "employee_code", "name", "request_type", "state", "reason", "submitted_at", "decided_at"],
            "SELECT ARRAY[q.attendance_date::text, e.employee_code, e.full_name, q.request_type, q.state, q.reason, q.created_at::text, q.decided_at::text]
             FROM attendance.regularisation_request q JOIN workforce.employee e ON e.id = q.employee_id
             WHERE q.employee_id = ANY($1) AND q.attendance_date BETWEEN $2 AND $3 ORDER BY q.attendance_date, e.employee_code",
        ),
        // R5 flagged punches (AC-C13-02.7; device status: no device registry rows yet).
        "R5" => (
            vec!["punch_at", "employee_code", "name", "source", "reason", "state"],
            "SELECT ARRAY[p.event_at::text, e.employee_code, e.full_name, p.source, f.reason, f.state]
             FROM timecap.punch_flag f JOIN timecap.raw_punch p ON p.id = f.punch_id JOIN workforce.employee e ON e.id = p.employee_id
             WHERE p.employee_id = ANY($1) AND p.event_at >= $2::date AND p.event_at < $3::date + 1 ORDER BY p.event_at",
        ),
        // R6 leave balance (as at `to`) and days taken in the range (AC-C13-03.1).
        "R6" => (
            vec!["employee_code", "name", "leave_type", "balance", "taken"],
            "SELECT ARRAY[e.employee_code, e.full_name, t.code,
                          to_char(coalesce((SELECT sum(l.quantity) FROM leave.leave_ledger_entry l WHERE l.account_id = a.id AND l.entry_date <= $3), 0), 'FM9990.00'),
                          to_char(coalesce((SELECT sum(x.fraction) FROM leave.leave_request_day x JOIN leave.leave_request q ON q.id = x.leave_request_id
                                            WHERE q.employee_id = e.id AND q.leave_type_id = t.id AND q.state = 'approved' AND x.leave_date BETWEEN $2 AND $3), 0), 'FM9990.00')]
             FROM leave.leave_account a JOIN workforce.employee e ON e.id = a.employee_id JOIN leave.leave_type t ON t.id = a.leave_type_id
             WHERE e.id = ANY($1) ORDER BY e.employee_code, t.code",
        ),
        // R7 absences with informed / uninformed counts in separate columns (AC-C13-03.2/03.3; sandwich days not built).
        "R7" => (
            vec!["employee_code", "name", "absent_days", "informed_leave", "unpaid_leave", "uninformed_absence"],
            "SELECT ARRAY[e.employee_code, e.full_name, count(r.id) FILTER (WHERE r.day_status = 'absent')::text,
                          count(c.id) FILTER (WHERE c.classification = 'informed_leave')::text, count(c.id) FILTER (WHERE c.classification = 'unpaid_leave')::text,
                          count(c.id) FILTER (WHERE c.classification = 'uninformed_absence')::text]
             FROM workforce.employee e
             LEFT JOIN attendance.attendance_day d ON d.employee_id = e.id AND d.attendance_date BETWEEN $2 AND $3
             LEFT JOIN attendance.day_result r ON r.id = d.current_result_id
             LEFT JOIN attendance.absence_classification c ON c.decided_by_result_id = r.id
             WHERE e.id = ANY($1) GROUP BY e.id, e.employee_code, e.full_name ORDER BY e.employee_code",
        ),
        // R8 permission and on-duty register (AC-C13-03.4).
        "R8" => (
            vec!["kind", "from_date", "to_date", "employee_code", "name", "detail", "state", "reason"],
            "SELECT * FROM (
               SELECT ARRAY['permission', q.permission_date::text, q.permission_date::text, e.employee_code, e.full_name,
                            to_char(q.start_time, 'HH24:MI') || '-' || to_char(q.end_time, 'HH24:MI'), q.state, q.reason] AS row
               FROM perm.permission_request q JOIN workforce.employee e ON e.id = q.employee_id
               WHERE q.employee_id = ANY($1) AND q.permission_date BETWEEN $2 AND $3
               UNION ALL
               SELECT ARRAY['on_duty', o.from_date::text, o.to_date::text, e.employee_code, e.full_name,
                            o.duration_kind || coalesce(' ' || to_char(o.from_time, 'HH24:MI') || '-' || to_char(o.to_time, 'HH24:MI'), ''), o.state,
                            o.purpose || ' @ ' || o.place]
               FROM perm.on_duty_request o JOIN workforce.employee e ON e.id = o.employee_id
               WHERE o.employee_id = ANY($1) AND o.from_date <= $3 AND o.to_date >= $2) x
             ORDER BY row[2], row[4]",
        ),
        // R9 comp-off earned, taken, expired in the range, and the balance at its end (AC-C13-04.1).
        "R9" => (
            vec!["employee_code", "name", "earned", "taken", "expired", "balance"],
            "SELECT ARRAY[e.employee_code, e.full_name,
                          to_char(coalesce(sum(l.quantity) FILTER (WHERE l.entry_kind = 'credit' AND l.entry_date BETWEEN $2 AND $3), 0), 'FM990.00'),
                          to_char(coalesce(-sum(l.quantity) FILTER (WHERE l.entry_kind IN ('debit','reversal') AND l.entry_date BETWEEN $2 AND $3), 0), 'FM990.00'),
                          to_char(coalesce(-sum(l.quantity) FILTER (WHERE l.entry_kind = 'expiry' AND l.entry_date BETWEEN $2 AND $3), 0), 'FM990.00'),
                          to_char(coalesce(sum(l.quantity) FILTER (WHERE l.entry_date <= $3), 0), 'FM990.00')]
             FROM workforce.employee e
             LEFT JOIN leave.comp_off_credit c ON c.employee_id = e.id
             LEFT JOIN leave.comp_off_ledger_entry l ON l.credit_id = c.id
             WHERE e.id = ANY($1) GROUP BY e.id, e.employee_code, e.full_name ORDER BY e.employee_code",
        ),
        // R10 people: one row per employee with branch, department, category and type (AC-C13-05.1..05.3).
        "R10" => (
            vec!["employee_code", "name", "status", "joining_date", "exit_date", "branch", "department", "category", "employee_type"],
            "SELECT ARRAY[e.employee_code, e.full_name, e.employment_status, e.joining_date::text, e.exit_date::text,
                          (SELECT b.name FROM workforce.employee_assignment a JOIN org.branch b ON b.id = a.branch_id
                           WHERE a.employee_id = e.id AND a.kind = 'master_branch' AND a.valid @> least(coalesce(e.exit_date, $3), $3) LIMIT 1),
                          (SELECT l.name FROM workforce.employee_assignment a JOIN org.list_entry l ON l.id = a.list_entry_id
                           WHERE a.employee_id = e.id AND a.kind = 'department' AND a.valid @> least(coalesce(e.exit_date, $3), $3) LIMIT 1),
                          (SELECT l.name FROM workforce.employee_assignment a JOIN org.list_entry l ON l.id = a.list_entry_id
                           WHERE a.employee_id = e.id AND a.kind = 'category' AND a.valid @> least(coalesce(e.exit_date, $3), $3) LIMIT 1),
                          (SELECT l.name FROM workforce.employee_assignment a JOIN org.list_entry l ON l.id = a.list_entry_id
                           WHERE a.employee_id = e.id AND a.kind = 'employee_type' AND a.valid @> least(coalesce(e.exit_date, $3), $3) LIMIT 1)]
             FROM workforce.employee e WHERE e.id = ANY($1) ORDER BY e.employee_code",
        ),
        // R25 confirmed opening balances with as-of dates (AC-C13-07.1).
        "R25" => (
            vec!["employee_code", "name", "balance_kind", "type", "value", "as_of_date", "confirmed_at"],
            "SELECT ARRAY[e.employee_code, e.full_name, x.balance_kind, coalesce(t.code, pt.code), to_char(x.value, 'FM99990.00'), x.as_of_date::text, b.confirmed_at::text]
             FROM openbal.opening_balance_entry x JOIN openbal.opening_balance_batch b ON b.id = x.batch_id JOIN workforce.employee e ON e.id = x.employee_id
             LEFT JOIN leave.leave_type t ON t.id = x.leave_type_id LEFT JOIN perm.permission_type pt ON pt.id = x.permission_type_id
             WHERE x.employee_id = ANY($1) AND b.state = 'confirmed' AND x.superseded_by_entry_id IS NULL ORDER BY e.employee_code, x.balance_kind",
        ),
        // R26 settings changes (G9): who, when, what, old and new values (AC-C13-07.2).
        "R26" => (
            vec!["occurred_at", "actor_kind", "actor_id", "action", "target_type", "target_id", "before", "after"],
            "SELECT ARRAY[a.occurred_at::text, a.actor_kind, a.actor_id::text, a.action, a.target_type, a.target_id::text, a.before_value::text, a.after_value::text]
             FROM audit.audit_entry a
             WHERE a.occurred_at >= $2::date AND a.occurred_at < $3::date + 1 AND cardinality($1) >= 0
               AND (a.action LIKE 'policy%' OR a.action LIKE '%settings%' OR a.action LIKE 'notification_rules%' OR a.target_type IN ('policy_version','policy_assignment','organisation_settings'))
             ORDER BY a.occurred_at",
        ),
        // R27 audit trail (AC-C13-07.3); vendor actions under a support grant included (AC-C13-07.4).
        "R27" => (
            vec!["occurred_at", "actor_kind", "actor_id", "support_grant_id", "action", "target_type", "target_id", "before", "after"],
            "SELECT ARRAY[a.occurred_at::text, a.actor_kind, a.actor_id::text, a.support_grant_id::text, a.action, a.target_type, a.target_id::text,
                          a.before_value::text, a.after_value::text]
             FROM audit.audit_entry a WHERE a.occurred_at >= $2::date AND a.occurred_at < $3::date + 1 AND cardinality($1) >= 0 ORDER BY a.occurred_at",
        ),
        _ => return None,
    })
}

/// The rows of a report over the employees and dates (R13 is read from the period's totals by the caller).
pub async fn rows(tx: &mut OrgTx<'_>, report: &str, employees: &[Uuid], from: NaiveDate, to: NaiveDate) -> Result<Table, DomainError> {
    let (headers, query) = sql(report).ok_or(DomainError::NotFound)?;
    let rows: Vec<Vec<Option<String>>> = sqlx::query_scalar(&format!("{query} LIMIT {MAX_ROWS}"))
        .bind(employees)
        .bind(from)
        .bind(to)
        .fetch_all(&mut ***tx)
        .await?;
    Ok(Table { headers, rows: rows.into_iter().map(|r| r.into_iter().map(Option::unwrap_or_default).collect()).collect() })
}

/// CSV with every cell quoted when it needs to be (RFC 4180), and a guard against spreadsheet formula injection.
pub fn csv(t: &Table) -> Vec<u8> {
    let cell = |s: &str| {
        let s = if s.starts_with(['=', '+', '-', '@']) && s.parse::<f64>().is_err() { format!("'{s}") } else { s.to_string() };
        if s.contains([',', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
    };
    let mut out = String::new();
    for line in std::iter::once(t.headers.iter().map(|h| h.to_string()).collect::<Vec<_>>()).chain(t.rows.iter().cloned()) {
        out.push_str(&line.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_quotes_and_defuses_formulas() {
        let t = Table { headers: vec!["a", "b"], rows: vec![vec!["x,y".into(), "=SUM(A1)".into()], vec!["say \"hi\"".into(), "-1.50".into()]] };
        assert_eq!(String::from_utf8(csv(&t)).unwrap(), "a,b\r\n\"x,y\",'=SUM(A1)\r\n\"say \"\"hi\"\"\",-1.50\r\n");
    }
}
