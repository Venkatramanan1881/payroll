//! The punch decision as a pure function (BR-02: same inputs, same decision; TSD §9.2).
//! The client's own opinion of "inside the fence" is never an input: the server computes distance and inside/outside.

use chrono::{DateTime, Utc};
use domain_policy::pc::PcSettings;
use serde_json::{json, Value};
use uuid::Uuid;

const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// Great-circle distance in metres (haversine).
pub fn distance_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// D-05c: until decided, these reasons do not count; the rest count.
pub fn counts_before_decision(reason: &str) -> bool {
    !matches!(reason, "outside_fence" | "branch_not_allowed" | "location_unavailable" | "ip_check_failed")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loc {
    pub lat: f64,
    pub lon: f64,
    pub accuracy_m: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fence {
    pub branch_id: Uuid,
    pub lat: f64,
    pub lon: f64,
    pub radius_m: i64,
}

#[derive(Debug, Clone)]
pub struct PunchInput {
    /// "app" or "web".
    pub source: String,
    pub location: Option<Loc>,
    pub has_selfie: bool,
    pub event_at: DateTime<Utc>,
    pub received_at: DateTime<Utc>,
    /// Phone clock minus the server's estimate of the true time; None when it cannot be measured (phone rebooted).
    pub clock_skew_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct PunchContext {
    /// None: the PC rule group is switched off (G2, BR-14), so no PC rule is evaluated.
    pub settings: Option<PcSettings>,
    pub master_branch: Uuid,
    /// Branches the employee may punch at besides the master (EM-06), on the punch date.
    pub allowed_branches: Vec<Uuid>,
    /// EM-07 = block (otherwise record and flag).
    pub branch_not_allowed_blocks: bool,
    /// Every enabled fence of the organisation.
    pub fences: Vec<Fence>,
    /// Web with PC-05 on: whether the client IP is listed for an applicable branch. None when not checked.
    pub ip_ok: Option<bool>,
    /// Weekly off, holiday or no shift on the punch's local date (PC-06).
    pub off_day: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlagSpec {
    pub reason: &'static str,
    pub counts_before_decision: bool,
    pub evidence: Value,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Located {
    /// The branch the punch is attributed to: the fence it fell inside, else None.
    pub branch_id: Option<Uuid>,
    /// Metres to the centre of the nearest applicable fence, rounded (PC-R13).
    pub fence_distance_m: Option<i32>,
    pub inside_fence: Option<bool>,
    /// Inside the fence of a branch the employee may not punch at (EM-07).
    pub not_allowed_branch: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Decision {
    /// (rule_ref, message shown to the employee). Set = nothing is stored except the refused attempt (E18).
    pub refused: Option<(&'static str, String)>,
    pub flags: Vec<FlagSpec>,
    pub located: Located,
}

impl PunchContext {
    /// PC-16: the master branch, plus the allowed branches unless PC-16 = master.
    pub fn applicable_branches(&self) -> Vec<Uuid> {
        let master_only = self.settings.as_ref().is_some_and(|s| s.fences == "master");
        let mut b = vec![self.master_branch];
        if !master_only {
            b.extend(self.allowed_branches.iter().copied());
        }
        b
    }
}

/// Where a reading falls (PC-16 picks the applicable fences). Used by the caller before `evaluate` to find the
/// punch's branch for the off-day check (D-04: holiday calendar of the punch branch).
pub fn locate(loc: Option<Loc>, ctx: &PunchContext) -> Located {
    let Some(loc) = loc else { return Located::default() };
    let applicable = ctx.applicable_branches();
    let dist = |f: &Fence| distance_m(loc.lat, loc.lon, f.lat, f.lon);
    let nearest = ctx
        .fences
        .iter()
        .filter(|f| applicable.contains(&f.branch_id))
        .map(|f| (f, dist(f)))
        .min_by(|a, b| a.1.total_cmp(&b.1));
    let Some((_, d)) = nearest else {
        // No applicable fence configured: nothing to compare against.
        let foreign = ctx.fences.iter().find(|f| !applicable.contains(&f.branch_id) && dist(f) <= f.radius_m as f64);
        return Located { not_allowed_branch: foreign.filter(|f| !allowed(ctx, f.branch_id)).map(|f| f.branch_id), ..Located::default() };
    };
    // Inside any applicable fence wins, even if a nearer centre has a smaller radius.
    let inside = ctx.fences.iter().filter(|f| applicable.contains(&f.branch_id)).find(|f| dist(f) <= f.radius_m as f64);
    let foreign = if inside.is_none() {
        ctx.fences.iter().find(|f| !applicable.contains(&f.branch_id) && !allowed(ctx, f.branch_id) && dist(f) <= f.radius_m as f64)
    } else {
        None
    };
    Located {
        branch_id: inside.map(|f| f.branch_id).or(foreign.map(|f| f.branch_id)),
        fence_distance_m: Some(d.round() as i32),
        inside_fence: Some(inside.is_some()),
        not_allowed_branch: foreign.map(|f| f.branch_id),
    }
}

fn allowed(ctx: &PunchContext, branch: Uuid) -> bool {
    branch == ctx.master_branch || ctx.allowed_branches.contains(&branch)
}

fn flag(reason: &'static str, evidence: Value) -> FlagSpec {
    FlagSpec { reason, counts_before_decision: counts_before_decision(reason), evidence }
}

/// Decides one app or web punch. Rule order: PC-01 source, PC-02 selfie, location (PC-02/PC-12, PC-13), accuracy (PC-14),
/// branch (EM-07), fence and IP (PC-04, PC-05, PC-15), day (PC-06), late sync (PC-07), clock (PC-08).
pub fn evaluate(input: &PunchInput, ctx: &PunchContext) -> Decision {
    let located = locate(input.location, ctx);
    let mut d = Decision { located: located.clone(), ..Decision::default() };
    let Some(s) = &ctx.settings else { return d };
    let refuse = |mut d: Decision, rule: &'static str, msg: &str| {
        d.refused = Some((rule, msg.to_string()));
        d.flags.clear();
        d
    };
    let web = input.source == "web";

    let source_on = if web { s.sources.web } else { s.sources.app };
    if !source_on {
        return refuse(d, "PC-01", "Punching from here is switched off for your organisation.");
    }
    if !web && s.app_selfie && !input.has_selfie {
        return refuse(d, "PC-02", "A selfie is needed to punch from the app.");
    }

    let location_needed = if web { s.web_location } else { s.app_location };
    if location_needed && input.location.is_none() {
        if !web {
            // OQ-C04-10 ruling: app punches need a location when PC-02 location is on.
            return refuse(d, "PC-02", "Your location is needed to punch from the app. Turn on location and try again.");
        }
        if s.web_location_unavailable == "block" {
            return refuse(d, "PC-13", "Your location is needed to punch from the browser. Allow location access and try again.");
        }
        d.flags.push(flag("location_unavailable", json!({})));
    }

    if let Some(loc) = input.location {
        if loc.accuracy_m > s.accuracy_threshold_m {
            match s.accuracy_action.as_str() {
                "block" => return refuse(d, "PC-14", "Your location reading is not accurate enough. Move to an open spot and try again."),
                "flag" => d.flags.push(flag("low_accuracy", json!({ "accuracy_m": loc.accuracy_m, "threshold_m": s.accuracy_threshold_m }))),
                _ => {}
            }
        }
    }

    if let Some(branch) = located.not_allowed_branch {
        if ctx.branch_not_allowed_blocks {
            return refuse(d, "EM-07", "You are not allowed to punch at this branch.");
        }
        d.flags.push(flag("branch_not_allowed", json!({ "branch_id": branch })));
    } else {
        let fence_checked = (if web { s.web_location && s.web_inside_fence } else { s.app_inside_fence }) && located.inside_fence.is_some();
        let fence_fail = fence_checked && located.inside_fence == Some(false);
        let ip_checked = web && s.web_ip_restricted && ctx.ip_ok.is_some();
        let ip_fail = ip_checked && ctx.ip_ok == Some(false);
        // PC-15 either: one passing check is enough when both are on.
        let either_passes = fence_checked && ip_checked && s.ip_and_location == "either" && !(fence_fail && ip_fail);
        if fence_fail && !either_passes {
            match s.outside_fence.as_str() {
                "block" => return refuse(d, "PC-04", "You are outside the allowed punch area."),
                "flag" => d.flags.push(flag("outside_fence", json!({ "fence_distance_m": located.fence_distance_m }))),
                _ => {}
            }
        }
        if ip_fail && !either_passes {
            d.flags.push(flag("ip_check_failed", json!({})));
        }
    }

    if ctx.off_day {
        match s.off_day.as_str() {
            "block" => return refuse(d, "PC-06", "You have no shift today, so punching is not allowed."),
            "flag" => d.flags.push(flag("no_shift_or_off_day", json!({}))),
            _ => {}
        }
    }

    let delay = input.received_at - input.event_at;
    if delay > chrono::Duration::days(s.late_sync_days) {
        d.flags.push(flag("late_sync", json!({ "delay_hours": delay.num_hours(), "limit_days": s.late_sync_days })));
    }

    if let Some(skew) = input.clock_skew_ms {
        if skew.abs() > s.clock_tolerance_min * 60_000 {
            d.flags.push(flag("clock_mismatch", json!({ "skew_seconds": skew / 1000, "tolerance_minutes": s.clock_tolerance_min })));
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    // ~111.2 m per 0.001 degree of latitude.
    const LAT: f64 = 12.9716;
    const LON: f64 = 77.5946;

    fn branch(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn ctx() -> PunchContext {
        PunchContext {
            settings: Some(PcSettings::default()),
            master_branch: branch(1),
            allowed_branches: vec![branch(2)],
            branch_not_allowed_blocks: false,
            fences: vec![
                Fence { branch_id: branch(1), lat: LAT, lon: LON, radius_m: 200 },
                Fence { branch_id: branch(2), lat: LAT + 0.1, lon: LON, radius_m: 200 },
                Fence { branch_id: branch(3), lat: LAT + 0.2, lon: LON, radius_m: 200 },
            ],
            ip_ok: None,
            off_day: false,
        }
    }

    fn input(source: &str, metres_north: Option<f64>) -> PunchInput {
        let now = Utc::now();
        PunchInput {
            source: source.into(),
            location: metres_north.map(|m| Loc { lat: LAT + m / 111_195.0, lon: LON, accuracy_m: 10 }),
            has_selfie: false,
            event_at: now,
            received_at: now,
            clock_skew_ms: None,
        }
    }

    fn reasons(d: &Decision) -> Vec<&str> {
        d.flags.iter().map(|f| f.reason).collect()
    }

    #[test]
    fn distance_known_pair() {
        let d = distance_m(0.0, 0.0, 0.0, 0.008_993_216);
        assert!((d - 1000.0).abs() < 1.0, "{d}");
    }

    #[test]
    fn e18_sixty_metres_accepted() {
        let d = evaluate(&input("web", Some(60.0)), &ctx());
        assert!(d.refused.is_none() && d.flags.is_empty(), "{d:?}");
        assert_eq!((d.located.inside_fence, d.located.branch_id, d.located.fence_distance_m), (Some(true), Some(branch(1)), Some(60)));
    }

    #[test]
    fn e18_450_metres_flagged_not_counted() {
        let d = evaluate(&input("web", Some(450.0)), &ctx());
        assert!(d.refused.is_none());
        assert_eq!(reasons(&d), ["outside_fence"]);
        assert!(!d.flags[0].counts_before_decision);
        assert_eq!((d.located.inside_fence, d.located.fence_distance_m, d.located.branch_id), (Some(false), Some(450), None));
    }

    #[test]
    fn e18_web_location_refused_blocks_pc13() {
        let d = evaluate(&input("web", None), &ctx());
        assert_eq!(d.refused.as_ref().map(|r| r.0), Some("PC-13"));
        let mut c = ctx();
        c.settings.as_mut().unwrap().web_location_unavailable = "flag".into();
        assert_eq!(reasons(&evaluate(&input("web", None), &c)), ["location_unavailable"]);
    }

    #[test]
    fn app_without_location_is_blocked() {
        assert_eq!(evaluate(&input("app", None), &ctx()).refused.map(|r| r.0), Some("PC-02"));
        let mut c = ctx();
        c.settings.as_mut().unwrap().app_location = false;
        assert!(evaluate(&input("app", None), &c).refused.is_none(), "PC-02 location off: no location needed");
    }

    #[test]
    fn outside_fence_actions() {
        let mut c = ctx();
        c.settings.as_mut().unwrap().outside_fence = "block".into();
        assert_eq!(evaluate(&input("app", Some(450.0)), &c).refused.map(|r| r.0), Some("PC-04"));
        c.settings.as_mut().unwrap().outside_fence = "allow".into();
        assert!(evaluate(&input("app", Some(450.0)), &c).flags.is_empty());
    }

    #[test]
    fn allowed_branch_fence_counts_as_inside_and_costs_there() {
        let d = evaluate(&input("app", Some(0.1 * 111_195.0 + 20.0)), &ctx());
        assert!(d.flags.is_empty(), "{d:?}");
        assert_eq!(d.located.branch_id, Some(branch(2)));
        let mut c = ctx();
        c.settings.as_mut().unwrap().fences = "master".into(); // PC-16: allowed branch's fence no longer applies
        let d = evaluate(&input("app", Some(0.1 * 111_195.0 + 20.0)), &c);
        assert_eq!(reasons(&d), ["outside_fence"]);
    }

    #[test]
    fn e17_branch_not_allowed_flagged_or_blocked() {
        let d = evaluate(&input("app", Some(0.2 * 111_195.0)), &ctx());
        assert_eq!(reasons(&d), ["branch_not_allowed"]);
        assert!(!d.flags[0].counts_before_decision);
        assert_eq!(d.located.branch_id, Some(branch(3)));
        let mut c = ctx();
        c.branch_not_allowed_blocks = true;
        assert_eq!(evaluate(&input("app", Some(0.2 * 111_195.0)), &c).refused.map(|r| r.0), Some("EM-07"));
    }

    #[test]
    fn accuracy_actions() {
        let mut i = input("app", Some(10.0));
        i.location.as_mut().unwrap().accuracy_m = 101;
        let mut c = ctx();
        assert_eq!(reasons(&evaluate(&i, &c)), ["low_accuracy"]);
        assert!(evaluate(&i, &c).flags[0].counts_before_decision);
        c.settings.as_mut().unwrap().accuracy_action = "block".into();
        assert_eq!(evaluate(&i, &c).refused.map(|r| r.0), Some("PC-14"));
        c.settings.as_mut().unwrap().accuracy_action = "accept".into();
        assert!(evaluate(&i, &c).flags.is_empty());
        i.location.as_mut().unwrap().accuracy_m = 100; // exactly the threshold passes
        c.settings.as_mut().unwrap().accuracy_action = "block".into();
        assert!(evaluate(&i, &c).refused.is_none());
    }

    #[test]
    fn pc15_either_and_both() {
        let mut c = ctx();
        c.settings.as_mut().unwrap().web_ip_restricted = true;
        c.ip_ok = Some(true);
        assert!(evaluate(&input("web", Some(450.0)), &c).flags.is_empty(), "either: IP passing is enough");
        c.ip_ok = Some(false);
        assert!(evaluate(&input("web", Some(60.0)), &c).flags.is_empty(), "either: fence passing is enough");
        assert_eq!(reasons(&evaluate(&input("web", Some(450.0)), &c)), ["outside_fence", "ip_check_failed"]);
        c.settings.as_mut().unwrap().ip_and_location = "both".into();
        assert_eq!(reasons(&evaluate(&input("web", Some(60.0)), &c)), ["ip_check_failed"]);
    }

    #[test]
    fn off_day_actions() {
        let mut c = ctx();
        c.off_day = true;
        let d = evaluate(&input("app", Some(10.0)), &c);
        assert_eq!(reasons(&d), ["no_shift_or_off_day"]);
        assert!(d.flags[0].counts_before_decision);
        c.settings.as_mut().unwrap().off_day = "block".into();
        assert_eq!(evaluate(&input("app", Some(10.0)), &c).refused.map(|r| r.0), Some("PC-06"));
    }

    #[test]
    fn late_sync_boundary() {
        let mut i = input("app", Some(10.0));
        i.event_at = i.received_at - chrono::Duration::days(3);
        assert!(evaluate(&i, &ctx()).flags.is_empty(), "exactly PC-07 days is not late");
        i.event_at -= chrono::Duration::seconds(1);
        assert_eq!(reasons(&evaluate(&i, &ctx())), ["late_sync"]);
    }

    #[test]
    fn clock_tolerance_boundary() {
        let mut i = input("app", Some(10.0));
        i.clock_skew_ms = Some(-5 * 60_000);
        assert!(evaluate(&i, &ctx()).flags.is_empty());
        i.clock_skew_ms = Some(5 * 60_000 + 1);
        assert_eq!(reasons(&evaluate(&i, &ctx())), ["clock_mismatch"]);
    }

    #[test]
    fn source_off_and_selfie_required() {
        let mut c = ctx();
        c.settings.as_mut().unwrap().sources.web = false;
        assert_eq!(evaluate(&input("web", Some(10.0)), &c).refused.map(|r| r.0), Some("PC-01"));
        let mut c = ctx();
        c.settings.as_mut().unwrap().app_selfie = true;
        assert_eq!(evaluate(&input("app", Some(10.0)), &c).refused.map(|r| r.0), Some("PC-02"));
    }

    #[test]
    fn pc_group_switched_off_evaluates_nothing() {
        let mut c = ctx();
        c.settings = None;
        c.off_day = true;
        let d = evaluate(&input("web", None), &c);
        assert!(d.refused.is_none() && d.flags.is_empty());
    }
}
