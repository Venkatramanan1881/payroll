# State machines (V1)

Status: Draft for Approval Gate 2
Sources: BRD v0.7 (authoritative), TSD v0.1 §10 (job names only), `DOMAIN_MODEL.md`, `../product/BUSINESS_RULES.md`, story files in `../product/USER_STORIES/`, `../requirements/OPEN_QUESTIONS.md`
Phase: 2. No code, schema or API.

Each lifecycle below is **independent** (DOMAIN_MODEL §1.5). No aggregate has a shared generic `status`.

---

## 0. Conventions (apply to every machine)

**Actors** (BRD §5 only): **VA** Vendor Admin · **OA** Organisation Admin · **HR** HR Manager · **PO** Payroll Officer · **MGR** Manager (Approver) · **SUP** Supervisor · **EMP** Employee · **AUD** Auditor (never triggers a transition). **SYS** = system job; TSD §10 job number given where one exists (J1 nightly finalisation, J3 escalations, J4 device offline, J6 comp-off expiry, J7 period-end and pre-lock checklist, J8 absconding, J10 renewal and limits). Vendor sub-roles (VC-05) are named only where VC-R6 needs them (OQ-V01-14).

**Audit** (§16, G9, DOMAIN_MODEL §10): every transition, manual or automatic, writes one AuditEntry: actor (user, vendor user under grant, or job run), organisation, time, from-state, to-state, reason where required, before/after or change reference. Tables list only *extra* audit duties.

**Idempotency** (TB-03, AT21, TSD §5.5): every command names its target and is checked against the current state.
- Command whose target state already holds with the same parameters → returns the existing outcome; **no** second side effect (ledger posts are keyed by cause: LedgerPoster).
- Command from a state not listed as a valid "From" → refused as an illegal transition; nothing changes.
- Timed jobs re-run for the same date → no change (guard on state + due date).
- Concurrent conflicting commands → first committed wins; the second sees the new state and is refused or returns it.

**Tenant guards**: every transition is inside one organisation (TB-01). While the Organisation is **Suspended** (SM-ORG), every manual command below is refused except view/export (VC-R4); whether system jobs keep running is OQ-V01-08. A disabled module or rule group (MD-03, BR-14) hides its commands; what happens to its in-flight items is **GAP-SM-17**.

**Lock guard**: no transition may create or change a result for a date in a Locked period (BR-06, PL-R3). The change goes to a PeriodAdjustment (SM-ADJ) instead.

**Gap marker**: "GAP → id" means the BRD does not decide it. Existing ids are OQ-xxx / DP-xx; new ones are GAP-SM-nn (listed in §15).

---

## 1. SM-ORG Organisation

| State | Meaning |
|---|---|
| Active | Normal operation. |
| Suspended | Punches still stored; users can only view and export (VC-R4). |

No closed/terminated state exists in the BRD → GAP-SM-01.

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| O1 | — | CreateOrganisation | Active | VA | Sector template, plan, modules, employee limit, first admin user (VC-R1); A1 (no self-registration) | Template copied (OQ-V01-10); OA user created; `OrganisationCreated` | Manual |
| O2 | Active | Suspend | Suspended | VA | VC-04 any value | `OrganisationSuspended`; manual commands refused | Manual |
| O3 | Active | AutoSuspend | Suspended | SYS (no TSD job named; J10 nearest → GAP-SM-01) | VC-04 = automatic; renewal date + N days passed; "not renewed" undefined (OQ-V01-09) | as O2 | Auto |
| O4 | Suspended | Reactivate | Active | VA | — | `OrganisationReactivated` | Manual |

Plan, module and limit changes (VC-R2, MD-01) are dated history, **not** states.

- Terminal: none.
- Idempotency: Suspend on Suspended / Reactivate on Active → no-op, current state returned. O3 re-run → no-op.
- Illegal: OA/HR/any org user suspends or reactivates; any org user changes data while Suspended (VC-R4); VA changes organisation results in any state (§5).
- Gaps: OQ-V01-08 (jobs and app/web punches while suspended), OQ-V01-09, GAP-SM-01.

---

## 2. SM-SUPPORT SupportAccessGrant

| State | Meaning |
|---|---|
| Requested | Waiting for OA approval (VC-03 = on). |
| Active | Vendor user may open the organisation until window end; every action logged and visible to OA (VC-R6). |
| Rejected | OA refused. Terminal. |
| Expired | Window ended. Terminal. |
| Ended | Ended before window end. Terminal. Existence → OQ-V01-13. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| S1 | — | RequestAccess | Requested | VA / VC-05 vendor user (OQ-V01-14) | VC-03 = on; reason; window length (OQ-V01-13) | Notify OA; `SupportAccessRequested` | Manual |
| S2 | — | RequestAccess | Active | as S1 | VC-03 = off | `SupportAccessGranted`; OA can see grant | Manual |
| S3 | Requested | Approve | Active | OA of that organisation | Approver is not a vendor user | `SupportAccessGranted` | Manual |
| S4 | Requested | Reject | Rejected | OA | — | `SupportAccessRejected` | Manual |
| S5 | Active | WindowEnds | Expired | SYS (time check on every access) | now ≥ window end | Access refused from then on | Auto |
| S6 | Active | EndEarly | Ended | OA? → OQ-V01-13 | — | Access refused | Manual |
| — | Requested | (no answer) | ? | — | GAP-SM-02 | — | — |

- Extra audit: every action under Active carries the grant id and is visible to OA (VC-R6).
- Reopen: none. VC-03 "each time" → a new grant is a new request (S1). Extension not in BRD.
- Idempotency: Approve on Active → returns grant; Approve on Rejected/Expired → refused.
- Illegal: any access with no Active grant (TB-01); vendor user approving own grant; VA changing results (§5). Write actions under a grant: GAP → OQ-V01-12 (not decided here).
- Gaps: OQ-V01-12, OQ-V01-13, OQ-V01-14, GAP-SM-02.

---

## 3. SM-EMP Employee employment status (EM-R5)

The absconding flag is **not** a state here (see SM-ABS).

| State | Meaning |
|---|---|
| On probation | Employed, probation running (EM-04). Relation to employee type "probation" → OQ-C01-05. |
| Active | Employed, confirmed. |
| On notice | Employed, serving notice (LM-15, PM-25 refer to it). |
| Exited | Employment ended at exit date. Terminal for this record. |

Every state change carries an effective date (EM-R5 "with dates").

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| E1 | — | CreateEmployee (screen or upload EM-R7) | On probation or Active | HR | Mandatory fields (EM-02); code (EM-01); one master branch (EM-R9); joining date; VC-01 limit: block → refused, allow → `EmployeeLimitExceeded` alert to vendor | Enrolment = Not enrolled (SM-ENROL); `EmployeeCreated`. Which initial state → GAP (OQ-C01-05) | Manual |
| E2 | On probation | Confirm | Active | HR | Confirmation reminder from EM-04; automatic or manual → OQ-C02-06 | Eligibility "after probation" (LM-01, PM-25) re-evaluated from date; recalculation if a policy depends on it | Manual (auto → OQ-C02-06) |
| E3 | Active | StartNotice | On notice | HR | Trigger and fields not in V1 BRD (DC-R4 is V3) → GAP-SM-03 | LM-15 applies to leave from date | Manual |
| E4 | On probation | StartNotice | On notice | HR | Allowed? → GAP-SM-03 | as E3 | Manual |
| E5 | On notice | Exit | Exited | HR (SYS on notice end? → GAP-SM-03) | Exit date ≥ joining date | Assignments end at exit date; leaver pro-rating (LM-04, PM-24); punches after exit → OQ-C02-07; user access → OQ-C03-12; limit count → OQ-V01-06; `EmployeeExited` | Manual |
| E6 | Active / On probation | Exit (direct, incl. LM-32 offboarding) | Exited | HR | as E5 | as E5 | Manual |
| E7 | On notice | ExtendNotice | On notice | SYS on leave approval | LM-15 = extends notice | Notice end moves (unit → OQ-C08-07) | Auto |
| E8 | Exited | Rejoin | new record or reopened | HR | EM-05 code rule; reopen vs new → OQ-C02-14 | — | Manual |

- Terminal: Exited (rejoin per OQ-C02-14).
- Reversal: notice withdrawn (On notice → Active) not in BRD → GAP-SM-03. Exit reversal not in BRD.
- Idempotency: same target state and same date → no-op.
- Illegal: any change by MGR, SUP, EMP, PO, AUD, VA; effective date before joining or after exit; Exited → On probation/Active/On notice on the same record (unless OQ-C02-14 says reopen); back-dating into a locked period (OQ-C02-08 → otherwise adjustment).
- Gaps: OQ-C01-05, OQ-C02-06, OQ-C02-07, OQ-C02-08, OQ-C02-14, OQ-C03-12, OQ-V01-06, GAP-SM-03.

---

## 4. SM-ENROL Enrolment status (PC-R8)

| State | Meaning |
|---|---|
| Not enrolled | No fingerprint/face enrolment mapped. Device punches cannot resolve to this employee. |
| Enrolled | Enrolment ID mapped (PC-R3); device punches resolve to the employee for the mapped dates. |
| Re-enrolment needed | Mapping exists but enrolment must be redone. Whether punches still resolve meanwhile: not stated → OQ-C04-12. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| N1 | — | EmployeeCreated | Not enrolled | SYS | — | — | Auto |
| N2 | Not enrolled | RecordEnrolment / MapEnrolmentId | Enrolled | HR (PC-R8) | ID unique within scope (DP-07, OQ-C02-03) on the date | Earlier unmapped punches with that ID → OQ-C04-04; `EnrolmentMapped` | Manual |
| N3 | Enrolled | MarkReEnrolmentNeeded | Re-enrolment needed | ? → OQ-C04-12 | — | Shown to HR | ? |
| N4 | Re-enrolment needed | RecordEnrolment | Enrolled | HR | as N2 | — | Manual |
| N5 | Enrolled / Re-enrolment needed | Unmap | Not enrolled | ? | Not in BRD → GAP-SM-04 | — | — |

- Extra audit: mapping changes with dates (DOMAIN_MODEL §10).
- Idempotency: mapping the same ID to the same employee for the same dates → no-op.
- Illegal: mapping an ID already mapped to another employee in scope on the same date (DP-07); any actor other than HR (PC-R8).
- Gaps: DP-07, OQ-C04-04, OQ-C04-12, GAP-SM-04 (also: status per employee vs per device mapping).

---

## 5. SM-DEVICE Device online / offline (PC-R2, PC-10)

| State | Meaning |
|---|---|
| Online | Device has contacted the server recently. |
| Offline | No contact within the offline threshold. |

Registration (PC-R2) creates the Device; there is no BRD lifecycle for deregistration, move or replacement → GAP-SM-05.

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| D1 | — | RegisterDevice | Offline until first contact (initial state not in BRD → GAP-SM-05) | HR (§5 device responsibility) / OA (branches) → OQ-C01-01 | Serial unique globally (TSD §9.1); branch exists | `DeviceRegistered` | Manual |
| D2 | Offline | ContactReceived (push or heartbeat) | Online | SYS (gateway) | Registered serial | Last-contact updated; queued punches ingested (PC-R7) | Auto |
| D3 | Online | NoContact | Offline | SYS J4 | Offline threshold not in BRD → GAP-SM-05 | `DeviceWentOffline` | Auto |
| D4 | Offline | OfflineTooLong | Offline (alerted) | SYS J4 | Offline > PC-10 N minutes; alert not yet sent for this offline spell | Alert HR (PC-R10; channel per OQ-C14-06); counts in VC-R3/VC-R8 | Auto |

- Idempotency: D4 sends **one** alert per offline spell; J4 re-runs do not resend.
- Illegal: push from unregistered serial → refused and logged, no state (DOMAIN_MODEL §3.5).
- Device failure handling is manual attendance (PC-R9), not a device state.
- Gaps: OQ-V01-15, OQ-C14-06, GAP-SM-05.

---

## 6. SM-FLAG PunchFlag decision

One PunchFlag per punch per reason. Reasons and their sources: outside fence (PC-04), branch not allowed (EM-07), weekly off / holiday / no shift (PC-06), late sync (PC-07), clock mismatch (PC-08, TSD §9.3), location unavailable (PC-13), low accuracy (PC-14), IP check (PC-05/PC-15, flag-vs-block not stated → OQ-C04-11). A "block" outcome creates **no** RawPunch and no flag; it is a PunchAttempt audit entry (E18). Device "block" → OQ-C04-06.

| State | Meaning |
|---|---|
| Raised | Awaiting decision. Whether the punch counts meanwhile → DP-05 (E18: outside-fence does not count). |
| Accepted | Punch counts in the DayResult. Terminal. |
| Rejected | Punch kept as evidence, excluded from the DayResult (BR-01). Terminal. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| F1 | — | PunchStored with flag condition | Raised | SYS (ingestion) | Setting for that reason = record and flag / flag | NT-08; `PunchFlagged`; day recalculation per DP-05; day appears in pre-lock checklist (PL-R2) | Auto |
| F2 | Raised | Accept | Accepted | Approver per reason (E18: MGR; others → DP-05 / OQ-C04-02) | Approver ≠ punch's employee (BR-07); approver in scope | Day recalculated (BR-03); `PunchFlagAccepted` | Manual |
| F3 | Raised | Reject | Rejected | as F2 | Reason (DOMAIN_MODEL §3.5) | Day recalculated excluding punch; `PunchFlagRejected` | Manual |
| F4 | Raised | No decision in time | ? | — | No escalation setting → OQ-C04-02 | — | — |

- Lock: a decision for a date in a locked period → PeriodAdjustment (OQ-C12-10); a Raised flag at lock follows PL-05.
- Reversal: Accepted ↔ Rejected not in BRD → GAP-SM-06. Fence change re-evaluation → OQ-C04-15.
- Idempotency: Accept on Accepted → returns decision, no second recalculation request beyond the idempotent work item.
- Illegal: employee decides own flag (BR-07); deleting the punch on Reject (BR-01); decision by AUD/SUP/VA.
- Gaps: DP-05, OQ-C04-01, OQ-C04-02, OQ-C04-06, OQ-C04-11, OQ-C04-15, OQ-C12-10, GAP-SM-06 (also multiple flags on one punch).

---

## 7. SM-DAY AttendanceDay processing state

This is the processing state of the day. DayResults are a separate append-only history (BRX-03); every recalculation adds a new current DayResult **without** changing this state unless the table says so.

| State | Meaning | BRD basis |
|---|---|---|
| Provisional | Day not yet finalised by J1; results recalculated on every input. | **Not in BRD**; TSD J1 only → GAP-SM-07 |
| Exception | Current DayResult has an open exception sent to regularisation: MP-01/MP-02 = flag; MP-05 auto check-out (always flagged); PR-05; PR-06; BK-07 flag; EC-14 flag for HR; LV-01 flag; LA-05 hold check-in (OQ-C06-07); Raised PunchFlag (DP-05). | §9.6 step 5 and 8 |
| Absence pending | Absent with no leave; held for N days (DS-10 = pending). | DS-10; DP-06 |
| Finalised | Processed, no open exception or pending hold. | BRD §2 success measure uses "finalised" (undefined) → GAP-SM-07 |
| Locked | Period locked; frozen (PL-R3). Basis: actual, or assumed present for days after cut-off (PL-03), or pending treated per PL-05. | PL-R3, PL-03, PL-05 |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| A1 | — | First input for date or J1 run | Provisional | SYS | Employee employed on date | DayResult v1 | Auto |
| A2 | Provisional | Finalise (previous day) | Finalised / Exception / Absence pending (by result) | SYS J1 | Date < today in branch zone (BR-11, DP-04) | MP-04 next-morning reminder (NT-01); NT-02 timing → OQ-C14-05 | Auto |
| A3 | Finalised | Recalculated with new exception | Exception | SYS (RecalculationScheduler) | Period open | New DayResult | Auto |
| A4 | Exception | Regularisation approved / HR correction (RG-10, DP-08) / bulk regularisation (RG-11) / flag decided / waiver (LA-17) | Finalised (or stays Exception if another remains) | SYS after the approving actor | Period open; RG-04 window | New DayResult; counters rebuilt | Auto |
| A5 | Exception | Cut-off reached unresolved | Finalised (MP-03 outcome recorded) | SYS | Which cut-off → OQ-C06-03 / G-K | New DayResult with MP-03 explanation | Auto |
| A6 | Absence pending | Leave approved within hold | Finalised | SYS | — | Absence classified informed (LM-R6) | Auto |
| A7 | Absence pending | N days elapse | Finalised | SYS (no TSD job named) | DS-10 N; calendar/working → OQ-C06-05 | LOP or auto-debit (DS-10 vs LM-28 → DP-06); LedgerPoster keyed by day | Auto |
| A8 | Finalised / Exception / Absence pending | Period locked | Locked | SYS on SM-PERIOD P5 | PL-05 decides treatment of Exception / pending days | Current DayResult frozen; pending at lock → OQ-C06-05, OQ-C12-03 | Auto |
| A9 | Locked | Period unlocked | prior state re-evaluated | SYS on SM-PERIOD P7 | PL-06 ≠ never | → OQ-C12-07 | Auto |
| A10 | Locked | Any changed input | Locked (unchanged) | SYS | — | PeriodAdjustment created (SM-ADJ); days assumed present under PL-03 → OQ-C12-02 | Auto |
| A11 | Any non-Locked | Recalculation, same fingerprint | unchanged | SYS | — | Skipped (BR-02, AT5) | Auto |

- Terminal: Locked (except unlock A9).
- Idempotency: J1 twice for the same date → no change; recalculation work item keyed (org, employee, date); identical inputs → no new DayResult.
- Illegal: new current DayResult while Locked (BR-06); editing a DayResult (BRX-03); setting a day status by hand outside regularisation / HR correction (BR-01, RG-10); treating a missing check-out as early (BR-09).
- Gaps: DP-04, DP-05, DP-06, DP-08, OQ-C06-03, OQ-C06-05, OQ-C06-07, OQ-C12-02, OQ-C12-03, OQ-C12-07, OQ-C14-05, GAP-SM-07.

---

## 8. SM-REQ Approval request (shared)

Used by LeaveRequest, PermissionRequest, OnDutyRequest, RegularisationRequest, CompOffClaim, ShiftChangeRequest. Routing, levels, delegation, escalation and BR-07 are the ApprovalService's (DOMAIN_MODEL §6); side effects belong to each request type (§8.2).

| State | Meaning |
|---|---|
| Draft | **Not in BRD** → GAP-SM-08. Not modelled. |
| Pending L1 | Submitted; with level-1 approver (AC-02 route). NT-05 "submitted". |
| Pending L2 | Level 1 approved; with HR (two-level settings RG-05, LM-13, PM-16, OD-05). |
| (escalated) | Flag on Pending L1/L2, not a separate state: routed onward after timeout (NT-05 "escalated"). Target → G-F (OQ-C08-09, OQ-C09-06). |
| Approved | Final approval, manual or automatic (basis recorded: approver, auto on submit, auto on timeout). |
| Rejected | Final refusal, manual or auto on timeout. Terminal. |
| Cancelled | Cancelled before or after approval. Terminal. "Withdrawn" is not a BRD term → treated as Cancel from Pending (GAP-SM-08). |

Refused at submission (window, limit = block, eligibility, balance, blackout) creates **no** request.

### 8.1 Transitions

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| R1 | — | Submit | Pending L1 | EMP (requester) | Type enabled; type-specific checks (§8.2); route resolved; approver ≠ requester else → OQ-C03-08 | NT-05 submitted; `RequestSubmitted`; reservation → OQ-C08-08 / OQ-C09-04 | Manual |
| R2 | — | Submit | Approved | EMP + SYS | Auto-approve on submit (PM-16 within limit, RG-06 types, HW-02 off) | Approve side effects (§8.2) | Auto |
| R3 | — | Submit (over limit, route = HR only) | Pending L2 (HR) | EMP | RG-08 / PM-18 = HR approval | as R1 | Manual |
| R4 | Pending L1 | Approve | Pending L2 | L1 approver or delegate (AC-03) | Two levels; approver ≠ requester (BR-07, incl. delegate) | `RequestApprovedAtLevel` | Manual |
| R5 | Pending L1 / L2 | Approve (final level) | Approved | approver or delegate | BR-07; re-validate balance/limit at decision (LM-07) | NT-05; approve side effects | Manual |
| R6 | Pending L1 / L2 | Reject | Rejected | approver or delegate | Reason; BR-07 | NT-05 | Manual |
| R7 | Pending L1 / L2 | Timeout = escalate | same state, escalated | SYS J3 | N days/hours elapsed (LM-13, RG-07, PM-17, OD-05); counting → G-F; level 2 → GAP-SM-10 | NT-05 escalated; new approver | Auto |
| R8 | Pending L1 / L2 | Timeout = auto-approve | Approved | SYS J3 | as R7 | as R5 (BR-07 not breached: rule, not requester — C06 note) | Auto |
| R9 | Pending L1 / L2 | Timeout = auto-reject | Rejected | SYS J3 | as R7 | NT-05 | Auto |
| R10 | Pending L1 / L2 | Cancel | Cancelled | per type (§8.2) | per type | Release reservation, if any | Manual |
| R11 | Approved | Cancel | Cancelled | per type (§8.2) | per type; date in locked period → adjustment | Reverse side effects (§8.2) | Manual |
| R12 | Approved | Change | ? | per type | PM-29 only; fresh approval → OQ-C09-10 | — | Manual |
| R13 | Pending L1 / L2 | Period locked | ? | SYS | PL-05; RG-04 "before lock" → OQ-C12-03 | — | Auto |

- Bulk approve (LM-R3, OT-R3): one R4/R5 per request; per-item result.
- Resubmission after Rejected: not in BRD → GAP-SM-08 (a new request is the only path modelled).
- Terminal: Rejected, Cancelled. Approved is terminal except R11/R12.
- Idempotency: Approve on Approved → same result, **no** second ledger post or recalculation item; Reject on Rejected → same; Approve on Rejected/Cancelled → refused; J3 re-run → no double escalation (guard on level + deadline).
- Illegal (all types): requester or their delegate approves own request (BR-07); approver outside scope (AC-07); approve a level twice; skip L1 when two levels apply (except R3); any decision by AUD/SUP/VA; approve after lock creating a new DayResult in the locked period (→ adjustment only); edit a submitted request in place (change = R12 only).

### 8.2 Per request type

| Type | Levels | Auto on submit | Timeout setting | Who may cancel / until when | On approve | On cancel after approval | Gaps |
|---|---|---|---|---|---|---|---|
| LeaveRequest | LM-13: MGR / MGR→HR | — | LM-13 escalate / auto-approve / auto-reject | OQ-C08-10. Also SYS auto-cancel under LV-01 = auto-cancel when full hours worked | LeaveAccount debit (+ sandwich debit LM-16..20); recalc dates; uninformed → informed (LM-27); LM-21 extra cut reversed if LM-23; LM-15 notice extension | Reversal ledger entry (not delete: AC-C08-06.2); recalc; locked dates → adjustment | OQ-C08-08, OQ-C08-09, OQ-C08-10, OQ-M02-08 (comp-off taken as leave) |
| PermissionRequest | PM-16: MGR / MGR→HR | PM-16 auto within limit | PM-17 hours | PM-29: EMP before start time; HR after (reason) | PermissionAccount usage entry; recalc | Usage reversal; recalc; locked → adjustment | OQ-C09-04, OQ-C09-06, OQ-C09-10, OQ-C09-09; converted permission (LA-15/EC-13) → GAP-SM-11 |
| OnDutyRequest | OD-05: MGR / MGR→HR | — | OD-05 | OQ-C09-10 | Recalc (no ledger; OD-12); comp-off if OD-10 | Recalc from punches (AC-C09-11.6) | OQ-C09-06, OQ-C09-10, OQ-C09-12, OQ-C09-13 |
| RegularisationRequest | RG-05: MGR / MGR→HR | RG-06 types | RG-07 | Not in BRD → GAP-SM-09 | Manual RawPunch (BR-01) or override record; recalc; SM-DAY A4 | Not in BRD → GAP-SM-09 | OQ-C12-03; submission window RG-04, limit RG-03/RG-08 |
| CompOffClaim | 1 (approver → OQ-M02-05) | HW-02 off → credited by SYS | None → OQ-M02-05 | Not in BRD → GAP-SM-09 | CompOffAccount credit (SM-COMPOFF C2) | Not in BRD | Created by SYS when earned (AC-M02-03.1), not by EMP; OQ-M02-02, OQ-M02-04, OQ-M02-07 |
| ShiftChangeRequest | 1 (SH-12 approver) | — | None → OQ-C05-12 | Not in BRD → GAP-SM-09 | Roster change; recalc affected employees/dates | Not in BRD | OQ-C05-12 (swap consent, two approvers) |

---

## 9. SM-PERIOD AttendancePeriod

Cycle owner → DP-01 (G-A). The pre-lock checklist (PL-R2) is a computed view refreshed by J7, **not** a state; its snapshot is stored at lock.

| State | Meaning |
|---|---|
| Open | Accepting inputs; days recalculated (BR-03). |
| Past cut-off | PL-02 cut-off passed; days after it follow PL-03. Still open for inputs. |
| Awaiting lock | Period end passed; period-end rules applied (BR-10); shown as "waiting to be locked" (VC-R8, DB-R4). |
| Locked | Frozen (PL-R3, BR-06). |
| Unlocked | Reopened under PL-06; must be locked again. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| P1 | — | GeneratePeriod | Open | SYS (job not named → GAP-SM-12) | PL-01 per cycle owner (DP-01); transition period on PL-01 change → OQ-C12-09 | — | Auto |
| P2 | Open | CutOffReached | Past cut-off | SYS | PL-02 set (default when not set → OQ-C12-09) | MP-03 cut-off handling if OQ-C06-03 = PL-02 | Auto |
| P3 | Past cut-off | PeriodEndReached | Awaiting lock | SYS J7 | End date passed | PeriodEngine: penalties (LA-06..12, EC-05..11), absence cuts (LM-21..27), order → OQ-X-05; NT-07; NT-09 | Auto |
| P4 | Awaiting lock | Re-run on changed input | Awaiting lock | SYS J7 / recalculation | RG-04 window | Penalties and totals rebuilt (derived) | Auto |
| P5 | Awaiting lock | Lock | Locked | PO (V1 existence → OQ-C12-05 / G-D) or SYS on date (PL-04) | PL-05 = block → checklist empty (pending requests, flagged days, unresolved missed punches); order → OQ-C12-09; auto-lock blocked → OQ-C12-08 | Checklist snapshot; PeriodTotals frozen; all days → Locked (SM-DAY A8); PL-05 treatment of pending; PayrollInputExport (timing → GAP-SM-12); `PeriodLocked` | Manual / Auto |
| P6 | Locked | Lock | Locked | — | — | No-op | — |
| P7 | Locked | Unlock | Unlocked | Request: PO/HR? (not stated) · Approve: OA (PL-06) | PL-06 = with approval; reason; approver ≠ requester (BR-07) | Days re-evaluated (SM-DAY A9); adjustments and exports already made → OQ-C12-07 | Manual |
| P8 | Unlocked | Lock | Locked | as P5 | as P5 | as P5 | Manual / Auto |

- Terminal: Locked (unless PL-06 permits P7).
- Idempotency: Lock on Locked → returns the lock record, no second export; automatic lock job re-run → no-op.
- Illegal: lock before period end (not addressed → GAP-SM-12); lock with pending items when PL-05 = block; unlock when PL-06 = never; unlock without reason or OA approval; any new DayResult or total in Locked; lock or unlock by MGR/SUP/EMP/AUD/VA.
- Gaps: DP-01, OQ-X-04, OQ-X-05, OQ-C06-03, OQ-C12-01, OQ-C12-03, OQ-C12-05, OQ-C12-07, OQ-C12-08, OQ-C12-09, GAP-SM-12.

---

## 10. SM-ADJ PeriodAdjustment

| State | Meaning |
|---|---|
| Posted | Difference (vs locked result) computed and posted to the next open period, tagged with the original date (PL-R4, E15). Creation and posting are one step: the BRD has no approval or holding step. |
| Frozen | Target period locked; adjustment is part of its totals. |
| Exported | Included in a PayrollInputExport (MD-06). Terminal. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| Q1 | — | Changed input for a locked date | Posted | SYS | Source period Locked; within PL-07 periods (older → OQ-C12-04); difference ≠ 0 | NT-10; `AdjustmentPosted`; counted once (AT8) | Auto |
| Q2 | Posted | Target period locked | Frozen | SYS (SM-PERIOD P5) | — | — | Auto |
| Q3 | Frozen | Export produced | Exported | SYS / PO (export timing → GAP-SM-12) | — | Export references adjustment | Auto |
| Q4 | Posted | Same date changed again | ? | SYS | Measured vs locked or vs adjusted → OQ-C12-06 | — | Auto |

- Reversal: none modelled; a later change is a further difference (OQ-C12-06). Source period unlocked → OQ-C12-07.
- Idempotency: keyed by (employee, original date, new result id); the same recalculation twice → one adjustment.
- Illegal: edit or delete an adjustment; post into the source (locked) period or any locked period; money amounts (BRX-02); post beyond PL-07 unless OQ-C12-04 allows.
- Gaps: OQ-C12-02, OQ-C12-04, OQ-C12-06, OQ-C12-07, OQ-C12-10, GAP-SM-12.

---

## 11. SM-OB OpeningBalanceBatch

| State | Meaning |
|---|---|
| Draft | Entries (OB-R2) being entered on screen or by upload (OB-R3); editable. Use in calculations before confirmation → OQ-C15-08. |
| Pending approval | OB-01 = with OA approval; whether approval covers entry, confirmation or both → OQ-C15-07. |
| Confirmed | Locked (OB-R5); opening entries posted to ledgers/counters from the as-of date (OB-R4). |

"Changed with reason" is a transition on Confirmed, not a state.

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| B1 | — | CreateBatch / Upload | Draft | HR | Upload with bad rows → OQ-C15-09 / G-H | — | Manual |
| B2 | Draft | EditEntries | Draft | HR | Validation → OQ-C15-03 | — | Manual |
| B3 | Draft | Confirm | Confirmed | HR | OB-01 = HR Manager | Opening ledger/counter entries posted (keyed by batch + entry); comp-off expiry → OQ-C15-02; `OpeningBalanceConfirmed` | Manual |
| B4 | Draft | SubmitForApproval | Pending approval | HR | OB-01 = with OA approval | — | Manual |
| B5 | Pending approval | Approve | Confirmed | OA | OA ≠ submitter (BR-07) | as B3 | Manual |
| B6 | Pending approval | Reject | ? | OA | Outcome not in BRD → GAP-SM-13 | — | Manual |
| B7 | Confirmed | ChangeEntry | Confirmed | HR (+ OA? → OQ-C15-07) | OB-02 = allowed with reason; reason mandatory | Reversal + new entry (no overwrite); locked as-of / used balance → OQ-C15-04 | Manual |

- Terminal: Confirmed (only B7 after it).
- Idempotency: Confirm on Confirmed → no second posting.
- Illegal: edit entries after Confirmed except B7; B7 when OB-02 = not allowed; B7 without reason; confirm by anyone except HR (B3) or OA approval (B5); overwrite a posted ledger entry (OB-R5, LM-R4).
- Gaps: OQ-C15-02, OQ-C15-03, OQ-C15-04, OQ-C15-07, OQ-C15-08, OQ-C15-09, GAP-SM-13 (also: is "confirmed once" per batch or per employee balance; discard of a draft).

---

## 12. SM-ABS AbscondingCase (LM-29..32)

| State | Meaning |
|---|---|
| Open | Employee flagged as absconding; HR alerted; pay hold on if LM-31 hold = on (V1 meaning → OQ-C08-13). |
| Closed – returned | HR returned employee to active; hold released. Terminal. |
| Closed – offboarding | HR started offboarding. Terminal. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| K1 | — | Trigger | Open | SYS J8 | LM-29 on; N continuous uninformed days (LM-30; working/calendar → G-L, OQ-C08-17); no Open case for employee | Alert HR (OQ-C14-06); pay-hold flag in PayrollInput (DOMAIN_MODEL §7); `AbscondingFlagged`. SM-EMP unchanged | Auto |
| K2 | Open | ReturnToActive | Closed – returned | HR | Reason | Hold released | Manual |
| K3 | Open | StartOffboarding | Closed – offboarding | HR | V1 meaning (offboarding is V3) → GAP-SM-14 | SM-EMP E6 path | Manual |
| K4 | Open | Leave approved for counted days | ? | SYS | → OQ-C08-16 | — | Auto? |
| K5 | Open | Employee punches again | ? | SYS | Not in BRD → GAP-SM-14 | — | — |

- Reopen: a new case may open after Closed – returned if K1 fires again (no rule against it).
- Idempotency: J8 re-run or continuing absence while Open → no second case.
- Illegal: MGR/SUP/EMP/PO closing; SYS releasing hold on its own (LM-32 "HR decides") unless OQ-C08-16 says so.
- Gaps: G-L, OQ-C08-13, OQ-C08-16, OQ-C08-17, OQ-C14-06, GAP-SM-14.

---

## 13. SM-COMPOFF Comp-off credit (M02, HW)

One credit per earned worked day (CompOffAccount ledger, DOMAIN_MODEL §3.9). The approval itself is a CompOffClaim on SM-REQ.

| State | Meaning |
|---|---|
| Pending approval | Earned (HW-04 hours met) but not in balance (HW-02 on). |
| Credited | In balance; full or half (HW-04); expiry set (HW-05; from worked or credit date → OQ-M02-10). |
| Partially used | Full credit with half taken. |
| Used | Fully taken. Terminal unless leave cancelled. |
| Expired – lapsed | Unused part lapsed at expiry (HW-06 = lapse). Terminal. |
| Expired – paid | HW-06 = pay one extra day: **V2**. V1 handling → OQ-M02-01. |
| Not credited | Claim rejected. Terminal. |

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| C1 | — | Earned on day result | Pending approval | SYS | HW-01 = comp-off (or employee chose it, CO-R4; no choice → OQ-M02-07); HW-03 → OQ-M02-02; OD-10 | CompOffClaim submitted (SM-REQ R1) | Auto |
| C2 | Pending approval | Claim approved | Credited | approver (OQ-M02-05) | BR-07 | Credit ledger entry (keyed by claim) | Manual |
| C3 | — | Earned, HW-02 off | Credited | SYS | — | Credit entry "credited by the system" | Auto |
| C4 | Pending approval | Claim rejected | Not credited | approver | Reason | — | Manual |
| C5 | Pending approval | Day recalculated | Pending approval (hours/half-full updated) | SYS | Period open (AC-M02-02.6) | — | Auto |
| C6 | Credited | Half taken (leave approved) | Partially used | SYS on LeaveRequest approve | Consumption order → OQ-M02-09; leave rules → OQ-M02-08 | Debit entry | Auto |
| C7 | Credited / Partially used | Rest taken | Used | SYS | as C6 | Debit entry | Auto |
| C8 | Used / Partially used | Comp-off leave cancelled | Credited / Partially used | SYS | Credit already past expiry → GAP-SM-15 | Reversal entry | Auto |
| C9 | Credited / Partially used | Expiry date reached | Expired – lapsed | SYS J6 | HW-06 = lapse (V1) | Lapse entry; NT-11 → OQ-C14-03 | Auto |
| C10 | Credited / Used | Worked day corrected after credit | ? | SYS | → OQ-M02-04 | — | — |

Opening comp-off (OB) enters as Credited (B3); expiry → OQ-C15-02.

- Idempotency: J6 re-run → one lapse entry per credit (keyed by credit + expiry date).
- Illegal: use after expiry; credit without approval when HW-02 on; balance below zero unless OQ-M02-08 says LM-07 applies; approver approves own comp-off (BR-07, AC-M02-03.5).
- Gaps: OQ-M02-01, OQ-M02-02, OQ-M02-04, OQ-M02-05, OQ-M02-07, OQ-M02-08, OQ-M02-09, OQ-M02-10, OQ-C15-02, GAP-SM-15.

---

## 14. SM-POLICYVER PolicyVersion (G5, G8, G9)

| State | Meaning |
|---|---|
| Draft | Being edited; not resolvable by PolicyResolver. Preview (G8) allowed; stores no results. |
| Pending approval | AC-04 = with OA approval. State not named in BRD; implied by AC-04 → GAP-SM-16. |
| Scheduled | Saved with effective-from in the future. Editable or withdrawable? → GAP-SM-16. |
| In force | Effective-from ≤ date and not yet superseded. Never edited (G5). |
| Superseded | A later version's effective-from has passed. Still resolves for earlier dates (BR-05). Terminal. |

Scheduled → In force → Superseded are **date-derived**: PolicyResolver picks the version in force on the attendance date; no job flips them.

| # | From | Event / command | To | Actor | Preconditions | Side effects / events | Mode |
|---|---|---|---|---|---|---|---|
| V1 | — | CreateDraft (copy of current) | Draft | HR | Rights (AC-01) | — | Manual |
| V2 | Draft | Preview | Draft | HR | Pay-affecting settings (G8) | Difference report on last period; no results stored | Manual |
| V3 | Draft | Save (AC-04 = HR only) | Scheduled or In force (by date) | HR | Schema-valid (TSD §5.4); no overlap within set; reason (G9); PD warning (§10.3.4, V2) | SettingsChange (old, new, who, when, reason); in force → bulk recalculation of open dates (BR-03); back-dated into locked period → GAP-SM-16 | Manual |
| V4 | Draft | Submit (AC-04 = with approval) | Pending approval | HR | as V3 | — | Manual |
| V5 | Pending approval | Approve | Scheduled or In force | OA | OA ≠ submitter (BR-07; same person → OQ-C03-11) | as V3 | Manual |
| V6 | Pending approval | Reject | ? | OA | → GAP-SM-16 | — | Manual |
| V7 | Scheduled | Effective date reached | In force | (date-derived) | — | Recalculation only if open days are affected | Auto |
| V8 | In force | Next version in force | Superseded | (date-derived) | — | — | Auto |

- Idempotency: saving identical content as a new version → refused or no-op (no new version with no change); preview has no side effects.
- Illegal: edit In force or Superseded or any version "in use" (G5; "in use" undefined → GAP-SM-16); overlapping effective periods; save failing schema; save without reason (G9); change by VA, MGR, SUP, EMP, PO, AUD; disabling a rule group retroactively for locked dates (BR-06).
- Gaps: DP-03, DP-10, OQ-C03-11, GAP-SM-16.

---

## 15. New gaps raised by this document

| ID | Gap |
|---|---|
| GAP-SM-01 | Organisation has no closed/terminated state (subscription end, data retention); no job is named for VC-04 automatic suspension. |
| GAP-SM-02 | A support-access request the OA never answers: does it lapse; can the vendor withdraw it? |
| GAP-SM-03 | What moves an employee into "on notice" and then "exited" in V1 (no resignation / last-working-day fields until V3), can probation go straight to notice, and can notice be withdrawn? |
| GAP-SM-04 | When enrolment returns to "not enrolled" (unmap, exit, device replaced), and whether status is per employee or per device mapping. |
| GAP-SM-05 | Device offline threshold (PC-10 is only the alert delay), initial state before first contact, and deregistration / branch move / replacement. |
| GAP-SM-06 | Can a punch-flag decision be reversed; with several flags on one punch, does it count only when all are accepted? |
| GAP-SM-07 | "Finalised" day (BRD §2, TSD J1) is undefined: what finalisation changes and what a day is before it. |
| GAP-SM-08 | Requests: no draft state, no withdraw-vs-cancel distinction, no resubmission after rejection. |
| GAP-SM-09 | Cancellation rules for RegularisationRequest, CompOffClaim and ShiftChangeRequest. |
| GAP-SM-10 | Does the timeout action apply at level 2 too, and may the original approver still act after escalation? |
| GAP-SM-11 | Is a late/early converted to a permission (LA-15/EC-13 automatic) a PermissionRequest, and does it need approval? |
| GAP-SM-12 | Lock before period end; which job generates periods; when the PayrollInputExport is produced and whether it can be re-produced. |
| GAP-SM-13 | Opening-balance batch rejected by OA (back to draft?), draft discard, and whether "confirmed once" is per batch or per employee balance. |
| GAP-SM-14 | What "start offboarding" (LM-32) means in V1, and whether an absconding case closes when the employee punches again. |
| GAP-SM-15 | Comp-off leave cancelled after the credit's expiry date: restored or lapsed? |
| GAP-SM-16 | Policy version: definition of "in use"; edit/withdraw of a scheduled version; back-dating into locked periods; AC-04 rejection outcome. |
| GAP-SM-17 | In-flight items (pending requests, open absconding cases, unexpired comp-off, raised flags) when a module or rule group is switched off. |

---

## 16. Lifecycles added in Phase 4 (need Gate 4 approval)

Found while writing the API contracts; tables in migration 0006.

### SM-CONTACT Employee contact-detail change request (EM-03)
| From | Event | To | Actor | Notes |
|---|---|---|---|---|
| — | Submit | pending | EMP (self) | EM-03 = on, with approval |
| — | Submit | applied | EMP (self) | EM-03 = on, without approval; change applied at once |
| pending | Approve | approved → applied | HR (approver per AC-02 `contact_change`) | BR-07: approver ≠ requester; applied in the same transaction |
| pending | Reject | rejected | HR | Reason required. Terminal |
Illegal: any command on applied/rejected; edits to fields other than contact details. Gaps: who approves by default (OQ-C02-15 for the sign-in mobile number).

### SM-SETCHG Settings change awaiting approval (AC-04 = with Organisation Admin approval)
| From | Event | To | Actor | Notes |
|---|---|---|---|---|
| — | Submit | pending | HR | Only when AC-04 requires approval; otherwise the change is saved directly |
| pending | Approve | approved | OA | Creates the new settings version effective from the requested date. BR-07; one person with both roles: OQ-C03-11 |
| pending | Reject | rejected | OA | Terminal. What happens to the draft: GAP-SM-16 |
