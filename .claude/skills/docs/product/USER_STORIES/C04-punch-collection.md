# C04 Punch collection — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §3, §4.3 (VC-R4, VC-R8), §6, §9.2 (EM-R9, EM-06 to EM-08), §9.4, §12 (BR-01, BR-03, BR-05, BR-06, BR-07, BR-11), §13.1 (E17, E18), §14 (R5), §15 (N4, N5, N9, N12), §16, §18 (K5, K9), §19 (AT13, AT16, AT19, AT20, AT21), §22.2 (TB-01, TB-02, TB-04); TSD §5.5, §9 (context only)

## Scope
- In scope (V1): eSSL device registration, online status and direct push (PC-R1, PC-R2, TB-04); enrolment mapping and enrolment status (PC-R3, PC-R8); storing every punch unchanged (PC-R6, BR-01, BR-11); face punches from a device or app selfie (PC-R4); app punches with location, offline capture and later sync (PC-R5, N4); web punches with location (PC-R11); geo-fence setup, test and evaluation for app and web (PC-R12 to PC-R15, PC-02 to PC-04, PC-12 to PC-17); punches at master, allowed and not-allowed branches (EM-R9, EM-07, E17, AT16); late, out-of-order, late-sync and clock-mismatch handling (PC-R7, PC-07, PC-08); device offline alert (PC-R10, PC-10); manual attendance during a device failure (PC-R9, PC-11); punches on a weekly off, holiday or with no shift (PC-06); work-from-home punches (PC-09); punch sources on/off (PC-01).
- Out of scope / later release: other device makes and the vendor's own device (PC-R1, BRD §21 "more device makes" not yet scheduled; V1 must only not block them, N12). Reading punches into sessions, the reading mode and repeat-punch cleaning belong to C06 (PR-01 to PR-06). Regularisation requests belong to C06 (RG-xx). Notification channels belong to C14 (NT-08). Branch costing in pay is V2 (M03); V1 records the branch worked for EM-08 and R10.
- Dependency notes: the direction of a punch (in or out) is not decided by C04; it comes from the reading mode PR-01 (C06). Attendance date assignment uses the shift window and SH-06 (C05, C06).

## Stories

### US-C04-01 Register devices against a branch and see whether they are online
As an HR Manager, I want each biometric device registered against a branch with its online status shown, so that I know which punches come from where and which devices need attention.
Source: PC-R1, PC-R2, OR-R1, VC-R3, VC-R8, DB-R3, R5, TB-01, TB-04, A2, N12
Settings involved: PC-01 (biometric device source on/off)

Acceptance criteria
- AC-C04-01.1 Given an HR Manager with access to branch A, When they register an eSSL device against branch A, Then the device is listed under branch A of their organisation only.  (Source: PC-R2, OR-R1, TB-01)
- AC-C04-01.2 Given a registered device, When HR opens the device list, Then each device shows whether it is online or offline.  (Source: PC-R2, R5)
- AC-C04-01.3 Given devices in several organisations, When the Vendor Admin opens the health view, Then device sync failures across organisations are visible, and an organisation's own users see only their organisation's devices.  (Source: VC-R3, VC-R8, AT20)
- AC-C04-01.4 Given an HR Manager whose access is limited to branch A, When they view devices, Then devices of branch B are not shown.  (Source: UM-R2, AC-07, AT13)
- AC-C04-01.5 Given a registered eSSL device, When it pushes punches, Then the punches reach the server over a secure connection, directly or through an approved relay where the model cannot connect securely.  (Source: TB-04, Q15)
- AC-C04-01.6 Given a device is registered, re-assigned or removed, When the change is saved, Then the audit log records who, when and the change.  (Source: §16 audit log, G9)

Edge and negative cases
- Push from a device that is not registered to any organisation: OQ-C04-04.
- Device model that cannot connect securely: AC-C04-01.5; the relay decision is pending BRD Q15.

### US-C04-02 Receive and store device punches unchanged
As an HR Manager, I want every punch pushed by an eSSL device stored exactly as received with its source, device, time and branch time zone, so that punches remain the untouched evidence for attendance and pay.
Source: PC-R1, PC-R6, BR-01, BR-11, A2, §16 (Punch), VC-R4, N4, AT21; TSD §5.5, §9.1 (context)
Settings involved: PC-01 (biometric device on/off), PC-08 (clock mismatch tolerance, default 5 minutes)

Acceptance criteria
- AC-C04-02.1 Given a registered device at branch A mapped to employee enrolment ID 101, When the device pushes a punch for ID 101, Then a punch is stored for that employee with source "biometric device", the device, the punch time as sent by the device, the time the server received it, and branch A's time zone.  (Source: PC-R6, BR-11, §16)
- AC-C04-02.2 Given a stored punch, When anyone views it, Then it carries no in or out direction of its own; direction is decided later by the reading mode set in PR-01 (C06).  (Source: §6 Punch, PR-01)
- AC-C04-02.3 Given a device resends a punch it already pushed (retry), When the server receives it again, Then no second punch is created and the device's retry still succeeds.  (Source: BR-01, AT5; TSD §5.5)
- AC-C04-02.4 Given two genuinely different punches by the same employee a few seconds apart, When both are pushed, Then both are stored; merging repeats is left to PR-02 and PR-03 (C06).  (Source: PC-R6, PR-02)
- AC-C04-02.5 Given the organisation is suspended, When a device pushes punches, Then the punches are still stored.  (Source: VC-R4)

Edge and negative cases
- Enrolment ID not mapped to any employee: OQ-C04-04.
- Device push while PC-01 biometric device is off: OQ-C04-07.
- Device clock wrong: US-C04-14.
- Punch arriving days late: US-C04-14.

### US-C04-03 Map employees to device enrolment IDs and track enrolment status
As an HR Manager, I want to enrol each employee's fingerprint or face and map their enrolment ID, with an enrolment status per employee, so that every device punch is attributed to the right person.
Source: PC-R3, PC-R8, EM-R3, EM-R6, K5
Settings involved: none

Acceptance criteria
- AC-C04-03.1 Given an employee with no enrolment, When HR views the employee, Then the enrolment status shows "not enrolled".  (Source: PC-R8)
- AC-C04-03.2 Given HR enrols the employee and maps enrolment ID 101, When saved, Then the status shows "enrolled" and later punches from ID 101 are attributed to that employee.  (Source: PC-R3, PC-R8)
- AC-C04-03.3 Given an employee whose enrolment must be redone, When the status is set to "re-enrolment needed", Then it is shown on the employee and in the enrolment list.  (Source: PC-R8)
- AC-C04-03.5 Given a user without the HR Manager role (or an equivalent custom role permission), When they try to change an enrolment mapping, Then the change is refused.  (Source: PC-R8, §5 HR Manager, AC-01)
- AC-C04-03.6 Given an enrolment mapping changes, When saved, Then the change is dated and kept in history with who made it.  (Source: EM-R6, G9, §16)

Edge and negative cases
- Who or what sets "re-enrolment needed": OQ-C04-12.
- Same enrolment ID mapped to two employees, or used on devices at different branches: OQ-C04-04.

### US-C04-04 Be alerted when a device is offline too long
As an HR Manager, I want an alert when a device has been offline longer than the set time, so that I can fix it or record manual attendance before punches are lost.
Source: PC-R10, PC-10, DB-R3, R5, VC-R8, TB-03, AT21
Settings involved: PC-10 (offline alert after N minutes, default 30)

Acceptance criteria
- AC-C04-04.1 Given PC-10 = 30 and a device last seen 29 minutes ago, When the check runs, Then no alert is sent.  (Source: PC-10)
- AC-C04-04.2 Given PC-10 = 30 and a device offline for more than 30 minutes, When the check runs, Then the HR Manager responsible for that branch is alerted once for that outage.  (Source: PC-R10, PC-10)
- AC-C04-04.3 Given the offline check runs twice for the same moment, When it completes, Then the HR Manager does not receive a duplicate alert.  (Source: TB-03, AT21)
- AC-C04-04.4 Given a device is offline, When HR or the Organisation Admin opens their dashboard, Then the device appears under devices offline for its branch.  (Source: DB-R3)
- AC-C04-04.5 Given an HR Manager scoped to branch B, When a device at branch A goes offline, Then they do not receive the alert.  (Source: UM-R2, AT13)

Edge and negative cases
- Device comes back online and pushes stored punches: US-C04-14 (late punches recalculated).

### US-C04-05 Record manual attendance when a device fails
As an HR Manager, I want to record manual attendance for an affected date and branch on one screen with a reason, so that employees are not marked absent because a device failed.
Source: PC-R9, PC-11, BR-01, BR-03, BR-07, RG-11, §16, §5 HR Manager
Settings involved: PC-11 (HR Manager only / can delegate to a Supervisor, default HR Manager only), PC-01 (entry by HR on/off)

Acceptance criteria
- AC-C04-05.1 Given a device failure at branch A on 10 November, When the HR Manager records attendance for the affected employees of branch A for 10 November on one screen with a reason, Then each entry is stored as a new punch marked manual, with the reason, the person who entered it and the time of entry.  (Source: PC-R9, BR-01, §16)
- AC-C04-05.2 Given manual entries are saved for an open period, When saving completes, Then the affected days are recalculated.  (Source: BR-03, PC-R7)
- AC-C04-05.3 Given PC-11 = HR Manager only, When a Supervisor tries to record manual attendance, Then it is refused.  (Source: PC-11)
- AC-C04-05.4 Given PC-11 = HR Manager can delegate to a Supervisor and the HR Manager has delegated to Supervisor S for branch A, When S records manual attendance for branch A, Then it is accepted and logged under S.  (Source: PC-11)
- AC-C04-05.5 Given the reason is left blank, When HR tries to save, Then the save is refused.  (Source: PC-R9)
- AC-C04-05.6 Given any raw device punches exist for that date, When manual entries are added, Then the raw punches remain unchanged and both are visible, the manual ones marked manual.  (Source: BR-01, PC-R6)
- AC-C04-05.7 Given the date falls in a locked period, When HR records manual attendance: the locked period is unchanged and only the difference posts to the next open period.  (Source: BR-06, PL-R4)

Edge and negative cases
- Who is the "approver" BR-01 requires when HR enters the manual punch itself: OQ-C04-08.
- PC-01 "entry by HR" switched off while a device fails: OQ-C04-07.
- Relationship to bulk regularisation for a device outage (RG-11, C06): reference only; not specified here.

### US-C04-06 Keep raw punches as immutable evidence and correct only by adding
As an Auditor, I want raw punches never edited or deleted and every correction recorded as a new manual punch with reason and approver, so that any day's result can be traced back to untouched evidence.
Source: BR-01, BR-08, BR-11, PC-R6, §16, AT5, AC-05
Settings involved: AC-05 (employee can see own punches, on/off)

Acceptance criteria
- AC-C04-06.1 Given any stored punch, When any user (including Vendor Admin under support access, HR Manager or Organisation Admin) tries to edit or delete it, Then no screen, import or API allows it.  (Source: BR-01, VC-R6)
- AC-C04-06.2 Given a correction is approved (for example a missed check-out approved under regularisation in C06), When it is applied, Then a new punch marked manual is added with the reason and the approver, and the original punches are unchanged.  (Source: BR-01, E13)
- AC-C04-06.3 Given a day result uses punches, When HR views the explanation for that day, Then it shows which raw and manual punches were used.  (Source: BR-08, §16 Daily result)
- AC-C04-06.4 Given a punch, When it is stored, Then its exact time is kept (no rounding) in UTC together with the branch's time zone.  (Source: BR-11, RD rules note)
- AC-C04-06.5 Given AC-05 own punches = on, When an employee views their attendance, Then they see their own punches including which are manual; given AC-05 own punches = off, Then they do not.  (Source: AC-05)
- AC-C04-06.6 Given a user of organisation X, When they request a punch of organisation Y by any screen, API, report or export, Then nothing is returned.  (Source: TB-01, AT20, N1)
- AC-C04-06.7 Given the same punches are reprocessed, When processing completes, Then the punch records and the day results are identical to the first run.  (Source: AT5, BR-02)

Edge and negative cases
- Punch captured by mistake (wrong person on a shared device): cannot be deleted (BR-01); excluding it from the day is a C06 regularisation ("wrong punch", RG-02).

### US-C04-07 Punch from the app with location and optional selfie
As an Employee, I want to check in and check out from the app with my location and, where required, a selfie, so that my attendance is recorded where there is no device.
Source: PC-R4, PC-R5, PC-R13, PC-R15, PC-02, PC-01, MD-02, K5, N9, TB-02
Settings involved: PC-01 (mobile app on/off), PC-02 (app requirements: location, selfie, inside geo-fence, each on/off)

Acceptance criteria
- AC-C04-07.1 Given PC-01 mobile app = on, the plan includes the app, and PC-02 location = on, When the employee punches, Then the punch is stored with source "mobile app", time, latitude, longitude, accuracy and distance from the nearest allowed fence.  (Source: PC-R5, PC-R13, MD-02)
- AC-C04-07.2 Given PC-02 selfie = on, When the employee punches without a selfie, Then the punch is not accepted and the employee is told a selfie is needed.  (Source: PC-02, PC-R4)
- AC-C04-07.3 Given PC-02 selfie = on, When the employee punches with a selfie, Then the selfie is stored with the punch as a private file of the organisation, and HR can open it only after an access check.  (Source: PC-R4, TB-02, K5)
- AC-C04-07.4 Given PC-01 mobile app = off, or the plan does not include the app, When an employee tries to punch from the app, Then the punch is refused and the reason shown.  (Source: PC-01, MD-02)
- AC-C04-07.5 Given an employee has punched in, When time passes until the next punch, Then no location is read or stored between punches.  (Source: PC-R15)
- AC-C04-07.6 Given an app punch, When it is stored, Then it is stored unchanged as in US-C04-02, with no in or out direction of its own (PR-01).  (Source: PC-R6, BR-01, PR-01)

Edge and negative cases
- Geo-fence outcome: US-C04-11.
- App cannot read location (permission refused or no fix): OQ-C04-10 (PC-13 covers web only).
- Whether a selfie is matched against the enrolled face: OQ-C04-09.

### US-C04-08 Punch from the app without a network and sync later
As an Employee, I want my app punch recorded even when there is no network and sent later with its original time, so that poor coverage does not cost me attendance.
Source: PC-R5, N4, PC-R7, PC-07, PC-08, BR-01, BR-03, BR-05, BR-06; TSD §9.3 (context)
Settings involved: PC-07 (late-sync flag after N days, default 3), PC-08 (clock mismatch tolerance, default 5 minutes)

Acceptance criteria
- AC-C04-08.1 Given the phone has no network, When the employee punches at 09:02 branch time, Then the app records the punch and shows it as waiting to sync.  (Source: PC-R5, N4)
- AC-C04-08.2 Given the punch is synced at 11:30 the same day, When it is stored, Then the punch time is 09:02 (original time), and the server receipt time 11:30 is also kept.  (Source: N4, BR-11)
- AC-C04-08.3 Given the same offline punch is sent twice (sync retried), When the server processes both, Then only one punch exists.  (Source: BR-01; TSD §5.5, §9.3)
- AC-C04-08.4 Given an offline punch syncs after other punches for the same day were already processed, When it arrives, Then the affected day is recalculated with all punches in time order.  (Source: PC-R7, BR-03)
- AC-C04-08.5 Given PC-07 = 3 days, When an offline punch arrives more than 3 days after its punch time, Then it is stored and flagged "late sync", not rejected.  (Source: PC-07, NT-08)
- AC-C04-08.6 Given an offline punch carried location, When it syncs, Then the geo-fence outcome is decided on the server from the captured location using the settings version in force on the punch date.  (Source: PC-R12, BR-05, G5)
- AC-C04-08.8 Given the punch date falls in a period that was locked before the sync, When it arrives, Then the locked period is unchanged and only the difference posts to the next open period.  (Source: BR-06, PL-R4, PL-07)

Edge and negative cases
- Phone clock changed before an offline punch: OQ-C04-05.
- Offline punch flagged "late sync": counting before approval is OQ-C04-01.
- Offline punch synced after the employee has left or the period is beyond PL-07 correction limit: OQ-C04-13.

### US-C04-09 Punch from the web with location
As an Employee, I want to check in and check out from a browser with my location captured at the moment of the punch, so that I can mark attendance without a device or the app.
Source: PC-R11, PC-R12, PC-R13, PC-R15, PC-01, PC-05, PC-12, PC-13, PC-14, PC-15, E18, AT19, K9
Settings involved: PC-01 (web on/off), PC-12 (web requirements: location, inside geo-fence; default both on), PC-13 (location not available on web, default block), PC-14 (accuracy threshold, default 100 metres, flagged), PC-05 (IP restriction, default off), PC-15 (location and IP both on: default either may pass)

Acceptance criteria
- AC-C04-09.1 Given PC-12 location = on and the employee allows location in the browser, When they punch, Then the punch stores latitude, longitude, accuracy and distance from the nearest allowed fence, read at that moment only.  (Source: PC-R11, PC-R13, PC-R15)
- AC-C04-09.2 (E18) Given branch fence radius 200 metres and PC-04 = record and flag, When a web check-in is made 60 metres from the centre, Then it is accepted.  (Source: E18, PC-03, PC-04)
- AC-C04-09.3 (E18) Given the same fence and PC-04 = record and flag, When a web check-in is made 450 metres away, Then it is recorded and flagged, and not counted until the manager approves.  (Source: E18, PC-04)
- AC-C04-09.4 (E18) Given PC-13 = block, When the employee refuses location permission in the browser, Then the punch is blocked and the employee is told why.  (Source: E18, PC-13)
- AC-C04-09.5 Given PC-13 = record and flag, When the browser cannot provide location, Then the punch is recorded and flagged for approval with no location.  (Source: PC-13)
- AC-C04-09.6 Given PC-14 = 100 metres, flagged, When the browser reports accuracy of 100 metres or better, Then no accuracy flag is raised; When it reports accuracy worse than 100 metres, Then the punch is recorded and flagged.  (Source: PC-14, K9)
- AC-C04-09.7 Given PC-14 = blocked, When accuracy is worse than the threshold, Then the punch is blocked and the employee is told why.  (Source: PC-14)
- AC-C04-09.8 Given PC-05 = on and PC-12 location = off, When the employee punches from an IP address not on the list, Then the punch is refused; from a listed IP address, it is accepted.  (Source: PC-05)
- AC-C04-09.9 Given PC-05 = on, PC-12 location = on and PC-15 = either may pass, When the IP is listed but the location is outside the fence, Then the punch passes; When the IP is not listed but the location is inside the fence, Then the punch passes.  (Source: PC-15)
- AC-C04-09.10 Given PC-15 = both must pass, When either the IP check or the location check fails, Then the punch is handled as failing that check (PC-04 for location, refusal for IP).  (Source: PC-15, PC-04, PC-05)
- AC-C04-09.11 (AT19) Given an app punch and a web punch at the same spot with the same settings, When both are evaluated, Then both get the same outcome (accepted, flagged or blocked).  (Source: AT19, PC-R12)
- AC-C04-09.12 Given PC-01 web = off, When an employee tries a web punch, Then it is refused.  (Source: PC-01)

Edge and negative cases
- PC-15 = either may pass when location is unavailable and PC-13 = block but IP passes: OQ-C04-11.
- Flag combined with another flag on the same punch: OQ-C04-01.

### US-C04-10 Set and test a branch geo-fence
As an HR Manager, I want to set each branch's fence by coordinates or a map point and test it from the branch before switching it on, so that genuine punches at the branch are not wrongly flagged.
Source: PC-R14, PC-03, PC-16, OR-R1, G9, K9
Settings involved: PC-03 (fence per branch: centre and radius, more than one allowed, default radius 200 metres), PC-16 (fences that apply: default master and allowed branches)

Acceptance criteria
- AC-C04-10.1 Given HR enters a centre point by coordinates or by picking on a map, When saved without a radius, Then the radius is 200 metres.  (Source: PC-R14, PC-03)
- AC-C04-10.2 Given a branch needs two areas, When HR adds a second fence, Then both fences belong to the branch and a punch inside either is inside that branch's fence.  (Source: PC-03)
- AC-C04-10.3 Given a fence that is not yet switched on, When HR tests it from the branch, Then HR sees whether the test location is inside, and the distance and accuracy, and no punch is created.  (Source: PC-R14)
- AC-C04-10.4 Given a fence is not yet switched on, When employees punch, Then that fence has no effect on outcomes.  (Source: PC-R14, G2)
- AC-C04-10.5 Given HR changes a fence, When saved, Then the old and new values, who, when and reason are recorded, and the change applies from its start date.  (Source: G5, G9)
- AC-C04-10.6 Given an HR Manager scoped to branch A, When they try to change branch B's fence, Then it is refused.  (Source: UM-R2, AC-07)

Edge and negative cases
- Fence changed after punches were flagged under the old fence: whether flags are re-evaluated is OQ-C04-15.

### US-C04-11 Decide the geo-fence outcome on the server
As an HR Manager, I want each app and web punch's location checked against the fences that apply to the employee, with the outcome set by our settings, so that location rules are enforced the same way for everyone and cannot be faked by the device.
Source: PC-R12, PC-R13, PC-02, PC-04, PC-12, PC-14, PC-16, PC-17, EM-07, AT19, OD-07; TSD §9.2 (context)
Settings involved: PC-04 (outside fence: default record and flag), PC-16 (default master and allowed branches), PC-17 (on-duty and WFH days: default not applied, location still captured), PC-02 / PC-12 (inside geo-fence requirement on/off)

Acceptance criteria
- AC-C04-11.1 Given an app or web punch with location, When the outcome is decided, Then it is decided by the system from the stored location and the fence definitions; any "inside" or "outside" claim sent by the phone or browser has no effect on the outcome.  (Source: PC-R12, PC-R13; TSD §9.2)
- AC-C04-11.2 Given PC-16 = master and allowed branches, When the punch is inside the fence of the master or an allowed branch, Then it is accepted for geo-fence purposes.  (Source: PC-R12, PC-16)
- AC-C04-11.3 Given PC-16 = master branch only, When the punch is inside an allowed branch's fence but outside the master branch's fence, Then it is treated as outside the fence (PC-04).  (Source: PC-16, PC-04)
- AC-C04-11.4 Given the punch is outside every applicable fence, When PC-04 = block, Then it is blocked and the employee told why; When PC-04 = record and flag, Then it is recorded and flagged for approval; When PC-04 = allow, Then it is accepted with its location and distance stored.  (Source: PC-04, PC-R13)
- AC-C04-11.5 Given the punch is inside the fence of a branch of the organisation that is not one of the employee's allowed branches, When the outcome is decided, Then EM-07 applies (default record and flag for approval).  (Source: PC-R12, EM-07)
- AC-C04-11.6 Given PC-17 = not applied and the employee has an approved on-duty or work-from-home day, When they punch from anywhere, Then no geo-fence outcome is applied and location is still captured; Given PC-17 = applied, Then AC-C04-11.2 to 11.4 apply.  (Source: PC-17, OD-07)
- AC-C04-11.7 Given PC-02 inside geo-fence = off (app) or PC-12 inside geo-fence = off (web), When the employee punches, Then no geo-fence outcome is applied and location (if on) is still stored.  (Source: PC-02, PC-12)
- AC-C04-11.8 Given any flagged punch, When it is stored, Then the flag reason is recorded on the punch and the punch appears in the exceptions report.  (Source: R5, NT-08)

Edge and negative cases
- Relationship between PC-02 "inside geo-fence" and PC-04 = allow: AC-C04-11.4 and 11.7 give the same visible result; no conflict assumed.
- On-duty with OD-07 = check-in at the place: the on-duty place is not a branch fence; how the place is checked is OQ-C04-16.

### US-C04-12 Punch at the master, an allowed, or a not-allowed branch
As an HR Manager, I want punches at the master and allowed branches accepted under the employee's own rules, and punches at other branches handled as configured, so that staff moving between branches are paid correctly.
Source: EM-R9, EM-R10, EM-06, EM-07, EM-08, PC-R12, E17, AT16, R10
Settings involved: EM-06 (allowed branches, default master only), EM-07 (punch at a not-allowed branch, default record and flag), EM-08 (costed to: default the branch worked)

Acceptance criteria
- AC-C04-12.1 (E17) Given master branch A and allowed branch B, When the employee punches in and out at B, Then the day is present under the employee's own shift and rules, and the day is costed to branch B.  (Source: E17, EM-R9, EM-08)
- AC-C04-12.2 (E17) Given the same employee punches at branch C, which is not allowed, and EM-07 = record and flag, When the punch arrives, Then it is recorded and flagged for approval.  (Source: E17, EM-07)
- AC-C04-12.3 Given EM-07 = block, When the employee punches by app or web at branch C, Then the punch is refused and the reason shown.  (Source: EM-07, PC-R12)
- AC-C04-12.4 Given EM-08 = master branch, When the employee works at B, Then the day is costed to master branch A.  (Source: EM-08)
- AC-C04-12.5 Given EM-06 = master branch only, When the employee punches at any other branch, Then that branch is treated as not allowed (EM-07).  (Source: EM-06, EM-07)
- AC-C04-12.6 (AT16) Given the punch is at an allowed branch, When the day is processed, Then the employee's own policy set and payroll group apply, not branch B's.  (Source: AT16, EM-R9)
- AC-C04-12.7 Given days at several branches in a period, When HR opens the days-worked-by-branch report, Then each day is counted under the branch it is costed to.  (Source: EM-R10, R10)

Edge and negative cases
- Allowed branch in a different time zone: OQ-C04-03.
- EM-07 = block for a biometric device punch, which is already captured: OQ-C04-06.

### US-C04-13 Handle punches on a weekly off, holiday or with no shift
As an HR Manager, I want punches on a weekly off, a holiday or a day with no shift handled as configured, so that unexpected work is visible and controlled.
Source: PC-06, SH-09, SH-10, HD-R1, HW-01, CO-R1, NT-08
Settings involved: PC-06 (allow / allow and flag / block, default allow and flag)

Acceptance criteria
- AC-C04-13.1 Given PC-06 = allow and flag, When the employee punches on their weekly off, Then the punch is stored and flagged.  (Source: PC-06)
- AC-C04-13.2 Given PC-06 = allow, When the employee punches on a holiday, Then the punch is stored with no flag.  (Source: PC-06)
- AC-C04-13.3 Given PC-06 = block, When the employee tries an app or web punch on a day with no shift assigned, Then it is refused and the reason shown.  (Source: PC-06)
- AC-C04-13.4 Given a punch on a holiday or weekly off is stored, When the day is processed, Then what the work earns (comp-off, overtime, ignore) follows HW-01 in M02, not C04.  (Source: HW-01, CO-R1)
- AC-C04-13.5 Given a night-shift punch after midnight on the calendar day of a weekly off, When deciding whether PC-06 applies, Then the weekly off is judged against the punch's attendance date (SH-06), not the calendar date of the punch.  (Source: SH-06, SH-07)

Edge and negative cases
- PC-06 = block for a biometric device punch: OQ-C04-06.

### US-C04-14 Accept late, out-of-order, late-synced and clock-mismatched punches
As an HR Manager, I want punches that arrive late or out of order accepted and the affected days recalculated, with late sync and clock mismatch flagged rather than rejected, so that no genuine punch is lost and suspicious ones are visible.
Source: PC-R7, PC-07, PC-08, BR-03, BR-06, BR-11, NT-08, R5, VC-R8; TSD §9.1 (context)
Settings involved: PC-07 (late-sync, default 3 days), PC-08 (clock mismatch tolerance, default 5 minutes)

Acceptance criteria
- AC-C04-14.1 Given a device was offline and pushes yesterday's punches today, When they arrive, Then they are stored with their device time and the receipt time, and yesterday is recalculated.  (Source: PC-R7, BR-03, BR-11)
- AC-C04-14.2 Given a check-out punch arrives before the check-in punch of the same day, When both are stored, Then the day is processed using punch times in order, not arrival order.  (Source: PC-R7)
- AC-C04-14.3 Given PC-07 = 3, When a punch arrives 3 days or less after its punch time, Then no late-sync flag is raised; When it arrives more than 3 days after, Then it is stored and flagged "late sync".  (Source: PC-07)
- AC-C04-14.4 Given PC-08 = 5 minutes, When a device's clock differs from server time by 5 minutes or less, Then no flag is raised; When it differs by more than 5 minutes, Then the punch is stored with its device time unchanged and flagged "clock mismatch".  (Source: PC-08, BR-01)
- AC-C04-14.5 Given a flagged punch (late sync or clock mismatch), When the employee and HR are notified (NT-08 on), Then the flag reason is stated; the punch is never rejected for these reasons alone.  (Source: PC-07, PC-08, NT-08)
- AC-C04-14.6 Given a late punch belongs to a locked period, When it arrives, Then the locked period is unchanged and the difference posts to the next open period tagged with the original date.  (Source: BR-06, PL-R4)

Edge and negative cases
- How a clock mismatch is measured for a punch delivered late: OQ-C04-05.
- Whether late-sync and clock-mismatch punches count before approval: OQ-C04-01.

### US-C04-15 Review and act on flagged punches
As a Manager (Approver), I want to see flagged punches for my team and approve or reject them, so that genuine attendance counts and doubtful punches do not.
Source: PC-04, PC-06, PC-07, PC-08, PC-13, PC-14, EM-07, E17, E18, AC-02, BR-07, BR-03, NT-05, NT-08, R5, PL-R2
Settings involved: AC-02 (approver per request type), PC-04, EM-07, PC-06, PC-13, PC-14

Acceptance criteria
- AC-C04-15.1 (E18) Given a web check-in 450 metres outside a 200-metre fence was recorded and flagged, When the manager has not yet acted, Then the punch is not counted in the day's result; When the manager approves, Then it is counted and the day is recalculated.  (Source: E18, BR-03)
- AC-C04-15.2 Given a flagged punch, When the approver acts, Then the original punch is unchanged and the approval, approver and time are recorded against it.  (Source: BR-01, §16 Punch approver)
- AC-C04-15.3 Given an employee who is also an approver, When their own punch is flagged, Then they cannot approve it.  (Source: BR-07)
- AC-C04-15.4 Given a manager, When they open flagged punches, Then they see only punches of employees they approve for, with the flag reason, location, accuracy and distance.  (Source: AC-02, PC-R13, AT13)
- AC-C04-15.5 Given flagged punches are still undecided when the Payroll Officer prepares to lock, When the pre-lock checklist is shown, Then they are listed as flagged items.  (Source: PL-R2)

Edge and negative cases
- Whether every flag type (EM-07, PC-06, PC-07, PC-08, PC-13, PC-14) is held back until approval like E18's geo-fence flag: OQ-C04-01.
- Who approves each flag type, what happens if nobody acts, and what a rejection does to the day: OQ-C04-02.

### US-C04-16 Punch on work-from-home days
As an Employee, I want to punch from home on approved work-from-home days, so that my attendance is recorded when I work remotely.
Source: PC-09, PC-17, LV-03
Settings involved: PC-09 (WFH punch: default only on approved WFH days), PC-17 (default geo-fence not applied, location still captured), LV-03 (WFH on/off, attendance by web punch, default on)

Acceptance criteria
- AC-C04-16.1 Given PC-09 = only on approved days and the employee has an approved WFH day, When they punch from home by web, Then the punch is accepted and location is captured but not checked against the fence (PC-17 default).  (Source: PC-09, PC-17, LV-03)
- AC-C04-16.2 Given PC-09 = only on approved days and no approved WFH day, When they punch from home outside the fence, Then the normal geo-fence outcome applies (PC-04).  (Source: PC-09, PC-04)
- AC-C04-16.3 Given PC-09 = never, When an employee punches from home, Then the normal geo-fence outcome applies.  (Source: PC-09)
- AC-C04-16.4 Given PC-09 = always, When an employee punches from home, Then it is treated as a WFH punch on any day.  (Source: PC-09)

Edge and negative cases
- How a WFH day is requested and approved: OQ-C04-14.

### US-C04-17 Control which punch sources the organisation uses
As an HR Manager, I want to switch each punch source on or off, so that only the methods our organisation uses can create punches.
Source: PC-01, G2, BR-14, MD-02
Settings involved: PC-01 (biometric device, web, mobile app, kiosk, file or API import, entry by HR; each on/off)

Acceptance criteria
- AC-C04-17.1 Given a source is switched off, When a punch from that source is attempted from the app or web, Then it is refused with a reason.  (Source: PC-01, G2)
- AC-C04-17.2 Given a source is switched off, When punches stored earlier from that source exist, Then they remain stored and unchanged.  (Source: BR-01, MD-03)
- AC-C04-17.3 Given a source setting changes, When saved, Then the settings history records who, when, old value, new value and reason.  (Source: G9)

Edge and negative cases
- Device pushes while PC-01 biometric device is off: OQ-C04-07.
- Kiosk and file or API import are listed in PC-01 but not described anywhere: OQ-C04-07.

### US-C04-18 See full punch details and punch exceptions
As an HR Manager, I want to see every punch with its source, device, times, location, accuracy, distance, manual marker and flags, and an exceptions report, so that I can investigate any attendance result.
Source: PC-R6, PC-R13, R1, R5, §16 (Punch), AC-05, AC-07, N5, N9, AT13, AT20, K5
Settings involved: AC-05, AC-07

Acceptance criteria
- AC-C04-18.1 Given an app or web punch, When HR opens the punch record, Then latitude, longitude, accuracy and distance from the nearest allowed fence are shown.  (Source: PC-R13)
- AC-C04-18.2 Given punches in a period, When HR opens the exceptions report, Then flagged punches (geo-fence, late sync, clock mismatch, branch not allowed, weekly off/holiday/no shift) and device status are listed, filterable by period, branch, department and employee, and exportable.  (Source: R5, §14)
- AC-C04-18.3 Given HR scoped to branch A, When they open punches or reports, Then punches of employees outside their scope are not shown.  (Source: AC-07, AT13)
- AC-C04-18.4 Given an Auditor, When they open punch records, Then they can view but not change anything.  (Source: §5 Auditor)

Edge and negative cases
- Who may see location and selfies (sensitive data): OQ-C04-17.

## Open questions
- OQ-C04-01 Before approval, does a punch flagged under "record and flag" count in the day's result for every flag type? E18 says a geo-fence-flagged punch is "not counted until the manager approves", but EM-07, PC-06 (allow and flag), PC-07 late sync, PC-08 clock mismatch, PC-13 and PC-14 do not say. Source: E18, PC-04, PC-06, PC-07, PC-08, PC-13, PC-14, EM-07. Why it matters: decides whether a day is present or absent (and later paid) while flags are pending.
- OQ-C04-02 Who approves each flagged punch type (E18 says the manager; AC-02 has no "flagged punch" request type), what happens when the approver does not act (no escalation setting like RG-07), and what a rejection does to the day given the punch can never be deleted (BR-01). Source: E18, AC-02, BR-01, PL-05. Why it matters: unresolved flags can block lock (PL-05 default) or silently change attendance.
- OQ-C04-03 When an employee punches at an allowed branch in a different time zone from the master branch, which time zone is used to store the punch, decide the attendance date and measure late and early: the punching branch (BR-11 "branch's time zone") or the master branch whose shift and rules apply (EM-R9)? Source: BR-11, EM-R9, SH-06. Why it matters: can move a punch to another date or create false late marks.
- OQ-C04-04 What happens to a punch from a device not registered to the organisation, or with an enrolment ID not mapped to any employee (stored and flagged for later mapping, or refused); and are enrolment IDs unique per organisation or per device/branch? Source: PC-R2, PC-R3, PC-R6. Why it matters: PC-R6 says every punch is stored, but an unattributed punch cannot be shown to any employee.
- OQ-C04-05 What does PC-08 "device clock mismatch" compare, given a late push legitimately has device time far from receipt time; and does it apply to phones (offline app punches with a changed phone clock) and to web punches? Source: PC-08, PC-07, N4. Why it matters: without a defined reference, every late-synced punch may also be flagged as clock mismatch, or changed phone clocks go undetected.
- OQ-C04-06 What does "block" mean for a biometric device punch under EM-07 = block or PC-06 = block, given the device has already captured it and BR-01 forbids deleting raw punches (stored but ignored, or never stored)? Source: EM-07, PC-06, BR-01, PC-R6. Why it matters: decides whether evidence of the attempt is kept.
- OQ-C04-07 PC-01 lists kiosk and file or API import as sources but no requirement describes them: are they in V1, and what happens to device pushes when the biometric device source is off, or to manual attendance when "entry by HR" is off during a device failure (PC-R9)? Source: PC-01, PC-R9, §21. Why it matters: scope and whether a source switch can make punches disappear.
- OQ-C04-08 Who is the approver BR-01 requires for manual attendance entered by the HR Manager under PC-R9, given BR-07 (nobody approves their own request)? Is the HR Manager's entry self-approved, or does it need a second person? Source: PC-R9, BR-01, BR-07, RG-06. Why it matters: affects the audit trail and whether manual entries count immediately.
- OQ-C04-09 For selfie punches (PC-R4), is the selfie only stored as evidence, or must it be matched to the employee's enrolled face, and with what outcome on no match? Source: PC-R4, PC-02, K5. Why it matters: decides whether a selfie prevents buddy punching or is only a record.
- OQ-C04-10 What happens when the app cannot obtain location (permission refused or no fix) while PC-02 location = on? PC-13 covers web only. Source: PC-02, PC-13, PC-R5. Why it matters: app and web must behave the same at the same spot (AT19), but the app has no rule.
- OQ-C04-11 With PC-15 = either may pass, PC-13 = block and the IP check passing, is a web punch without location accepted or blocked? Source: PC-13, PC-15, PC-05. Why it matters: the two defaults point in opposite directions.
- OQ-C04-12 Who or what sets an employee's enrolment status to "re-enrolment needed" (HR by hand, the device, repeated failures)? Source: PC-R8. Why it matters: decides whether this is a manual field or an automatic signal.
- OQ-C04-13 Is there a limit on how late an offline or device punch may arrive and still be applied (for example beyond PL-07 past periods, or after the employee has exited)? Source: PC-R7, PC-07, PL-07, BR-06. Why it matters: PC-07 only flags; a very old punch could post adjustments beyond the correction window.
- OQ-C04-14 How is a work-from-home day requested and approved (PC-09 "approved WFH days"), since LV-03 only switches WFH on and C09 covers only permissions and on-duty? Source: PC-09, LV-03, PM-R5. Why it matters: PC-09 default depends on an approval that no module defines.
- OQ-C04-15 When a fence is changed or switched on, are punches already flagged under the old fence re-evaluated, or does the change apply only to new punches? Source: PC-R14, G5, BR-03. Why it matters: affects pending approvals and recalculation.
- OQ-C04-16 When OD-07 = app check-in at the on-duty place with location, how is the place checked (no fence exists for it), and is any distance stored? Source: OD-07, PM-R7, PC-17, PC-R13. Why it matters: PC-R13 requires a "distance from the nearest allowed fence" that is meaningless for an on-duty place.
- OQ-C04-17 Which roles may see punch location and selfie photos, and for how long are they kept? Source: K5, N9, N10, AC-05. Why it matters: location and face images are sensitive personal data needing a privacy review.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| PC-R1 | US-C04-01, US-C04-02 | AC-C04-01.5, AC-C04-02.1 |
| PC-R2 | US-C04-01 | AC-C04-01.1, AC-C04-01.2 |
| PC-R3 | US-C04-03 | AC-C04-03.2 (OQ-C04-04) |
| PC-R4 | US-C04-07 | AC-C04-07.2, AC-C04-07.3 |
| PC-R5 | US-C04-07, US-C04-08 | AC-C04-07.1, AC-C04-08.1, AC-C04-08.2 |
| PC-R6 | US-C04-02, US-C04-06 | AC-C04-02.1, AC-C04-02.4, AC-C04-06.1 |
| PC-R7 | US-C04-08, US-C04-14 | AC-C04-08.4, AC-C04-14.1, AC-C04-14.2 (OQ-C04-13) |
| PC-R8 | US-C04-03 | AC-C04-03.1, AC-C04-03.3, AC-C04-03.5 |
| PC-R9 | US-C04-05 | AC-C04-05.1 to AC-C04-05.7 |
| PC-R10 | US-C04-04 | AC-C04-04.1 to AC-C04-04.5 |
| PC-R11 | US-C04-09 | AC-C04-09.1 |
| PC-R12 | US-C04-09, US-C04-11 | AC-C04-09.11, AC-C04-11.1 to AC-C04-11.5 |
| PC-R13 | US-C04-07, US-C04-09, US-C04-18 | AC-C04-07.1, AC-C04-09.1, AC-C04-18.1 |
| PC-R14 | US-C04-10 | AC-C04-10.1 to AC-C04-10.4 |
| PC-R15 | US-C04-07, US-C04-09 | AC-C04-07.5, AC-C04-09.1 |
| PC-01 | US-C04-17, US-C04-07, US-C04-09 | AC-C04-17.1 to AC-C04-17.3, AC-C04-07.4, AC-C04-09.12 |
| PC-02 | US-C04-07, US-C04-11 | AC-C04-07.2, AC-C04-11.7 |
| PC-03 | US-C04-10 | AC-C04-10.1, AC-C04-10.2 |
| PC-04 | US-C04-09, US-C04-11 | AC-C04-09.2, AC-C04-09.3, AC-C04-11.4 |
| PC-05 | US-C04-09 | AC-C04-09.8 |
| PC-06 | US-C04-13 | AC-C04-13.1 to AC-C04-13.5 |
| PC-07 | US-C04-08, US-C04-14 | AC-C04-08.5, AC-C04-14.3 |
| PC-08 | US-C04-14 | AC-C04-14.4 (OQ-C04-05) |
| PC-09 | US-C04-16 | AC-C04-16.1 to AC-C04-16.4 |
| PC-10 | US-C04-04 | AC-C04-04.1, AC-C04-04.2 |
| PC-11 | US-C04-05 | AC-C04-05.3, AC-C04-05.4 |
| PC-12 | US-C04-09, US-C04-11 | AC-C04-09.1, AC-C04-11.7 |
| PC-13 | US-C04-09 | AC-C04-09.4, AC-C04-09.5 |
| PC-14 | US-C04-09 | AC-C04-09.6, AC-C04-09.7 |
| PC-15 | US-C04-09 | AC-C04-09.9, AC-C04-09.10 |
| PC-16 | US-C04-10, US-C04-11 | AC-C04-11.2, AC-C04-11.3 |
| PC-17 | US-C04-11, US-C04-16 | AC-C04-11.6, AC-C04-16.1 |
| BR-01 | US-C04-06, US-C04-02, US-C04-05, US-C04-15 | AC-C04-06.1, AC-C04-06.2, AC-C04-02.3, AC-C04-05.6, AC-C04-15.2 |
| BR-11 | US-C04-02, US-C04-06, US-C04-08 | AC-C04-02.1, AC-C04-06.4, AC-C04-08.2 |
| N4 | US-C04-08 | AC-C04-08.1 to AC-C04-08.3 |
| TB-04 | US-C04-01 | AC-C04-01.5 |
| E17 | US-C04-12 | AC-C04-12.1, AC-C04-12.2 |
| E18 | US-C04-09, US-C04-15 | AC-C04-09.2, AC-C04-09.3, AC-C04-09.4, AC-C04-15.1 |
| AT16 | US-C04-12 | AC-C04-12.1, AC-C04-12.6 |
| AT19 | US-C04-09 | AC-C04-09.11 |
| EM-07 | US-C04-11, US-C04-12 | AC-C04-11.5, AC-C04-12.2, AC-C04-12.3 |
| EM-08 | US-C04-12 | AC-C04-12.1, AC-C04-12.4 |
| AT20 / TB-01 | US-C04-01, US-C04-06 | AC-C04-01.3, AC-C04-06.6 |
