//! Typed attendance settings (rule groups PR, BK, LA, EC, EA, MP, DS) with the BRD defaults.
//! Keys this version does not evaluate are accepted and reported back as warnings.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SlabAction {
    None,
    Mark,
    DeductMinutes,
    QuarterDay,
    HalfDay,
    FullDay,
}

impl SlabAction {
    /// Day deduction in hundredths of a day.
    pub fn days_pct(self) -> i64 {
        match self {
            SlabAction::QuarterDay => 25,
            SlabAction::HalfDay => 50,
            SlabAction::FullDay => 100,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Slab {
    pub from: i64,
    /// Inclusive; None = no upper limit.
    pub to: Option<i64>,
    pub action: SlabAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MpAction {
    Flag,
    AutoClose,
    PairsOnly,
    HalfDay,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Basis {
    Shift,
    Hours,
    Hybrid,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AttendanceSettings {
    pub pr_first_in_last_out: bool, // PR-01 (direction_tagged reads as paired until sources send tags)
    pub repeat_window_s: i64,       // PR-02
    pub keep_last_repeat: bool,     // PR-03
    pub min_gap_min: i64,           // PR-04
    pub max_session_min: i64,       // PR-05 (set in hours)
    pub max_punches: i64,           // PR-06
    pub break_deduct: bool,         // BK-01 none vs fixed auto-deduct
    pub break_paid: bool,           // BK-02
    pub break_min: i64,             // BK-03
    pub late_on: bool,              // LA-01
    pub late_from_grace_end: bool,  // LA-02
    pub late_grace: i64,            // LA-03
    pub late_slabs: Vec<Slab>,      // LA-05
    pub slab_cut_counts_as_mark: bool, // LA-23
    pub early_on: bool,             // EC-01
    pub early_grace: i64,           // EC-02
    pub early_slabs: Vec<Slab>,     // EC-04
    pub hours_met_override: Option<bool>, // EC-15; None = by shift type
    pub ea_from_punch: bool,        // EA-01
    pub ea_cap_min: i64,            // EA-02
    pub mp_odd: MpAction,           // MP-01
    pub mp_single: MpAction,        // MP-02
    pub mp_cutoff: MpAction,        // MP-03
    pub basis: Option<Basis>,       // DS-01; None = by shift type
    pub full_day_min: Option<i64>,  // DS-02 (minutes); None = by basis
    pub half_day_min: Option<i64>,  // DS-03 (minutes); None = 50% of required
    pub shortfall_tolerance_min: i64, // DS-04
    pub shortfall_deducts_minutes: bool, // DS-05 deduct actual minutes (else half day)
    pub quarter_days: bool,         // DS-06
    pub below_half_proportional: bool, // DS-07
}

impl Default for AttendanceSettings {
    fn default() -> Self {
        AttendanceSettings {
            pr_first_in_last_out: false,
            repeat_window_s: 60,
            keep_last_repeat: false,
            min_gap_min: 5,
            max_session_min: 16 * 60,
            max_punches: 12,
            break_deduct: true,
            break_paid: false,
            break_min: 60,
            late_on: true,
            late_from_grace_end: false,
            late_grace: 10,
            late_slabs: vec![Slab { from: 11, to: Some(120), action: SlabAction::Mark }, Slab { from: 121, to: None, action: SlabAction::HalfDay }],
            slab_cut_counts_as_mark: false,
            early_on: true,
            early_grace: 5,
            early_slabs: vec![Slab { from: 6, to: Some(120), action: SlabAction::Mark }, Slab { from: 121, to: None, action: SlabAction::HalfDay }],
            hours_met_override: None,
            ea_from_punch: false,
            ea_cap_min: 60,
            mp_odd: MpAction::Flag,
            mp_single: MpAction::Flag,
            mp_cutoff: MpAction::PairsOnly,
            basis: None,
            full_day_min: None,
            half_day_min: None,
            shortfall_tolerance_min: 15,
            shortfall_deducts_minutes: false,
            quarter_days: false,
            below_half_proportional: false,
        }
    }
}

/// The rule groups this module types.
pub const GROUPS: [&str; 7] = ["PR", "BK", "LA", "EC", "EA", "MP", "DS"];

fn int(v: &Value, key: &str, min: i64, max: i64) -> Result<i64, String> {
    v.as_i64().filter(|n| (min..=max).contains(n)).ok_or(format!("{key} must be a whole number from {min} to {max}"))
}

fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v.as_bool().ok_or(format!("{key} must be true or false"))
}

fn pick<'a>(v: &'a Value, key: &str, allowed: &[&str]) -> Result<&'a str, String> {
    v.as_str().filter(|s| allowed.contains(s)).ok_or(format!("{key} must be one of {}", allowed.join(", ")))
}

fn slabs(v: &Value, key: &str, mark: &str, warnings: &mut Vec<String>) -> Result<Vec<Slab>, String> {
    let list = v.as_array().filter(|a| a.len() <= 5).ok_or(format!("{key} must be a list of up to 5 slabs"))?;
    let mut out: Vec<Slab> = Vec::new();
    for (i, s) in list.iter().enumerate() {
        let at = format!("{key}[{i}]");
        let o = s.as_object().ok_or(format!("{at} must be an object"))?;
        if let Some(k) = o.keys().find(|k| !["from", "to", "action"].contains(&k.as_str())) {
            return Err(format!("{at}.{k} is not a known field"));
        }
        let from = int(o.get("from").unwrap_or(&Value::Null), &format!("{at}.from"), 0, 1440)?;
        let to = match o.get("to") {
            None | Some(Value::Null) => None,
            Some(t) => Some(int(t, &format!("{at}.to"), from, 1440)?),
        };
        let mut allowed = vec!["no_action", mark, "deduct_actual_minutes", "quarter_day", "half_day", "full_day"];
        if key == "LA-05" {
            allowed.push("hold_check_in");
        }
        let action = match pick(o.get("action").unwrap_or(&Value::Null), &format!("{at}.action"), &allowed)? {
            "no_action" => SlabAction::None,
            "deduct_actual_minutes" => SlabAction::DeductMinutes,
            "quarter_day" => SlabAction::QuarterDay,
            "half_day" => SlabAction::HalfDay,
            "full_day" => SlabAction::FullDay,
            "hold_check_in" => {
                warnings.push(format!("{at}: hold_check_in is evaluated as a late mark until held check-ins are built"));
                SlabAction::Mark
            }
            _ => SlabAction::Mark,
        };
        if out.last().is_some_and(|p| p.to.is_none_or(|t| from <= t)) {
            return Err(format!("{at} overlaps the slab before it; list slabs in ascending, non-overlapping order"));
        }
        out.push(Slab { from, to, action });
    }
    Ok(out)
}

fn mp(v: &Value, key: &str, cutoff: bool) -> Result<MpAction, String> {
    let allowed: &[&str] = if cutoff { &["pairs_only", "half_day", "absent"] } else { &["flag", "auto_close", "pairs_only", "half_day", "absent"] };
    Ok(match pick(v, key, allowed)? {
        "flag" => MpAction::Flag,
        "auto_close" => MpAction::AutoClose,
        "pairs_only" => MpAction::PairsOnly,
        "half_day" => MpAction::HalfDay,
        _ => MpAction::Absent,
    })
}

impl AttendanceSettings {
    /// Applies one rule group's document. Returns warnings for keys stored but not evaluated yet.
    /// A switched-off group is simply not applied (defaults stay).
    pub fn apply(&mut self, group: &str, doc: &Value) -> Result<Vec<String>, String> {
        let obj = doc.as_object().ok_or("settings must be an object")?;
        let mut w = Vec::new();
        for (key, v) in obj {
            match (group, key.as_str()) {
                ("PR", "PR-01") => {
                    let m = pick(v, key, &["paired", "first_in_last_out", "direction_tagged"])?;
                    if m == "direction_tagged" {
                        w.push("PR-01: direction_tagged reads punches as paired until a source sends directions".into());
                    }
                    self.pr_first_in_last_out = m == "first_in_last_out";
                }
                ("PR", "PR-02") => self.repeat_window_s = int(v, key, 0, 3600)?,
                ("PR", "PR-03") => self.keep_last_repeat = pick(v, key, &["first", "last"])? == "last",
                ("PR", "PR-04") => self.min_gap_min = int(v, key, 0, 240)?,
                ("PR", "PR-05") => self.max_session_min = int(v, key, 1, 48)? * 60,
                ("PR", "PR-06") => self.max_punches = int(v, key, 2, 100)?,
                ("BK", "BK-01") => {
                    let k = pick(v, key, &["none", "fixed_auto_deduct", "window", "flexible", "flexible_with_cap"])?;
                    if !["none", "fixed_auto_deduct"].contains(&k) {
                        w.push(format!("BK-01: {k} is evaluated as fixed_auto_deduct until break punches are built"));
                    }
                    self.break_deduct = k != "none";
                }
                ("BK", "BK-02") => self.break_paid = pick(v, key, &["paid", "unpaid"])? == "paid",
                ("BK", "BK-03") => self.break_min = int(v, key, 0, 480)?,
                ("LA", "LA-01") => self.late_on = boolean(v, key)?,
                ("LA", "LA-02") => self.late_from_grace_end = pick(v, key, &["shift_start", "end_of_grace"])? == "end_of_grace",
                ("LA", "LA-03") => self.late_grace = int(v, key, 0, 120)?,
                ("LA", "LA-05") => self.late_slabs = slabs(v, key, "late_mark", &mut w)?,
                ("LA", "LA-23") => self.slab_cut_counts_as_mark = boolean(v, key)?,
                ("EC", "EC-01") => self.early_on = boolean(v, key)?,
                ("EC", "EC-02") => self.early_grace = int(v, key, 0, 120)?,
                ("EC", "EC-04") => self.early_slabs = slabs(v, key, "early_mark", &mut w)?,
                ("EC", "EC-15") => self.hours_met_override = Some(boolean(v, key)?),
                ("EA", "EA-01") => {
                    let k = pick(v, key, &["shift_start", "punch_time", "punch_time_when_approved"])?;
                    if k == "punch_time_when_approved" {
                        w.push("EA-01: punch_time_when_approved is evaluated as shift_start until approvals are built".into());
                    }
                    self.ea_from_punch = k == "punch_time";
                }
                ("EA", "EA-02") => self.ea_cap_min = int(v, key, 0, 600)?,
                ("MP", "MP-01") => self.mp_odd = mp(v, key, false)?,
                ("MP", "MP-02") => self.mp_single = mp(v, key, false)?,
                ("MP", "MP-03") => self.mp_cutoff = mp(v, key, true)?,
                ("DS", "DS-01") => {
                    self.basis = Some(match pick(v, key, &["shift", "hours", "hybrid"])? {
                        "shift" => Basis::Shift,
                        "hours" => Basis::Hours,
                        _ => Basis::Hybrid,
                    })
                }
                ("DS", "DS-02") => self.full_day_min = Some(int(v, key, 1, 1440)?),
                ("DS", "DS-03") => self.half_day_min = Some(int(v, key, 1, 1440)?),
                ("DS", "DS-04") => self.shortfall_tolerance_min = int(v, key, 0, 240)?,
                ("DS", "DS-05") => {
                    let k = pick(v, key, &["half_day", "deduct_actual_minutes", "make_up_within_week"])?;
                    if k == "make_up_within_week" {
                        w.push("DS-05: make_up_within_week is evaluated as half_day until weekly make-up is built".into());
                    }
                    self.shortfall_deducts_minutes = k == "deduct_actual_minutes";
                }
                ("DS", "DS-06") => self.quarter_days = boolean(v, key)?,
                ("DS", "DS-07") => self.below_half_proportional = pick(v, key, &["absent", "proportional"])? == "proportional",
                _ => w.push(format!("{key} is stored but not evaluated yet")),
            }
        }
        Ok(w)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_match_brd() {
        let s = AttendanceSettings::default();
        assert_eq!((s.late_grace, s.early_grace, s.break_min, s.shortfall_tolerance_min), (10, 5, 60, 15));
        assert_eq!(s.late_slabs[1], Slab { from: 121, to: None, action: SlabAction::HalfDay });
        assert_eq!((s.mp_odd, s.mp_cutoff), (MpAction::Flag, MpAction::PairsOnly));
    }

    #[test]
    fn typed_values_apply_and_unknown_keys_warn() {
        let mut s = AttendanceSettings::default();
        let w = s.apply("LA", &json!({ "LA-03": 15, "LA-05": [{ "from": 16, "to": 60, "action": "late_mark" }, { "from": 61, "action": "full_day" }], "LA-06": true })).unwrap();
        assert_eq!(s.late_grace, 15);
        assert_eq!(s.late_slabs[1].action, SlabAction::FullDay);
        assert_eq!(w, ["LA-06 is stored but not evaluated yet"]);
    }

    #[test]
    fn wrong_values_refused() {
        for (g, bad) in [
            ("LA", json!({ "LA-03": 121 })),
            ("LA", json!({ "LA-05": [{ "from": 11, "to": 120, "action": "early_mark" }] })),
            ("LA", json!({ "LA-05": [{ "from": 11, "action": "late_mark" }, { "from": 50, "action": "half_day" }] })),
            ("EC", json!({ "EC-04": [{ "from": 6, "action": "hold_check_in" }] })),
            ("MP", json!({ "MP-03": "flag" })),
            ("DS", json!({ "DS-01": "shift_based" })),
            ("PR", json!({ "PR-05": 0 })),
        ] {
            assert!(AttendanceSettings::default().apply(g, &bad).is_err(), "{g} {bad}");
        }
    }
}
