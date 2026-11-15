# C16 Dashboards — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.1 (C16), §4.2 (MD-03), §4.3 (VC-R3, VC-R8), §5, §9.3, §9.16, §19 (AT2, AT13, AT17, AT20), §21; TSD §10 (J11 dashboard refresh every 15 minutes) for context only

## Scope
- In scope (V1): each role landing on its own dashboard; the V1 widgets of DB-R2 to DB-R7; access scope and hiding of disabled modules (DB-R8); widget choice per role (DB-01); default branch and period filters per user (DB-02).
- Out of scope / later release (widgets hidden in V1 because their module is not yet available):
  - Vendor: pay run failures (DB-R2) — V2 (M03).
  - Organisation Admin / HR Manager: documents expiring (DB-R3) — V3 (C11).
  - Payroll Officer: pay runs in progress, payments pending by bank and cash, advances and loans outstanding (DB-R4) — V2 (M03, C10).
  - Supervisor: daily cash payouts due (DB-R6) — V3 (daily allowance, DA-R5).
  - Employee: latest payslip (DB-R7) — V2 (M03).

## Stories

### US-C16-01 Each role lands on its own dashboard
As any signed-in user, I want to land on the dashboard for my role, so that I see what matters to me first.
Source: DB-R1, AT17, §5
Settings involved: DB-01

Acceptance criteria
- AC-C16-01.1 Given a user with role Vendor Admin, Organisation Admin, HR Manager, Payroll Officer, Manager, Supervisor or Employee, When they sign in, Then they land on the dashboard for that role.  (Source: DB-R1, AT17)
- AC-C16-01.2 Given an Employee signs in on the app or web, When they land, Then they see the Employee dashboard and no admin dashboard.  (Source: DB-R1, DB-R7, AT13)

Edge and negative cases
- Custom roles (AC-06) and users holding more than one role → OQ-C16-01.
- Auditor has no dashboard listed in DB-R2 to DB-R7 → OQ-C16-02.
- Payroll Officer in V1 organisations → OQ-C16-03.

### US-C16-02 Vendor dashboard
As a Vendor Admin, I want a dashboard across organisations, so that I can see plans, limits, renewals and failures at a glance.
Source: DB-R2, VC-R3, VC-R5, VC-R8, VC-02, §5 Vendor Admin
Settings involved: VC-02 (default 15 days)

Acceptance criteria
- AC-C16-02.1 Given the Vendor Admin signs in, When the dashboard loads, Then it shows organisations by plan and by status, active employees against each organisation's limit, renewals due, and device sync failures.  (Source: DB-R2, VC-R3, VC-R8)
- AC-C16-02.2 Given VC-02 = 15, When an organisation's renewal date is within 15 days, Then it appears under renewals due.  (Source: DB-R2, VC-02)
- AC-C16-02.3 Given V1, When the dashboard loads, Then no pay run failures widget is shown.  (Source: DB-R2, §4.1 M03, §21)
- AC-C16-02.4 Given an organisation user (any role), When they try to open the vendor dashboard, Then it is refused.  (Source: AT13, AT20, TB-01)
- AC-C16-02.5 Given the Vendor Admin views the dashboard, When they act from it, Then they cannot change an organisation's results.  (Source: §5 Vendor Admin)

### US-C16-03 Organisation Admin and HR Manager dashboard
As an HR Manager, I want today's attendance and pending work by branch, so that I can act on problems during the day.
Source: DB-R3, PC-R2, PC-10, EM-R5, AC-07
Settings involved: PC-10 (default 30 minutes), DB-02

Acceptance criteria
- AC-C16-03.1 Given the HR Manager or Organisation Admin signs in, When the dashboard loads, Then it shows today's present, absent, late and on-leave counts by branch; missed punches; pending approvals; devices offline; and joiners and leavers this month.  (Source: DB-R3)
- AC-C16-03.2 Given PC-10 = 30 minutes, When a device has been offline longer than 30 minutes, Then it appears under devices offline.  (Source: DB-R3, PC-R2, PC-10)
- AC-C16-03.3 Given an HR Manager limited to branches X and Y, When the dashboard loads, Then every widget counts only employees and devices in X and Y.  (Source: DB-R8, AC-07, AT13)
- AC-C16-03.4 Given V1, When the dashboard loads, Then no documents expiring widget is shown.  (Source: DB-R3, DB-R8, §4.1 C11)

Edge and negative cases
- When during the day an employee counts as "absent" and how fresh "today" figures are → OQ-C16-04.

### US-C16-04 Payroll Officer dashboard (lock work)
As a Payroll Officer, I want to see periods to lock and pre-lock issues, so that I lock each period on time.
Source: DB-R4, PL-R2, PL-04
Settings involved: PL-04 (default manual)

Acceptance criteria
- AC-C16-04.1 Given the Payroll Officer signs in, When the dashboard loads, Then it shows the periods waiting to be locked and the count of pre-lock issues (pending requests, flagged days, unresolved missed punches) for each.  (Source: DB-R4, PL-R2)
- AC-C16-04.2 Given V1, When the dashboard loads, Then pay runs in progress, payments pending by bank and cash, and advances and loans outstanding are not shown.  (Source: DB-R4, DB-R8, §21)
- AC-C16-04.3 Given the Payroll Officer's scope is branch X, When pre-lock issues are counted, Then only issues for employees in X are counted.  (Source: DB-R8, AT13)

Edge and negative cases
- Whether the Payroll Officer exists in V1 and who sees this dashboard otherwise → OQ-C16-03 (see OQ-C12-05).

### US-C16-05 Manager dashboard
As a Manager (Approver), I want my team's attendance, approvals, leave calendar and trends, so that I can manage my team day to day.
Source: DB-R5, AC-02
Settings involved: AC-02

Acceptance criteria
- AC-C16-05.1 Given a Manager signs in, When the dashboard loads, Then it shows team attendance today, pending approvals, the team leave calendar, and late and absence trends.  (Source: DB-R5)
- AC-C16-05.2 Given a Manager, When the dashboard loads, Then it includes only their own team and the approvals assigned to them, never another team's employees.  (Source: DB-R5, DB-R8, AC-02, AT13)

Edge and negative cases
- What period and granularity "late and absence trends" cover → OQ-C16-05.

### US-C16-06 Supervisor dashboard
As a Supervisor, I want to see who is in now and the roster for today and tomorrow, so that I can staff the floor.
Source: DB-R6, SH-R4
Settings involved: none

Acceptance criteria
- AC-C16-06.1 Given a Supervisor signs in, When the dashboard loads, Then it shows who is in now and the roster for today and tomorrow for their scope.  (Source: DB-R6, DB-R8)
- AC-C16-06.2 Given V1, When the dashboard loads, Then no daily cash payouts due widget is shown.  (Source: DB-R6, DA-R5, §21 Version 3)

Edge and negative cases
- How current "who is in now" must be → OQ-C16-04.

### US-C16-07 Employee dashboard
As an Employee, I want my punch status, attendance and balances on one screen, so that I know where I stand.
Source: DB-R7, AC-05, LM-R4, LM-R8, PM-R4
Settings involved: AC-05 (each item on/off)

Acceptance criteria
- AC-C16-07.1 Given an Employee signs in, When the dashboard loads, Then it shows today's punch status, this month's attendance, leave and permission balance, and pending requests — their own only.  (Source: DB-R7, AT13)
- AC-C16-07.2 Given AC-05 hides the permission balance (or own punches, day status, counters) from employees, When the dashboard loads, Then that item is not shown.  (Source: AC-05)
- AC-C16-07.3 Given M02 is on, When the dashboard loads, Then the comp-off balance appears alongside leave; Given M02 is off, Then it is not shown.  (Source: LM-R8, DB-R8)
- AC-C16-07.4 Given V1, When the dashboard loads, Then no latest payslip widget is shown.  (Source: DB-R7, §21 Version 2)

Edge and negative cases
- "This month" versus the attendance period → OQ-C16-05.

### US-C16-08 Scope, disabled modules, widget choice and default filters
As an Organisation Admin, I want to choose which widgets each role sees, and every user to have default branch and period filters, while dashboards always respect scope and enabled modules.
Source: DB-R8, DB-01, DB-02, MD-03, AT2, AT13, AT20, TB-01, G9
Settings involved: DB-01 (chosen by the Organisation Admin from the role's list), DB-02 (per user)

Acceptance criteria
- AC-C16-08.1 Given the Organisation Admin, When they set the widgets for a role, Then they can choose only from that role's widget list, and users of that role then see only the chosen widgets.  (Source: DB-01)
- AC-C16-08.2 Given a widget fed by a module that is disabled (or not yet released), When the Organisation Admin chooses widgets, Then that widget is not offered; When the module is enabled again, Then the widget can be chosen and shows the kept data.  (Source: DB-R8, MD-03, AT2)
- AC-C16-08.3 Given a user without permission to set widgets (anyone other than the Organisation Admin), When they try, Then it is refused.  (Source: DB-01, UM-R1)
- AC-C16-08.4 Given a user with default branch B and a default period, When they open their dashboard, Then it opens filtered to branch B and that period, and they can change the filter only within their access scope.  (Source: DB-02, DB-R8, AT13)
- AC-C16-08.5 Given a user of organisation A, When any widget loads or is drilled into, Then no data of another organisation is shown or reachable.  (Source: AT20, TB-01)
- AC-C16-08.6 Given widget choices change, When saved, Then who, when, old value and new value are recorded.  (Source: G9)

Edge and negative cases
- Default widget set for a role before the Organisation Admin chooses, and who chooses widgets for the vendor dashboard → OQ-C16-06.

## Open questions
- OQ-C16-01 Which dashboard does a user with a custom role (AC-06), or a user holding more than one role (for example Manager who is also an Employee), land on? Source: DB-R1, AC-06, AT17. Why it matters: AT17 test cases for real users.
- OQ-C16-02 The Auditor role (§5) has no dashboard in DB-R2 to DB-R7. Does the Auditor get a dashboard, and with what widgets? Source: §5, DB-R1, AT17. Why it matters: AT17 says each role lands on its own dashboard.
- OQ-C16-03 Does the Payroll Officer role exist in V1 organisations (Payroll is V2)? If not, do the V1 lock widgets of DB-R4 go to the HR Manager dashboard? Source: DB-R4, §5, PL-04 (see OQ-C12-05). Why it matters: lock work may have no dashboard.
- OQ-C16-04 How current must live widgets be ("who is in now", "today's present, absent, late")? TSD J11 refreshes every 15 minutes. And when during the day is an employee counted absent (before shift start, after the shift window, after finalisation)? Source: DB-R3, DB-R6; TSD §10 J11. Why it matters: Supervisors staff the floor from these numbers.
- OQ-C16-05 What do "late and absence trends" (DB-R5) cover — how many periods, by day, week or period? Is the Employee's "this month's attendance" (DB-R7) the calendar month or the attendance period (OQ-C12-01)? Source: DB-R5, DB-R7, PL-01. Why it matters: undefined widget content.
- OQ-C16-06 What widgets does each role see before the Organisation Admin makes a choice under DB-01, and who chooses the vendor dashboard widgets? Source: DB-01, G6, VC-05. Why it matters: G6 requires a default for every setting.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| DB-R1, AT17 | US-C16-01 | AC-C16-01.1, 01.2; OQ-C16-01, 02 |
| DB-R2 | US-C16-02 | AC-C16-02.1–02.5 |
| DB-R3 | US-C16-03 | AC-C16-03.1–03.4 |
| DB-R4 | US-C16-04 | AC-C16-04.1–04.3; OQ-C16-03 |
| DB-R5 | US-C16-05 | AC-C16-05.1, 05.2; OQ-C16-05 |
| DB-R6 | US-C16-06 | AC-C16-06.1, 06.2 |
| DB-R7 | US-C16-07 | AC-C16-07.1–07.4 |
| DB-R8 | US-C16-03, -04, -05, -06, -07, -08 | AC-C16-03.3, 03.4, 04.2, 04.3, 05.2, 06.1, 07.3, 08.2, 08.4 |
| DB-01 | US-C16-08 | AC-C16-08.1–08.3; OQ-C16-06 |
| DB-02 | US-C16-08 | AC-C16-08.4 |
| AC-05 | US-C16-07 | AC-C16-07.2 |
| MD-03, AT2 | US-C16-08 | AC-C16-08.2 |
| AT13 | US-C16-01, -03, -04, -05, -07, -08 | AC-C16-01.2, 03.3, 04.3, 05.2, 07.1, 08.4 |
| AT20, TB-01 | US-C16-02, US-C16-08 | AC-C16-02.4, AC-C16-08.5 |
| VC-02 | US-C16-02 | AC-C16-02.2 |
| G9 | US-C16-08 | AC-C16-08.6 |
