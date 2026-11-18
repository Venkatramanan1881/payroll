//! Typed regularisation settings (RG rule group, BRD §9) with the BRD defaults.

use db::{invalid, DomainError, OrgTx};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

pub const TYPES: [&str; 5] = ["missed_punch", "late_arrival", "early_checkout", "wrong_punch", "whole_day"];

#[derive(Debug, Clone, PartialEq)]
pub struct RgSettings {
    pub enabled: bool,                   // RG-01
    pub types_on: BTreeMap<String, bool>, // RG-02
    pub limit_overall: i64,              // RG-03
    pub limit_per_type: BTreeMap<String, i64>,
    pub window_days: i64,                // RG-04
    pub two_levels: bool,                // RG-05
    pub auto_types: Vec<String>,         // RG-06
    pub over_limit_blocks: bool,         // RG-08
    pub attachment_types: Vec<String>,   // RG-09
    pub hr_direct: bool,                 // RG-10
    pub bulk: bool,                      // RG-11
}

impl Default for RgSettings {
    fn default() -> Self {
        RgSettings {
            enabled: true,
            types_on: TYPES.iter().map(|t| (t.to_string(), true)).collect(),
            limit_overall: 3,
            limit_per_type: BTreeMap::new(),
            window_days: 7,
            two_levels: false,
            auto_types: vec![],
            over_limit_blocks: false,
            attachment_types: vec![],
            hr_direct: true,
            bulk: true,
        }
    }
}

fn boolean(v: &Value, key: &str) -> Result<bool, String> {
    v.as_bool().ok_or(format!("{key} must be true or false"))
}

fn count(v: &Value, key: &str) -> Result<i64, String> {
    v.as_i64().filter(|n| (0..=1000).contains(n)).ok_or(format!("{key} must be a whole number from 0 to 1000"))
}

fn type_list(v: &Value, key: &str) -> Result<Vec<String>, String> {
    let list = v.as_array().ok_or(format!("{key} must be a list of request types"))?;
    list.iter()
        .map(|t| t.as_str().filter(|s| TYPES.contains(s)).map(str::to_string).ok_or(format!("{key}: unknown request type {t}")))
        .collect()
}

impl RgSettings {
    pub fn parse(doc: &Value) -> Result<RgSettings, String> {
        let obj = doc.as_object().ok_or("settings must be an object")?;
        let mut s = RgSettings::default();
        for (key, v) in obj {
            match key.as_str() {
                "RG-01" => s.enabled = boolean(v, key)?,
                "RG-02" => {
                    let o = v.as_object().ok_or("RG-02 must be an object of request types")?;
                    for (t, on) in o {
                        if !TYPES.contains(&t.as_str()) {
                            return Err(format!("RG-02.{t} is not a request type"));
                        }
                        s.types_on.insert(t.clone(), boolean(on, &format!("RG-02.{t}"))?);
                    }
                }
                "RG-03" => match v {
                    Value::Object(o) => {
                        if let Some(k) = o.keys().find(|k| !["overall", "per_type"].contains(&k.as_str())) {
                            return Err(format!("RG-03.{k} is not a known field"));
                        }
                        if let Some(n) = o.get("overall") {
                            s.limit_overall = count(n, "RG-03.overall")?;
                        }
                        if let Some(p) = o.get("per_type") {
                            for (t, n) in p.as_object().ok_or("RG-03.per_type must be an object")? {
                                if !TYPES.contains(&t.as_str()) {
                                    return Err(format!("RG-03.per_type.{t} is not a request type"));
                                }
                                s.limit_per_type.insert(t.clone(), count(n, &format!("RG-03.per_type.{t}"))?);
                            }
                        }
                    }
                    _ => s.limit_overall = count(v, key)?,
                },
                "RG-04" => s.window_days = count(v, key)?,
                "RG-05" => s.two_levels = v.as_str().filter(|x| ["one", "two"].contains(x)).ok_or("RG-05 must be one or two")? == "two",
                "RG-06" => s.auto_types = type_list(v, key)?,
                "RG-07" => {
                    // Stored for the timeout job (J3), which arrives with deployment; shape checked now.
                    let o = v.as_object().ok_or("RG-07 must be an object")?;
                    count(o.get("days").unwrap_or(&Value::Null), "RG-07.days")?;
                    o.get("action")
                        .and_then(Value::as_str)
                        .filter(|a| ["escalate", "auto_approve", "auto_reject"].contains(a))
                        .ok_or("RG-07.action must be escalate, auto_approve or auto_reject")?;
                }
                "RG-08" => s.over_limit_blocks = v.as_str().filter(|x| ["block", "hr_approval"].contains(x)).ok_or("RG-08 must be block or hr_approval")? == "block",
                "RG-09" => s.attachment_types = type_list(v, key)?,
                "RG-10" => s.hr_direct = boolean(v, key)?,
                "RG-11" => s.bulk = boolean(v, key)?,
                other => return Err(format!("{other} is not an RG setting")),
            }
        }
        Ok(s)
    }
}

/// RG settings for a policy version: a switched-off group means regularisation is off (G2).
pub async fn load(tx: &mut OrgTx<'_>, version: Option<Uuid>) -> Result<RgSettings, DomainError> {
    let Some(version) = version else { return Ok(RgSettings::default()) };
    let row: Option<(bool, Value)> = sqlx::query_as("SELECT enabled, settings FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = 'RG'")
        .bind(version)
        .fetch_optional(&mut ***tx)
        .await?;
    match row {
        None => Ok(RgSettings::default()),
        Some((false, _)) => Ok(RgSettings { enabled: false, hr_direct: false, bulk: false, ..RgSettings::default() }),
        Some((true, doc)) => RgSettings::parse(&doc).map_err(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn defaults_and_values() {
        assert_eq!(RgSettings::parse(&json!({})).unwrap(), RgSettings::default());
        let s = RgSettings::parse(&json!({ "RG-02": { "whole_day": false }, "RG-03": { "overall": 5, "per_type": { "late_arrival": 2 } }, "RG-05": "two", "RG-06": ["whole_day"], "RG-08": "block" })).unwrap();
        assert!(!s.types_on["whole_day"] && s.types_on["missed_punch"]);
        assert_eq!((s.limit_overall, s.limit_per_type["late_arrival"], s.two_levels, s.over_limit_blocks), (5, 2, true, true));
    }

    #[test]
    fn wrong_values_refused() {
        for bad in [json!({ "RG-02": { "selfie": true } }), json!({ "RG-05": 2 }), json!({ "RG-06": ["holiday"] }), json!({ "RG-07": { "days": 2, "action": "ignore" } }), json!({ "RG-12": true })] {
            assert!(RgSettings::parse(&bad).is_err(), "{bad}");
        }
    }
}
