//! One employee's day: punches to sessions, worked minutes, late/early, missed punches, status and effects.
//! Times are instants; the caller turns the shift's local times into instants in the schedule's zone.

use crate::settings::{AttendanceSettings, Basis, MpAction, Slab, SlabAction};
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Bump when a rule's meaning changes, so unchanged inputs are still recalculated (BR-02, AT5).
pub const ENGINE_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Expected {
    Working { start_at: DateTime<Utc>, end_at: DateTime<Utc>, required_min: i64, flexible: bool },
    WeeklyOff,
    Holiday,
    /// No schedule applies.
    Unscheduled,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PunchIn {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    /// Set when the punch must not count yet (rejected flag, or raised flag that does not count before decision: D-05c).
    pub excluded: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DayInput {
    pub expected: Expected,
    /// The punches inside the day's window (SH-07), chosen by the caller.
    pub punches: Vec<PunchIn>,
    /// The missed-punch cut-off (D-08) has passed.
    pub cutoff_passed: bool,
    pub settings: AttendanceSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub in_at: DateTime<Utc>,
    pub out_at: Option<DateTime<Utc>>,
    pub minutes: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PunchUse {
    pub id: Uuid,
    pub direction: Option<&'static str>,
    pub session_no: Option<i64>,
    pub excluded: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Effect {
    pub kind: &'static str,
    /// Hundredths of a day.
    pub days_pct: Option<i64>,
    pub minutes: Option<i64>,
    pub reason_code: String,
    pub rule_ref: &'static str,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct DayOutput {
    pub sessions: Vec<Session>,
    pub punches_used: Vec<PunchUse>,
    pub gross_min: i64,
    pub break_min: i64,
    pub net_min: i64,
    pub late_min: i64,
    pub early_min: i64,
    pub early_arrival_min: i64,
    pub late_mark: bool,
    pub early_mark: bool,
    pub status: &'static str,
    /// Hundredths of a day present.
    pub fraction_pct: i64,
    pub flags: Vec<String>,
    pub effects: Vec<Effect>,
    pub explanation: Vec<Value>,
    /// SM-DAY processing state: provisional, exception or absence_pending.
    pub state: &'static str,
}

pub fn fingerprint(input: &DayInput) -> Vec<u8> {
    let bytes = serde_json::to_vec(input).expect("day input serialises");
    Sha256::new().chain_update(ENGINE_VERSION).chain_update([0u8]).chain_update(bytes).finalize().to_vec()
}

fn minutes(from: DateTime<Utc>, to: DateTime<Utc>) -> i64 {
    (to - from).num_minutes().max(0)
}

struct Pair {
    in_p: Uuid,
    in_at: DateTime<Utc>,
    out: Option<(Uuid, DateTime<Utc>)>,
    /// Inner out/in punches absorbed by PR-04.
    inner: Vec<(Uuid, &'static str)>,
}

fn slab_for(slabs: &[Slab], m: i64) -> Option<&Slab> {
    slabs.iter().find(|s| m >= s.from && s.to.is_none_or(|t| m <= t))
}

fn status_of(pct: i64, quarter_days: bool) -> &'static str {
    match pct {
        100.. => "present",
        50..=99 => "half_day",
        25..=49 if quarter_days => "quarter_day",
        _ => "absent",
    }
}

pub fn compute_day(input: &DayInput) -> DayOutput {
    let s = &input.settings;
    let mut out = DayOutput { state: "provisional", ..DayOutput::default() };

    // Punches: flag exclusions (D-05c), then repeats (PR-02/03).
    let mut punches: Vec<&PunchIn> = input.punches.iter().collect();
    punches.sort_by_key(|p| (p.at, p.id));
    let mut kept: Vec<&PunchIn> = Vec::new();
    let mut prev_at: Option<DateTime<Utc>> = None;
    for p in punches {
        if let Some(r) = &p.excluded {
            out.punches_used.push(PunchUse { id: p.id, direction: None, session_no: None, excluded: Some(r.clone()) });
            continue;
        }
        let repeat = prev_at.is_some_and(|t| (p.at - t).num_seconds() < s.repeat_window_s);
        prev_at = Some(p.at);
        if repeat {
            let dropped = if s.keep_last_repeat { std::mem::replace(kept.last_mut().expect("repeat follows a kept punch"), p) } else { p };
            out.punches_used.push(PunchUse { id: dropped.id, direction: None, session_no: None, excluded: Some("repeat_punch".into()) });
            continue;
        }
        kept.push(p);
    }
    if kept.len() as i64 > s.max_punches {
        out.flags.push("too_many_punches".into());
        step(&mut out.explanation, "PR-06", json!({ "punches": kept.len(), "max": s.max_punches }), "flagged".into());
    }

    // Pairing (PR-01).
    let mut pairs: Vec<Pair> = Vec::new();
    if s.pr_first_in_last_out && kept.len() >= 2 {
        let (first, last) = (kept[0], kept[kept.len() - 1]);
        for p in &kept[1..kept.len() - 1] {
            out.punches_used.push(PunchUse { id: p.id, direction: None, session_no: None, excluded: Some("not_first_or_last".into()) });
        }
        pairs.push(Pair { in_p: first.id, in_at: first.at, out: Some((last.id, last.at)), inner: vec![] });
    } else {
        for c in kept.chunks(2) {
            pairs.push(Pair { in_p: c[0].id, in_at: c[0].at, out: c.get(1).map(|p| (p.id, p.at)), inner: vec![] });
        }
    }
    // PR-05: a session longer than the maximum is a missed punch; its out does not count.
    for pr in &mut pairs {
        if let Some((id, at)) = pr.out {
            if minutes(pr.in_at, at) > s.max_session_min {
                pr.out = None;
                out.punches_used.push(PunchUse { id, direction: None, session_no: None, excluded: Some("session_longer_than_PR-05".into()) });
                out.flags.push("session_too_long".into());
            }
        }
    }
    // PR-04: a short out-then-in gap is continuous work.
    let mut merged: Vec<Pair> = Vec::new();
    for pr in pairs {
        if let Some(last) = merged.last_mut() {
            if let Some((out_id, out_at)) = last.out {
                if pr.out.is_some() && minutes(out_at, pr.in_at) < s.min_gap_min {
                    last.inner.push((out_id, "out"));
                    last.inner.push((pr.in_p, "in"));
                    last.out = pr.out;
                    continue;
                }
            }
        }
        merged.push(pr);
    }

    // Missed punches (MP-01/02 before the cut-off, MP-03 at it). BR-09: a missing out is never an early check-out.
    let open = merged.iter().any(|p| p.out.is_none());
    let mp_action = open.then(|| if input.cutoff_passed { s.mp_cutoff } else if kept.len() == 1 { s.mp_single } else { s.mp_odd });
    let working = match &input.expected {
        Expected::Working { start_at, end_at, required_min, flexible } => Some((*start_at, *end_at, *required_min, *flexible)),
        _ => None,
    };
    if open {
        out.flags.push("missed_punch".into());
        let rule = if input.cutoff_passed { "MP-03" } else if kept.len() == 1 { "MP-02" } else { "MP-01" };
        step(&mut out.explanation, rule, json!({ "cutoff_passed": input.cutoff_passed }), format!("{:?}", mp_action.expect("open day has an action")));
    }
    if mp_action == Some(MpAction::AutoClose) {
        if let Some((_, end_at, _, _)) = working {
            for pr in merged.iter_mut().filter(|p| p.out.is_none() && p.in_at < end_at) {
                pr.out = Some((pr.in_p, end_at)); // closed at shift end; the in punch stands for both ends
                out.flags.push("auto_closed".into());
            }
        }
    }

    // Sessions and minutes. EA-01: work counts from shift start, or from the punch up to EA-02 minutes early.
    let count_from = working.map(|(start, ..)| if s.ea_from_punch { start - chrono::Duration::minutes(s.ea_cap_min) } else { start });
    for (i, pr) in merged.iter().enumerate() {
        let n = i as i64 + 1;
        out.punches_used.push(PunchUse { id: pr.in_p, direction: Some("in"), session_no: Some(n), excluded: None });
        for (id, dir) in &pr.inner {
            out.punches_used.push(PunchUse { id: *id, direction: Some(dir), session_no: Some(n), excluded: None });
        }
        let out_at = pr.out.map(|(id, at)| {
            if id != pr.in_p {
                out.punches_used.push(PunchUse { id, direction: Some("out"), session_no: Some(n), excluded: None });
            }
            at
        });
        let from = count_from.map_or(pr.in_at, |c| pr.in_at.max(c));
        let m = out_at.map_or(0, |o| minutes(from, o));
        out.sessions.push(Session { in_at: pr.in_at, out_at, minutes: m });
    }
    out.gross_min = out.sessions.iter().map(|x| x.minutes).sum();
    if s.break_deduct && out.gross_min > s.break_min {
        out.break_min = s.break_min;
    }
    out.net_min = out.gross_min - if s.break_paid { 0 } else { out.break_min };
    if out.break_min > 0 {
        step(&mut out.explanation, "BK-03", json!({ "gross": out.gross_min, "break": out.break_min, "paid": s.break_paid }), format!("net {} minutes", out.net_min));
    }

    let first_in = merged.first().map(|p| p.in_at);
    let last_closed_out = merged.last().and_then(|p| p.out.map(|o| o.1));

    match &input.expected {
        Expected::WeeklyOff | Expected::Holiday => {
            out.status = if input.expected == Expected::WeeklyOff { "weekly_off" } else { "holiday" };
            if !kept.is_empty() {
                out.flags.push("worked_on_off_day".into());
            }
            step(&mut out.explanation, "SH-09", json!({}), out.status.to_string());
            return out;
        }
        Expected::Unscheduled => {
            out.flags.push("no_shift".into());
            out.fraction_pct = if out.net_min > 0 { 100 } else { 0 };
            out.status = status_of(out.fraction_pct, false);
            out.state = if kept.is_empty() { "absence_pending" } else if open && !input.cutoff_passed { "exception" } else { "provisional" };
            step(&mut out.explanation, "DS-01", json!({ "net": out.net_min }), format!("no shift: {}", out.status));
            return out;
        }
        Expected::Working { .. } => {}
    }
    let (start_at, end_at, required, flexible) = working.expect("working day");
    if kept.is_empty() {
        out.status = "absent";
        out.state = "absence_pending";
        step(&mut out.explanation, "DS-10", json!({}), "absent; pending (leave or loss of pay decided later)".into());
        return out;
    }
    if let Some(fi) = first_in.filter(|fi| *fi < start_at) {
        out.early_arrival_min = if s.ea_from_punch { minutes(fi, start_at).min(s.ea_cap_min) } else { 0 };
    }
    let basis = s.basis.unwrap_or(if flexible { Basis::Hours } else { Basis::Shift });
    let slabs_apply = basis != Basis::Hours;

    // Late (LA-01..05, LA-23).
    let mut slab_effects: Vec<Effect> = Vec::new();
    if let (true, Some(fi)) = (s.late_on, first_in) {
        let raw = minutes(start_at, fi);
        if raw > s.late_grace {
            out.late_min = if s.late_from_grace_end { raw - s.late_grace } else { raw };
            let slab = slab_for(&s.late_slabs, out.late_min).map(|x| x.action).unwrap_or(SlabAction::None);
            if slabs_apply {
                apply_slab(slab, out.late_min, "late_slab", "LA-05", s.slab_cut_counts_as_mark, &mut out.late_mark, &mut slab_effects);
            }
            step(&mut out.explanation, "LA-05", json!({ "late_minutes": out.late_min, "grace": s.late_grace }), format!("{slab:?}"));
        }
    }
    // Early (EC-01..04, EC-15). BR-09: only from a real check-out.
    if open {
        step(&mut out.explanation, "BR-09", json!({}), "missing check-out is a missed punch, not an early check-out".into());
    } else if let (true, Some(lo)) = (s.early_on, last_closed_out) {
        let raw = minutes(lo, end_at);
        let hours_met = s.hours_met_override.unwrap_or(flexible) && out.net_min >= required;
        if raw > s.early_grace {
            out.early_min = raw;
            let slab = slab_for(&s.early_slabs, raw).map(|x| x.action).unwrap_or(SlabAction::None);
            if slabs_apply && !hours_met {
                apply_slab(slab, raw, "early_slab", "EC-04", s.slab_cut_counts_as_mark, &mut out.early_mark, &mut slab_effects);
            }
            step(&mut out.explanation, "EC-04", json!({ "early_minutes": raw, "grace": s.early_grace, "hours_met": hours_met }), format!("{slab:?}"));
        }
    }

    // Status (DS-01..07) and BR-04: one shortfall, one effect — the larger.
    let has_pair = out.sessions.iter().any(|x| x.out_at.is_some());
    let slab_days: i64 = slab_effects.iter().filter_map(|e| e.days_pct).sum();
    let shift_pct = if has_pair { (100 - slab_days).max(0) } else { 0 };
    let full = s.full_day_min.unwrap_or(if basis == Basis::Hybrid { required * 75 / 100 } else { required });
    let half = s.half_day_min.unwrap_or(required / 2);
    let shortfall = full - out.net_min;
    let (hours_pct, hours_effect) = if !has_pair {
        (0, None)
    } else if shortfall <= s.shortfall_tolerance_min {
        (100, None)
    } else if out.net_min >= half {
        if s.shortfall_deducts_minutes { (100, Some((None, Some(shortfall)))) } else { (50, Some((Some(50), None))) }
    } else if s.quarter_days && out.net_min >= half / 2 {
        (25, Some((Some(75), None)))
    } else if s.below_half_proportional {
        let pct = out.net_min * 100 / full.max(1);
        (pct, Some((Some(100 - pct), None)))
    } else {
        (0, Some((Some(100), None)))
    };
    let hours_eff = |(days, mins): (Option<i64>, Option<i64>)| Effect { kind: "hours_shortfall", days_pct: days, minutes: mins, reason_code: format!("short_{shortfall}_minutes"), rule_ref: "DS-05" };
    let (pct, effects) = match basis {
        Basis::Shift => (shift_pct, slab_effects),
        Basis::Hours => (hours_pct, hours_effect.map(hours_eff).into_iter().collect()),
        Basis::Hybrid => {
            if 100 - hours_pct > slab_days {
                (hours_pct, hours_effect.map(hours_eff).into_iter().collect())
            } else {
                (shift_pct, slab_effects)
            }
        }
    };
    out.fraction_pct = pct;
    out.effects = effects;
    step(&mut out.explanation, "DS-01", json!({ "basis": format!("{basis:?}"), "net": out.net_min, "full_day_min": full, "half_day_min": half }), format!("{pct}% present"));

    // MP half day / absent cut the day further.
    let mp_cap = match mp_action {
        Some(MpAction::HalfDay) => Some(50),
        Some(MpAction::Absent) => Some(0),
        _ => None,
    };
    if let Some(cap) = mp_cap.filter(|c| *c < out.fraction_pct) {
        out.effects.push(Effect { kind: "missed_punch", days_pct: Some(out.fraction_pct - cap), minutes: None, reason_code: "unresolved_missed_punch".into(), rule_ref: "MP-03" });
        out.fraction_pct = cap;
    }
    out.status = status_of(out.fraction_pct, s.quarter_days);
    out.state = if open && !input.cutoff_passed { "exception" } else { "provisional" };
    out
}

fn step(explanation: &mut Vec<Value>, rule: &str, inputs: Value, outcome: String) {
    let n = explanation.len() + 1;
    explanation.push(json!({ "step": n, "rule_ref": rule, "inputs": inputs, "outcome": outcome }));
}

fn apply_slab(action: SlabAction, m: i64, kind: &'static str, rule: &'static str, cut_counts_as_mark: bool, mark: &mut bool, effects: &mut Vec<Effect>) {
    match action {
        SlabAction::None => {}
        SlabAction::Mark => *mark = true,
        SlabAction::DeductMinutes => effects.push(Effect { kind, days_pct: None, minutes: Some(m), reason_code: format!("{m}_minutes"), rule_ref: rule }),
        cut => {
            *mark = cut_counts_as_mark;
            effects.push(Effect { kind, days_pct: Some(cut.days_pct()), minutes: None, reason_code: format!("{m}_minutes"), rule_ref: rule });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(hm: &str) -> DateTime<Utc> {
        format!("2026-03-02T{hm}:00Z").parse().unwrap()
    }

    fn shift() -> Expected {
        Expected::Working { start_at: t("09:00"), end_at: t("18:00"), required_min: 480, flexible: false }
    }

    fn day(expected: Expected, times: &[&str]) -> DayInput {
        DayInput {
            expected,
            punches: times.iter().enumerate().map(|(i, hm)| PunchIn { id: Uuid::from_u128(i as u128 + 1), at: t(hm), excluded: None }).collect(),
            cutoff_passed: false,
            settings: AttendanceSettings::default(),
        }
    }

    #[test]
    fn e1_within_grace() {
        let o = compute_day(&day(shift(), &["09:08", "18:00"]));
        assert_eq!((o.status, o.late_min, o.late_mark, o.effects.len()), ("present", 0, false, 0));
    }

    #[test]
    fn e2_late_mark() {
        let o = compute_day(&day(shift(), &["09:25", "18:00"]));
        assert_eq!((o.status, o.late_min, o.late_mark), ("present", 25, true));
        assert_eq!((o.gross_min, o.break_min, o.net_min), (515, 60, 455));
        assert!(o.effects.is_empty());
    }

    #[test]
    fn e6_very_late_half_day_no_mark() {
        let o = compute_day(&day(shift(), &["11:30", "18:00"]));
        assert_eq!((o.status, o.fraction_pct, o.late_min, o.late_mark), ("half_day", 50, 150, false));
        assert_eq!(o.effects, [Effect { kind: "late_slab", days_pct: Some(50), minutes: None, reason_code: "150_minutes".into(), rule_ref: "LA-05" }]);
    }

    #[test]
    fn e7_early_check_out() {
        let o = compute_day(&day(shift(), &["09:00", "17:20"]));
        assert_eq!((o.status, o.early_min, o.early_mark), ("present", 40, true));
    }

    #[test]
    fn e8_hours_based_half_day_or_minutes() {
        let flex = Expected::Working { start_at: t("09:00"), end_at: t("18:00"), required_min: 480, flexible: true };
        let o = compute_day(&day(flex.clone(), &["10:15", "18:30"]));
        assert_eq!((o.net_min, o.status, o.late_mark, o.early_mark), (435, "half_day", false, false));
        assert_eq!(o.effects[0].kind, "hours_shortfall");
        let mut i = day(flex, &["10:15", "18:30"]);
        i.settings.shortfall_deducts_minutes = true;
        let o = compute_day(&i);
        assert_eq!((o.status, o.effects[0].minutes), ("present", Some(45)));
    }

    #[test]
    fn e13_missed_check_out_before_and_after_cutoff() {
        let mut i = day(shift(), &["09:00"]);
        let o = compute_day(&i);
        assert_eq!((o.state, o.early_mark, o.early_min), ("exception", false, 0));
        assert!(o.flags.contains(&"missed_punch".to_string()));
        assert!(o.explanation.iter().any(|s| s["rule_ref"] == "BR-09"));
        i.cutoff_passed = true;
        let o = compute_day(&i);
        assert_eq!((o.state, o.status, o.fraction_pct), ("provisional", "absent", 0), "MP-03 pairs only: no pair -> absent");
        i.settings.mp_cutoff = MpAction::HalfDay;
        assert_eq!(compute_day(&i).status, "absent", "half day cannot raise a day with no pairs");
    }

    #[test]
    fn repeats_flagged_punches_and_short_gaps() {
        let mut i = day(shift(), &["09:00", "09:00", "13:00", "13:03", "18:00"]);
        i.punches[1].at += chrono::Duration::seconds(30);
        let o = compute_day(&i);
        assert_eq!(o.sessions.len(), 1, "{o:?}");
        assert_eq!((o.gross_min, o.status), (540, "present"));
        assert!(o.punches_used.iter().any(|u| u.excluded.as_deref() == Some("repeat_punch")));

        let mut i = day(shift(), &["09:00", "18:00"]);
        i.punches[0].excluded = Some("flag_outside_fence_not_decided".into());
        let o = compute_day(&i);
        assert_eq!((o.state, o.status), ("exception", "absent"), "the counted single punch is a missed punch");
    }

    #[test]
    fn no_punches_absent_pending_and_off_days() {
        let o = compute_day(&day(shift(), &[]));
        assert_eq!((o.status, o.state), ("absent", "absence_pending"));
        let o = compute_day(&day(Expected::WeeklyOff, &["10:00", "14:00"]));
        assert_eq!(o.status, "weekly_off");
        assert!(o.flags.contains(&"worked_on_off_day".to_string()));
        let o = compute_day(&day(Expected::Holiday, &[]));
        assert_eq!((o.status, o.state), ("holiday", "provisional"));
    }

    #[test]
    fn hybrid_takes_the_larger_deduction_once() {
        let mut i = day(shift(), &["11:30", "18:00"]);
        i.settings.basis = Some(Basis::Hybrid);
        let o = compute_day(&i);
        assert_eq!(o.effects.len(), 1, "BR-04: {o:?}");
        assert_eq!(o.fraction_pct, 50);
    }

    #[test]
    fn night_shift_is_one_day() {
        let night = Expected::Working { start_at: t("22:00"), end_at: t("22:00") + chrono::Duration::hours(8), required_min: 420, flexible: false };
        let mut i = day(night, &["22:05"]);
        i.punches.push(PunchIn { id: Uuid::from_u128(9), at: t("22:00") + chrono::Duration::hours(8), excluded: None });
        let o = compute_day(&i);
        assert_eq!((o.status, o.late_mark, o.sessions.len()), ("present", false, 1));
    }

    #[test]
    fn fingerprint_changes_with_inputs_only() {
        let a = day(shift(), &["09:00", "18:00"]);
        assert_eq!(fingerprint(&a), fingerprint(&a.clone()));
        let mut b = a.clone();
        b.cutoff_passed = true;
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }
}
