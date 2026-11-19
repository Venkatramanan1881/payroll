//! Typed comp-off settings (HW rule group, BRD HW-01..HW-06, M02) with the V1 defaults. Options whose behaviour is
//! not built yet are refused when the policy is saved (D-25..D-28), never stored and ignored.

use db::{invalid, DomainError, OrgTx};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct HwSettings {
    pub comp_off: bool,       // HW-01: comp_off (true) or ignore (false)
    pub approval: bool,       // HW-02
    pub half_minutes: i64,    // HW-04 half day
    pub full_minutes: i64,    // HW-04 full day
    pub expiry_days: i64,     // HW-05, calendar days from the worked date (D-27)
}

impl Default for HwSettings {
    fn default() -> Self {
        HwSettings { comp_off: true, approval: true, half_minutes: 240, full_minutes: 480, expiry_days: 60 }
    }
}

impl HwSettings {
    /// Days earned (in hundredths) for net minutes worked on an off day: 0, 50 or 100 (HW-04).
    pub fn earned_pct(&self, net_min: i64) -> i64 {
        if net_min >= self.full_minutes {
            100
        } else if net_min >= self.half_minutes {
            50
        } else {
            0
        }
    }

    pub fn parse(doc: &Value) -> Result<HwSettings, String> {
        let mut s = HwSettings::default();
        let obj = doc.as_object().ok_or("settings must be an object")?;
        let hours = |v: Option<&Value>, k: &str| -> Result<Option<i64>, String> {
            v.map(|v| v.as_f64().filter(|h| (0.5..=24.0).contains(h) && (h * 60.0).fract() == 0.0).map(|h| (h * 60.0) as i64).ok_or(format!("HW-04.{k} is hours, 0.5-24")))
                .transpose()
        };
        for (key, v) in obj {
            match key.as_str() {
                "HW-01" => {
                    s.comp_off = match v.as_str() {
                        Some("comp_off") => true,
                        Some("ignore") => false,
                        Some("overtime") => return Err("HW-01: overtime waits for the overtime module (V3)".into()),
                        Some("employee_chooses") => return Err("HW-01: employee_chooses is not available until OQ-M02-06/07 are decided (D-28)".into()),
                        _ => return Err("HW-01 is ignore, overtime, comp_off or employee_chooses".into()),
                    }
                }
                "HW-02" => s.approval = v.as_bool().ok_or("HW-02 must be true or false")?,
                "HW-03" if v == false => {}
                "HW-03" => return Err("HW-03: prior approval to work on an off day has no request type yet (OQ-M02-02)".into()),
                "HW-04" => {
                    let o = v.as_object().ok_or("HW-04 is {half_day_hours, full_day_hours}")?;
                    if let Some(k) = o.keys().find(|k| !matches!(k.as_str(), "half_day_hours" | "full_day_hours")) {
                        return Err(format!("HW-04.{k} is not a setting"));
                    }
                    s.half_minutes = hours(o.get("half_day_hours"), "half_day_hours")?.unwrap_or(s.half_minutes);
                    s.full_minutes = hours(o.get("full_day_hours"), "full_day_hours")?.unwrap_or(s.full_minutes);
                }
                "HW-05" => s.expiry_days = v.as_i64().filter(|d| (1..=3650).contains(d)).ok_or("HW-05 is days, 1-3650")?,
                "HW-06" if v == "lapse" => {}
                "HW-06" => return Err("HW-06: only lapse is available in V1; paying an extra day needs payroll (V2, D-26)".into()),
                _ => return Err(format!("{key} is not an HW setting")),
            }
        }
        if s.half_minutes >= s.full_minutes {
            return Err("HW-04: half_day_hours is below full_day_hours".into());
        }
        Ok(s)
    }
}

pub async fn load(tx: &mut OrgTx<'_>, version: Option<Uuid>) -> Result<HwSettings, DomainError> {
    let Some(version) = version else { return Ok(HwSettings::default()) };
    let row: Option<(bool, Value)> = sqlx::query_as("SELECT enabled, settings FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = 'HW'")
        .bind(version)
        .fetch_optional(&mut ***tx)
        .await?;
    match row {
        None => Ok(HwSettings::default()),
        Some((false, _)) => Ok(HwSettings { comp_off: false, ..HwSettings::default() }),
        Some((true, doc)) => HwSettings::parse(&doc).map_err(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_values_and_refusals() {
        let d = HwSettings::parse(&json!({})).unwrap();
        assert_eq!(d, HwSettings::default());
        assert_eq!((d.earned_pct(239), d.earned_pct(240), d.earned_pct(479), d.earned_pct(480)), (0, 50, 50, 100));
        let s = HwSettings::parse(&json!({ "HW-01": "ignore", "HW-02": false, "HW-04": { "half_day_hours": 3.5, "full_day_hours": 7 }, "HW-05": 30, "HW-06": "lapse" })).unwrap();
        assert_eq!(s, HwSettings { comp_off: false, approval: false, half_minutes: 210, full_minutes: 420, expiry_days: 30 });
        for bad in [json!({ "HW-01": "overtime" }), json!({ "HW-01": "employee_chooses" }), json!({ "HW-03": true }), json!({ "HW-06": "pay" }),
                    json!({ "HW-04": { "half_day_hours": 8, "full_day_hours": 4 } }), json!({ "HW-05": 0 }), json!({ "HW-99": 1 })] {
            assert!(HwSettings::parse(&bad).is_err(), "{bad}");
        }
    }
}
