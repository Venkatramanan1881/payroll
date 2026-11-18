# Phase 5 Slice 3: Punch Collection (app and web) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Employees punch from the app or browser; the server alone decides, from the punch settings in force (PC rule group), whether each punch is accepted, flagged or refused; offline batches sync safely; HR records manual attendance for device failures; managers and HR decide flagged punches.

**Architecture:** New crate `domain-timecap`. The decision is a **pure function** `evaluate(&PunchInput, &PunchContext) -> Decision` (BR-02: same inputs, same decision; TSD §3.3), unit-tested without a database. Routes gather the context (resolved PC settings via slice 2's `resolve_policy`, fences, allowed branches, expected day, IP list), call `evaluate`, then persist. A refusal writes `timecap.punch_attempt_refused` and **commits it before** answering 422, so the evidence survives (E18). Raw punches are insert-only (BR-01).

**Tech Stack:** As before. No new dependencies (haversine distance is a few lines of `f64` math on stored `numeric` coordinates).

**Spec:** `docs/api/openapi/punches.yaml` (`timecapRecordPunch`, `timecapSyncOfflinePunches`, `timecapListPunchFlags`, `timecapAcceptPunchFlag`, `timecapRejectPunchFlag`, `timecapRecordManualAttendance`, `timecapListPunches`, `timecapGetPunch`, `timecapListRefusedPunchAttempts`); BRD v0.8 §9.4 (PC-R1..R15, PC-01..17), E17, E18, EM-06/07; `docs/requirements/DECISIONS.md` D-04, D-05, D-05b, D-05c; `docs/domain/STATE_MACHINES.md` SM-FLAG; PERMISSION_MATRIX TCP-03/04/06/07/08/09.

## Global Constraints

- The client's opinion of "inside the fence" is never an input; the server computes distance and inside/outside (TSD §9.2).
- Settings come from the **PC rule group of the policy version resolved for the employee on the punch's local date** (BR-05; date = event time in `captured_tz`). A missing key takes its BRD default; a switched-off PC group means no PC rule is evaluated (G2, BR-14).
- Flag `counts_before_decision` follows D-05c: **do not count** until decided — outside_fence, branch_not_allowed, location_unavailable, ip_check_failed; **count** — low_accuracy, no_shift_or_off_day, late_sync, clock_mismatch.
- Nobody decides a flag on their own punch (BR-07). Out of scope or another organisation → 404.
- Direction is never assigned (BRX-01). Raw punches are never updated (BR-01).

## PC settings (typed, with BRD defaults) — validated on `PUT .../rule-groups/PC`

| Key | Shape | Default |
|---|---|---|
| PC-01 | `{device, web, app, kiosk, import, hr_entry}` booleans | all true |
| PC-02 | `{location, selfie, inside_fence}` booleans (app) | location true, selfie false, inside_fence true (no BRD default; mirrors PC-12) |
| PC-04 | `"block" \| "flag" \| "allow"` | `"flag"` |
| PC-05 | boolean (web IP restriction) | false |
| PC-06 | `"allow" \| "flag" \| "block"` | `"flag"` |
| PC-07 | integer days ≥ 0 (late sync) | 3 |
| PC-08 | integer minutes ≥ 0 (clock tolerance) | 5 |
| PC-12 | `{location, inside_fence}` booleans (web) | both true |
| PC-13 | `"block" \| "flag"` (web location unavailable) | `"block"` |
| PC-14 | `{threshold_m: int > 0, action: "accept" \| "flag" \| "block"}` | 100, `"flag"` |
| PC-15 | `"both" \| "either"` | `"either"` |
| PC-16 | `"master" \| "master_and_allowed"` | `"master_and_allowed"` |

PC-03 (fences) lives in `org.geo_fence`; PC-09/PC-17 (WFH, on-duty) wait for those flows (DP-09); PC-10/PC-11 belong to devices/permissions. EM-07 (branch not allowed: `block`/`flag`, default `flag`) is read from organisation settings. Unknown PC keys or wrong types → 422.

## Review Focus

1. **E18 exactly:** fence radius 200 m, PC-04 = flag → web punch 60 m from centre accepted; 450 m recorded, flagged `outside_fence`, `counts_before_decision = false`; location refused in the browser → blocked (PC-13), attempt recorded, employee told why. Test in Task 3.
2. **A retried offline batch** → each already-stored punch reports `duplicate`, nothing is stored twice. Test in Task 4.
3. **An employee deciding a flag on their own punch** → 403 SELF_APPROVAL_FORBIDDEN (BR-07). Test in Task 5.
4. **A punch near midnight** → the PC settings and the off-day check use the local date in `captured_tz`, not the UTC date. Test in Task 3.
5. **A punch inside another branch's fence that is not allowed** (E17 / EM-07) → recorded and flagged `branch_not_allowed` (default), costed to that branch. Test in Task 3.

---

## File Structure

```
backend/crates/domain-policy/src/pc.rs            (typed PC settings; lives with policy so timecap can depend on it)
backend/crates/domain-timecap/src/{lib,evaluate,punch,flags}.rs
backend/crates/api/src/routes/punches.rs
backend/crates/api/tests/{punch_settings,punches,punch_sync,punch_flags,manual_attendance}.rs
```

### Task 1: Typed PC settings
`settings.rs`: `PcSettings::from_document(Option<&Value>, enabled: bool) -> Result<PcSettings, String>` with the defaults above; `validate(&Value) -> Result<(), String>`. Policy `put_document` calls it for group PC (after the structural check). Tests: unit (defaults; each wrong type rejected); API `put_pc_wrong_type_422`, `put_pc_valid_200`.

### Task 2: Pure evaluation
`evaluate.rs`: `distance_m(lat1, lon1, lat2, lon2) -> f64` (haversine, Earth radius 6 371 008.8 m); `evaluate(&PunchInput, &PunchContext) -> Decision { refused: Option<(rule_ref, message)>, flags: Vec<FlagSpec>, branch_id: Option<Uuid>, fence_distance_m: Option<i32>, inside_fence: Option<bool> }`. Rule order: PC-01 source → location requirement (PC-02 / PC-12, PC-13) → accuracy (PC-14) → fence (nearest applicable fence; PC-04) with IP (PC-05/PC-15, web) → branch allowed (EM-07) → day (PC-06) → late sync (PC-07) → clock (PC-08). Tests: `distance_known_pair` (two points 1 km apart ±1 m), E18 cases, accuracy actions, PC-15 either/both, branch not allowed, off day, late sync boundary (exactly N days = no flag), clock tolerance boundary.

### Task 3: Record a punch — `POST /v1/punches`
Context gathering in `punch.rs`; persistence of punch + flags + outbox `PunchRecorded`/`PunchFlagged`; refusal → attempt committed + 422 `PUNCH_REFUSED` with `rule_ref`, `attempt_id`. Retry with the same `idempotency_key` → 200 with the stored punch. Tests: E18 trio, midnight local date, E17 branch, retry 200, another organisation's fences never considered.

### Task 4: Offline sync — `POST /v1/punches/sync`
Per-item results (`accepted`/`flagged`/`duplicate`/`refused`), stored in event-time order. Clock check (PC-08): expected elapsed = `(sync_uptime_ms − item.uptime_ms)`; phone-clock elapsed = `sync_client_time − item.client_time`; difference > tolerance → `clock_mismatch`; skipped when `sync_uptime_ms < item.uptime_ms` (phone rebooted). Tests: duplicate on retry, late_sync, clock_mismatch, reboot skip, one bad item does not fail the batch.

### Task 5: Flag decisions
List (`state`, `reason`, `employee_id`, dates; scope), accept/reject (reject needs reason), SM-FLAG transitions, BR-07, decision row + flag state + audit + outbox `PunchFlagDecided`; repeat of the same decision → 200 unchanged; opposite decision after one → 409 ILLEGAL_TRANSITION (GAP-SM-06 neutral). Tests for each.

### Task 6: Manual attendance and reads
`POST /v1/manual-attendance` (TCP-04; PC-01 `hr_entry`): manual punches with reason and entered_by (BR-01), each employee in scope, branch given. `GET /v1/punches` (scope; location fields only with TCP-08), `GET /v1/punches/{id}`, `GET /v1/punch-attempts` (TCP-09). Tests, then `cargo test --workspace` and clippy clean.

## Rulings taken in this slice (neutral items kept open)
- App punch with no location (OQ-C04-10): blocked (`PUNCH_REFUSED`, rule_ref PC-02) when PC-02 location is on — decided by the product owner 2026-11-18. Offline items without location are refused the same way.
- IP check failure (OQ-C04-11): flagged `ip_check_failed`; PC-15 `either` lets a passing fence or IP clear both.
- Who decides a flag (G-16): anyone with TCP-06 in scope; managers (`approve`) for their direct reports. Per-reason deciders from D-05c wait for the approval engine.
- Manual attendance with PC-01 `hr_entry` off (OQ-C04-07) and PC-11 delegation: not enforced yet.
- Org settings keys follow slice 1 (`"EM-06"`: master_only / chosen_branches / all_branches; `"EM-07"`: block / record_and_flag).

## Not in this slice
Devices, enrolment and the device gateway (eSSL adapter blocked on Q15); selfie upload links (files slice); WFH/on-duty fence exemptions (PC-09/PC-17, DP-09); bulk device-outage regularisation (TCP-05, with regularisation).
