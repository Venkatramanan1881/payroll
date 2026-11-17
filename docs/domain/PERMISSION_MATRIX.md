# Permission matrix (V1)

Status: Draft for Approval Gate 2
Sources: BRD v0.7 (§4.2, §4.3, §5, §9, §12, §14, §16, §22.2), TSD v0.1 (§4.3, §4.4, §7, §15), `DOMAIN_MODEL.md`, Gate 1 user stories in `docs/product/USER_STORIES/`
Companions: `DOMAIN_MODEL.md`, `STATE_MACHINES.md`, `GLOSSARY.md`

This document gives the **default** grants for the BRD §5 system roles. It defines no code, schema or API routes.

---

## 1. Principles

| # | Principle | Source |
|---|---|---|
| P1 | A permission is granted per **business operation or state transition** on an aggregate, never per screen. A screen shows what the operations behind it allow. | UM-R1, TSD §7 |
| P2 | Every check runs on the **server**, on every call, whatever the client hides. | N5, TSD §7 "a route with no declaration fails the build" |
| P3 | The **organisation always comes from the authenticated principal** (or the registered device for pushes), never from the request content. No cell in this matrix crosses organisations. The only crossing is an active SupportAccessGrant (§3.1). | TB-01, AT20, DOMAIN_MODEL §8 |
| P4 | **Data scope is applied on top of the operation right.** A user may act only on employees inside their scope. Scope kinds: all branches / chosen branches / chosen departments (AC-07); **Manager = own team** (reporting line, §5 "for their team"); **Employee = self** (§5 "own"). Supervisor "team" is undefined (G-15). | AC-07, UM-R2, AT13, §5 |
| P5 | **BR-07: nobody approves their own request**, whatever their role or rights, including when acting as a delegate (AC-03), and including the AC-04 / OB-01 / PL-06 / VC-03 approvals. | BR-07, DOMAIN_MODEL §6 |
| P6 | An **approve right (A)** is used only on a request routed to the user by AC-02 (reporting manager / named person / department head / HR) at that level, or delegated to them (AC-03). A "named person" or "department head" may hold any role; the routing then gives them A for that request type, still subject to AC-01 and P5. | AC-01, AC-02, AC-03 |
| P7 | **Module disabled ⇒ every operation of that module is unavailable** (refused on direct call, hidden in menus, reports and dashboards). Rule group switched off ⇒ its operations and settings are unavailable from the effective date. | MD-03, MD-05, BR-14, AT2, AT3, TSD §4.4 |
| P8 | **Suspended organisation ⇒ view and export only.** Every create, change, approve, lock or delete is refused for every organisation role. Device punches are still stored (system). | VC-R4, AC-V01-07.3/07.4 |
| P9 | **AC-01 makes role rights configurable.** This matrix is the default for the §5 system roles. Custom roles (AC-06) start with no rights. Whether system roles can be edited beyond §5 is open (G-09). | AC-01, AC-06, OQ-C03-01 |
| P10 | **Fixed denials** that no role configuration can grant: vendor-only operations to organisation users (MD-01, A1); organisation rule-group or policy changes to vendor users (G1, O2); changing an organisation's results to any vendor user (§5 Vendor Admin); editing or deleting an AuditEntry or RawPunch to anyone (BR-01, §16, TSD §15); changing a locked period's results to anyone (BR-06). | as cited |
| P11 | **Auditor** has read-only access to results, settings history and the audit trail, within their scope. Every write is denied. | §5, US-C03-11 |
| P12 | **Vendor users are never members of an organisation.** Outside a SupportAccessGrant they see organisation-level facts only (plan, usage, health), never employee data. | VC-05, TSD §4.3, AC-V01-06.3, AC-V01-13.3 |
| P13 | Every write and approval in this matrix produces an AuditEntry (who, when, before/after, reason where required). | §16, G9, DOMAIN_MODEL §10 |
| P14 | Operations performed by the **system** (daily processing, recalculation, accrual, escalation, auto-approve/reject, period-end penalties, automatic lock under PL-04, PeriodAdjustment creation) are not role grants and are not listed as rows. | TB-03, BR-03, BR-10 |

---

## 2. Cell legend

| Code | Meaning |
|---|---|
| Y | Allowed, all data of the organisation (or platform, for vendor rows) |
| S | Allowed within the user's data scope (P4) |
| Self | Allowed on the user's own records only |
| A | Approve-type right (approve / reject / decide), on requests routed to the user (P6), within scope, never own (P5) |
| Cfg(X) | Depends on setting X. Combined with the code it qualifies, e.g. `S Cfg(PC-11)` |
| — | Not granted by default (outside the role's §5 description, or a fixed denial P10) |
| ? | The BRD does not say who may do it. See §6 gap ID in the notes column |
| † | Actor taken from a Gate 1 user story where the BRD itself names no actor. Listed for confirmation in §6 |

Row IDs (PLT-, IAM-, ORG-, WRK-, TCP-, SCH-, POL-, ATT-, LVE-, PRM-, CMP-, PER-, OBL-, NTF-, RPT-, DSH-, ADT-) are local to this matrix. Two-letter IDs in the notes (PL-04, OB-01, DB-02 …) are BRD settings.

Columns: **VA** Vendor Admin, **VS** Vendor Support, **VR** Vendor Read-only (VC-05), **OA** Organisation Admin, **HR** HR Manager, **PO** Payroll Officer, **MGR** Manager (Approver), **SUP** Supervisor, **EMP** Employee, **AUD** Auditor.

Self-service by users who hold another role as well (a Manager raising their own leave) depends on G-38 (OQ-C03-03). In this matrix, Self cells sit in the EMP column only.

---

## 3. Operation catalogue and matrix

### 3.1 Platform (V01, vendor console)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| PLT-01 | Create Organisation from SectorTemplate (plan, modules, limit, first admin user) | Y | ? | — | — | — | — | — | — | — | — | VC-R1, A1, MD-01. VS: G-01 |
| PLT-02 | Change ModuleEntitlement (switch module on / off) | Y | ? | — | — | — | — | — | — | — | — | MD-01 "Vendor Admin only", MD-04, AC-V01-03.8 |
| PLT-03 | Change Organisation plan or employee limit | Y | ? | — | — | — | — | — | — | — | — | VC-R2 |
| PLT-04 | Maintain Plan (modules, limit, app included) | Y | ? | — | — | — | — | — | — | — | — | MD-02 |
| PLT-05 | Record SubscriptionRecord (plan, start, renewal, billing status) | Y | ? | — | — | — | — | — | — | — | — | VC-R5 |
| PLT-06 | Suspend / reactivate Organisation | Y | ? | — | — | — | — | — | — | — | — | VC-R4, VC-04; OA denied: AC-V01-07.8 |
| PLT-07 | Maintain SectorTemplate | Y | ? | — | — | — | — | — | — | — | — | VC-R7 (template part) |
| PLT-08 | Set vendor settings VC-01, VC-02, VC-04 | Y | ? | — | — | — | — | — | — | — | — | §4.3 vendor console settings |
| PLT-09 | Set VC-03 (support access needs OA approval) | ? | ? | — | ? | — | — | — | — | — | — | G-04 |
| PLT-10 | View usage, health view and vendor dashboard | Y | ? | Y | — | — | — | — | — | — | — | VC-R3, VC-R8, DB-R2; org-level facts only (P12) |
| PLT-11 | Send Announcement to chosen organisations | Y | ? | — | — | — | — | — | — | — | — | VC-R9 |
| PLT-12 | Create VendorUser / change vendor role | Y† | ? | — | — | — | — | — | — | — | — | VC-05 names roles, not who manages them: G-06 |
| PLT-13 | Request SupportAccessGrant | Y | ? | ? | — | — | — | — | — | — | — | VC-R6 "the vendor"; which vendor roles: G-01 (OQ-V01-14) |
| PLT-14 | Approve / reject SupportAccessGrant | — | — | — | A Cfg(VC-03) | — | — | — | — | — | — | VC-03 "Organisation Admin's approval each time"; when VC-03 = off no approval step |
| PLT-15 | End SupportAccessGrant before its time limit | ? | ? | — | ? | — | — | — | — | — | — | G-03 (OQ-V01-13) |
| PLT-16 | View data inside an organisation under an active SupportAccessGrant | ? | ? | ? | — | — | — | — | — | — | — | VC-R6; extent: G-02 (OQ-V01-12) |
| PLT-17 | Change settings or data inside an organisation under an active SupportAccessGrant | ? | ? | — | — | — | — | — | — | — | — | G-02 (OQ-V01-12). Changing **results** is a fixed denial (§5 "cannot change an organisation's results") |
| PLT-18 | View support-access action log for own organisation | — | — | — | Y | — | — | — | — | — | S | VC-R6 "visible to the Organisation Admin"; AUD: audit trail (§5) |
| PLT-19 | View vendor history of plan / module / limit changes for own organisation | Y | ? | Y | ? | — | — | — | — | — | ? | VC-R2; org visibility: G-05 (OQ-V01-05) |

### 3.2 Identity & Access (C03)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| IAM-01 | Create / deactivate UserAccount | — | — | — | Y | ? | — | — | — | — | — | §5 OA "manages users"; HR: G-07 (OQ-C03-12) |
| IAM-02 | Assign Role and AccessScope to a UserAccount | — | — | — | Y | ? | — | — | — | — | — | §5, AC-07; HR: G-07 |
| IAM-03 | Change Role grants (view / edit / approve per operation) | — | — | — | Y | — | — | — | — | — | — | §5 OA "manages roles", AC-01; editing system roles: G-09 (OQ-C03-01) |
| IAM-04 | Create custom Role | — | — | — | Y Cfg(AC-06) | — | — | — | — | — | — | AC-06 |
| IAM-05 | Set ApproverRule per request type | — | — | — | ? | Y Cfg(AC-04) | — | — | — | — | — | AC-02; §5 HR "owns settings"; US-C03-07. OA: G-08 |
| IAM-06 | Change access settings AC-03, AC-04, AC-06, AC-08 | — | — | — | ? | ? | — | — | — | — | — | BRD names no actor; G-08 (OQ-C03-11) |
| IAM-07 | Set employee self-visibility (AC-05) | — | — | — | — | Y Cfg(AC-04) | — | — | — | — | — | §5 "owns settings", US-C03-10 |
| IAM-08 | Approve settings change (AC-04 = with OA approval) | — | — | — | A Cfg(AC-04) | — | — | — | — | — | — | AC-04; same person in both roles: P5, OQ-C03-11 |
| IAM-09 | Create Delegation for an approver | — | — | — | ? | ? | — | ? | — | — | — | AC-03 on/off only; G-10 (OQ-C03-10) |
| IAM-10 | Act as delegate on a delegated request | — | — | — | A Cfg(AC-03) | A Cfg(AC-03) | A Cfg(AC-03) | A Cfg(AC-03) | A Cfg(AC-03) | — | — | AC-03; never own request (BR-07) |
| IAM-11 | Choose own language | — | — | — | Self | Self | Self | Self | Self | Self | Self | UM-R4, OR-03 |

### 3.3 Organisation Structure (C01)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ORG-01 | Create / change Branch (address, time zone) | — | — | — | Y | ? | — | — | — | — | — | §5 OA "manages branches"; HR: G-11 (OQ-C01-01) |
| ORG-02 | Set / test / switch on GeoFence for a Branch | — | — | — | ? | S | — | — | — | — | — | PC-R14 "HR sets each branch's fence"; US-C01-01 puts it under OA: G-11 |
| ORG-03 | Set Branch allowed web IP list | — | — | — | ? | S Cfg(AC-04) | — | — | — | — | — | PC-05 (setting, §5 HR); OA: G-11 |
| ORG-04 | Maintain Department / Designation / Category / EmployeeType lists | — | — | — | ? | ? | — | — | — | — | — | OR-R2, OR-R3; G-11 (OQ-C01-01) |
| ORG-05 | Maintain Contractor | — | — | — | ? | ? | — | — | — | — | — | OR-R4; G-11 (OQ-C01-01) |
| ORG-06 | Change OrganisationSettings (OR-01..04, EM-01..09) | — | — | — | — | Y Cfg(AC-04) | — | — | — | — | — | §5 HR "owns settings", G1 |
| ORG-07 | View organisation structure and settings | — | — | — | ? | Y | ? | ? | ? | — | Y | AUD: settings (§5). Others: G-14 |

### 3.4 Workforce (C02)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| WRK-01 | Create Employee (screen or spreadsheet upload) | — | — | — | — | S† Cfg(VC-01) | — | — | — | — | — | EM-R7; actor US-C02-01/09: G-13. VC-01 = block refuses at limit |
| WRK-02 | Change Employee personal, contact, job details and custom field values | — | — | — | — | S† | — | — | — | — | — | EM-R1, EM-R2, EM-R8; G-13 |
| WRK-03 | Change dated assignment (master / allowed branches, department, designation, category, type, reporting manager, contractor, schedule source, policy-set assignment) | — | — | — | — | S† | — | — | — | — | — | EM-R6, EM-06, EM-09; G-13. Transfer to a branch outside scope: OQ-C02-09 |
| WRK-04 | Change employment status (confirm probation, notice, exit, rejoin) | — | — | — | — | S† | — | — | — | — | — | EM-R5, EM-05; G-13 |
| WRK-05 | Define employee custom field | — | — | — | — | Y† Cfg(AC-04) | — | — | — | — | — | EM-R8; G-13 |
| WRK-06 | Edit own contact details | — | — | — | — | — | — | — | — | Self Cfg(EM-03) | — | EM-03 |
| WRK-07 | Approve Employee's own contact-detail change | — | — | — | — | A Cfg(EM-03) | — | — | — | — | — | EM-03 "with HR approval" |
| WRK-08 | View Employee record (non-sensitive fields) | — | — | — | ? | S | ? | ? | ? | Self | ? | EMP §5 "own"; others: G-14. Sensitive fields: §4 |

### 3.5 Time Capture (C04)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TCP-01 | Register / move / retire Device | — | — | — | ? | ? | — | — | — | — | — | PC-R2 names no actor; US-C04-01 (HR) vs US-C01-01 (OA): G-12 |
| TCP-02 | Map EnrolmentMapping and set enrolment status | — | — | — | — | S | — | — | — | — | — | PC-R8, §5 HR "device enrolment" |
| TCP-03 | Record own RawPunch (app / web / device) | — | — | — | — | — | — | — | — | Self Cfg(PC-01, PC-02, PC-12, MD-02) | — | §5 Employee "punches"; app needs plan (MD-02); suspended org: G-40 |
| TCP-04 | Record manual attendance for device failure | — | — | — | — | S | — | — | S Cfg(PC-11) | — | — | PC-R9, §5 HR, PC-11 (delegate to Supervisor), PC-01 "entry by HR". BR-01 approver: G-17 |
| TCP-05 | Bulk regularise a device outage | — | — | — | — | S Cfg(RG-11) | — | — | — | — | — | RG-11 |
| TCP-06 | Decide PunchFlag (accept / reject) | — | — | — | — | ? | — | A | — | — | — | E18 "until the manager approves" (geo-fence); other reasons and HR: G-16 (OQ-C04-02, DP-05) |
| TCP-07 | View RawPunch times, source, device, manual marker, flags | — | — | — | ? | S | ? | ? | ? | Self Cfg(AC-05) | ? | PC-R13 "HR can see"; AC-05 "own punches"; others: G-36 |
| TCP-08 | View punch location / selfie | — | — | — | — | S (location) / ? (selfie) | — | ? | ? | ? | ? | PC-R13 (location, HR); selfie and everyone else: G-18 (OQ-C04-17). See §4 |
| TCP-09 | View refused PunchAttempt and its reason | — | — | — | — | S | — | — | — | Self | S | E18 "the employee is told why"; AUD: audit trail |
| TCP-10 | View device online status / receive offline alert | — | — | — | ? | S | — | — | — | — | — | PC-R2, PC-R10 "HR Manager is alerted"; DB-R3 shows devices offline to OA |
| TCP-11 | Change punch settings (PC-01..17) | — | — | — | — | Y Cfg(AC-04) | — | — | — | — | — | §5 "owns settings" |

### 3.6 Scheduling (C05, C07)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| SCH-01 | Create Shift / publish Shift version | — | — | — | — | Y Cfg(AC-04) | — | — | — | — | — | SH-01..11 settings, §5; AC-04 coverage: G-08 |
| SCH-02 | Assign shift / build, upload, copy, rotate Roster | — | — | — | — | S† | — | ? | ? | — | — | SH-R2, SH-R3 name no actor; US-C05-03 (HR); MGR/SUP: G-19 |
| SCH-03 | View Roster | — | — | — | ? | S | — | ? | S | Self | ? | SH-R4 (employee), DB-R6 (Supervisor today/tomorrow); others: G-14, G-15 |
| SCH-04 | Set WeeklyOffPattern | — | — | — | — | Y Cfg(AC-04) | — | — | — | — | — | SH-09, HD-R2 |
| SCH-05 | Maintain HolidayCalendar | — | — | — | ? | S† | — | — | — | — | — | HD-R1, OR-R1 (part of a branch); US-C07-01: G-11 |
| SCH-06 | Choose optional holidays | — | — | — | — | — | — | — | — | Self Cfg(HD-01) | — | HD-R3; approval: G-20 (OQ-C07-04) |
| SCH-07 | Submit ShiftChangeRequest (change / swap) | — | — | — | — | — | — | — | — | Self Cfg(SH-12) | — | SH-12 |
| SCH-08 | Approve ShiftChangeRequest | — | — | — | — | A Cfg(SH-12) | — | A Cfg(SH-12) | — | — | — | SH-12 "with approver", AC-02; swap rules: G-21 (OQ-C05-12) |

### 3.7 Policy (G2–G9)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| POL-01 | Create PolicySet / draft PolicyVersion | — | — | — | — | Y | — | — | — | — | — | §5 HR "owns policy sets", G3 |
| POL-02 | Publish PolicyVersion (effective-dated) | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | G5, AC-04; vendor denied (G1, AC-V01-14.6) |
| POL-03 | Run PolicyVersion preview on last period | — | — | — | — | Y | — | — | — | — | — | G8 "HR sees … can test" |
| POL-04 | Create / end PolicyAssignment | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | G3, §5 |
| POL-05 | Create / end Exemption | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | G7, LA-22 |
| POL-06 | Switch rule group on / off | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | MD-05, G1, G2; vendor denied (G1) |
| POL-07 | View PolicySet, versions, assignments and settings history (R26) | — | — | — | ? | Y | ? | — | — | — | S | AUD §5 "settings history"; OA/PO: G-34 |

### 3.8 Attendance (C06)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ATT-01 | View AttendanceDay (current DayResult, history, Explanation) | — | — | — | ? | S | ? | S | S | Self Cfg(AC-05) | S | §5 MGR/SUP "team attendance", DB-R5/6; AC-05 day status and deduction reason; AUD "results". OA/PO: G-14 |
| ATT-02 | Submit RegularisationRequest | — | — | — | — | — | — | — | — | Self Cfg(RG-01, RG-02, RG-03, RG-04) | — | §5 "raises requests", RG-01..04 |
| ATT-03 | Approve RegularisationRequest (level 1) | — | — | — | — | A | — | A | — | — | — | RG-05 "manager", §5; HR when AC-02 = HR |
| ATT-04 | Approve RegularisationRequest (level 2) | — | — | — | — | A Cfg(RG-05) | — | — | — | — | — | RG-05 "manager then HR" |
| ATT-05 | Approve over-limit RegularisationRequest | — | — | — | — | A Cfg(RG-08) | — | — | — | — | — | RG-08 "HR approval only" |
| ATT-06 | Record HRCorrection (direct correction with reason) | — | — | — | — | S Cfg(RG-10) | — | — | — | — | — | RG-10; second person: G-17 (DP-08) |
| ATT-07 | Create / withdraw BulkWaiver | — | — | — | — | S Cfg(LA-17, EC-16) | — | — | — | — | — | LA-17, EC-16, E14, §5 HR "waivers" |
| ATT-08 | Accept suggested late / early → permission conversion | — | — | — | — | — | — | — | — | Self Cfg(LA-15, EC-13) | — | LA-15, EC-13 "suggest to the employee" |
| ATT-09 | Release held check-in (LA-05 "hold check-in until the manager approves") | — | — | — | — | — | — | A Cfg(LA-05) | — | — | — | LA-05; device behaviour OQ-C06-07 |
| ATT-10 | Approve leaving early (EC-14 prior approval) | — | — | — | — | ? | — | ? | — | — | — | EC-14 names no request type or approver: G-22 |
| ATT-11 | Act on LV-01 flag (punches on a full-day leave day) | — | — | — | — | ? | — | ? | — | ? | — | LV-01; G-23 (OQ-C08-18) |
| ATT-12 | Return AbscondingCase employee to active / start offboarding | — | — | — | — | S | — | — | — | — | — | LM-32 "HR then …", E19 |
| ATT-13 | View PeriodCounterSet and PeriodPenalty | — | — | — | ? | S | ? | S | — | Self Cfg(AC-05) | S | AC-05 "counters"; DB-R5 trends; AUD results. OA/PO: G-14 |

### 3.9 Leave & Absence (C08)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| LVE-01 | Create LeaveType / publish LeaveType version | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | LM-01..15, §5 |
| LVE-02 | Submit own LeaveRequest | — | — | — | — | — | — | — | — | Self | — | LM-R3, §5 |
| LVE-03 | Submit LeaveRequest on behalf of an employee | — | — | — | — | ? | — | ? | — | — | — | BRD silent: G-25 |
| LVE-04 | Approve LeaveRequest (level 1), singly or in bulk | — | — | — | — | A | — | A | — | — | — | LM-13 "manager", LM-R3 bulk, §5 |
| LVE-05 | Approve LeaveRequest (level 2) | — | — | — | — | A Cfg(LM-13) | — | — | — | — | — | LM-13 "manager then HR" |
| LVE-06 | Cancel LeaveRequest (pending or approved) | — | — | — | — | ? | — | ? | — | ? | — | G-24 (OQ-C08-10) |
| LVE-07 | Post manual LeaveAccount adjustment with reason | — | — | — | — | ? | — | — | — | — | — | DOMAIN_MODEL §3.9 ledger "adjustment"; BRD names no actor: G-26 |
| LVE-08 | View LeaveAccount balance and ledger | — | — | — | ? | S | ? | ? | — | Self | S | LM-R4 employee; MGR (DB-R5 leave calendar only): G-39 |

### 3.10 Permission & On-duty (C09)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| PRM-01 | Create PermissionType / publish version; set OD settings | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | PM-01..30, OD-01..12, §5 |
| PRM-02 | Submit own PermissionRequest | — | — | — | — | — | — | — | — | Self Cfg(PM-01) | — | PM-R1 |
| PRM-03 | Approve PermissionRequest (level 1) | — | — | — | — | A | — | A Cfg(PM-16) | — | — | — | PM-16 "manager" (or auto-approve within limit) |
| PRM-04 | Approve PermissionRequest (level 2) | — | — | — | — | A Cfg(PM-16) | — | — | — | — | — | PM-16 "manager then HR" |
| PRM-05 | Approve over-limit PermissionRequest as unpaid minutes | — | — | — | — | A Cfg(PM-18) | — | — | — | — | — | PM-18, E11 |
| PRM-06 | Cancel / change PermissionRequest before start time | — | — | — | — | — | — | — | — | Self | — | PM-29 "by the employee before the start time" |
| PRM-07 | Cancel / change PermissionRequest after start time | — | — | — | — | S | — | — | — | — | — | PM-29 "by HR afterwards" |
| PRM-08 | Submit own OnDutyRequest | — | — | — | — | — | — | — | — | Self Cfg(OD-01, OD-02) | — | PM-R5 |
| PRM-09 | Approve OnDutyRequest (level 1) | — | — | — | — | A | — | A | — | — | — | OD-05 "manager" |
| PRM-10 | Approve OnDutyRequest (level 2) | — | — | — | — | A Cfg(OD-05) | — | — | — | — | — | OD-05 "manager then HR" |
| PRM-11 | Cancel / change OnDutyRequest | — | — | — | — | ? | — | ? | — | ? | — | G-27 (OQ-C09-10) |
| PRM-12 | Submit / approve work-from-home day | — | — | — | — | ? | — | ? | — | ? | — | PC-09, LV-03; G-28 (DP-09, OQ-C04-14) |
| PRM-13 | View PermissionAccount usage and balance | — | — | — | ? | S | ? | ? | — | Self Cfg(AC-05) | S | PM-R4, AC-05 (conflict OQ-C09-11); MGR: G-39 |

### 3.11 Comp-off (M02; all rows need M02 enabled)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| CMP-01 | Change comp-off settings (HW-01..06) | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | §5, AC-04 |
| CMP-02 | Request prior approval to work on a holiday / weekly off | — | — | — | — | — | — | — | — | Self Cfg(HW-03) | — | HW-03 |
| CMP-03 | Approve prior work on a holiday / weekly off | — | — | — | — | A Cfg(HW-03) | — | A Cfg(HW-03) | — | — | — | HW-03, AC-02 routing |
| CMP-04 | Choose pay or comp-off for the worked day | — | — | — | — | — | — | — | — | Self Cfg(HW-01) | — | CO-R4, HW-01 "employee chooses" |
| CMP-05 | Approve CompOffClaim (credit) | — | — | — | — | A Cfg(HW-02) | — | A Cfg(HW-02) | — | — | — | CO-R1, HW-02, AC-02; escalation OQ-M02-05 |
| CMP-06 | Take comp-off (submit as leave) | — | — | — | — | — | — | — | — | Self | — | CO-R2; approval follows LVE-04/05 (OQ-M02-08) |
| CMP-07 | View CompOffAccount | — | — | — | ? | S | ? | ? | — | Self | S | LM-R8; MGR: G-39 |

### 3.12 Period (C12)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| PER-01 | Change period settings (PL-02..07) | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | §5, US-C12-02 |
| PER-02 | View pre-lock checklist | — | — | — | — | Y (D-10) | Y | — | — | — | S | PL-R2, NT-09, DB-R4 (PO, V2); HR in V1: D-10 |
| PER-03 | Lock AttendancePeriod | — | — | — | — | Y Cfg(PL-04, PL-05) (D-10) | Y Cfg(PL-04, PL-05) | — | — | — | — | §5 PO "locks periods" (V2), PL-04 manual; HR locks in V1: D-10 |
| PER-04 | Request period unlock | — | — | — | — | ? | ? | — | — | — | — | PL-06 names only the approver: G-30 |
| PER-05 | Approve period unlock | — | — | — | A Cfg(PL-06) | — | — | — | — | — | — | §5 OA "approves period unlock", PL-06 (default never); OQ-C12-07 |
| PER-06 | Export PayrollInput (produce PayrollInputExport) | — | — | — | — | ? | ? | — | — | — | — | MD-06 names no actor: G-31 (OQ-C12-05, OQ-C12-11); AUD write denied |
| PER-07 | Download an existing PayrollInputExport | — | — | — | — | ? | ? | — | — | — | S | G-31; AUD results (§5); download is audited (DOMAIN_MODEL §10) |
| PER-08 | View PeriodTotals and PeriodAdjustment | — | — | — | ? | S | ? | — | — | — | S | AUD results; others: G-14 |

### 3.13 Opening Balances (C15)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| OBL-01 | Enter / upload OpeningBalanceBatch (draft) | — | — | — | — | S Cfg(OB-01) | — | — | — | — | — | OB-R1, OB-01, §5 HR "opening balances" |
| OBL-02 | Confirm OpeningBalanceBatch | — | — | — | A Cfg(OB-01) | S Cfg(OB-01) | — | — | — | — | — | OB-R5, OB-01 "with Organisation Admin approval"; scope of approval: G-32 (OQ-C15-07) |
| OBL-03 | Change confirmed opening balance (reversal + new entry, reason) | — | — | — | ? | S Cfg(OB-02) | — | — | — | — | — | OB-02; OA approval: G-32 |
| OBL-04 | View opening balances as confirmed (R25) | — | — | — | ? | S | ? | — | — | Self | S | R25, AUD §5; EMP via balances (US-C15-05); OA/PO: G-34 |

### 3.14 Notifications settings (C14)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| NTF-01 | Change NotificationRule (event × channel × recipient) | — | — | — | A Cfg(AC-04) | Y Cfg(AC-04) | — | — | — | — | — | §9.14, §5 settings, US-C14-01 |

### 3.15 Reports (C13; every report: filter, scope, export, module-gated per §14)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| RPT-01 | Run / export attendance reports R1–R5 | — | — | — | ? | S | ? | ? | ? | ? | S | §14; AUD results; others: G-33 (OQ-C13-04) |
| RPT-02 | Run / export leave, permission, comp-off reports R6–R9 | — | — | — | ? | S | ? | ? | ? | ? | S | §14; G-33 |
| RPT-03 | Run / export people report R10 | — | — | — | ? | S | ? | ? | ? | — | ? | §14; G-33, G-14 |
| RPT-04 | Run / export payroll input summary R13 | — | — | — | — | ? | ? | — | — | — | S | §14; G-29/G-31 |
| RPT-05 | Run / export R25 opening balances, R26 settings history | — | — | — | ? | ? | — | — | — | — | S | §5 AUD; others: G-34 |
| RPT-06 | Run / export R27 audit trail | — | — | — | ? | ? | — | — | — | — | S | §5 AUD; OA sees support log (PLT-18); others: G-34 |

### 3.16 Dashboards (C16)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| DSH-01 | View own role dashboard | Y | ? | ? | S | S | S | S | S | Self | ? | DB-R1..R8; AUD: G-35 (OQ-C16-02); vendor widgets: OQ-C16-06 |
| DSH-02 | Choose dashboard widgets per role | — | — | — | Y | — | — | — | — | — | — | DB-01 "chosen by the Organisation Admin" |
| DSH-03 | Set default branch and period filters for a user | — | — | — | ? | — | — | — | — | — | — | DB-02 names no actor (user or admin): G-35 |

### 3.17 Audit (§16)

| ID | Operation | VA | VS | VR | OA | HR | PO | MGR | SUP | EMP | AUD | Source / notes |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ADT-01 | View AuditEntry trail | — | — | — | ? | ? | — | — | — | — | S | §5 AUD; OA support log PLT-18; others: G-34 |
| ADT-02 | View SettingsChange history | — | — | — | ? | ? | — | — | — | — | S | §5 AUD, G9; others: G-34 |
| ADT-03 | Edit or delete AuditEntry | — | — | — | — | — | — | — | — | — | — | Fixed denial: §16, TSD §15 insert-only |
| ADT-04 | Edit or delete RawPunch | — | — | — | — | — | — | — | — | — | — | Fixed denial: BR-01, PC-R6 |

### 3.18 Not in V1 (placeholders, no grants)

| Operation | §5 role | Release | Source |
|---|---|---|---|
| Approve overtime | Manager | V3 | §5, OT-05 |
| Approve advance / loan | Manager | V2 | §5, AL-06 |
| Run payroll, release payslips, record payments | Payroll Officer | V2 | §5, §10.3.1 |
| Create / assign payroll groups | HR Manager | V2 | §5, PG-R1 |
| Mark daily cash payout paid | Supervisor | V3 | §5, DA-R5, DA-07 |
| Build client-specific payroll rules | Vendor Admin | V2 | §5, CR-01 |

---

## 4. Sensitive data visibility

Default: no one sees a sensitive field unless listed. Every grant is within scope (P4). Masking per TSD §15. Files (selfies) only through short-lived links after an access check (TB-02).

| Data | Held in V1? | Who may view (default) | Masked by default | Unmask right | Each view logged | Source / gap |
|---|---|---|---|---|---|---|
| Bank details (account number, IFSC, bank/cash split) | ? (OQ-C02-05) | HR ?; EMP Self ?; others — ; vendor — | Yes (TSD §15) | ? | ? (BRD has no V1 rule; DC-R8 view logging is V3 and documents only) | EM-R4, N9, K5; G-37 |
| Identity numbers | ? | HR ?; EMP Self ?; others — | Yes (TSD §15) | ? | ? (as above) | EM-R1, N9, K5; G-37 |
| Punch location (lat, long, accuracy, fence distance) | Yes | HR S (PC-R13); MGR ? (needs it to decide a geo flag, E18); EMP Self ?; AUD ?; others — | Not stated | n/a | Not stated | PC-R13, PC-R15, K5; G-18 (OQ-C04-17) |
| Selfie photo | Yes, where PC-02 selfie = on | ? for every role | Not stated | n/a | Not stated | PC-R4, K5, TB-02; G-18 (OQ-C04-17) |
| Audit trail | Yes | AUD S; OA support-access entries (VC-R6); others ? | No | n/a | Not required by BRD | §5, VC-R6, §16; G-34 |
| Personal, contact, emergency details | Yes | HR S; EMP Self; others ? | No | n/a | Not stated | EM-R1, N9; G-14 |
| Salary, rate, components | V2 placeholder | V2 (PO, HR expected) | Not stated | V2 | V2 | EM-R4, PY-R4 |
| Documents (identity, bank proof) | V3 | V3 | — | — | Yes, every view and download (DC-R8) | DC-R8 |

Vendor users see none of these outside an active SupportAccessGrant (P12). What they may see inside one is G-02.

---

## 5. Counts

| Context | Operations |
|---|---|
| Platform | 19 |
| Identity & Access | 11 |
| Organisation Structure | 7 |
| Workforce | 8 |
| Time Capture | 11 |
| Scheduling | 8 |
| Policy | 7 |
| Attendance | 13 |
| Leave & Absence | 8 |
| Permission & On-duty | 13 |
| Comp-off | 7 |
| Period | 8 |
| Opening Balances | 4 |
| Notifications settings | 1 |
| Reports | 6 |
| Dashboards | 3 |
| Audit | 4 |
| **Total** | **138** |

---

## 6. Gaps

| Gap | Cells | Proposed question | Existing OQ |
|---|---|---|---|
| G-01 | VS column; PLT-13 VR | What may the VC-05 "support" role do in the console that "read-only" cannot, and which vendor roles may request support access? | OQ-V01-14 |
| G-02 | PLT-16, PLT-17; §4 vendor | Under support access, may vendor users only view, or also change settings and non-result data? Which vendor roles? | OQ-V01-12 |
| G-03 | PLT-15 | How long is the support window, and may the Organisation Admin (or the vendor) end it early? | OQ-V01-13 |
| G-04 | PLT-09 | Who sets VC-03: the vendor (console setting) or the Organisation Admin (it protects the organisation)? | new |
| G-05 | PLT-19 | May the Organisation Admin and Auditor see the vendor's plan / module / limit history for their organisation? | OQ-V01-05 |
| G-06 | PLT-12 | Who creates vendor users and assigns VC-05 roles: full admin only? | new (US-V01-13 assumes full admin) |
| G-07 | IAM-01, IAM-02 | May the HR Manager create users and assign roles and scope, or only the Organisation Admin? | OQ-C03-12 |
| G-08 | IAM-05, IAM-06, SCH-01 | Who changes AC-02, AC-03, AC-04, AC-06, AC-08, and does AC-04 approval cover access settings, shifts, holidays and types as well as policy? | OQ-C03-11 |
| G-09 | P9, all AUD cells | Can system roles be edited beyond §5 (for example Auditor given writes)? | OQ-C03-01 |
| G-10 | IAM-09 | Who may set a delegation (approver, HR, OA), and must the delegate's scope cover the delegator's team? | OQ-C03-10 |
| G-11 | ORG-01..05, SCH-05 | Who maintains branches, geo-fences, IP lists, lists, contractors and holiday calendars: OA, HR or both? (PC-R14 says HR for fences; US-C01-01 says OA.) | OQ-C01-01 |
| G-12 | TCP-01 | Who registers, moves and retires devices: OA (branch owner) or HR (enrolment owner)? | new (extends OQ-C01-01) |
| G-13 | WRK-01..05 † | Confirm the HR Manager maintains the employee master (§5 does not say); may the OA also? | new; transfers out of scope OQ-C02-09 |
| G-14 | OA/PO/MGR/SUP/AUD read cells in ORG-07, WRK-08, SCH-03, ATT-01, ATT-13, PER-08, RPT-03, §4 | What may the OA, PO, Manager, Supervisor and Auditor read beyond their §5 duties (employee master, results, period totals)? | new (relates OQ-C13-04) |
| G-15 | P4, SUP S cells | What is a Supervisor's "team" (reporting line, branch, shift, roster group), and does a Manager's scope default to their reporting team? | OQ-C03-04 (Manager part) + new (Supervisor) |
| G-16 | TCP-06 | Who decides each PunchFlag reason (manager per E18, HR, or by reason), and with what escalation? | OQ-C04-02, DP-05 |
| G-17 | TCP-04, ATT-06 | Who is the BR-01 approver of an HR manual entry or HR correction: self-approved or a second person? | OQ-C04-08, DP-08 |
| G-18 | TCP-08, §4 location / selfie | Which roles may see punch location and selfies, is a view logged, and how long are they kept? | OQ-C04-17 |
| G-19 | SCH-02 | May Managers or Supervisors build or change rosters for their team? | new |
| G-20 | SCH-06 | Does an optional-holiday pick need approval, and by whom? | OQ-C07-04 |
| G-21 | SCH-08 | For a swap, must the colleague agree, and who approves when the two have different approvers? | OQ-C05-12 |
| G-22 | ATT-10 | Under EC-14, what is the request (a permission, a new request type), and who approves it? | new |
| G-23 | ATT-11 | Who acts on an LV-01 "flag for leave cancellation" flag? | OQ-C08-18 |
| G-24 | LVE-06 | Who may cancel leave (employee, approver, HR), until when, and does cancelling approved leave need approval? | OQ-C08-10 |
| G-25 | LVE-03 | May HR or a Manager raise a leave request on behalf of an employee (for example a worker without a phone), and who approves it then? | new |
| G-26 | LVE-07 | Who may post a manual leave ledger adjustment, and does it need approval? | new |
| G-27 | PRM-11 | Who may cancel or change on-duty, and until when? | OQ-C09-10 |
| G-28 | PRM-12 | How is a work-from-home day requested and approved, and by whom? | DP-09, OQ-C04-14 |
| G-29 | PER-02, PER-03, RPT-04 | Does the Payroll Officer exist in V1; if not, does the HR Manager lock periods and see the checklist? | OQ-C12-05, OQ-C16-03 |
| G-30 | PER-04 | Who requests a period unlock (PO, HR), and can the requester be the OA (BR-07)? | new (relates OQ-C12-07) |
| G-31 | PER-06, PER-07, RPT-04 | Who produces and downloads the payroll input export in V1? | OQ-C12-05, OQ-C12-11 |
| G-32 | OBL-02, OBL-03 | Under OB-01 "with OA approval", does approval apply to entry, confirmation, or both, and to OB-02 changes? | OQ-C15-07 |
| G-33 | RPT-01..03 | Default report rights for OA, PO, Manager (own team), Supervisor, Employee (self)? | OQ-C13-04 |
| G-34 | POL-07, OBL-04, RPT-05, RPT-06, ADT-01, ADT-02 | May the OA and HR read settings history (R26), opening balances (R25) and the audit trail (R27), or only the Auditor? | new (extends OQ-C13-04) |
| G-35 | DSH-01, DSH-03 | Does the Auditor get a dashboard; who sets per-user default filters (DB-02); who picks vendor widgets? | OQ-C16-02, OQ-C16-06 + new (DB-02) |
| G-36 | TCP-07 | May OA, PO, Manager, Supervisor and Auditor see raw punches (evidence), or only results? Is a raw punch a "result" for the Auditor? | new |
| G-37 | §4 bank / identity | Are bank details and identity numbers held in V1; who may view and unmask them; is each unmasked view logged? | OQ-C02-05 + new |
| G-38 | EMP Self cells for other roles | Can one person hold several roles, and how does a Manager or HR Manager use self-service? | OQ-C03-03 |
| G-39 | LVE-08, PRM-13, CMP-07 | May a Manager see team members' leave, permission and comp-off balances (DB-R5 shows only a leave calendar)? | new (relates OQ-C13-04) |
| G-40 | TCP-03 | While suspended, are app and web punches accepted (VC-R4 "punches are still stored" vs "view and export only")? | OQ-V01-08 |
