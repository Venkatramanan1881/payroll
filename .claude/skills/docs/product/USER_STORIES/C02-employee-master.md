# C02 Employee master — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §9.2 (EM-R1..EM-R10, EM-01..EM-09), §5, §7 (G5, G9), §12 (BR-05, BR-06, BR-07), §13 (E17), §16, §19 (AT13, AT16, AT20), §22.2 (TB-01); TSD §4.2 (context only)

## Scope
- In scope (V1): employee personal, contact, emergency, job and attendance details; employee code; mandatory and custom fields; module-dependent pay fields; status and probation; dated history and transfers; master and allowed branches; costing of days worked at another branch; spreadsheet bulk create and update; employee self-edit of contact details; rejoining employees.
- Out of scope / later release:
  - Handling of a punch at a branch that is not allowed (EM-07 behaviour, PC-R12, the E17 branch C part, AT16 second half): C04 Punch collection. This file only holds the employee's allowed-branch list and the EM-07 setting value; E17 is used here for costing (EM-08) only.
  - Device enrolment status and enrolment ID mapping workflow (PC-R3, PC-R8): C04. This file only holds the enrolment ID field (EM-R3).
  - Pay master use in pay runs, wage types, bank and cash split in payments: M03 Payroll, V2. Daily allowance amount field: V3.
  - Client-specific fields on the employee master (CR-R7): V2.
  - Absconding flag (E19, LM-29): C08.
  - Onboarding and offboarding checklists: C11, V3.

## Stories

### US-C02-01 Create an employee with personal and job details
As an HR Manager, I want to create an employee with personal, contact, emergency and job details, so that the employee can be scheduled, can punch and can be reported on.
Source: EM-R1, EM-R2, EM-01, EM-02, VC-01, UM-R2, AT13, TB-01; TSD §4.2
Settings involved: EM-01 (default automatic with a prefix per branch), EM-02 (chosen by the organisation), VC-01 (default allow and alert)

Acceptance criteria
- AC-C02-01.1 Given an HR Manager, When they create an employee, Then they can record personal, contact and emergency details and the job details: employee code, master branch, allowed branches, department, designation, category, employee type, reporting manager, joining date and contractor if any. (Source: EM-R1, EM-R2)
- AC-C02-01.2 Given EM-01 = automatic and branch A has prefix "CHN", When an employee is created with master branch A, Then the employee code is generated with the prefix "CHN" and is not entered by hand. (Source: EM-01)
- AC-C02-01.3 Given EM-01 = entered manually, When HR enters a code already held by another employee of the same organisation, Then the save is refused. (Source: EM-01; TSD §4.2)
- AC-C02-01.4 Given organisation B already has an employee with code "E001", When organisation A creates an employee with code "E001", Then it is accepted and neither organisation can see the other's employee. (Source: TB-01, AT20; TSD §4.2)
- AC-C02-01.5 Given EM-02 marks "date of birth" mandatory, When HR saves an employee without it, Then the save is refused naming the missing field; Given EM-02 does not mark it, Then the save is accepted. (Source: EM-02)
- AC-C02-01.6 Given an HR Manager limited to branch A, When they try to create an employee with master branch B, or open an employee whose master branch is B, Then it is refused. (Source: UM-R2, AC-07, AT13)
- AC-C02-01.7 Given a Manager, Supervisor or Employee without that right under AC-01, When they try to create an employee, Then it is refused. (Source: UM-R1, AC-01)
- AC-C02-01.8 Given the employee limit is reached, When HR creates an employee, Then VC-01 applies as in AC-V01-05.2 and AC-V01-05.3. (Source: VC-01)
- AC-C02-01.9 Given an employee is created, When the audit log is checked, Then it shows who created it and when. (Source: §16)

Edge and negative cases
- Rejoining employees: US-C02-11.
- Reporting manager who is not an employee of the organisation, or the employee themselves: forbidden by BR-07 for approval (C03); whether HR can save it is OQ-C02-01.
- Changing the branch prefix in EM-01 after codes have been issued: OQ-C02-02.

### US-C02-02 Hold attendance details
As an HR Manager, I want to record each employee's device enrolment ID, shift or roster and attendance policy set, so that their punches are matched and their days are calculated by the right rules.
Source: EM-R3, G3, BR-05, PC-R3
Settings involved: none in this module (policy set content: C06)

Acceptance criteria
- AC-C02-02.1 Given an employee, When HR records the device enrolment ID, shift or roster and policy set, Then each is saved on the employee's attendance details. (Source: EM-R3)
- AC-C02-02.2 Given a policy set is assigned directly to an employee and another to their department, When the employee's day is calculated, Then the employee-level assignment is used. (Source: G3, BR-05)

Edge and negative cases
- Enrolment ID uniqueness scope (per organisation, per branch or per device): OQ-C02-03.

### US-C02-03 Show pay detail fields only when their module is on
As an HR Manager, I want pay detail fields shown only when the module or option they belong to is enabled, so that the master does not ask for data the organisation does not use.
Source: EM-R4, MD-03, BR-14, AT2
Settings involved: none

Acceptance criteria
- AC-C02-03.1 Given a module or option is off for the organisation, When HR opens the employee master, Then that module's pay fields (for example the daily allowance amount) are not shown. (Source: EM-R4, MD-03)
- AC-C02-03.2 Given a module held employee field values and is then switched off and on again, When HR opens the employee master, Then the earlier values are shown unchanged. (Source: MD-03, AT2)

Edge and negative cases
- Which of the EM-R4 fields (payroll group, wage type, salary or rate, bank details, bank and cash split) are held in V1, before Payroll (V2) exists: OQ-C02-05.

### US-C02-04 Track employee status and probation
As an HR Manager, I want to track each employee's status (active, on probation, on notice, exited) with dates and be reminded when probation is due for confirmation, so that rules and headcount use the correct status.
Source: EM-R5, EM-04, EM-R6
Settings involved: EM-04 (default 6 months)

Acceptance criteria
- AC-C02-04.1 Given an employee, When HR changes the status, Then the new status and its date are saved and the earlier status stays in the history. (Source: EM-R5, EM-R6)
- AC-C02-04.2 Given EM-04 = 6 months and an employee joined on 1 January on probation, When the confirmation reminder is due, Then it refers to a probation end of 1 July. (Source: EM-04)
- AC-C02-04.3 Given a status change, When the audit log is checked, Then it shows who, when, old and new status. (Source: §16, G9)

Edge and negative cases
- Who receives the confirmation reminder and how many days ahead: OQ-C02-06.
- Relation of status "on probation" to type "probation": OQ-C01-05.
- Punches received for an employee after the exit date: OQ-C02-07.

### US-C02-05 Keep a dated history of employee changes
As an HR Manager, I want every change to branch, department, shift, payroll group and pay to be dated and kept, so that each day is calculated with the details in force on that day.
Source: EM-R6, G5, BR-05, BR-06, BR-03
Settings involved: none

Acceptance criteria
- AC-C02-05.1 Given employee X in department D1, When HR moves X to department D2 from 1 March, Then days up to 28 February show D1 and days from 1 March show D2. (Source: EM-R6, G5)
- AC-C02-05.2 Given X's shift is changed from 1 March, When 27 February is recalculated, Then the shift in force on 27 February is used. (Source: EM-R6, BR-05)
- AC-C02-05.3 Given a change dated inside an open period, When it is saved, Then the affected days and totals are recalculated. (Source: BR-03)
- AC-C02-05.4 Given a change dated inside a locked period, When it is saved, Then the locked period is unchanged and only the difference posts to the next open period, tagged with the original date. (Source: BR-06, PL-R4)
- AC-C02-05.5 Given any history entry, When it is viewed, Then it shows the effective date, who made the change, when, the old value and the new value; history entries cannot be edited or deleted. (Source: EM-R6, G9, §16)

Edge and negative cases
- Whether a change can be back-dated into a locked period at all, and how far back (PL-07 default 3): OQ-C02-08.

### US-C02-06 Transfer an employee to a new master branch
As an HR Manager, I want to change an employee's master branch with a date and review their shift, policy set and payroll group at the same time, so that the transfer is complete and traceable.
Source: EM-09, EM-R6, EM-R9, EM-01
Settings involved: EM-09

Acceptance criteria
- AC-C02-06.1 Given employee X with master branch A, When HR changes the master branch to B from 1 April, Then days up to 31 March belong to A and days from 1 April belong to B, and the change is in the history. (Source: EM-09, EM-R6)
- AC-C02-06.2 Given a master branch change is being saved, When HR confirms it, Then the system shows X's current shift, policy set and payroll group for review at the same time. (Source: EM-09)
- AC-C02-06.3 Given X was transferred from branch A to branch B from 1 April and an HR Manager is limited to branch A, When that HR Manager opens X's records, Then records dated up to 31 March are visible and records dated from 1 April are not. (Source: UM-R2, AC-07, AT13)

Edge and negative cases
- Whether the review is mandatory (the transfer cannot be saved until each of the three is confirmed or changed): OQ-C02-10.
- Whether the employee code changes when EM-01 uses a branch prefix: OQ-C02-02.
- A branch-limited HR Manager transferring out of their scope: OQ-C02-09.

### US-C02-07 Set an employee's allowed branches
As an HR Manager, I want to set the branches, other than the master branch, where an employee may punch, so that staff who work across branches are recorded correctly.
Source: EM-R9, EM-06, EM-07, AT16
Settings involved: EM-06 (default master branch only), EM-07 (default record and flag)

Acceptance criteria
- AC-C02-07.1 Given EM-06 = master branch only, When HR opens an employee, Then no allowed branches can be added. (Source: EM-06)
- AC-C02-07.2 Given EM-06 = chosen branches, When HR adds branch B as an allowed branch for an employee whose master branch is A, Then B is saved as allowed and A stays the master branch. (Source: EM-06, EM-R9)
- AC-C02-07.3 Given EM-06 = all branches, When HR opens the employee, Then every branch of the organisation counts as allowed. (Source: EM-06)
- AC-C02-07.4 Given an employee has one master branch, When HR tries to give them a second master branch, Then it is refused. (Source: EM-R9)
- AC-C02-07.5 Given EM-07 is set to block or record and flag, When the setting is saved, Then it is kept in the settings history; its effect on punches is tested in C04. (Source: EM-07, G9)
- AC-C02-07.6 Given allowed branches are changed, When the history is checked, Then it shows the effective date, who, old and new list. (Source: EM-R6, G9)

Edge and negative cases
- Whether EM-06 is an organisation-wide setting or chosen per employee ("Allowed branches per employee"): OQ-C02-11.
- Whether "all branches" includes branches created later: OQ-C02-11.

### US-C02-08 Cost a day worked at another branch
As an HR Manager, I want a day worked at an allowed branch to follow the employee's own rules and be costed as configured, so that branch costs are right without changing how the employee is paid.
Source: EM-R9, EM-R10, EM-08, E17, AT16, R10
Settings involved: EM-08 (default the branch worked)

Acceptance criteria
- AC-C02-08.1 (E17) Given master branch A, allowed branch B and EM-08 = the branch worked, When the employee punches in and out at B, Then the day is present under the employee's own shift and rules, and the day is costed to branch B. (Source: E17, EM-R9, EM-08, AT16)
- AC-C02-08.2 Given the same punches and EM-08 = the master branch, When the day is costed, Then it is costed to branch A and the day status is unchanged. (Source: EM-08, AT16)
- AC-C02-08.3 Given two employees with identical punches at branch B, one with master A under policy set P1 and one with master B under policy set P2, When their days are calculated, Then each result follows their own policy set. (Source: EM-R9, AT4)
- AC-C02-08.4 Given an employee worked 18 days at A and 4 days at B in a month with EM-08 = the branch worked, When report R10 is run for that month, Then it shows 18 days at A and 4 days at B for that employee. (Source: EM-R10, R10)
- AC-C02-08.5 Given EM-08 is changed, When the settings history is checked, Then it shows who, when, old value, new value and reason. (Source: G9)

Edge and negative cases
- Punch in at one branch and out at another on the same day: OQ-C02-12.
- Costing of a day whose punch at a non-allowed branch is later approved (EM-07): OQ-C02-12.
- Whether "days worked by branch" (EM-R10) follows EM-08 or always the branch worked: OQ-C02-12.

### US-C02-09 Create and update employees by spreadsheet upload
As an HR Manager, I want to create and update employees in bulk by spreadsheet, with validation and an error report, so that large workforces can be loaded quickly and safely.
Source: EM-R7, EM-02, EM-01, VC-01, UM-R2
Settings involved: EM-01 (default automatic), EM-02, VC-01 (default allow and alert)

Acceptance criteria
- AC-C02-09.1 Given a spreadsheet of new employees with all mandatory fields, When HR uploads it, Then each employee is created and the result shows the count created. (Source: EM-R7)
- AC-C02-09.2 Given a row missing a field marked mandatory under EM-02, When the file is uploaded, Then that row is not applied and the error report names the row and the field. (Source: EM-R7, EM-02)
- AC-C02-09.3 Given a row with a branch, department or employee type that does not exist in the organisation, When the file is uploaded, Then that row is not applied and the error report gives the reason. (Source: EM-R7, OR-R1..OR-R3)
- AC-C02-09.4 Given a row updating an existing employee's department with an effective date, When the file is uploaded, Then the change is dated and kept in history as for a screen change. (Source: EM-R7, EM-R6)
- AC-C02-09.5 Given an HR Manager limited to branch A, When their file contains a row with master branch B, Then that row is refused and reported. (Source: UM-R2, AT13)
- AC-C02-09.6 Given a row names an employee code that exists only in another organisation, When the file is uploaded, Then the other organisation's employee is neither read nor changed, and the error report reveals nothing about it. (Source: TB-01, AT20)

Edge and negative cases
- Whether a file with some bad rows is applied row by row or rejected as a whole: OQ-C02-04.
- Employee limit with VC-01 = block during upload: AC-V01-05.4 and OQ-C02-04.

### US-C02-10 Add organisation-defined custom fields
As an HR Manager, I want to add custom fields to the employee master, so that my organisation can keep information the standard form does not hold.
Source: EM-R8, EM-02, OR-R5
Settings involved: EM-02

Acceptance criteria
- AC-C02-10.1 Given the HR Manager adds a custom field "Uniform size", When they open any employee of the organisation, Then the field is shown and can be filled. (Source: EM-R8)
- AC-C02-10.2 Given a custom field exists, When the HR Manager sets mandatory fields under EM-02, Then the custom field is offered in the list. (Source: EM-02 "from the full field list")
- AC-C02-10.3 Given organisation A adds a custom field, When users of organisation B open their employee master, Then the field is not shown. (Source: TB-01, AT20)

Edge and negative cases
- Removing a custom field that holds data: OQ-C02-13.

### US-C02-11 Handle a rejoining employee
As an HR Manager, I want a former employee who rejoins to get a new code or their old code as configured, so that rejoiners are recorded consistently.
Source: EM-05, EM-R5, EM-R6
Settings involved: EM-05 (default reuse old code)

Acceptance criteria
- AC-C02-11.1 Given employee "CHN0042" exited on 31 March and EM-05 = reuse old code, When HR rejoins them on 1 September, Then the employee's code is "CHN0042" and the status is active from 1 September. (Source: EM-05, EM-R5)
- AC-C02-11.2 Given the same employee and EM-05 = new code, When HR rejoins them, Then a new code is issued under EM-01 and "CHN0042" is not reused. (Source: EM-05, EM-01)
- AC-C02-11.3 Given a rejoined employee, When the employee's records for the earlier employment are opened, Then attendance and history from before the exit are still present and unchanged. (Source: EM-R6, BR-06)

Edge and negative cases
- Whether "reuse old code" means one employee record continued, or a new record sharing the code; and whether leave balances, counters and history carry over: OQ-C02-14.
- Rejoining at a different branch when the code carries a branch prefix: OQ-C02-02.
- Whether a rejoiner counts toward the employee limit as new: OQ-V01-06.

### US-C02-12 Employee edits own contact details
As an Employee, I want to update my own contact details, so that the organisation can reach me, with HR approval when the organisation requires it.
Source: EM-03, BR-07, AC-02
Settings involved: EM-03 (default on, with approval)

Acceptance criteria
- AC-C02-12.1 Given EM-03 = on, with approval, When an employee changes their mobile number, Then the change is pending and the old number stays in use until HR approves. (Source: EM-03)
- AC-C02-12.2 Given a pending contact change, When HR rejects it, Then the old value stays and the employee sees the rejection. (Source: EM-03)
- AC-C02-12.3 Given EM-03 = on, without approval, When an employee changes their contact details, Then the change applies at once and is logged with who and when. (Source: EM-03, §16)
- AC-C02-12.4 Given EM-03 = off, When an employee tries to change their contact details, Then the edit is not available. (Source: EM-03)
- AC-C02-12.5 Given an employee tries to edit their own job, attendance or pay details, When they save, Then it is refused. (Source: EM-03 covers contact details only)
- AC-C02-12.6 Given an HR Manager who is also an employee raises a change to their own contact details, When they try to approve it, Then it is refused. (Source: BR-07)

Edge and negative cases
- A change to the mobile number used for sign-in (UM-R3): whether the new number is used for sign-in only after approval: OQ-C02-15.

## Open questions
- OQ-C02-01 Can a reporting manager be saved who is the employee themselves, an exited employee, or someone outside the employee's access scope? Source: EM-R2, BR-07, AC-02. Why it matters: approval routing depends on it.
- OQ-C02-02 With EM-01 = automatic with a branch prefix, does an employee's code change on transfer to another branch, on rejoining at another branch, or when the prefix is changed? Source: EM-01, EM-09, EM-05. Why it matters: codes appear on devices, reports and payslips.
- OQ-C02-03 Is the device enrolment ID unique per organisation, per branch or per device? Source: EM-R3, PC-R3. Why it matters: employees working at allowed branches may be enrolled on several devices.
- OQ-C02-04 For a spreadsheet upload with some bad rows (or rows over the employee limit when VC-01 = block), are the good rows applied and the bad ones reported, or is the whole file rejected? Source: EM-R7, VC-01. Why it matters: partial loads can leave a half-updated workforce.
- OQ-C02-05 Which pay details (payroll group, wage type, salary or rate, bank details, bank and cash split) are held in V1 when Payroll (V2) is not yet available, and are they needed for the V1 payroll input export? Source: EM-R4, MD-06, §21. Why it matters: decides V1 employee form and upload template.
- OQ-C02-06 Who receives the probation confirmation reminder, how many days before the end, and does confirmation change status automatically or only when HR acts? Source: EM-04, EM-R5. Why it matters: reminder content and status flow.
- OQ-C02-07 What happens to punches received for an employee after their exit date (stored and flagged, ignored for results)? Source: EM-R5, BR-01. Why it matters: devices may keep an exited employee enrolled.
- OQ-C02-08 Can an employee master change be back-dated into a locked period, and is the limit PL-07 (default 3 past periods)? Source: EM-R6, BR-06, PL-07. Why it matters: back-dated transfers and pay changes are common.
- OQ-C02-09 Can an HR Manager limited to chosen branches transfer an employee to a branch outside their scope? Source: EM-09, UM-R2, AC-07. Why it matters: scope enforcement on transfers.
- OQ-C02-10 Does "reviewed at the time of transfer" (EM-09) mean the transfer cannot be saved until shift, policy set and payroll group are each confirmed or changed, or is it a reminder only? Source: EM-09. Why it matters: blocking versus advisory flow.
- OQ-C02-11 Is EM-06 an organisation-wide setting or chosen per employee, and does "all branches" include branches created later? Source: EM-06, EM-R9. Why it matters: geo-fence and punch acceptance depend on the allowed list.
- OQ-C02-12 How is a day costed when the employee punches in at one branch and out at another, or when a punch at a non-allowed branch is approved; and does R10 "days worked by branch" follow EM-08 or always the branch worked? Source: EM-08, EM-07, EM-R10, E17. Why it matters: branch costing totals.
- OQ-C02-13 Can a custom field that holds data be removed, and is its data kept? Source: EM-R8. Why it matters: data loss.
- OQ-C02-14 For EM-05 "reuse old code": is the earlier employee record reopened (one continuous history) or is a new record created with the same code; and do leave balances, comp-off, counters and dated history carry over? Source: EM-05, EM-R6. Why it matters: history, balances and reports for rejoiners.
- OQ-C02-15 When an employee changes the mobile number used for sign-in (UM-R3) under EM-03 with approval, is the new number used for sign-in only after approval, and is the old number's session ended? Source: EM-03, UM-R3. Why it matters: account takeover risk and lock-out.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| EM-R1 | US-C02-01 | AC-C02-01.1 |
| EM-R2 | US-C02-01 | AC-C02-01.1 |
| EM-R3 | US-C02-02 | AC-C02-02.1 |
| EM-R4 | US-C02-03 | AC-C02-03.1, 03.2 |
| EM-R5 | US-C02-04, US-C02-11 | AC-C02-04.1, 11.1 |
| EM-R6 | US-C02-05 | AC-C02-05.1–05.5, 06.1, 07.6, 09.4, 11.3 |
| EM-R7 | US-C02-09 | AC-C02-09.1–09.6 |
| EM-R8 | US-C02-10 | AC-C02-10.1–10.3 |
| EM-R9 | US-C02-07, US-C02-08 | AC-C02-07.2, 07.4, 08.1, 08.3 |
| EM-R10 | US-C02-08 | AC-C02-08.4 |
| EM-01 | US-C02-01, US-C02-11 | AC-C02-01.2, 01.3, 11.2 |
| EM-02 | US-C02-01, US-C02-09, US-C02-10 | AC-C02-01.5, 09.2, 10.2 |
| EM-03 | US-C02-12 | AC-C02-12.1–12.5 |
| EM-04 | US-C02-04 | AC-C02-04.2 |
| EM-05 | US-C02-11 | AC-C02-11.1, 11.2 |
| EM-06 | US-C02-07 | AC-C02-07.1, 07.2, 07.3 |
| EM-07 | US-C02-07 | AC-C02-07.5 (behaviour in C04) |
| EM-08 | US-C02-08 | AC-C02-08.1, 08.2, 08.5 |
| EM-09 | US-C02-06 | AC-C02-06.1, 06.2 |
| E17 | US-C02-08 | AC-C02-08.1 (costing part) |
| AT16 | US-C02-08 | AC-C02-08.1, 08.2 (allowed-branch part) |
| BR-05 / G5 | US-C02-02, US-C02-05 | AC-C02-02.2, 05.1, 05.2 |
| BR-06 | US-C02-05 | AC-C02-05.4 |
| BR-07 | US-C02-12 | AC-C02-12.6 |
| UM-R2 / AT13 | US-C02-01, 06, 09 | AC-C02-01.6, 06.3, 09.5 |
| TB-01 / AT20 | US-C02-01, 09, 10 | AC-C02-01.4, 09.6, 10.3 |
| VC-01 | US-C02-01, US-C02-09 | AC-C02-01.8 |
