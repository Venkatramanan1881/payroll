# V01 Vendor console and MD Module control — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.2 (MD-01..MD-05), §4.3 (VC-R1..VC-R9, VC-01..VC-05), §5, §7 (G1, G2, G5, G6, G9), §12 (BR-14), §17 (A1), §19 (AT2, AT3, AT12, AT20), §21 note 5, §22.2 (TB-01, TB-03); TSD §4.3, §4.4 (context only)

## Scope
- In scope (V1): creating organisations from sector templates; plans, modules and employee limits with dated history; module on/off and dependency checks; rule-group switches inside a module; usage view; suspension and reactivation; subscription details and renewal reminders; time-limited support access; sector template maintenance; health view; announcements; vendor users and their roles.
- Out of scope / later release:
  - MD-06 payroll input export when Payroll is off (covered by another module file).
  - DB-R2 vendor dashboard (C16).
  - Library of client-specific payroll rules (VC-R7 second half, §10.3.7 CR-R1..CR-R7): M03 Payroll is V2 (§4.1, §21). Only the sector-template part of VC-R7 is written here.
  - "Last pay run" in VC-R3 and "failed pay runs" in VC-R8 depend on M03 Payroll (V2). In V1 these show no data.
  - Dependency pairs in MD-04 all name modules from V2 or later (Payroll, Reimbursements, Statutory, Bonus, piece rate, project-based pay). The check is specified here; it becomes testable when those modules exist.
  - Invoicing (VC-R5: outside the app).

## Stories

### US-V01-01 Create an organisation from a sector template
As a Vendor Admin, I want to create an organisation from a sector template with its plan, modules, employee limit and first admin user, so that a new client is live on the common modules without any code change.
Source: VC-R1, MD-01, MD-02, G6, A1, AT12, §5 (Vendor Admin, Organisation Admin)
Settings involved: VC-01 (default allow and alert), VC-02 (default 15), VC-04 (default manual only)

Acceptance criteria
- AC-V01-01.1 Given the sector templates Factory, Supermarket, Restaurant and bakery, and Office and services exist, When the Vendor Admin creates an organisation and picks one of them, Then the organisation starts with that template's settings and lists, and every other setting holds its BRD default. (Source: VC-R1, G6)
- AC-V01-01.2 Given a new organisation is being created, When the Vendor Admin saves it, Then a plan, the enabled modules, the employee limit and the first admin user are all recorded, and the save is refused if any of them is missing. (Source: VC-R1, MD-02)
- AC-V01-01.3 Given the organisation has been created, When the first admin user signs in, Then that user holds the Organisation Admin role for that organisation only. (Source: VC-R1, §5)
- AC-V01-01.4 Given a new organisation has been created from a template, When it is checked, Then all common modules C01–C09, C12–C16 that are in V1 are available and no code change or deployment was needed. (Source: AT12, §4.1)
- AC-V01-01.5 Given any user who is not a Vendor Admin (including an Organisation Admin), When they try to create an organisation, Then the action is not available and is refused if attempted directly. (Source: A1, MD-01, N5)
- AC-V01-01.6 Given organisation A already exists, When organisation B is created from the same template, Then B holds none of A's data, users or settings changes. (Source: TB-01, AT20)
- AC-V01-01.7 Given an organisation is created, When the audit log is checked, Then it shows the creation with the vendor user, time, template, plan, modules and limit. (Source: §16, G9)

Edge and negative cases
- Organisation created with an employee limit of zero or no limit: OQ-V01-01.
- Template contents beyond the four named sectors: OQ-V01-10.

### US-V01-02 Maintain plans
As a Vendor Admin, I want each plan to define its modules, employee limit and whether the Android and iOS app is included, so that organisations get what their subscription covers.
Source: MD-02, VC-R1, VC-R2, §3 (Platforms), Q5
Settings involved: none

Acceptance criteria
- AC-V01-02.1 Given a plan, When the Vendor Admin views it, Then it shows its modules, its employee limit and whether the mobile app is included. (Source: MD-02)
- AC-V01-02.2 Given an organisation's plan does not include the mobile app, When an employee of that organisation tries to use the Android or iOS app, Then the app cannot be used for that organisation. (Source: MD-02, §3)
- AC-V01-02.3 Given an organisation's plan includes the mobile app, When an employee signs in on the app, Then the app is available subject to their role and the organisation's settings. (Source: MD-02, §3)
- AC-V01-02.4 Given a user who is not a Vendor Admin, When they try to create or change a plan, Then it is refused. (Source: MD-01, §5)

Edge and negative cases
- Which modules and limits belong in each plan is open in the BRD (Q5): OQ-V01-02.
- Whether an organisation's modules or limit may differ from its plan: OQ-V01-02.

### US-V01-03 Switch modules on and off for an organisation
As a Vendor Admin, I want to switch the optional modules on or off for each organisation, so that the client gets only the modules in its plan.
Source: MD-01, MD-03, MD-04, G1, BR-14, AT2, §4.1
Settings involved: none (module control is not an organisation setting)

Acceptance criteria
- AC-V01-03.1 Given an organisation, When the Vendor Admin switches an optional module on, Then its menus, screens, reports and dashboard widgets appear for users whose role allows them. (Source: MD-01, AT2, DB-R8, §14)
- AC-V01-03.2 Given an optional module is on and holds data (for example M02 comp-off earned balances), When the Vendor Admin switches it off, Then it is hidden from every menu, report and dashboard, its rules are not evaluated, it has no effect on any day status or total, and its data is kept. (Source: MD-03, BR-14, AT2)
- AC-V01-03.3 Given a module was switched off with data kept, When it is switched back on, Then its earlier data is available again unchanged. (Source: AT2, MD-03)
- AC-V01-03.4 Given a module is off, When any user calls one of its actions directly (not through a menu), Then the call is refused. (Source: MD-03, N5; TSD §4.4)
- AC-V01-03.5 Given a common module (C01–C16), When the Vendor Admin views module control, Then it cannot be switched off. (Source: §4.1 "every client, always on")
- AC-V01-03.6 Given a module that is not yet released (V2 or later), When the Vendor Admin views module control in V1, Then it cannot be switched on. (Source: §21)
- AC-V01-03.7 Given a module that MD-04 says needs Payroll, When the Vendor Admin tries to switch it on while Payroll is off, Then the switch is refused with the reason naming Payroll. (Source: MD-04; testable from V2)
- AC-V01-03.8 Given an Organisation Admin, HR Manager or any other organisation user, When they look for module control, Then it is not available to them. (Source: MD-01, G1)
- AC-V01-03.9 Given a module is switched on or off, When the history is checked, Then it shows who, when, old state, new state and reason. (Source: VC-R2, G9)

Edge and negative cases
- Switching off Payroll while a dependent module is on: block or switch both off: OQ-V01-03.
- Effective date of a module switch, and whether switching off mid-period recalculates open days already processed: OQ-V01-04.

### US-V01-04 Change plan, modules and limits with a dated history
As a Vendor Admin, I want to change an organisation's plan, modules and limits at any time and keep a dated history, so that upgrades and downgrades are traceable.
Source: VC-R2, G9, §16
Settings involved: VC-01 (default allow and alert)

Acceptance criteria
- AC-V01-04.1 Given an active organisation, When the Vendor Admin changes its plan, modules or employee limit, Then the change is saved and a history entry records the date, vendor user, old value, new value and reason. (Source: VC-R2, G9)
- AC-V01-04.2 Given several changes over time, When the Vendor Admin opens the history, Then every change is listed in date order and no entry can be edited or deleted. (Source: VC-R2, §16)

Edge and negative cases
- Limit lowered below the current number of active employees: OQ-V01-01.
- Future-dated plan changes: OQ-V01-04.
- Whether the organisation's Organisation Admin or Auditor can see this history: OQ-V01-05.

### US-V01-05 Apply the employee limit
As a Vendor Admin, I want the system to act when an organisation reaches its employee limit, so that usage stays within the plan or I am told when it does not.
Source: VC-01, VC-R3, MD-02, EM-R5, EM-R7
Settings involved: VC-01 (default allow and alert)

Acceptance criteria
- AC-V01-05.1 Given limit 100, 99 active employees and VC-01 = allow and alert, When HR adds one employee, Then the employee is added and no alert is sent. (Source: VC-01)
- AC-V01-05.2 Given limit 100, 100 active employees and VC-01 = allow and alert, When HR adds one employee, Then the employee is added and the vendor is alerted that the organisation is over its limit. (Source: VC-01)
- AC-V01-05.3 Given limit 100, 100 active employees and VC-01 = block, When HR adds one employee on screen, Then the employee is not created and HR is told the employee limit has been reached. (Source: VC-01)
- AC-V01-05.4 Given limit 100, 98 active employees, VC-01 = block, When HR uploads a spreadsheet of 5 new employees, Then no more than 100 active employees exist afterwards and the rows not created appear in the error report with the reason. (Source: VC-01, EM-R7; see OQ-C02-04 for whole-file versus row-by-row handling)
- AC-V01-05.5 Given VC-01 is changed, When the history is checked, Then the change is recorded with who, when, old and new value. (Source: VC-R2, G9)

Edge and negative cases
- Which employee statuses count toward the limit: OQ-V01-06.
- Limit lowered below headcount: OQ-V01-01.
- Rejoining employees: they count as new or not: OQ-V01-06.

### US-V01-06 See usage per organisation
As a Vendor Admin, I want to see each organisation's usage, so that I can compare it with its plan and spot problems.
Source: VC-R3, TB-01; TSD §4.3
Settings involved: none

Acceptance criteria
- AC-V01-06.1 Given an organisation, When the Vendor Admin opens its usage, Then it shows active employees against the limit, number of branches, devices online, last pay run and last sign-in. (Source: VC-R3)
- AC-V01-06.2 Given Payroll (V2) is not available, When usage is shown in V1, Then "last pay run" shows as not applicable rather than a value. (Source: VC-R3, §21)
- AC-V01-06.3 Given the usage view, When the Vendor Admin uses it without support access, Then it shows organisation-level figures only and no employee records, punches or pay data. (Source: TB-01, VC-R6; TSD §4.3)
- AC-V01-06.4 Given a user of an organisation, When they try to open the vendor usage view, Then it is refused. (Source: TB-01, N5)

Edge and negative cases
- "Last sign-in" means any user, the Organisation Admin, or per role: OQ-V01-07.

### US-V01-07 Suspend and reactivate an organisation
As a Vendor Admin, I want to suspend and reactivate an organisation, so that I can restrict an account (for example for non-payment) without losing punches.
Source: VC-R4, VC-04, BR-01
Settings involved: VC-04 (default manual only)

Acceptance criteria
- AC-V01-07.1 Given an active organisation, When the Vendor Admin suspends it, Then its status shows suspended with the date and the vendor user, and the action is logged. (Source: VC-R4, §16)
- AC-V01-07.2 Given a suspended organisation, When an eSSL device pushes a punch, Then the punch is stored unchanged. (Source: VC-R4, PC-R6)
- AC-V01-07.3 Given a suspended organisation, When any of its users opens a screen or report they are allowed to see, Then they can view it and export it. (Source: VC-R4)
- AC-V01-07.4 Given a suspended organisation, When any of its users tries to create, change, approve, lock or delete anything (for example add an employee, approve leave, change a setting), Then the action is refused with a message that the account is suspended. (Source: VC-R4)
- AC-V01-07.5 Given a suspended organisation, When the Vendor Admin reactivates it, Then users can act again as their roles allow, and every punch received during suspension is present. (Source: VC-R4)
- AC-V01-07.6 Given VC-04 = manual only, When the renewal date passes, Then the organisation is not suspended automatically. (Source: VC-04)
- AC-V01-07.7 Given VC-04 = automatic N days after the renewal date with N = 10, renewal date 1 March and the subscription not renewed, When 10 days have passed after the renewal date, Then the organisation is suspended automatically and the suspension is logged as automatic. (Source: VC-04; the exact day boundary and what counts as "not renewed" are OQ-V01-09)
- AC-V01-07.8 Given an Organisation Admin, When they try to suspend or reactivate their own organisation, Then it is refused. (Source: VC-R4, §5)

Edge and negative cases
- App and web punches during suspension: OQ-V01-08.
- Whether timed jobs (daily processing, accrual, reminders) and recalculation of stored punches run during suspension: OQ-V01-08.
- Automatic suspension condition and day count: OQ-V01-09.

### US-V01-08 Record subscription details and send renewal reminders
As a Vendor Admin, I want to record each organisation's plan, start date, renewal date and billing status and be reminded before renewal, so that renewals are not missed.
Source: VC-R5, VC-02
Settings involved: VC-02 (default 15 days)

Acceptance criteria
- AC-V01-08.1 Given an organisation, When the Vendor Admin records plan, start date, renewal date and billing status, Then they are saved and shown on the organisation's record. (Source: VC-R5)
- AC-V01-08.2 Given the subscription details, When they are changed, Then the change is kept in the dated history. (Source: VC-R2, G9)
- AC-V01-08.3 Given renewal date 30 June and VC-02 = 15, When 15 June is reached, Then a renewal reminder is sent to the vendor and to the organisation, once. (Source: VC-02, TB-03, AT21)
- AC-V01-08.4 Given the subscription screen, When any user looks for invoice creation, Then the app does not produce invoices. (Source: VC-R5)

Edge and negative cases
- Who in the organisation receives the reminder, and the list of billing status values: OQ-V01-11.

### US-V01-09 Open an organisation's account for support
As a Vendor Admin, I want to open an organisation's account for a limited time to help them, with every action logged and visible to the Organisation Admin, so that support is possible without weakening data separation.
Source: VC-R6, VC-03, TB-01, §5 (Vendor Admin cannot change results); TSD §4.3
Settings involved: VC-03 (default on)

Acceptance criteria
- AC-V01-09.1 Given VC-03 = on, When a vendor user requests support access, Then no client data is visible until the Organisation Admin of that organisation approves that request. (Source: VC-R6, VC-03)
- AC-V01-09.2 Given VC-03 = on and the Organisation Admin rejects the request, When the vendor user tries to open the account, Then access is refused. (Source: VC-03)
- AC-V01-09.3 Given VC-03 = on and an earlier request was approved and has ended, When the vendor needs access again, Then a new approval is needed. (Source: VC-03 "each time")
- AC-V01-09.4 Given VC-03 = off, When a vendor user opens support access, Then access starts without approval but is still time-limited and logged. (Source: VC-R6, VC-03)
- AC-V01-09.5 Given support access was granted, When its time limit passes, Then the vendor user can no longer see or act in that organisation's account. (Source: VC-R6, TB-01)
- AC-V01-09.6 Given support access is in use, When the vendor user views or does anything, Then each action is logged with the vendor user, time and action, and the Organisation Admin can see that log. (Source: VC-R6)
- AC-V01-09.7 Given support access to organisation A, When the vendor user tries to see organisation B's data in the same session, Then it is refused. (Source: TB-01, AT20)
- AC-V01-09.8 Given an organisation's own users, When they look at results changed during a support session, Then the change appears in the audit trail as made by the vendor user, not by an organisation user. (Source: VC-R6, §16)

Edge and negative cases
- What a vendor user may do during support access, given §5 says the Vendor Admin cannot change an organisation's results: OQ-V01-12.
- Length of the time limit, and whether the Organisation Admin can end access early: OQ-V01-13.
- Which vendor roles (VC-05) may request support access: OQ-V01-14.

### US-V01-10 Maintain sector templates
As a Vendor Admin, I want to maintain the sector templates and see which organisation uses which, so that new clients start from sensible settings.
Source: VC-R7 (template part), G6, AT12
Settings involved: none

Acceptance criteria
- AC-V01-10.1 Given the vendor console, When the Vendor Admin opens templates, Then Factory, Supermarket, Restaurant and bakery, and Office and services are available. (Source: G6)
- AC-V01-10.2 Given a template, When the Vendor Admin changes it, Then the change is saved with who, when, old and new values. (Source: VC-R7, G9)
- AC-V01-10.3 Given organisations created from templates, When the Vendor Admin views a template, Then it lists the organisations created from it. (Source: VC-R7)
- AC-V01-10.4 Given an organisation user, When they try to change a sector template, Then it is refused. (Source: VC-R7, §5)

Edge and negative cases
- Whether a template change affects organisations already created from it: OQ-V01-10.
- Whether the vendor can add new sector templates beyond the four: OQ-V01-10.

### US-V01-11 Health view across organisations
As a Vendor Admin, I want one view of problems across organisations, so that I can act before clients are affected.
Source: VC-R8, TB-03
Settings involved: PC-10 (default 30 minutes; C04)

Acceptance criteria
- AC-V01-11.1 Given device sync failures in any organisation, When the Vendor Admin opens the health view, Then each organisation with a failure is listed with the device and branch. (Source: VC-R8)
- AC-V01-11.2 Given a timed job (daily processing, reminders, escalations, period-end penalties, leave accrual, comp-off expiry, device-offline alerts) failed for an organisation and was retried, When the health view is opened, Then the failure and its retry result are shown for that organisation. (Source: TB-03)
- AC-V01-11.3 Given an organisation has periods waiting to be locked, When the health view is opened, Then those periods are listed for that organisation. (Source: VC-R8)
- AC-V01-11.4 Given V1, When the health view is opened, Then "failed pay runs" shows no data because Payroll is V2. (Source: VC-R8, §21)
- AC-V01-11.5 Given the health view, When it is shown, Then it contains organisation-level facts only, not employee records. (Source: TB-01; TSD §4.3)

Edge and negative cases
- What counts as a "device sync failure" (offline beyond PC-10, rejected pushes, clock mismatch) and when a period is "waiting to be locked": OQ-V01-15.

### US-V01-12 Send announcements to chosen organisations
As a Vendor Admin, I want to send announcements to the organisations I choose, so that clients hear about releases, maintenance or renewals.
Source: VC-R9, TB-01
Settings involved: none

Acceptance criteria
- AC-V01-12.1 Given organisations A and B are chosen and C is not, When the Vendor Admin sends an announcement, Then users of A and B receive it and users of C do not. (Source: VC-R9)
- AC-V01-12.2 Given an announcement was sent, When the Vendor Admin views it, Then it shows the sender, time and the organisations chosen. (Source: VC-R9, §16)
- AC-V01-12.3 Given an organisation user, When they try to send an announcement to other organisations, Then the action is not available. (Source: VC-R9, TB-01)

Edge and negative cases
- Which users inside an organisation receive it, and through which channel: OQ-V01-16.

### US-V01-13 Manage vendor users and their roles
As a Vendor Admin, I want to create vendor users with the role full admin, support or read-only, so that each vendor staff member has only the access they need.
Source: VC-05, §5; TSD §4.3
Settings involved: VC-05 (full admin / support / read-only)

Acceptance criteria
- AC-V01-13.1 Given a vendor user with full admin, When they create another vendor user, Then they can assign full admin, support or read-only. (Source: VC-05)
- AC-V01-13.2 Given a vendor user with read-only, When they try any change in the vendor console (create organisation, change plan, suspend, send announcement), Then it is refused, and they can still view. (Source: VC-05)
- AC-V01-13.3 Given a vendor user account, When it is used to sign in to an organisation's admin panel or employee app, Then it is refused; vendor users reach client data only through support access. (Source: VC-R6, TB-01; TSD §4.3)
- AC-V01-13.4 Given an organisation user (any role), When they try to sign in to the vendor console, Then it is refused. (Source: TB-01, MD-01)
- AC-V01-13.5 Given a vendor user's role is changed, When the history is checked, Then it shows who, when, old and new role. (Source: G9, §16)

Edge and negative cases
- What the "support" role may and may not do: OQ-V01-14.
- Sign-in method for vendor users (UM-R3 covers organisation users only): OQ-V01-17.

### US-V01-14 Switch rule groups inside an enabled module
As an HR Manager, I want each rule group inside an enabled module to have its own on/off switch, so that my organisation uses only the rules it needs.
Source: MD-05, G1, G2, G5, G9, BR-14, AT3, AC-04
Settings involved: AC-04 (default HR Manager only)

Acceptance criteria
- AC-V01-14.1 Given a module is enabled, When the HR Manager switches one of its rule groups off with an effective date, Then from that date the rule is not evaluated and has no effect on any day status or total. (Source: MD-05, G2, AT3, BR-14)
- AC-V01-14.2 Given a rule group was switched off from 10 March, When days before 10 March are recalculated, Then the rule still applies to them. (Source: G5, BR-05, AT3)
- AC-V01-14.3 Given a module is off, When the HR Manager looks for its rule groups, Then they are not shown. (Source: MD-03)
- AC-V01-14.4 Given a rule group switch is changed, When the settings history is checked, Then it shows who, when, old value, new value and reason. (Source: G9)
- AC-V01-14.5 Given AC-04 = HR Manager only, When a Manager, Supervisor, Payroll Officer or Employee tries to switch a rule group, Then it is refused. (Source: AC-04)
- AC-V01-14.6 Given a Vendor Admin, When they try to switch an organisation's rule group, Then it is refused (the vendor switches modules; HR switches rules). (Source: G1, O2)

Edge and negative cases
- Effective date in a locked period: governed by BR-06 (difference posts to the next open period); no further rule assumed.

## Open questions
- OQ-V01-01 What happens to existing employees when the employee limit is lowered below the current active headcount (nothing until the next addition, a block on further additions only, or something else), and is a limit of zero or "no limit" allowed? Source: VC-01, VC-R2. Why it matters: downgrades must not silently deactivate employees or stop their punches.
- OQ-V01-02 Which modules and limits belong to each plan (BRD Q5), and may an organisation's modules or limit differ from its plan (VC-R1 lists plan, modules and limit separately)? Source: MD-02, VC-R1, Q5. Why it matters: decides whether module control is per plan or per organisation.
- OQ-V01-03 MD-04 says dependencies are "enforced when switching". When the prerequisite (Payroll) is switched off while a dependent module is on, is the switch blocked, or are dependants switched off too? Source: MD-04. Why it matters: different user flows and data effects.
- OQ-V01-04 Does a module change (on, off, plan change) take effect immediately or from a chosen date, and does switching a module off mid-period recalculate already-processed open days? Source: MD-03, VC-R2, AT2, BR-03. Why it matters: AT3 gives effective dates for rule groups but AT2 is silent for modules.
- OQ-V01-05 Can the Organisation Admin or Auditor of an organisation see the vendor's history of plan, module and limit changes for their organisation? Source: VC-R2, §5 (Auditor). Why it matters: transparency versus vendor-only data.
- OQ-V01-06 Which employee statuses count toward the employee limit (active, on probation, on notice; exited presumably not), and does a rejoining employee count as a new employee? Source: VC-01, VC-R3, EM-R5, EM-05. Why it matters: the limit check result depends on it.
- OQ-V01-07 "Last sign-in" in the usage view: of any user, of the Organisation Admin, or per role? Source: VC-R3. Why it matters: defines the usage metric.
- OQ-V01-08 While an organisation is suspended, are app and web punches accepted (VC-R4 says "punches are still stored" but users "can only view and export"), and do timed jobs and recalculation keep running? Source: VC-R4, PC-R5, PC-R11, TB-03. Why it matters: employees without devices may lose attendance, or results may be stale on reactivation.
- OQ-V01-09 For VC-04 automatic suspension, what counts as "not renewed" (billing status value?), and is the suspension on day N or day N+1 after the renewal date? Source: VC-04, VC-R5. Why it matters: automatic suspension must not fire on a paid client.
- OQ-V01-10 Does a change to a sector template affect organisations already created from it, and can the vendor add new sector templates beyond the four in G6? Source: VC-R7, G6. Why it matters: live clients' settings could change without HR action.
- OQ-V01-11 Who in the organisation receives the VC-02 renewal reminder (Organisation Admin?), through which channel, and what are the billing status values? Source: VC-02, VC-R5. Why it matters: reminder routing and the VC-04 trigger.
- OQ-V01-12 During support access, may the vendor user only view, or also change settings and data? §5 says the Vendor Admin "cannot change an organisation's results", while VC-R6 says "with every action logged". Source: VC-R6, §5. Why it matters: decides whether support access is read-only.
- OQ-V01-13 How long is the support access time limit, who sets it, and can the Organisation Admin end it early? Source: VC-R6, VC-03. Why it matters: testable expiry.
- OQ-V01-14 What can the VC-05 "support" vendor role do that "read-only" cannot, and which vendor roles may request support access? Source: VC-05, VC-R6. Why it matters: vendor-side permission matrix.
- OQ-V01-15 What counts as a "device sync failure" in the health view, and when is a period "waiting to be locked" (past period end, past cut-off PL-02)? Source: VC-R8, PC-10, PL-02. Why it matters: health view content is not testable without it.
- OQ-V01-16 Which users inside a chosen organisation receive a vendor announcement (all users, admins only, chosen roles), and through which channel (in-app, email)? Source: VC-R9, C14. Why it matters: reach and channel cost.
- OQ-V01-17 How do vendor users sign in (UM-R3 covers employees and admin users of organisations only)? Is a second step mandatory for them? Source: UM-R3, VC-05. Why it matters: vendor accounts reach every organisation through support access.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| VC-R1 | US-V01-01 | AC-V01-01.1, 01.2, 01.3 |
| VC-R2 | US-V01-04 | AC-V01-04.1, 04.2, 03.9, 08.2 |
| VC-R3 | US-V01-06 | AC-V01-06.1, 06.2 |
| VC-R4 | US-V01-07 | AC-V01-07.1–07.5, 07.8 |
| VC-R5 | US-V01-08 | AC-V01-08.1, 08.4 |
| VC-R6 | US-V01-09 | AC-V01-09.1–09.8 |
| VC-R7 | US-V01-10 | AC-V01-10.1–10.4 (template part only; rules library V2) |
| VC-R8 | US-V01-11 | AC-V01-11.1–11.5 |
| VC-R9 | US-V01-12 | AC-V01-12.1–12.3 |
| VC-01 | US-V01-05 | AC-V01-05.1–05.5 |
| VC-02 | US-V01-08 | AC-V01-08.3 |
| VC-03 | US-V01-09 | AC-V01-09.1–09.4 |
| VC-04 | US-V01-07 | AC-V01-07.6, 07.7 |
| VC-05 | US-V01-13 | AC-V01-13.1–13.5 |
| MD-01 | US-V01-03, US-V01-01 | AC-V01-03.1, 03.8, 01.5 |
| MD-02 | US-V01-02 | AC-V01-02.1–02.4 |
| MD-03 | US-V01-03 | AC-V01-03.2, 03.3, 03.4, 14.3 |
| MD-04 | US-V01-03 | AC-V01-03.7 (testable from V2) |
| MD-05 | US-V01-14 | AC-V01-14.1–14.6 |
| G1 | US-V01-03, US-V01-14 | AC-V01-03.8, 14.6 |
| G2 / AT3 | US-V01-14 | AC-V01-14.1, 14.2 |
| G6 / AT12 | US-V01-01, US-V01-10 | AC-V01-01.1, 01.4, 10.1 |
| G9 | all | AC-V01-01.7, 03.9, 04.1, 05.5, 10.2, 13.5, 14.4 |
| BR-14 / AT2 | US-V01-03 | AC-V01-03.2, 03.3 |
| TB-01 / AT20 | US-V01-01, 06, 09, 11, 12, 13 | AC-V01-01.6, 06.3, 06.4, 09.7, 11.5, 12.3, 13.3, 13.4 |
| TB-03 / AT21 | US-V01-08, US-V01-11 | AC-V01-08.3, 11.2 |
