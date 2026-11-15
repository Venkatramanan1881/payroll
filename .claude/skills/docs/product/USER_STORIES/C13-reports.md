# C13 Reports — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.1 (C13), §4.2 (MD-03), §5, §9.3, §9.13, §14, §16, §19 (AT2, AT5, AT13, AT20), §21, §22.2 (TB-01, TB-02); TSD §7 (background exports) and §11 (export links) for context only

## Scope
- In scope (V1): the common report rules (filters, access scope, spreadsheet export, hidden when module disabled); reports R1–R10, R13, R25–R27 for the V1 modules.
- Out of scope / later release:
  - R11 Joiners and leavers with onboarding and offboarding status; R12 Document status and asset register — V3 (C11).
  - R14 Pay register, R15 Bank and cash list, R17 Deductions with reasons, R19 Extra shifts / night allowance / unearned weekly offs, R21 Contractor statements, R22 Reimbursements, R23 Adjustments and arrears, R24 Accounting journal, R20 Advance and loan ledger — V2 (M03, C10, M04, §21 Version 2).
  - R16 Daily allowance — V3; R18 Overtime computed, approved and paid — V3 (M01).
  - The "paid" part of R9 (comp-off double pay, HW-06) — V2.
  - The payroll group filter, and the payroll group assignment and client-specific rule parts of R26 — V2 (payroll groups and CR are V2).

## Stories

### US-C13-01 Common report behaviour
As an HR Manager, I want every report to filter, respect my access scope and export to spreadsheet, so that I see and share only the data I am allowed to.
Source: §14 (closing rule), UM-R1, UM-R2, AC-01, AC-07, MD-03, AT2, AT13, AT20, TB-01, TB-02, VC-R4
Settings involved: AC-01 (role permissions), AC-07 (access scope per user)

Acceptance criteria
- AC-C13-01.1 Given any V1 report, When a user opens it, Then it can be filtered by period, branch, department and employee.  (Source: §14)
- AC-C13-01.2 Given a user whose scope is branch X (or department D), When they run any report, Then only employees and data within that scope appear, both on screen and in the export.  (Source: §14, UM-R2, AC-07, AT13)
- AC-C13-01.3 Given a user whose role has no permission for a report, When they look for it, Then it is not offered and a direct request for it is refused.  (Source: UM-R1, AC-01, AT13)
- AC-C13-01.4 Given any report, When the user exports it, Then a spreadsheet with the same rows and filters as the screen is produced.  (Source: §14)
- AC-C13-01.5 Given a user of organisation A, When they run or export any report or open its file link, Then no data of another organisation is included or reachable.  (Source: AT20, TB-01, TB-02)
- AC-C13-01.6 Given a report belongs to a module that is disabled for the organisation (for example R9 with M02 off), When any user looks for it, Then it is hidden; When the module is enabled again, Then the report shows the kept data.  (Source: §14, MD-03, AT2)
- AC-C13-01.7 Given an export too large to produce at once, When the user requests it, Then it is produced in the background and delivered as a file link opened only after an access check.  (Source: TB-02; TSD §7 context)
- AC-C13-01.8 Given an organisation is suspended, When its users open reports, Then they can view and export them.  (Source: VC-R4)
- AC-C13-01.9 Given the same filters on a locked period, When a report is run twice, Then the results are identical.  (Source: BR-06, AT5)

Edge and negative cases
- Meaning of the "period" filter in V1 (attendance period, calendar month or any date range) → OQ-C13-01.
- Payroll group filter not available in V1 (groups are V2) — scope note above.
- Report columns for a rule group that is switched off (for example LM-21 off in R7) → OQ-C13-03.
- Default report permissions per standard role → OQ-C13-04.
- Report and export language → OQ-C13-05.

### US-C13-02 Attendance reports (R1–R5)
As an HR Manager, I want daily and monthly attendance, late and early, missed punch and exception reports, so that I can check results and act on problems.
Source: R1, R2, R3, R4, R5, BR-08, LA-05, LA-06, LA-10, EC-04, EC-05, EC-09, RG-01, PC-04, PC-07, PC-08, PC-R2
Settings involved: none owned by C13

Acceptance criteria
- AC-C13-02.1 Given a date, When R1 Daily attendance is run, Then each employee in scope shows day status, in time, out time, net hours and flags for that date.  (Source: R1)
- AC-C13-02.2 Given a period, When R2 Monthly attendance register is run, Then each employee in scope shows a day status for every date of the period.  (Source: R2)
- AC-C13-02.3 Given a period, When R3 is run, Then each employee shows late marks, late minutes, early marks, early minutes and any penalties applied.  (Source: R3, LA-05, EC-04, BR-10)
- AC-C13-02.4 Given E2 (in 09:25, out 18:00, default slabs), When R3 is run for that date, Then the employee shows one late mark and 25 late minutes.  (Source: E2, R3)
- AC-C13-02.5 Given E5 (LA-06 on, every 3rd late mark = half day, 4 late marks in the period), When R3 is run for the period, Then it shows 4 late marks and one half-day penalty.  (Source: E5, R3)
- AC-C13-02.6 Given a period, When R4 is run, Then it lists missed punches and regularisation requests with their status.  (Source: R4)
- AC-C13-02.7 Given a period, When R5 is run, Then it lists flagged punches (outside geo-fence, late sync, clock mismatch) and device status.  (Source: R5, PC-04, PC-07, PC-08, PC-R2)

Edge and negative cases
- A locked period with later adjustments: does R1/R2 show the locked value or the corrected value → OQ-C13-02.

### US-C13-03 Leave, absence, permission and on-duty reports (R6–R8)
As an HR Manager, I want leave balance, absence and permission/on-duty reports, so that I can see balances and the cuts applied.
Source: R6, R7, R8, LM-R4, LM-16, LM-21, LM-24, LM-27, PM-R4, PM-R5
Settings involved: none owned by C13

Acceptance criteria
- AC-C13-03.1 Given a period, When R6 is run, Then each employee shows balance and leave taken per leave type.  (Source: R6)
- AC-C13-03.2 Given a period, When R7 is run, Then it shows uninformed absence days, excess absence with informed and uninformed counts in separate columns, and sandwich days applied.  (Source: R7, LM-27)
- AC-C13-03.3 Given P7's inputs (5 approved paid leave days, 3 uninformed absent days, informed X = 3, uninformed X = 2, LM-21 off), When R7 is run, Then the informed count is 5 and the uninformed count is 3, shown separately, with 2 excess informed days and 1 excess uninformed day.  (Source: P7 day counts, R7, LM-27)
- AC-C13-03.4 Given a period, When R8 is run, Then it lists permissions and on-duty records per employee with type, date, time or duration and status.  (Source: R8, PM-R5)

Edge and negative cases
- P7's money amounts are V2 and are not shown in V1 (§21 note 1).

### US-C13-04 Comp-off report (R9)
As an HR Manager, I want a comp-off report, so that I can see comp-off earned, taken and expired.
Source: R9, CO-R1, CO-R2, HW-05, HW-06, MD-03
Settings involved: HW-05 (default 60 days)

Acceptance criteria
- AC-C13-04.1 Given M02 is on, When R9 is run for a period, Then each employee shows comp-off earned, taken and expired.  (Source: R9, CO-R1, CO-R2, HW-05)
- AC-C13-04.2 Given M02 is off, When any user looks for R9, Then it is hidden.  (Source: §14, MD-03)

Edge and negative cases
- "Paid" (HW-06 double pay) column — V2; not shown in V1.

### US-C13-05 Employee list, headcount and days worked by branch (R10)
As an HR Manager, I want an employee list with headcount and days worked by branch, so that I can see where people belong and where they actually worked.
Source: R10, EM-R10, EM-08, EM-R9, E17
Settings involved: EM-08 (default the branch worked)

Acceptance criteria
- AC-C13-05.1 Given a date or period, When R10 is run, Then it lists employees and headcount by branch, department, category and employee type.  (Source: R10)
- AC-C13-05.2 Given E17 (master branch A, in and out at allowed branch B) and EM-08 = branch worked, When days worked by branch are reported, Then that day is counted under branch B.  (Source: E17, EM-08, EM-R10)
- AC-C13-05.3 Given EM-08 = master branch for the same case, When days worked by branch are reported, Then that day is counted under branch A.  (Source: EM-08)

### US-C13-06 Payroll input summary (R13)
As a Payroll Officer (or HR Manager, see C12 OQ-C12-05), I want a payroll input summary per period, so that I can review what goes to external payroll.
Source: R13, MD-06, §21 note 1, PL-R4, E15
Settings involved: none

Acceptance criteria
- AC-C13-06.1 Given a period, When R13 is run, Then each employee in scope shows payable days, deduction days, late minutes, early minutes and overtime hours identified, in days and minutes only.  (Source: R13, MD-06, §21 note 1)
- AC-C13-06.2 Given E15 (28 October corrected to present after October was locked), When R13 is run for November, Then it shows the one-day arrear tagged 28 October, and R13 for October is unchanged.  (Source: E15, PL-R4, BR-06)

Edge and negative cases
- Whether R13 and the C12 payroll input export are the same content → C12 OQ-C12-11.

### US-C13-07 Control reports (R25–R27)
As an Auditor, I want opening balances, settings history and the audit trail as reports, so that I can verify what was set up and who did what.
Source: R25, R26, R27, §5 Auditor, G9, OB-R4, OB-R5, VC-R6, §16
Settings involved: none

Acceptance criteria
- AC-C13-07.1 Given opening balances are confirmed, When R25 is run, Then it shows each employee's opening balances as confirmed, with the as-of date.  (Source: R25, OB-R4, OB-R5)
- AC-C13-07.2 Given settings have changed, When R26 is run, Then each change shows who, when, old value, new value and reason, plus policy set assignments with effective dates.  (Source: R26, G9, G5)
- AC-C13-07.3 Given actions have been taken (create, change, approval, waiver, lock, settings change), When R27 is run, Then each shows the user and time.  (Source: R27, §16)
- AC-C13-07.4 Given vendor support access was used, When the Organisation Admin runs R27, Then every vendor action is visible.  (Source: VC-R6)
- AC-C13-07.5 Given an Auditor, When they use R25–R27, Then they can view and export but cannot change any data.  (Source: §5 Auditor)

## Open questions
- OQ-C13-01 In V1, what does the "period" filter mean: the attendance period from C12 (see OQ-C12-01), a calendar month, or any date range? Source: §14, PL-01. Why it matters: totals differ when the period is a custom cycle.
- OQ-C13-02 For a locked period that later received corrections, do R1–R8 show the values as locked, or the corrected values with the adjustment noted? Where are V1 adjustments listed, given R23 is V2? Source: BR-06, PL-R4, R23. Why it matters: HR may see two different numbers for the same day.
- OQ-C13-03 When a rule group inside an enabled module is switched off (for example LM-21, LM-24, LA-06), are the related report columns hidden, or shown empty? Source: §14, G2, BR-14. Why it matters: §14 only speaks of disabled modules.
- OQ-C13-04 What are the default report permissions for the standard roles (Manager, Supervisor, Employee, Auditor)? Can a Manager or Employee run R1–R9 for their own team or self? Source: §5, AC-01, AC-05. Why it matters: AC-01 makes it configurable but sector templates need defaults.
- OQ-C13-05 Are reports and exports shown in the user's language, given N7 names only the employee app, payslips and notifications? Source: N7, UM-R4. Why it matters: Tamil/Hindi-speaking HR users.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| §14 common rule | US-C13-01 | AC-C13-01.1–01.6 |
| UM-R1, UM-R2, AC-01, AC-07 | US-C13-01 | AC-C13-01.2, 01.3 |
| MD-03, AT2 | US-C13-01, US-C13-04 | AC-C13-01.6, AC-C13-04.2 |
| AT13 | US-C13-01 | AC-C13-01.2, 01.3 |
| AT20, TB-01, TB-02 | US-C13-01 | AC-C13-01.5, 01.7 |
| VC-R4 | US-C13-01 | AC-C13-01.8 |
| AT5, BR-06 | US-C13-01, US-C13-06 | AC-C13-01.9, AC-C13-06.2 |
| R1 | US-C13-02 | AC-C13-02.1 |
| R2 | US-C13-02 | AC-C13-02.2 |
| R3, E2, E5 | US-C13-02 | AC-C13-02.3–02.5 |
| R4 | US-C13-02 | AC-C13-02.6 |
| R5 | US-C13-02 | AC-C13-02.7 |
| R6 | US-C13-03 | AC-C13-03.1 |
| R7, LM-27, P7 | US-C13-03 | AC-C13-03.2, 03.3 |
| R8 | US-C13-03 | AC-C13-03.4 |
| R9 | US-C13-04 | AC-C13-04.1, 04.2 |
| R10, EM-R10, EM-08, E17 | US-C13-05 | AC-C13-05.1–05.3 |
| R13, MD-06, E15 | US-C13-06 | AC-C13-06.1, 06.2 |
| R25 | US-C13-07 | AC-C13-07.1 |
| R26, G9 | US-C13-07 | AC-C13-07.2 |
| R27, VC-R6 | US-C13-07 | AC-C13-07.3, 07.4 |
| §5 Auditor | US-C13-07 | AC-C13-07.5 |
