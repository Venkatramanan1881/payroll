//! Typed punch-collection settings (PC rule group, BRD §9.4) with the BRD defaults.
//! PC-03 (fences) lives in org.geo_fence; PC-09/PC-17 wait for WFH/on-duty (DP-09); PC-10/PC-11 belong to devices and permissions.

use db::{invalid, DomainError, OrgTx};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct Sources {
    pub device: bool,
    pub web: bool,
    pub app: bool,
    pub kiosk: bool,
    pub import: bool,
    pub hr_entry: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PcSettings {
    pub sources: Sources,               // PC-01
    pub app_location: bool,             // PC-02
    pub app_selfie: bool,               // PC-02
    pub app_inside_fence: bool,         // PC-02
    pub outside_fence: String,          // PC-04 block | flag | allow
    pub web_ip_restricted: bool,        // PC-05
    pub off_day: String,                // PC-06 allow | flag | block
    pub late_sync_days: i64,            // PC-07
    pub clock_tolerance_min: i64,       // PC-08
    pub web_location: bool,             // PC-12
    pub web_inside_fence: bool,         // PC-12
    pub web_location_unavailable: String, // PC-13 block | flag
    pub accuracy_threshold_m: i64,      // PC-14
    pub accuracy_action: String,        // PC-14 accept | flag | block
    pub ip_and_location: String,        // PC-15 both | either
    pub fences: String,                 // PC-16 master | master_and_allowed
}

impl Default for PcSettings {
    fn default() -> Self {
        PcSettings {
            sources: Sources { device: true, web: true, app: true, kiosk: true, import: true, hr_entry: true },
            app_location: true,
            app_selfie: false,
            app_inside_fence: true, // no BRD default; mirrors PC-12
            outside_fence: "flag".into(),
            web_ip_restricted: false,
            off_day: "flag".into(),
            late_sync_days: 3,
            clock_tolerance_min: 5,
            web_location: true,
            web_inside_fence: true,
            web_location_unavailable: "block".into(),
            accuracy_threshold_m: 100,
            accuracy_action: "flag".into(),
            ip_and_location: "either".into(),
            fences: "master_and_allowed".into(),
        }
    }
}

fn flags(v: &Value, key: &str, names: &[&str]) -> Result<Vec<Option<bool>>, String> {
    let obj = v.as_object().ok_or(format!("{key} must be an object"))?;
    if let Some(k) = obj.keys().find(|k| !names.contains(&k.as_str())) {
        return Err(format!("{key}.{k} is not a known field"));
    }
    names
        .iter()
        .map(|n| match obj.get(*n) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(format!("{key}.{n} must be true or false")),
        })
        .collect()
}

fn choice(v: &Value, key: &str, allowed: &[&str]) -> Result<String, String> {
    v.as_str().filter(|s| allowed.contains(s)).map(str::to_string).ok_or(format!("{key} must be one of {}", allowed.join(", ")))
}

fn whole(v: &Value, key: &str, min: i64, max: i64) -> Result<i64, String> {
    v.as_i64().filter(|n| (min..=max).contains(n)).ok_or(format!("{key} must be a whole number from {min} to {max}"))
}

impl PcSettings {
    /// Reads a PC document; missing keys keep their default. Wrong types, values or unknown keys are refused.
    pub fn parse(doc: &Value) -> Result<PcSettings, String> {
        let obj = doc.as_object().ok_or("settings must be an object")?;
        let mut s = PcSettings::default();
        for (key, v) in obj {
            match key.as_str() {
                "PC-01" => {
                    let f = flags(v, key, &["device", "web", "app", "kiosk", "import", "hr_entry"])?;
                    let src = &mut s.sources;
                    for (slot, val) in [&mut src.device, &mut src.web, &mut src.app, &mut src.kiosk, &mut src.import, &mut src.hr_entry].into_iter().zip(f) {
                        if let Some(b) = val {
                            *slot = b;
                        }
                    }
                }
                "PC-02" => {
                    let f = flags(v, key, &["location", "selfie", "inside_fence"])?;
                    s.app_location = f[0].unwrap_or(s.app_location);
                    s.app_selfie = f[1].unwrap_or(s.app_selfie);
                    s.app_inside_fence = f[2].unwrap_or(s.app_inside_fence);
                }
                "PC-04" => s.outside_fence = choice(v, key, &["block", "flag", "allow"])?,
                "PC-05" => s.web_ip_restricted = v.as_bool().ok_or("PC-05 must be true or false")?,
                "PC-06" => s.off_day = choice(v, key, &["allow", "flag", "block"])?,
                "PC-07" => s.late_sync_days = whole(v, key, 0, 365)?,
                "PC-08" => s.clock_tolerance_min = whole(v, key, 0, 1440)?,
                "PC-12" => {
                    let f = flags(v, key, &["location", "inside_fence"])?;
                    s.web_location = f[0].unwrap_or(s.web_location);
                    s.web_inside_fence = f[1].unwrap_or(s.web_inside_fence);
                }
                "PC-13" => s.web_location_unavailable = choice(v, key, &["block", "flag"])?,
                "PC-14" => {
                    let o = v.as_object().ok_or("PC-14 must be an object")?;
                    if let Some(k) = o.keys().find(|k| !["threshold_m", "action"].contains(&k.as_str())) {
                        return Err(format!("PC-14.{k} is not a known field"));
                    }
                    if let Some(t) = o.get("threshold_m") {
                        s.accuracy_threshold_m = whole(t, "PC-14.threshold_m", 1, 100_000)?;
                    }
                    if let Some(a) = o.get("action") {
                        s.accuracy_action = choice(a, "PC-14.action", &["accept", "flag", "block"])?;
                    }
                }
                "PC-15" => s.ip_and_location = choice(v, key, &["both", "either"])?,
                "PC-16" => s.fences = choice(v, key, &["master", "master_and_allowed"])?,
                // PC-03 lives in org.geo_fence; PC-09/10/11/17 wait for their flows. Stored, not evaluated (check_document vets the key).
                _ => {}
            }
        }
        Ok(s)
    }
}

/// PC settings for a resolved policy version: None when the PC group is switched off (G2, BR-14: no PC rule applies);
/// defaults when the version has no PC group or no version applies.
pub async fn load(tx: &mut OrgTx<'_>, version: Option<Uuid>) -> Result<Option<PcSettings>, DomainError> {
    let Some(version) = version else { return Ok(Some(PcSettings::default())) };
    let row: Option<(bool, Value)> = sqlx::query_as("SELECT enabled, settings FROM policy.policy_rule_group WHERE policy_version_id = $1 AND rule_group = 'PC'")
        .bind(version)
        .fetch_optional(&mut ***tx)
        .await?;
    match row {
        None => Ok(Some(PcSettings::default())),
        Some((false, _)) => Ok(None),
        Some((true, doc)) => PcSettings::parse(&doc).map(Some).map_err(invalid),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_document_gives_brd_defaults() {
        let s = PcSettings::parse(&json!({})).unwrap();
        assert_eq!(s, PcSettings::default());
        assert_eq!((s.late_sync_days, s.clock_tolerance_min, s.accuracy_threshold_m), (3, 5, 100));
        assert_eq!((s.outside_fence.as_str(), s.web_location_unavailable.as_str(), s.fences.as_str()), ("flag", "block", "master_and_allowed"));
    }

    #[test]
    fn partial_objects_keep_other_defaults() {
        let s = PcSettings::parse(&json!({ "PC-01": { "web": false }, "PC-14": { "action": "block" } })).unwrap();
        assert!(!s.sources.web && s.sources.app);
        assert_eq!((s.accuracy_threshold_m, s.accuracy_action.as_str()), (100, "block"));
    }

    #[test]
    fn known_unbuilt_keys_are_stored_not_refused() {
        assert_eq!(PcSettings::parse(&json!({ "PC-03": 200, "PC-09": "always", "PC-17": "applied" })).unwrap(), PcSettings::default());
    }

    #[test]
    fn wrong_values_are_refused() {
        for bad in [
            json!({ "PC-04": "maybe" }),
            json!({ "PC-05": "yes" }),
            json!({ "PC-07": -1 }),
            json!({ "PC-07": 1.5 }),
            json!({ "PC-07": 366 }),
            json!({ "PC-08": 100_000 }),
            json!({ "PC-14": { "threshold_m": 0 } }),
            json!({ "PC-02": { "gps": true } }),
            json!({ "PC-12": { "location": 1 } }),
            json!([]),
        ] {
            assert!(PcSettings::parse(&bad).is_err(), "{bad} should be refused");
        }
    }
}
