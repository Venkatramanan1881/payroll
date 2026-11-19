//! Typed on-duty settings (OD rule group, BRD OD-01..OD-12) with the BRD defaults. Options whose behaviour is not
//! built yet are refused when the policy is saved, never stored and ignored.

use db::{invalid, DomainError, OrgTx};
use serde_json::Value;
use uuid::Uuid;

pub const KINDS: [&str; 4] = ["hours", "half_day", "full_day", "several_days"];

#[derive(Debug, Clone, PartialEq)]
pub struct OdSettings {
    pub enabled: bool,               // OD-01
    pub kinds_on: [bool; 4],         // OD-02, in KINDS order
    pub before_only: bool,           // OD-04
    pub after_days: i64,             // OD-04 N
    pub two_levels: bool,            // OD-05
    pub daily_allowance: bool,       // OD-09: the daily allowance module (V3) reads it
    pub comp_off_on_off_days: bool,  // OD-10: the comp-off module reads it
}

impl Default for OdSettings {
    fn default() -> Self {
        OdSettings { enabled: true, kinds_on: [true; 4], before_only: false, after_days: 2, two_levels: false, daily_allowance: true, comp_off_on_off_days: true }
    }
}

impl OdSettings {
    pub fn kind_on(&self, kind: &str) -> bool {
        KINDS.iter().position(|k| *k == kind).is_some_and(|i| self.kinds_on[i])
    }

    pub fn parse(doc: &Value) -> Result<OdSettings, String> {
        let mut s = OdSettings::default();
        let obj = doc.as_object().ok_or("settings must be an object")?;
        let boolean = |v: &Value, k: &str| v.as_bool().ok_or(format!("{k} must be true or false"));
        for (key, v) in obj {
            match key.as_str() {
                "OD-01" => s.enabled = boolean(v, key)?,
                "OD-02" => {
                    let o = v.as_object().ok_or("OD-02 is an object of duration kinds")?;
                    for (k, on) in o {
                        let i = KINDS.iter().position(|x| x == k).ok_or(format!("OD-02.{k} is not a duration kind"))?;
                        s.kinds_on[i] = boolean(on, &format!("OD-02.{k}"))?;
                    }
                }
                "OD-03" if v == "none" => {}
                "OD-03" => return Err("OD-03: only \"none\" is available yet (the limit outcome is OQ-C09-12)".into()),
                "OD-04" => {
                    let o = v.as_object().ok_or("OD-04 is {mode, days}")?;
                    match o.get("mode").and_then(Value::as_str) {
                        Some("before_only") => s.before_only = true,
                        Some("before_or_after") => s.before_only = false,
                        _ => return Err("OD-04.mode is before_only or before_or_after".into()),
                    }
                    if let Some(d) = o.get("days") {
                        s.after_days = d.as_i64().filter(|n| (0..=365).contains(n)).ok_or("OD-04.days is 0-365")?;
                    }
                }
                "OD-05" => {
                    let o = v.as_object().ok_or("OD-05 is {levels, timeout_action}")?;
                    s.two_levels = match o.get("levels").and_then(Value::as_str) {
                        Some("manager") | None => false,
                        Some("manager_then_hr") => true,
                        _ => return Err("OD-05.levels is manager or manager_then_hr".into()),
                    };
                    if o.get("timeout_action").is_some_and(|a| a != "escalate") {
                        return Err("OD-05.timeout_action: only escalate is available yet (no on-duty timer runs)".into());
                    }
                }
                "OD-06" if v.get("proof").is_none_or(|p| p == "optional") => {}
                "OD-06" => return Err("OD-06: mandatory proof needs file attachments, which are not available yet".into()),
                "OD-07" if v == "not_required" => {}
                "OD-07" => return Err("OD-07: only not_required is available yet (punch rules during on-duty)".into()),
                "OD-08" if v == false => {}
                "OD-08" => return Err("OD-08: on-duty toward overtime waits for the overtime module (V3)".into()),
                "OD-09" => s.daily_allowance = boolean(v, key)?,
                "OD-10" => s.comp_off_on_off_days = boolean(v, key)?,
                "OD-11" if v.as_object().is_some_and(|o| o.is_empty()) => {}
                "OD-11" => return Err("OD-11: eligibility rules are not enforced yet; send {}".into()),
                "OD-12" => return Err("OD-12 is fixed (on-duty never uses the permission limit)".into()),
                _ => return Err(format!("{key} is not an OD setting")),
            }
        }
        Ok(s)
    }
}

pub async fn load(tx: &mut OrgTx<'_>, version: Option<Uuid>) -> Result<OdSettings, DomainError> {
    let Some(version) = version else { return Ok(OdSettings::default()) };
    let row: Option<(bool, Value)> = sqlx::query_as("SELECT enabled, settings FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = 'OD'")
        .bind(version)
        .fetch_optional(&mut ***tx)
        .await?;
    match row {
        None => Ok(OdSettings::default()),
        Some((false, _)) => Ok(OdSettings { enabled: false, ..OdSettings::default() }),
        Some((true, doc)) => OdSettings::parse(&doc).map_err(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_values_and_refusals() {
        assert_eq!(OdSettings::parse(&json!({})).unwrap(), OdSettings::default());
        let s = OdSettings::parse(&json!({ "OD-02": { "several_days": false }, "OD-04": { "mode": "before_only" }, "OD-05": { "levels": "manager_then_hr" } })).unwrap();
        assert!(!s.kind_on("several_days") && s.kind_on("hours") && s.before_only && s.two_levels);
        for bad in [json!({ "OD-03": "count" }), json!({ "OD-06": { "proof": "mandatory" } }), json!({ "OD-07": "app_checkin" }), json!({ "OD-08": true }),
                    json!({ "OD-05": { "timeout_action": "auto_approve" } }), json!({ "OD-11": { "category": ["a"] } }), json!({ "OD-99": 1 })] {
            assert!(OdSettings::parse(&bad).is_err(), "{bad}");
        }
    }
}
