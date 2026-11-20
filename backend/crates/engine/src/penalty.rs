//! Period-end penalties (BR-10): count-based (LA-06..LA-09, EC-05..EC-08) and minutes-based (LA-10/LA-11, EC-09),
//! with the recovery order (LA-12, EC-11). Pure: the caller supplies the period's counters.

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct PenaltyRule {
    pub count_on: bool,
    pub free: i64,
    pub every: i64,
    pub size_pct: i64,
    pub minutes_on: bool,
    pub threshold: i64,
    /// None = deduct the actual excess minutes; Some((block minutes, size in hundredths)) per block of excess.
    pub per_block: Option<(i64, i64)>,
    /// LA-12 / EC-11: leave type codes in order, then loss of pay (true) or report only (false).
    pub recovery: Vec<String>,
    pub then_lop: bool,
}

impl Default for PenaltyRule {
    fn default() -> Self {
        PenaltyRule { count_on: false, free: 0, every: 3, size_pct: 50, minutes_on: false, threshold: 60, per_block: None, recovery: vec![], then_lop: true }
    }
}

/// The LA / EC keys this module owns (a stored document refused here never costs the day settings).
pub const KEYS: [&str; 16] = ["LA-06", "LA-07", "LA-08", "LA-09", "LA-10", "LA-11", "LA-12", "LA-18", "EC-05", "EC-06", "EC-07", "EC-08", "EC-09", "EC-10", "EC-11", "EC-17"];

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Penalties {
    pub late: PenaltyRule,
    pub early: PenaltyRule,
}

fn int(v: &Value, key: &str, min: i64, max: i64) -> Result<i64, String> {
    v.as_i64().filter(|n| (min..=max).contains(n)).ok_or(format!("{key} must be a whole number from {min} to {max}"))
}

fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v.as_bool().ok_or(format!("{key} must be true or false"))
}

fn size(v: &Value, key: &str) -> Result<i64, String> {
    match v.as_str() {
        Some("quarter_day") => Ok(25),
        Some("half_day") => Ok(50),
        Some("full_day") => Ok(100),
        Some("fixed_amount") => Err(format!("{key}: fixed_amount is money, which waits for payroll (V2, OQ-C06-18)")),
        _ => Err(format!("{key} must be quarter_day, half_day or full_day")),
    }
}

fn fields(o: &serde_json::Map<String, Value>, key: &str, known: &[&str]) -> Result<(), String> {
    match o.keys().find(|k| !known.contains(&k.as_str())) {
        Some(k) => Err(format!("{key}.{k} is not a known field")),
        None => Ok(()),
    }
}

impl PenaltyRule {
    /// LA-11 / EC-09: {threshold_minutes, mode: actual_excess | per_block, block_minutes, size}.
    fn minutes_rule(&mut self, v: &Value, key: &str) -> Result<(), String> {
        let o = v.as_object().ok_or(format!("{key} must be an object"))?;
        fields(o, key, &["threshold_minutes", "mode", "block_minutes", "size"])?;
        if let Some(t) = o.get("threshold_minutes") {
            self.threshold = int(t, &format!("{key}.threshold_minutes"), 0, 100_000)?;
        }
        self.per_block = match o.get("mode").and_then(Value::as_str).unwrap_or("actual_excess") {
            "actual_excess" => None,
            "per_block" => {
                let b = int(o.get("block_minutes").unwrap_or(&Value::Null), &format!("{key}.block_minutes"), 1, 10_000)?;
                let s = size(o.get("size").unwrap_or(&json!("half_day")), &format!("{key}.size"))?;
                if s == 100 {
                    return Err(format!("{key}.size is quarter_day or half_day"));
                }
                Some((b, s))
            }
            _ => return Err(format!("{key}.mode must be actual_excess or per_block")),
        };
        Ok(())
    }

    /// LA-12 / EC-11: {leave_types: [codes], then: loss_of_pay | none} (D-29).
    fn recovery_rule(&mut self, v: &Value, key: &str) -> Result<(), String> {
        let o = v.as_object().ok_or(format!("{key} must be an object {{leave_types, then}}"))?;
        fields(o, key, &["leave_types", "then"])?;
        if let Some(list) = o.get("leave_types") {
            let list = list.as_array().filter(|a| a.len() <= 10).ok_or(format!("{key}.leave_types must be a list of up to 10 leave type codes"))?;
            self.recovery = list.iter().map(|x| x.as_str().map(str::to_string).ok_or(format!("{key}.leave_types: {x} is not a code"))).collect::<Result<_, _>>()?;
        }
        self.then_lop = match o.get("then").and_then(Value::as_str).unwrap_or("loss_of_pay") {
            "loss_of_pay" => true,
            "none" => false,
            "permission_balance" | "fixed_fine" => return Err(format!("{key}.then: permission balance and fixed fines are not available yet")),
            _ => return Err(format!("{key}.then must be loss_of_pay or none")),
        };
        Ok(())
    }

    /// A penalty for the period's marks and minutes: the larger of the count and minutes penalties (BR-04, D-30),
    /// as (rule, days in hundredths, minutes, explanation); None when neither applies. `day_min` converts minutes.
    pub fn apply(&self, count_ref: &'static str, minutes_ref: &'static str, marks: i64, minutes: i64, day_min: i64) -> Option<(&'static str, i64, Option<i64>, Value)> {
        let counted = if self.count_on && marks > self.free { (marks - self.free) / self.every.max(1) } else { 0 };
        let count_days = counted * self.size_pct;
        let excess = if self.minutes_on { (minutes - self.threshold).max(0) } else { 0 };
        // shortcut: excess minutes convert to days rounded down and part blocks count as nothing, until OQ-C06-25 says otherwise.
        let (minute_days, minute_mins) = match self.per_block {
            _ if excess == 0 => (0, None),
            None => (excess * 100 / day_min.max(1), Some(excess)),
            Some((block, s)) => ((excess / block) * s, None),
        };
        let why = json!({ "marks": marks, "free": self.free, "every": self.every, "penalties_counted": counted, "count_days_pct": count_days,
                          "minutes": minutes, "threshold": self.threshold, "excess_minutes": excess, "minutes_days_pct": minute_days, "day_minutes": day_min });
        match (count_days, minute_days) {
            (0, 0) => None,
            (c, m) if c >= m => Some((count_ref, c, None, why)),
            (_, m) => Some((minutes_ref, m, minute_mins, why)),
        }
    }
}

impl Penalties {
    /// Applies one LA or EC key it owns; Ok(false) when the key is not a penalty key.
    pub fn apply_key(&mut self, group: &str, key: &str, v: &Value, warnings: &mut Vec<String>) -> Result<bool, String> {
        match (group, key) {
            ("LA", "LA-06") => self.late.count_on = boolean(v, key)?,
            ("LA", "LA-07") => self.late.free = int(v, key, 0, 100)?,
            ("LA", "LA-08") => self.late.every = int(v, key, 1, 100)?,
            ("LA", "LA-09") => self.late.size_pct = size(v, key)?,
            ("LA", "LA-10") => self.late.minutes_on = boolean(v, key)?,
            ("LA", "LA-11") => self.late.minutes_rule(v, key)?,
            ("LA", "LA-12") => self.late.recovery_rule(v, key)?,
            ("LA", "LA-18") | ("EC", "EC-17") if v.as_str().is_some() || v.get("reset").is_some() => {
                let reset = v.as_str().or_else(|| v.get("reset").and_then(Value::as_str));
                if reset != Some("payroll_period") {
                    return Err(format!("{key}: counters reset every payroll period only (D-32)"));
                }
                if let Some(o) = v.as_object() {
                    if o.keys().any(|k| k != "reset") {
                        warnings.push(format!("{key}: only reset is evaluated yet"));
                    }
                }
            }
            ("LA", "LA-18") => return Err("LA-18 must be payroll_period".into()),
            ("EC", "EC-17") => warnings.push("EC-17: only reset is evaluated yet".into()),
            ("EC", "EC-05") => self.early.count_on = boolean(v, key)?,
            ("EC", "EC-06") => self.early.free = int(v, key, 0, 100)?,
            ("EC", "EC-07") => self.early.every = int(v, key, 1, 100)?,
            ("EC", "EC-08") => self.early.size_pct = size(v, key)?,
            ("EC", "EC-09") => match v {
                Value::Bool(b) => self.early.minutes_on = *b,
                _ => {
                    self.early.minutes_rule(v, key)?;
                    self.early.minutes_on = true;
                }
            },
            ("EC", "EC-10") if v == "separate" => {}
            ("EC", "EC-10") => return Err("EC-10: only separate counters are available yet".into()),
            ("EC", "EC-11") => self.early.recovery_rule(v, key)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_minutes_and_the_larger() {
        let mut p = Penalties::default();
        let mut w = vec![];
        p.apply_key("LA", "LA-06", &json!(true), &mut w).unwrap();
        // E5: every 3rd mark = half day; 4 marks -> one half day.
        assert_eq!(p.late.apply("LA-06", "LA-10", 4, 0, 480).map(|x| (x.0, x.1)), Some(("LA-06", 50)));
        assert_eq!(p.late.apply("LA-06", "LA-10", 2, 0, 480), None);
        p.apply_key("LA", "LA-07", &json!(1), &mut w).unwrap();
        assert_eq!(p.late.apply("LA-06", "LA-10", 4, 0, 480).map(|x| x.1), Some(50), "1 free, then every 3rd: (4-1)/3 = 1");
        p.apply_key("LA", "LA-10", &json!(true), &mut w).unwrap();
        // 300 late minutes, 60 free: 240 excess = 0.50 of a 480-minute day; equal to the count penalty -> count wins.
        assert_eq!(p.late.apply("LA-06", "LA-10", 4, 300, 480).map(|x| (x.0, x.1)), Some(("LA-06", 50)));
        assert_eq!(p.late.apply("LA-06", "LA-10", 4, 540, 480).map(|x| (x.0, x.1, x.2)), Some(("LA-10", 100, Some(480))), "the larger applies once (BR-04)");
        p.apply_key("LA", "LA-11", &json!({ "threshold_minutes": 30, "mode": "per_block", "block_minutes": 60, "size": "quarter_day" }), &mut w).unwrap();
        assert_eq!(p.late.apply("LA-06", "LA-10", 0, 150, 480).map(|x| x.1), Some(50), "(150 - 30) / 60 = 2 blocks of a quarter day");
    }

    #[test]
    fn refusals() {
        let mut p = Penalties::default();
        let mut w = vec![];
        for (g, k, v) in [("LA", "LA-09", json!("fixed_amount")), ("LA", "LA-12", json!({ "then": "fixed_fine" })), ("LA", "LA-18", json!("quarter")),
                          ("EC", "EC-10", json!("combined")), ("EC", "EC-17", json!({ "reset": "year" })), ("LA", "LA-11", json!({ "mode": "per_block" }))] {
            assert!(p.apply_key(g, k, &v, &mut w).is_err(), "{k} {v}");
        }
        assert!(!p.apply_key("LA", "LA-19", &json!(true), &mut w).unwrap(), "not a penalty key");
    }
}
