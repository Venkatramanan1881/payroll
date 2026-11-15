# C03 User management, roles and access — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §9.3 (UM-R1..UM-R4, AC-01..AC-08), §5, §7 (G9), §12 (BR-07), §15 (N1, N5, N7), §16, §19 (AT13, AT20), §22.2 (TB-01, TB-02); TSD §4, §8 (context only)

## Scope
- In scope (V1): roles from BRD §5 and their configurable rights; custom roles; users with role and access scope; employee and admin sign-in; second sign-in step; language choice; approver routing per request type and the "nobody approves their own request" rule; delegation; who may change settings; what employees can see about themselves; Auditor read-only access; separation of organisations.
- Out of scope / later release:
  - Vendor users and vendor console roles (VC-05): V01.
  - Vendor support access into an organisation (VC-R6): V01; referenced here only as the single exception to separation.
  - Approval steps inside each request type (leave, permission, regularisation, on-duty): their own modules (C06, C08, C09). Overtime approval is V3; advance approval is V2.
  - Period unlock approval by the Organisation Admin (PL-06): C12.
  - Dashboard per role (DB-R1, AT17): C16.
  - The choice of sign-in code provider (Q16) and the technical token design (TSD §8): technical, not stories.

## Stories

### US-C03-01 Configure what each role may view, edit and approve
As an Organisation Admin, I want each role's view, edit and approve rights to be configurable for every screen, report and action, so that people can do only what their job needs.
Source: UM-R1, AC-01, §5, N5, AT13
Settings involved: AC-01

Acceptance criteria
- AC-C03-01.1 Given a new organisation, When the Organisation Admin opens roles, Then the roles Organisation Admin, HR Manager, Payroll Officer, Manager (Approver), Supervisor, Employee and Auditor exist. (Source: §5)
- AC-C03-01.2 Given the Organisation Admin removes the "view" right for report R3 from the Supervisor role, When a Supervisor opens the reports menu, Then R3 is not listed, and a direct request for R3 is refused. (Source: UM-R1, AC-01, N5)
- AC-C03-01.3 Given a role has view but not edit on a screen, When a user with that role tries to save a change on it, Then the save is refused. (Source: UM-R1, AC-01)
- AC-C03-01.4 Given a role has no approve right for a request type, When a user with that role opens that request, Then they cannot approve or reject it. (Source: AC-01)
- AC-C03-01.5 Given a module is off for the organisation, When the Organisation Admin edits role rights, Then that module's screens and actions are not listed. (Source: MD-03, AT13)
- AC-C03-01.6 Given role rights are changed, When the settings history is checked, Then it shows who, when, old value, new value and reason. (Source: G9, §16)
- AC-C03-01.7 Given an HR Manager, Payroll Officer, Manager, Supervisor, Employee or Auditor without that right, When they try to change role rights, Then it is refused. (Source: §5 "Organisation Admin manages users, roles")

Edge and negative cases
- Whether predefined roles can be deleted or given rights beyond their §5 description (for example edit rights for the Auditor): OQ-C03-01.

### US-C03-02 Create custom roles
As an Organisation Admin, I want to create further roles when custom roles are on, so that the organisation can match its own structure.
Source: AC-06, AC-01, §5
Settings involved: AC-06 (default on)

Acceptance criteria
- AC-C03-02.1 Given AC-06 = on, When the Organisation Admin creates a role "Store Manager" and gives it rights, Then the role can be assigned to users and its rights apply as for any role. (Source: AC-06, AC-01)
- AC-C03-02.2 Given AC-06 = off, When the Organisation Admin tries to create a role, Then the action is not available. (Source: AC-06)
- AC-C03-02.3 Given organisation A creates a custom role, When organisation B lists its roles, Then A's role is not listed. (Source: TB-01, AT20)

Edge and negative cases
- What happens to existing custom roles and their users when AC-06 is switched off: OQ-C03-02.

### US-C03-03 Create users with a role and an access scope
As an Organisation Admin, I want to create users, give each a role and limit them to all branches, chosen branches or chosen departments, so that each user sees only the people and data they are responsible for.
Source: UM-R2, AC-07, AT13, §5, DB-R8, §14
Settings involved: AC-07 (all branches / chosen branches / chosen departments)

Acceptance criteria
- AC-C03-03.1 Given an HR Manager user with scope "chosen branches: A", When they open employee lists, attendance, reports and exports, Then only data for branch A is shown. (Source: UM-R2, AC-07, AT13, §14)
- AC-C03-03.2 Given the same user, When they request an employee of branch B directly (by link or code), Then it is refused. (Source: AC-07, N5, AT13)
- AC-C03-03.3 Given a user with scope "chosen departments: Bakery", When they run report R2, Then only employees of the Bakery department appear. (Source: AC-07, AT13, §14)
- AC-C03-03.4 Given a user with scope "all branches", When they open lists, Then every branch's data is shown, within their role's rights. (Source: AC-07)
- AC-C03-03.5 Given a module is off, When any user opens menus, reports or dashboards, Then nothing from that module is shown. (Source: AT13, DB-R8, §14)
- AC-C03-03.6 Given a user's role or scope is changed, When the audit log is checked, Then it shows who, when, old and new value. (Source: §16, G9)
- AC-C03-03.7 Given a user's access is removed, When they try to sign in or use an existing session, Then access is refused. (Source: UM-R1, N5)

Edge and negative cases
- Combined branch and department scope (for example branch A, department Bakery only): OQ-C03-04.
- Whether one person can hold more than one role, and how an employee who is also a Manager or HR Manager gets self-service: OQ-C03-03.
- Whether every employee automatically has an Employee user, and what happens to users on exit: OQ-C03-12.
- Whether the HR Manager can also manage users: OQ-C03-12.

### US-C03-04 Employee signs in with mobile number and one-time code
As an Employee, I want to sign in with my mobile number and a one-time code, so that I can punch and see my records without remembering a password.
Source: UM-R3, MD-02, §3, Q16
Settings involved: none

Acceptance criteria
- AC-C03-04.1 Given an employee whose mobile number is on their employee record, When they enter it and then the one-time code sent to it, Then they are signed in to their own organisation as an Employee. (Source: UM-R3)
- AC-C03-04.2 Given a wrong one-time code, When the employee submits it, Then sign-in is refused. (Source: UM-R3)
- AC-C03-04.3 Given a mobile number that is not on any employee record of an organisation, When it is entered, Then no code grants access to any organisation's data. (Source: UM-R3, TB-01)
- AC-C03-04.4 Given an employee who is signed in, When they open records, Then they see only their own records, as limited by AC-05. (Source: §5 Employee, AC-05)

Edge and negative cases
- Code validity, number of attempts, and lock-out: OQ-C03-05.
- The same mobile number on employee records in two organisations: OQ-C03-05.
- Sign-in on web when the plan excludes the mobile app: OQ-C03-05.

### US-C03-05 Admin user signs in with password and second step
As an Organisation Admin, I want admin users to sign in with a password and, when switched on, a second step, so that accounts with wide access are harder to misuse.
Source: UM-R3, AC-08
Settings involved: AC-08 (default on)

Acceptance criteria
- AC-C03-05.1 Given AC-08 = on, When an admin user enters the correct password, Then they are not signed in until the second step is completed. (Source: UM-R3, AC-08)
- AC-C03-05.2 Given AC-08 = on, When the second step fails, Then sign-in is refused. (Source: AC-08)
- AC-C03-05.3 Given AC-08 = off, When an admin user enters the correct password, Then they are signed in. (Source: AC-08)
- AC-C03-05.4 Given a wrong password, When an admin user submits it, Then sign-in is refused. (Source: UM-R3)
- AC-C03-05.5 Given AC-08 is changed, When the settings history is checked, Then it shows who, when, old value, new value and reason. (Source: G9)

Edge and negative cases
- Which roles count as "admin roles" (Manager and Supervisor in particular), and whether they sign in by password or by mobile code: OQ-C03-06.
- Password reset, password rules and lock-out after failed attempts: not in the BRD; OQ-C03-06.

### US-C03-06 Choose my language
As an Employee, I want to choose my language from those my organisation offers, so that I can use the app, payslips and notifications in a language I read.
Source: UM-R4, OR-03, N7, AT14
Settings involved: OR-03 (default English on)

Acceptance criteria
- AC-C03-06.1 Given OR-03 has English and Tamil on, When a user chooses Tamil, Then their screens and notifications are shown in Tamil. (Source: UM-R4, N7, AT14)
- AC-C03-06.2 Given OR-03 has Hindi off, When a user opens the language choice, Then Hindi is not offered. (Source: OR-03)
- AC-C03-06.3 Given user X chose Tamil and user Y chose English in the same organisation, When each signs in, Then each sees their own choice. (Source: UM-R4)

Edge and negative cases
- A language turned off in OR-03 after users chose it: OQ-C03-07.

### US-C03-07 Route each request type to its approver
As an HR Manager, I want to set the approver for each request type, so that every request reaches the right person and nobody approves their own request.
Source: AC-02, BR-07, §5 (Manager (Approver))
Settings involved: AC-02 (reporting manager / named person / department head / HR)

Acceptance criteria
- AC-C03-07.1 Given AC-02 for leave = reporting manager, When employee X, whose reporting manager is M, submits leave, Then the request goes to M for approval. (Source: AC-02, EM-R2)
- AC-C03-07.2 Given AC-02 for regularisation = named person P, When any employee submits a regularisation, Then it goes to P. (Source: AC-02)
- AC-C03-07.3 Given AC-02 for permission = HR, When an employee submits a permission, Then it goes to the HR Manager. (Source: AC-02)
- AC-C03-07.4 Given any routing, When a user tries to approve a request they raised themselves, Then it is refused, whatever their role or rights. (Source: BR-07)
- AC-C03-07.5 Given AC-02 for leave = named person P and P raises a leave request, When the request is routed, Then P is not able to approve it. (Source: BR-07)
- AC-C03-07.6 Given a user who is not the routed approver for a request and has no approve right for that request type, When they try to approve it, Then it is refused. (Source: AC-01, AC-02)
- AC-C03-07.7 Given an approval or rejection, When the audit log is checked, Then it shows the approver, time and decision. (Source: §16)

Edge and negative cases
- Where a request goes when the routed approver is the requester (for example a manager's own leave, or an HR Manager's own regularisation when AC-02 = HR): OQ-C03-08.
- How "department head" is defined: OQ-C03-09.
- Employee with no reporting manager when AC-02 = reporting manager: OQ-C03-08.

### US-C03-08 Delegate approvals while away
As a Manager (Approver), I want to delegate my approvals while I am away, when delegation is on, so that my team's requests are not held up.
Source: AC-03, BR-07, AC-02
Settings involved: AC-03 (on / off; default not stated)

Acceptance criteria
- AC-C03-08.1 Given AC-03 = on and manager M delegates to D from 1 to 10 May, When a request routed to M arrives on 5 May, Then D can approve or reject it. (Source: AC-03)
- AC-C03-08.2 Given the same delegation, When a request routed to M arrives on 11 May, Then D cannot act on it unless it was routed to D otherwise. (Source: AC-03)
- AC-C03-08.3 Given AC-03 = off, When M tries to set a delegate, Then the action is not available. (Source: AC-03)
- AC-C03-08.4 Given D is acting for M and D raised one of the requests routed to M, When D tries to approve it, Then it is refused. (Source: BR-07)
- AC-C03-08.5 Given D approves under delegation, When the audit log is checked, Then it shows D as the approver acting for M. (Source: §16)

Edge and negative cases
- Who may set a delegation (the approver, HR, or both), whether the delegate needs the same access scope, and the AC-03 default: OQ-C03-10.
- Requests still pending with D when the delegation ends: OQ-C03-10.

### US-C03-09 Control who can change settings
As an Organisation Admin, I want to decide whether settings changes are made by the HR Manager alone or need my approval, so that pay-affecting settings are controlled as the organisation requires.
Source: AC-04, §5 (HR Manager owns settings), G9, BR-07
Settings involved: AC-04 (default HR Manager only)

Acceptance criteria
- AC-C03-09.1 Given AC-04 = HR Manager only, When an HR Manager saves a settings change, Then it takes effect from its effective date without further approval. (Source: AC-04)
- AC-C03-09.2 Given AC-04 = HR Manager only, When a Payroll Officer, Manager, Supervisor, Employee or Auditor tries to change a setting, Then it is refused. (Source: AC-04, §5)
- AC-C03-09.3 Given AC-04 = HR Manager with Organisation Admin approval, When an HR Manager saves a change, Then the change is pending and the old value stays in force until the Organisation Admin approves. (Source: AC-04)
- AC-C03-09.4 Given a pending settings change, When the Organisation Admin rejects it, Then the old value stays in force and the HR Manager sees the rejection. (Source: AC-04)
- AC-C03-09.5 Given any settings change is applied, When the settings history is checked, Then it shows who changed it, who approved it (if needed), when, old value, new value and reason. (Source: G9, AC-04)

Edge and negative cases
- One person holding both HR Manager and Organisation Admin roles under the approval option (BR-07): OQ-C03-11.
- Whether AC-04 covers every setting, including AC-04 itself and role rights: OQ-C03-11.

### US-C03-10 Choose what employees can see about themselves
As an HR Manager, I want to switch on or off what employees can see about their own attendance, so that the app shows what the organisation is ready to explain.
Source: AC-05, §5 (Employee)
Settings involved: AC-05 (own punches, day status, counters, permission balance, reason for any deduction; each on / off; defaults not stated)

Acceptance criteria
- AC-C03-10.1 Given AC-05 own punches = on, When an employee opens a day, Then their punches for that day are shown. (Source: AC-05)
- AC-C03-10.2 Given AC-05 own punches = off, When an employee opens a day in the app or on the web, Then their punches are not shown, and a direct request for them is refused. (Source: AC-05, N5)
- AC-C03-10.3 Given AC-05 reason for any deduction = on and a day has a deduction, When the employee opens the day, Then the reason is shown in plain language. (Source: AC-05, BR-08)
- AC-C03-10.4 Given each of day status, counters and permission balance switched off one at a time, When the employee opens their attendance, Then only the switched-off item is hidden. (Source: AC-05)
- AC-C03-10.5 Given an employee, When they try to see another employee's punches or status, Then it is refused whatever AC-05 says. (Source: §5 Employee "own", AT13)

Edge and negative cases
- AC-05 defaults: OQ-C03-13.

### US-C03-11 Auditor reviews results, settings history and audit trail
As an Auditor, I want read-only access to results, settings history and the audit trail, so that I can check how figures were reached without being able to change them.
Source: §5 (Auditor), §16, R26, R27
Settings involved: AC-01, AC-07

Acceptance criteria
- AC-C03-11.1 Given an Auditor, When they open results, settings history (R26) and the audit trail (R27), Then they can view and export them within their access scope. (Source: §5, R26, R27, §14)
- AC-C03-11.2 Given an Auditor, When they try to create, change, approve, lock or delete anything, Then it is refused. (Source: §5 "read-only")
- AC-C03-11.3 Given any create, change, approval, waiver, lock or settings change in the organisation, When the Auditor opens the audit trail, Then the entry shows the user and time. (Source: §16)

Edge and negative cases
- Whether the Auditor role can be given edit rights under AC-01: OQ-C03-01.

### US-C03-12 Keep each organisation's data separate
As an Organisation Admin, I want my organisation's data to be unreachable by any other organisation, so that our employees' personal, pay and location data stays private.
Source: TB-01, TB-02, AT20, N1, N5, N9, VC-R6; TSD §4 (context)
Settings involved: none

Acceptance criteria
- AC-C03-12.1 Given a user signed in to organisation A, When they try to read any record of organisation B through any screen, API, report, export or file link, Then nothing from B is returned. (Source: TB-01, AT20)
- AC-C03-12.2 Given a user signed in to organisation A, When they try to change, approve or delete any record of organisation B through any screen or API, Then the change is refused and B's record is unchanged. (Source: TB-01, AT20)
- AC-C03-12.3 Given a user of organisation A sends a request naming organisation B's identifier in the request content, When it is processed, Then it is handled as organisation A only. (Source: TB-01; TSD §4.1)
- AC-C03-12.4 Given organisation A's export or document file link, When a user of organisation B, or someone not signed in, opens it, Then access is refused; Given a valid link for a user of A, When its short life has passed, Then it no longer opens. (Source: TB-02, AT20)
- AC-C03-12.5 Given a report or export run by a user of A with scope "all branches", When it completes, Then it contains only A's data. (Source: TB-01, AT20, §14)
- AC-C03-12.6 Given the application layer fails to apply the organisation check for a request, When the data is read, Then the database itself returns no other organisation's records. (Source: TB-01)
- AC-C03-12.7 Given vendor support access to organisation A is approved and in time, When the vendor user acts, Then it is the only cross-organisation access allowed, it reaches A only, and each action is logged for A's Organisation Admin. (Source: TB-01, VC-R6)

Edge and negative cases
- Timed jobs and notifications that process many organisations: each must act for one organisation only (TB-03, TSD §4.2); covered by AT20 test scope above.

## Open questions
- OQ-C03-01 Can the predefined §5 roles be deleted or renamed, and can their rights be changed beyond the §5 description (for example giving the Auditor edit rights, or the Vendor Admin's "cannot change results" for organisation roles)? Source: AC-01, §5. Why it matters: defines limits on AC-01.
- OQ-C03-02 When AC-06 is switched off, what happens to custom roles already created and the users holding them? Source: AC-06. Why it matters: users could lose all access.
- OQ-C03-03 Can one person hold more than one role (for example Manager and Employee, or HR Manager and Organisation Admin), and how does a Manager or HR Manager use employee self-service for their own punches and requests? Source: §5, UM-R3. Why it matters: affects sign-in method, scope and BR-07 checks.
- OQ-C03-04 Can AC-07 combine branch and department scope (for example branch A and department Bakery only), and does a Manager's scope default to their reporting team? Source: AC-07, UM-R2. Why it matters: AC-07 lists three exclusive-looking options.
- OQ-C03-05 How long is a one-time code valid, how many attempts are allowed, what is the lock-out, what happens when the same mobile number is on employee records in two organisations, and how do employees sign in on the web when the plan excludes the app? Source: UM-R3, MD-02, PC-R11, Q16. Why it matters: testable sign-in limits and multi-employer workers.
- OQ-C03-06 Which roles are "admin users" who sign in with a password (UM-R3) and get the AC-08 second step: all non-Employee roles, or only Organisation Admin, HR Manager, Payroll Officer and Auditor? What password rules, reset and lock-out apply? Source: UM-R3, AC-08. Why it matters: Managers and Supervisors may have only a phone.
- OQ-C03-07 If a language is turned off in OR-03 after users chose it, what language do they then see? Source: OR-03, UM-R4. Why it matters: users must not be left with no language.
- OQ-C03-08 Where does a request go when the routed approver is the requester (BR-07), or when the employee has no reporting manager: to the next level, to HR, or to a named fallback? Source: BR-07, AC-02. Why it matters: such requests would otherwise be stuck.
- OQ-C03-09 How is a "department head" (AC-02) identified: a field on the department, a designation, or a role? Source: AC-02, OR-R2. Why it matters: routing cannot be tested without it.
- OQ-C03-10 Who may set a delegation (the approver, HR, or both), must the delegate have the same access scope, what is the AC-03 default, and what happens to requests pending with the delegate when delegation ends? Source: AC-03. Why it matters: delegation can widen access.
- OQ-C03-11 Under AC-04 "with Organisation Admin approval", who approves when the same person holds both roles (BR-07), and does AC-04 also cover changes to AC-04 itself and to role rights (AC-01)? Source: AC-04, BR-07. Why it matters: one person could otherwise approve their own settings change.
- OQ-C03-12 Is an Employee user created automatically for every employee with a mobile number; what happens to a user's access when the employee exits; and may the HR Manager create and manage users, or only the Organisation Admin? Source: §5, UM-R3, EM-R5. Why it matters: access after exit and user administration rights.
- OQ-C03-13 What are the defaults for each AC-05 item (own punches, day status, counters, permission balance, reason for any deduction)? Source: AC-05, G6. Why it matters: G6 says every setting has a default but AC-05 states none.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| UM-R1 | US-C03-01 | AC-C03-01.2, 01.3, 03.7 |
| UM-R2 | US-C03-03 | AC-C03-03.1–03.4 |
| UM-R3 | US-C03-04, US-C03-05 | AC-C03-04.1–04.3, 05.1, 05.4 |
| UM-R4 | US-C03-06 | AC-C03-06.1–06.3 |
| AC-01 | US-C03-01 | AC-C03-01.1–01.7 |
| AC-02 | US-C03-07 | AC-C03-07.1–07.3, 07.6 |
| AC-03 | US-C03-08 | AC-C03-08.1–08.3, 08.5 |
| AC-04 | US-C03-09 | AC-C03-09.1–09.5 |
| AC-05 | US-C03-10 | AC-C03-10.1–10.5 |
| AC-06 | US-C03-02 | AC-C03-02.1–02.3 |
| AC-07 | US-C03-03 | AC-C03-03.1–03.4 |
| AC-08 | US-C03-05 | AC-C03-05.1–05.3, 05.5 |
| §5 roles | US-C03-01, US-C03-11 | AC-C03-01.1, 11.1, 11.2 |
| BR-07 | US-C03-07, US-C03-08 | AC-C03-07.4, 07.5, 08.4 |
| AT13 | US-C03-03, US-C03-10 | AC-C03-03.1, 03.2, 03.3, 03.5, 10.5 |
| TB-01 / AT20 | US-C03-12, US-C03-02 | AC-C03-12.1–12.7, 02.3 |
| TB-02 | US-C03-12 | AC-C03-12.4 |
| OR-03 / N7 / AT14 | US-C03-06 | AC-C03-06.1, 06.2 |
| G9 / §16 | all | AC-C03-01.6, 03.6, 05.5, 07.7, 08.5, 09.5, 11.3 |
