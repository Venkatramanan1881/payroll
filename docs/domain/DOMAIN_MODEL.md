# Domain model (V1)

Status: Draft for Approval Gate 2
Sources: BRD v0.7, TSD v0.1, Phase 0 artefacts in `docs/product/` (Gate 1 approved 2026-11-15)
Companions: `GLOSSARY.md`, `STATE_MACHINES.md`, `PERMISSION_MATRIX.md`, `../architecture/SERVICE_BOUNDARIES.md`

This document models business concepts. It does not define tables or APIs (those are Phases 3 and 4). Where an unanswered question changes the model, it is shown as an open **decision point (DP)**. The model is built so each answer can be slotted in without restructuring.

---

## 1. Modelling principles applied

1. **Facts, interpretations, decisions and outcomes are separate objects.** A raw punch is a fact. A day result is an interpretation. An approval is a decision. Deduction days are an outcome. None of them overwrites another (BR-01, BR-08).
2. **Configuration is versioned and effective-dated.** A version in use is never edited (G5). Every calculation records the versions it used (BR-08).
3. **Calculated results are append-only.** Recalculation produces a new result version that supersedes the old one, which stays readable (BR-03, BRX-03, AT5).
4. **Balances are ledgers.** A balance is the sum of its ledger entries, never a stored number that gets overwritten (LM-R4, OB-R4).
5. **Independent lifecycles get independent state.** A request's approval state, a day's processing state and a period's lock state are separate, with no shared generic `status`.
6. **Every tenant-owned object belongs to exactly one Organisation** (TB-01). Only Platform objects sit outside a tenant.
7. **The calculation engine is pure.** Its inputs are snapshots and its outputs are results with explanations. It has no access to storage, the clock or the network (TSD §3.3, §5.2).

---

## 2. Bounded contexts

| # | Context | Purpose | Owns (aggregates) | Release |
|---|---|---|---|---|
| 1 | **Platform** | Tenants, plans and module entitlement | Organisation, Plan, SectorTemplate, SupportAccessGrant, Announcement, VendorUser | V1 |
| 2 | **Identity & Access** | Who someone is and what they may do | UserAccount, Role, Delegation, ApproverRule | V1 |
| 3 | **Organisation Structure** | Where and how the organisation is organised | Branch, OrgUnitLists (Department, Designation, Category, EmployeeType), Contractor, OrganisationSettings | V1 |
| 4 | **Workforce** | Who works here, and their dated assignments | Employee | V1 |
| 5 | **Time Capture** | Evidence of presence | Device, RawPunch, PunchFlag | V1 |
| 6 | **Scheduling** | What was expected | Shift, Roster, WeeklyOffPattern, HolidayCalendar, ShiftChangeRequest | V1 |
| 7 | **Policy** | Organisation rules as versioned data | PolicySet, Exemption | V1 |
| 8 | **Attendance** | Interpreting a day and the period counters | AttendanceDay, RegularisationRequest, BulkWaiver, PeriodCounterSet | V1 |
| 9 | **Leave & Absence** | Leave, comp-off and absence classification | LeaveType, LeaveAccount (ledger), LeaveRequest, CompOffAccount (ledger), CompOffClaim, AbscondingCase | V1 |
| 10 | **Permission & On-duty** | Short absences and off-site work | PermissionType, PermissionAccount (ledger), PermissionRequest, OnDutyRequest | V1 |
| 11 | **Period** | Period close and handoff to payroll | AttendancePeriod, PeriodAdjustment, PayrollInputExport | V1 |
| 12 | **Opening Balances** | Starting points for mid-year joiners | OpeningBalanceBatch | V1 |
| 13 | **Notifications** | Outbound messages | NotificationRule, Notification | V1 |
| 14 | **Audit** | Insert-only record of actions | AuditEntry, SettingsChange | V1 |
| 15 | **Reporting & Dashboards** | Read models only; owns no authoritative data | — | V1 |
| — | Payroll, Loans, Reimbursements, Documents, Overtime pay | Later contexts; consume `PayrollInput` | — | V2/V3 |

Shared capability (not a context): **Approval workflow.** Each request aggregate owns its own lifecycle. The approval routing, levels, escalation and the BR-07 check are a shared domain service that every request context uses (see §6).

---

## 3. Aggregates

For each aggregate: root, main members, invariants, how it is dated, and what is audited. "Org-owned" means it carries its organisation and the database rejects cross-organisation access (TB-01).

### 3.1 Platform

**Organisation** (root, tenant). Platform-owned.
- Members: legal and display name, sector template used, `SubscriptionRecord` (plan, start, renewal, billing status: VC-R5), `ModuleEntitlement` history (module, on/off, from-date, set-by: MD-01, VC-R2), employee limit history, suspension state.
- Invariants: only the vendor changes entitlements (MD-01). Module dependencies are respected (MD-04). A suspended organisation still accepts punches but its users can only view and export (VC-R4).
- Dated: entitlements and limits are dated history (VC-R2).
- Audit: every plan, module, limit and suspension change.

**Plan**: modules, employee limit, app included (MD-02). Platform-owned. Content is open (Q5, OQ-V01-02).

**SectorTemplate**: a named bundle of default OrganisationSettings, policy sets and lists, used once at organisation creation (VC-R1, G6). Whether later template changes flow to existing organisations is open (OQ-V01-10). The model treats creation as a **copy**, so existing organisations are not affected unless that question is answered otherwise.

**SupportAccessGrant**: vendor user, organisation, reason, approved-by (VC-03), window start and end. Every action taken under it is audited and visible to the Organisation Admin (VC-R6). Lifecycle in STATE_MACHINES. Whether the vendor may change data during support is open (OQ-V01-12).

**VendorUser**: vendor role is full admin, support or read-only (VC-05). A vendor user is never a member of an organisation.

### 3.2 Identity & Access

**UserAccount** (root). Org-owned (vendor users are separate).
- Members: sign-in identity (mobile number for employees, email and password plus second step for admins: UM-R3, AC-08), language (UM-R4), role assignments, `AccessScope` (all, chosen branches or chosen departments: AC-07), link to the Employee if the user is one, active flag.
- Invariants: a user's organisation comes from their sign-in and is never supplied by a client. Scope never exceeds the organisation.

**Role**: name, permission grants (view, edit, approve per operation: AC-01), system or custom (AC-06). The §5 roles exist as system roles. Whether they can be edited is open (OQ-C03-01).

**Delegation**: approver, delegate, from and to (AC-03). Rules are open (OQ-C03-10).

**ApproverRule**: request type → approver source (reporting manager, named person, department head or HR: AC-02), plus the number of levels and escalation settings taken from the relevant policy (LM-13, PM-16/17, RG-05/07, OD-05).

### 3.3 Organisation Structure

**Branch** (root). Org-owned.
- Members: address, **time zone** (BR-11), `GeoFence` list (centre, radius, on/off, tested-at: PC-03, PC-R14), holiday calendar reference (OR-R1, SH-10), allowed web IP list (PC-05).
- Invariants: a fence radius is greater than 0. The time zone is a valid IANA zone.
- Dated: time-zone change behaviour is open (OQ-C01-03). Until decided, a time-zone change must be dated and must not reinterpret past punches.

**OrgUnitLists**: Department, Designation, Category and EmployeeType are organisation-extensible lists (OR-R2, OR-R3, OR-R5). Entries are retired, never deleted, once in use (OQ-C01-04 decides renaming).

**Contractor**: name, payment route (V2: CW-R2), and the workers linked to it.

**OrganisationSettings**: financial and leave year (OR-01), week start (OR-02), languages (OR-03), currency (OR-04), employee code mode (EM-01), and the remaining organisation-wide settings that are not part of policy sets. Changes are recorded as SettingsChange entries (G9).

### 3.4 Workforce

**Employee** (root). Org-owned.
- Identity: employee code (unique within the organisation), personal, contact and emergency details (EM-R1), custom field values (EM-R8).
- **Dated assignments** (EM-R6, EM-09), each an effective-dated record with from and to dates that never overlap for the same kind:
  - master branch (exactly one at any date: EM-R9)
  - allowed branches (zero or more: EM-06)
  - department, designation, category, employee type
  - reporting manager
  - contractor (optional)
  - schedule source: fixed shift or roster (EM-R3)
  - policy-set assignment at employee level (G3), if any
  - payroll group (V2, PG-R3)
- Employment lifecycle (EM-R5): active, on probation, on notice, exited. See STATE_MACHINES SM-EMP.
- Invariants:
  - Exactly one master branch on every date the employee is employed.
  - The reporting manager is not the employee (BR-07 depends on this; OQ-C02-01).
  - No assignment starts before the joining date or after the exit date.
  - A rejoiner either reuses the code or gets a new one (EM-05). Whether that reopens the old record is open (OQ-C02-14).
- Audit: every assignment change records who, when, old value, new value and reason (EM-R6, G9).

Not part of Employee: device enrolment (Time Capture), balances (Leave and Permission ledgers), user account (Identity).

### 3.5 Time Capture

**Device** (root). Org-owned.
- Members: serial number (globally unique, so the gateway can resolve the organisation: TSD §9.1), make and adapter, branch, last-contact time, online state (PC-R2, PC-10), and `EnrolmentMapping` entries (enrolment ID → employee, with dates and enrolment status: PC-R3, PC-R8).
- Invariants: a push from an unregistered serial number is rejected and logged. Within its scope an enrolment ID maps to at most one employee on a given date (DP-07).

**RawPunch** (root). Org-owned. **Insert-only. Never updated or deleted** (BR-01, PC-R6).
- Members: employee (if resolved), source (device, app, web, kiosk, import, HR manual: PC-01), device or client identifier, **original event time** (UTC) and the zone it was captured in, **server receipt time**, ingestion idempotency key (TSD §5.5, §9.3), raw payload reference, location (latitude, longitude, accuracy) and the computed distance to the nearest applicable fence (PC-R13), selfie file reference, IP address (web), phone uptime-clock reading (offline: TSD §9.3), and a direction tag only if the source provides one (PR-01).
- Manual punches (BR-01, PC-R9, RG-10) are RawPunches with source = manual, a reason, the entering user and the approval reference.
- Invariants:
  - Unique per (organisation, idempotency key). A retry returns the existing punch.
  - A punch carries **no inferred direction** (BRX-01). Direction is assigned only inside a DayResult.
  - The fence distance and "inside" are computed by the server. A client's claim is ignored (TSD §9.2).

**PunchFlag** (root, one per punch per reason). Org-owned.
- Reasons: outside fence, branch not allowed (EM-07), no shift or weekly off or holiday (PC-06), late sync (PC-07), clock mismatch (PC-08), location unavailable (PC-13), low accuracy (PC-14), IP check failed (PC-05).
- Members: reason, evidence values, `counts_before_decision` (DP-05), decision (accepted or rejected, by whom, when, why).
- A rejection does **not** delete the punch. The punch stays as evidence, and the DayResult excludes it (BR-01).

Blocked punches (for example web with no location under PC-13 = block) are **not** RawPunches: nothing was accepted. Each attempt is recorded as a `PunchAttempt` audit entry so the employee and HR can see why it was refused (E18).

### 3.6 Scheduling

**Shift** (root, versioned). Org-owned. Type: fixed, flexible, flexible with core hours, split, night or open (SH-R1, SH-01). It holds start and end or the flexible band, core hours, required hours, window before and after (SH-07), the half-day split point (SH-11), break rule references, and the night attendance-date rule (SH-06). Changes create a new version (G5). Split-shift shape is open (OQ-C05-01).

**Roster** (root per employee per date range). Roster entries are employee, date, shift version (or weekly off), and source (screen, upload, copy or rotation). Whether two shifts per day are allowed is open (OQ-C05-06).

**WeeklyOffPattern**: fixed days, alternating (for example the 2nd and 4th Saturday) or from the roster (SH-09).

**HolidayCalendar**: holidays (national, festival, optional) for a location (HD-R1), and the employee's optional-holiday choices (HD-R3, HD-01).

**ShiftChangeRequest**: swap or change requests (SH-12), approved through the approval service.

**ExpectedSchedule** (value, derived): for an employee and attendance date, it says what was expected: shift version, window, weekly off or holiday. It is resolved by a Scheduling domain service, and the engine receives it as a snapshot.

### 3.7 Policy

**PolicySet** (root). Org-owned.
- Members: name, and a list of **PolicyVersions** (immutable once in use, effective-from date, a settings document per rule group with each group's master switch: G2) validated against the settings schema (TSD §5.4), and **PolicyAssignments** (level: organisation, branch, department, category, shift or employee; target; from and to dates).
- Invariants:
  - A version in use is never edited (G5).
  - Effective periods of one set's versions do not overlap.
  - Assignments at the same level and target do not overlap in time.
- Resolution: **most specific assignment wins, using the version in force on the attendance date** (BR-05). How levels rank against each other is **DP-03**.
- Preview (G8): a policy version can be run against the last period's inputs **without** storing results. It produces a difference report only.

**Exemption**: rule group, scope (category, department, employee type, employee or date) and dates (G7, LA-22).

The settings catalogue (about 360 settings) is data in the policy schema, not domain classes. The domain recognises **rule groups** (PR, BK, LA, EC, EA, MP, DS, RD, RG, LM, LV, PM, OD, HW, PC …) as the unit that is switched on or off and versioned.

### 3.8 Attendance

**AttendanceDay** (root, one per employee per attendance date). Org-owned.
- Members:
  - a **DayResult** history, append-only. Each DayResult holds:
    - inputs: punch IDs used (with assigned direction and session pairing), flag decisions applied, ExpectedSchedule snapshot, policy version ID(s), overlay references (leave, permission, on-duty, holiday, weekly off, waiver, regularisation), and an input fingerprint
    - outputs: sessions, gross hours, break minutes, net hours, late minutes, early minutes, early-arrival minutes, marks (late or early), slab actions, **day status** (present, half day, quarter day, absent, leave, holiday, weekly off, on-duty … per DS), flags and exceptions, identified overtime minutes (computed only), and **day effects** (fractions of deduction days, with their reason)
    - an **Explanation**: ordered steps listing each rule applied and why (BR-08)
    - the engine version, the time it was produced and the trigger that produced it
  - a processing state (STATE_MACHINES SM-DAY).
- Invariants:
  - At most one **current** DayResult. Earlier ones are superseded, never edited (BRX-03).
  - Same inputs give the same outputs (BR-02, AT5). The fingerprint lets a re-run that produces an identical result be skipped.
  - A day in a locked period cannot get a new current DayResult. Corrections go to PeriodAdjustment (BR-06).
  - Results contain **no money** (BRX-02).
  - One shortfall has at most one day effect (BR-04).
  - A missing check-out is a missed punch, never early (BR-09).

**RegularisationRequest** (root). Type (missed punch, late, early, wrong punch, whole day: RG-02), date, proposed correction, reason and attachment (RG-09), approval state (RG-05/06/07). When approved it produces either a manual RawPunch or an override record that the engine reads. It never edits a punch.

**HRCorrection**: a direct correction by HR with a reason (RG-10). It is treated as a regularisation that HR approved itself, and it is audited. BR-07 does not apply because no request was raised. Whether that is acceptable is **DP-08**.

**BulkWaiver** (root). Scope (date and branch, location or shift), rule group (late or early), reason, set by HR (LA-17, EC-16, RG-11). Creating or withdrawing one triggers recalculation of the affected days (E14).

**PeriodCounterSet** (root per employee per counter period). Late and early marks and minutes counted so far, grace uses (LA-04, EC-03), and permission usage cross-references. The counter period follows LA-18 / EC-17. It is **derived**: rebuilt from the current DayResults plus the opening counter (OB-R2). Period-end penalties (LA-06..12, EC-05..11) are applied at period end as **PeriodPenalty** outcomes with explanations (BR-10).

### 3.9 Leave & Absence

**LeaveType** (root, versioned): paid or unpaid, eligibility, credit method, accrual timing, pro-rating, cap, year-end handling, negative allowance, half-day allowed, notice, maximum days, after-the-event window and document rule (LM-01..15).

**LeaveAccount** (root per employee per leave type). A **ledger** of entries: opening balance, accrual, grant, debit (request), reversal (cancellation), sandwich debit, penalty debit (LA-12), auto-debit for absence (DS-10 / LM-28), carry forward, lapse, encashment (V2), and adjustment with a reason. The balance is the sum (LM-R4). Invariant: the balance does not go below 0 unless the leave type allows it (LM-07). Whether pending requests reserve balance is open (OQ-C08-08).

**LeaveRequest** (root): employee, type, dates and half-day parts, reason and document, approval state (SM-REQ). Approval posts the ledger debit and triggers recalculation of the affected days. Cancellation posts a reversal.

**AbsenceClassification** (per AttendanceDay, owned by Attendance but decided with Leave data): informed leave, unpaid leave or uninformed absence (LM-R6). A day moves from uninformed to informed when leave is approved later (LM-27).

**AbscondingCase** (root): opened when the trigger fires (LM-30), with a pay-hold flag (LM-31), closed by HR returning the employee to active or starting offboarding (LM-32). SM-ABS.

**CompOffAccount** (root per employee): a ledger of **credits** (worked date, half or full, expiry date: HW-04, HW-05), debits (taken like leave: CO-R2), and expiry outcomes (lapse in V1; pay one extra day in V2: HW-06). **CompOffClaim**: the approval for a worked holiday or weekly off (CO-R1, HW-02), or the employee's choice (CO-R4).

### 3.10 Permission & On-duty

**PermissionType** (versioned): duration unit, minimum and maximum length, limit basis and period, caps, position, paid or not, counts-as-worked, notice, approval, over-limit handling, over-stay handling, eligibility, blackout dates, punch requirement (PM-02..30).

**PermissionAccount** (per employee per type per limit period): a usage ledger (count and minutes), including conversions from late or early (PM-30) and the opening "used this month" (OB-R2).

**PermissionRequest** (root): date, start and end, type, reason, approval state. **Origin**: applied for directly, or converted from a late arrival or early check-out (LA-15, EC-13).

**OnDutyRequest** (root): hours, half day, full day or several days, purpose, place, proof, punch requirement (OD-02..07). It never uses the permission limit (OD-12).

Work-from-home requests are **DP-09** (no BRD module defines them: G-E).

### 3.11 Period

**AttendancePeriod** (root). Org-owned.
- Members: **cycle owner** (DP-01), start and end dates, cut-off (PL-02), pre-lock checklist snapshot (PL-R2), lock state and history (SM-PERIOD), and the **PeriodTotals** per employee frozen at lock (layer 5: payable days, deduction days by reason, late and early minutes, identified overtime hours: none in V1, D-09).
- Invariants:
  - Once locked, its totals and every DayResult in it are frozen (BR-06, PL-R3).
  - Unlock happens only if PL-06 allows it, with Organisation Admin approval and a reason. What happens to adjustments and exports already posted is open (OQ-C12-07).

**PeriodAdjustment** (root): created when an input changes for a date in a locked period. It holds the original date, the locked result reference, the new result reference, the **difference only**, and the target open period (PL-R4, E15, AT8). It can be created only within PL-07 periods back (OQ-C12-04 decides older ones).

**PayrollInputExport** (root): an immutable snapshot of PeriodTotals plus the adjustments posted into the period, produced for MD-06. From V2 the Payroll context reads the same contract (`PayrollInput v1`, §7).

### 3.12 Opening Balances

**OpeningBalanceBatch** (root): entries (employee, balance kind, value, as-of date) for leave per type, comp-off, permissions used, and late and early counters (OB-R2). State is draft → confirmed (OB-R5). Confirming posts opening entries into the relevant ledgers and counters. A later change posts a reversal plus a new entry with a reason (OB-02). Nothing is overwritten.

### 3.13 Notifications, Audit

**NotificationRule**: event (NT-01..11), channel and recipient switches. **Notification**: one message instance, with delivery attempts. Defaults are open (OQ-C14-01).

**AuditEntry**: insert-only (§16, TSD §15). It records actor (user, vendor user under a support grant, or system job), organisation, action, target, time, reason where required, and before and after values or a change reference. **SettingsChange** is the G9 view of audit entries for settings and policy versions.

---

## 4. Cross-context domain services

| Service | Context | Does | Pure? |
|---|---|---|---|
| **PolicyResolver** | Policy | (employee, date) → the PolicyVersion in force (BR-05, DP-03) | Yes, given assignments |
| **ScheduleResolver** | Scheduling | (employee, date) → ExpectedSchedule, including the night-shift attendance date (SH-06) and holiday or weekly-off facts | Yes, given data |
| **PunchAttributor** | Attendance | Assigns raw punches to attendance dates using the shift window (SH-07, DP-04) | Yes |
| **DayEngine** | Calculation library | Daily process steps 1–8 (BUSINESS_RULES §3.1) → DayResult and Explanation | **Yes**, no I/O (TSD §5.2) |
| **PeriodEngine** | Calculation library | Period-end penalties, absence cuts, totals (BUSINESS_RULES §3.3) | Yes |
| **GeoEvaluator** | Time Capture | Distance to applicable fences, inside or outside, accuracy check (PC-12..17) | Yes |
| **ApprovalService** | Shared | Routing (AC-02), levels, delegation (AC-03), escalation timers, the BR-07 check | No (timers) |
| **RecalculationScheduler** | Attendance | Turns "input changed" into idempotent (organisation, employee, date) recalculation work (BR-03, TSD §5.2) | No |
| **LedgerPoster** | Leave, Permission, CompOff | Posts entries idempotently, keyed by their cause (request ID, day result ID, job run) | No |

The DayEngine and PeriodEngine receive an **input snapshot** and return results. The API, jobs, preview (G8) and tests all call the same library, so results cannot differ between them (TSD §3.3).

---

## 5. How a change flows (V1)

```
RawPunch saved ─┐
Request approved ┤
Waiver / RG / HR ┼─► RecalculationScheduler ─► (org, employee, date) work item (idempotent)
Policy/roster    ┤                                 │
Opening balance ─┘                                 ▼
                                PolicyResolver + ScheduleResolver + inputs ─► DayEngine
                                                                     │
                                       period open? ── yes ─► new current DayResult (old superseded)
                                                    └─ no ──► PeriodAdjustment (difference) in next open period
                                                                     │
                                                    ledgers / counters / notifications updated
Period end (job) ─► PeriodEngine ─► PeriodPenalty + PeriodTotals ─► lock ─► PayrollInputExport
```

---

## 6. Requests and approvals

Request aggregates: LeaveRequest, PermissionRequest, OnDutyRequest, RegularisationRequest, CompOffClaim, ShiftChangeRequest, PunchFlag decision, period unlock, SupportAccessGrant, settings change (AC-04 with approval), opening-balance confirmation (OB-01 with approval).

- Each request owns its business lifecycle and side effects (posting ledger entries, triggering recalculation).
- The ApprovalService owns: who the approver is at each level, delegation, escalation, auto-approve and auto-reject on timeout, and **BR-07: the approver can never be the requester**, including through delegation.
- Every approval action is an AuditEntry with actor, time, decision and comment.

---

## 7. Contract consumed by Payroll: `PayrollInput v1`

Produced by the Period context. It is the only way Payroll (V2) or an external payroll (MD-06) reads attendance.

Per employee per period: payable days, deduction days **by reason line** (LOP, uninformed-absence extra cut, excess informed, excess uninformed, late/early penalty, unearned weekly off: V2), late minutes and early minutes after grace and waivers (PD-07), identified overtime hours (null in V1: D-09; M01 is V3), comp-off expiries due for payout (V2), absconding pay-hold flag (LM-31), branch-costed day counts (EM-08), and adjustments from earlier periods tagged with their original date (PL-R4). Every line carries references to its Explanation sources (BR-08).

The contract is versioned. Payroll never reads DayResults directly.

---

## 8. Tenant ownership and isolation

- Every aggregate in contexts 2–14 is org-owned. Platform aggregates are vendor-owned, and Organisation is the tenant root.
- The organisation is derived from the authenticated principal (TB-01, TSD §4.1) or, for device pushes, from the registered device's organisation. It is never taken from the request body.
- Cross-organisation references are invalid for every org-owned object. The only crossing is a SupportAccessGrant, which is time-boxed and audited.
- Background work items carry exactly one organisation (TSD §4.2).

---

## 9. Effective dating and versioning summary

| Object | Mechanism | Source |
|---|---|---|
| PolicyVersion, LeaveType, PermissionType, Shift | Immutable versions with effective-from | G5 |
| PolicyAssignment, Employee assignments, ModuleEntitlement, employee limit | From and to ranges, history kept | G3, EM-R6, VC-R2 |
| DayResult | Append-only; current pointer | BR-03, BR-08 |
| Ledgers | Append-only entries | LM-R4, OB-R5 |
| Locked periods | Frozen; differences only | BR-06 |
| OrganisationSettings | Change history (G9); whether settings are effective-dated is DP-10 | G9 |

---

## 10. Audit requirements by object

Audited with before and after values or a change reference: organisation, plan, module and limit changes; support access and every action under it; user, role, scope and delegation changes; employee assignment changes; device registration and enrolment mapping; manual punches; flag decisions; policy versions and assignments; exemptions; shift, roster and holiday changes; all request submissions and approval actions; waivers; HR corrections; opening balances; period lock and unlock; adjustments; exports produced and downloaded; settings changes (G9); and refused punch attempts.

---

## 11. Open decision points

Decisions taken are logged in `../requirements/DECISIONS.md`.

Each DP is an unanswered Gate 1 question that affects the model. The model contains the **slot**; the answer fills it.

| DP | Question | Slot in the model | Source |
|---|---|---|---|
| DP-01 | What owns a V1 attendance period's cycle (organisation, branch or policy set)? **Decided D-02: organisation.** | `AttendancePeriod.cycle owner`. V2 adds payroll group as an owner kind. | G-A |
| DP-02 | Which leave-debit order governs penalties and absences (LA-12/EC-11 vs LV-04), and what if the named type does not exist? **Decided D-06: LA-12/EC-11.** | Debit-order value inside the policy version, read by LedgerPoster | G-B |
| DP-03 | Ranking between branch, department, category and shift assignments **Decided D-03: employee > shift > department > category > branch > organisation.** | PolicyResolver precedence list | OQ-X-03 |
| DP-04 | Which time zone and calendar apply when punching at another branch **Decided D-04: branch where the punch happened.** | PunchAttributor and ScheduleResolver input: "schedule zone" | G-J |
| DP-05 | Which flag reasons count before decision, who decides each, and what a rejection does **Decided D-05: per flag type; any non-Employee role can decide (defaults to confirm).** | `PunchFlag.counts_before_decision` and approver routing per reason | OQ-C04-01/02 |
| DP-06 | DS-10 vs LM-28 for an absent day with no leave **Decided D-07: DS-10.** | Single "unexplained absence" rule in the policy, one owner | G-C |
| DP-07 | Enrolment ID uniqueness scope | EnrolmentMapping uniqueness | G-G |
| DP-08 | Does a direct HR correction (RG-10) need a second person? | HRCorrection approval path | BR-07, RG-10 |
| DP-09 | Work-from-home request and approval | Possible WFHRequest aggregate in Permission & On-duty | G-E |
| DP-10 | Are organisation-wide settings effective-dated like policy versions? | OrganisationSettings versioning | G5, G9 |
| DP-11 | Does overtime identification run when M01 / OT-01 is off? **Decided D-09: no; V1 never identifies or exports overtime.** | DayEngine overtime step guard | G-N |
| DP-12 | Is Pub/Sub or Redis needed? Not assumed; recalculation uses Cloud Tasks **Closed by ADR-002.** | Infrastructure only, no domain impact | OQ-X-09 |
