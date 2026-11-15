# C01 Organisation setup — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §9.1 (OR-R1..OR-R5, OR-01..OR-04), §5, §7 (G5, G6, G9), §12 (BR-11), §19 (AT12, AT13, AT20), §22.2 (TB-01)

## Scope
- In scope (V1): branches with address, time zone, geo-fence, holiday calendar and devices; departments, designations and employee categories; employee types; contractors and their workers; organisation-wide settings OR-01..OR-04; extendable lists.
- Out of scope / later release:
  - How a geo-fence is drawn, tested and applied to punches (PC-R12, PC-R14, PC-03, PC-16): C04. This module only attaches the fence to the branch.
  - Device registration and online status (PC-R2): C04. This module only shows devices against the branch.
  - Holiday calendar contents (SH-10, C07): C07. This module only links a calendar to a branch.
  - Contractor statements (R21) and contract worker pay (§10.3.5): M03 Payroll, V2.

## Stories

### US-C01-01 Create and maintain branches
As an Organisation Admin, I want to create branches with address, time zone, geo-fence, holiday calendar and devices, so that punches and schedules are read against the right place and local time.
Source: OR-R1, BR-11, §5 (Organisation Admin manages branches), UM-R2, AT13
Settings involved: PC-03 (default radius 200 metres; C04), SH-10 (one holiday calendar per location; C07)

Acceptance criteria
- AC-C01-01.1 Given an Organisation Admin, When they create a branch with name, address and time zone, Then the branch is saved and can be chosen as a master or allowed branch in the employee master. (Source: OR-R1, EM-R2)
- AC-C01-01.2 Given a branch, When the Organisation Admin attaches a holiday calendar, a geo-fence and devices to it, Then the branch record shows each of them. (Source: OR-R1)
- AC-C01-01.3 Given a branch is being created without a time zone, When it is saved, Then the save is refused, because every punch time is held with its branch's time zone. (Source: OR-R1, BR-11)
- AC-C01-01.4 Given two branches in different time zones and one punch at 09:00 local time at each, When the punches are viewed, Then each shows 09:00 in its own branch's local time. (Source: BR-11)
- AC-C01-01.5 Given an HR Manager limited to branch A, When they list branches or open branch B, Then branch B's data is not shown to them. (Source: UM-R2, AC-07, AT13)
- AC-C01-01.6 Given an Employee, Supervisor or Manager, When they try to create or change a branch, Then it is refused unless their role was given that right under AC-01. (Source: §5, AC-01, UM-R1)
- AC-C01-01.7 Given a branch is created or changed, When the audit log is checked, Then it shows who, when, old and new values. (Source: G9, §16)
- AC-C01-01.8 Given organisations A and B each have a branch named "Main", When a user of A lists branches, Then only A's "Main" is listed. (Source: TB-01, AT20)

Edge and negative cases
- Whether an HR Manager may create or change branches, or only the Organisation Admin: OQ-C01-01.
- Closing or deactivating a branch that still has employees, devices or open periods: OQ-C01-02.
- Changing a branch's time zone after punches exist: OQ-C01-03.

### US-C01-02 Maintain departments, designations and employee categories
As an HR Manager, I want to create departments, designations and employee categories, so that employees are grouped the way my organisation works.
Source: OR-R2, OR-R5, G3, G6
Settings involved: none

Acceptance criteria
- AC-C01-02.1 Given an organisation created from a sector template, When the HR Manager opens categories, Then the template's starter list is present (for example unskilled, semi-skilled, skilled, staff) and can be changed. (Source: OR-R2, G6)
- AC-C01-02.2 Given the department, designation or category list, When the HR Manager adds a new entry, Then it can be chosen on the employee master straight away. (Source: OR-R2, OR-R5, EM-R2)
- AC-C01-02.3 Given departments and categories exist, When the HR Manager assigns a policy set, Then department and category are offered as assignment levels. (Source: G3)
- AC-C01-02.4 Given a user limited to chosen departments (AC-07), When they open the department list, Then they see only their departments' employees and data. (Source: UM-R2, AC-07, AT13)
- AC-C01-02.5 Given any change to these lists, When the audit log is checked, Then it shows who, when, old and new values. (Source: §16, G9)

Edge and negative cases
- Removing or renaming an entry that is in use by employees or policy set assignments: OQ-C01-04.
- Who maintains these lists (§5 gives branches to the Organisation Admin but is silent on departments and designations): OQ-C01-01.

### US-C01-03 Maintain employee types
As an HR Manager, I want the employee types permanent, probation, trainee, casual and contract available and extendable, so that rules can treat each type differently.
Source: OR-R3, OR-R5, G7
Settings involved: none

Acceptance criteria
- AC-C01-03.1 Given a new organisation, When the HR Manager opens employee types, Then permanent, probation, trainee, casual and contract are present. (Source: OR-R3)
- AC-C01-03.2 Given the HR Manager adds a new employee type, When they edit an employee, Then the new type can be chosen. (Source: OR-R5, EM-R2)
- AC-C01-03.3 Given employee types exist, When the HR Manager sets an exemption on a rule group, Then employee type is offered as an exemption criterion. (Source: G7)

Edge and negative cases
- The type "probation" (OR-R3) overlaps the status "on probation" (EM-R5): OQ-C01-05.
- Removing a type in use: OQ-C01-04.

### US-C01-04 Record contractors and the workers they supply
As an HR Manager, I want to record contractors and link the workers each one supplies, so that contract workers can be reported by contractor.
Source: OR-R4, EM-R2
Settings involved: none

Acceptance criteria
- AC-C01-04.1 Given the HR Manager creates a contractor, When they edit an employee, Then that contractor can be chosen in the employee's "contractor" field. (Source: OR-R4, EM-R2)
- AC-C01-04.2 Given contractor X supplies three employees, When the HR Manager opens contractor X, Then the three employees are listed. (Source: OR-R4)
- AC-C01-04.3 Given an employee is linked to a contractor, When the HR Manager removes the link, Then the change is kept in the employee's history and the audit log. (Source: EM-R6, §16)

Edge and negative cases
- Whether an employee of type "contract" must have a contractor, and whether a non-contract employee may have one: OQ-C01-06.
- Contractor statements: V2 (R21), not covered.

### US-C01-05 Set the organisation's year, week, languages and currency
As an HR Manager, I want to set the financial and leave year, week start, offered languages and currency, so that periods, accruals, reports and the app match my organisation.
Source: OR-01, OR-02, OR-03, OR-04, UM-R4, N7, G5, G9, AC-04
Settings involved: OR-01 (default April to March), OR-02 (default Monday), OR-03 (default English on), OR-04 (default Indian rupee)

Acceptance criteria
- AC-C01-05.1 Given a new organisation, When the HR Manager opens organisation settings, Then OR-01 = April to March, OR-02 = Monday, OR-03 = English on, OR-04 = Indian rupee, unless the sector template sets otherwise. (Source: OR-01..OR-04, G6)
- AC-C01-05.2 Given OR-02 = Monday, When a weekly report is produced, Then its weeks start on Monday; Given OR-02 is changed to Sunday, Then weeks in later reports start on Sunday. (Source: OR-02)
- AC-C01-05.3 Given OR-03 has English and Tamil on and Hindi off, When a user chooses their language, Then only English and Tamil are offered. (Source: OR-03, UM-R4)
- AC-C01-05.4 Given OR-04 = Indian rupee, When an amount is shown, Then it is shown in Indian rupees. (Source: OR-04; the default number format is OQ-C01-08)
- AC-C01-05.5 Given AC-04 = HR Manager only, When a Manager, Payroll Officer or Employee tries to change OR-01..OR-04, Then it is refused. (Source: AC-04)
- AC-C01-05.6 Given any of OR-01..OR-04 is changed, When the settings history is checked, Then it shows who, when, old value, new value and reason. (Source: G9)

Edge and negative cases
- Changing OR-01 after leave has accrued or a year is part-way through: OQ-C01-07.
- OR-01 is one setting for "financial year and leave year": can they differ? OQ-C01-07.
- Turning off a language that users have chosen: OQ-C03-07 (C03).
- Currencies other than the rupee, and the default number format: OQ-C01-08.

## Open questions
- OQ-C01-01 Who may create and change branches, departments, designations, categories, employee types and contractors: only the Organisation Admin (§5 names it for branches), the HR Manager, or both? Source: §5, OR-R1, OR-R2. Why it matters: default role permissions under AC-01.
- OQ-C01-02 Can a branch be closed or deactivated, and what happens to employees whose master or allowed branch it is, its devices, its geo-fence and its open periods? Source: OR-R1, EM-R9. Why it matters: branch closure is common in retail and restaurants.
- OQ-C01-03 If a branch's time zone is changed after punches exist, are past punches and results shown in the old or new time zone, and are open days recalculated? Source: OR-R1, BR-11, BR-03. Why it matters: could shift attendance dates.
- OQ-C01-04 Can a department, designation, category, employee type or contractor that is in use (by employees, policy set assignments or exemptions) be removed or renamed, and how are history and reports kept? Source: OR-R2, OR-R3, OR-R5, EM-R6. Why it matters: dated history must stay readable.
- OQ-C01-05 How do employee type "probation" (OR-R3) and status "on probation" (EM-R5) relate: is one derived from the other, and does confirmation (EM-04) change the type? Source: OR-R3, EM-R5, EM-04. Why it matters: rules that exempt or target probationers could read different fields.
- OQ-C01-06 Must an employee of type "contract" be linked to a contractor, and may other types be linked? Source: OR-R4, EM-R2. Why it matters: validation on the employee master and upload.
- OQ-C01-07 Can the financial year and the leave year be set differently, and what happens if OR-01 is changed mid-year (accruals, carry forward, reports)? Source: OR-01, G5. Why it matters: OR-01 is a single setting in the BRD but names two years.
- OQ-C01-08 Which currencies and number formats are offered besides the Indian rupee, and is the default number format Indian grouping (1,00,000) or international (100,000)? Source: OR-04. Why it matters: option list and default format are not stated.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| OR-R1 | US-C01-01 | AC-C01-01.1, 01.2, 01.3, 01.6, 01.7 |
| OR-R2 | US-C01-02 | AC-C01-02.1, 02.2, 02.3 |
| OR-R3 | US-C01-03 | AC-C01-03.1 |
| OR-R4 | US-C01-04 | AC-C01-04.1, 04.2, 04.3 |
| OR-R5 | US-C01-02, US-C01-03 | AC-C01-02.2, 03.2 |
| OR-01 | US-C01-05 | AC-C01-05.1 |
| OR-02 | US-C01-05 | AC-C01-05.1, 05.2 |
| OR-03 | US-C01-05 | AC-C01-05.1, 05.3 |
| OR-04 | US-C01-05 | AC-C01-05.1, 05.4 |
| BR-11 | US-C01-01 | AC-C01-01.3, 01.4 |
| G3 / G7 | US-C01-02, US-C01-03 | AC-C01-02.3, 03.3 |
| G6 | US-C01-02, US-C01-05 | AC-C01-02.1, 05.1 |
| G9 / §16 | all | AC-C01-01.7, 02.5, 04.3, 05.6 |
| AC-04 | US-C01-05 | AC-C01-05.5 |
| UM-R2 / AT13 | US-C01-01, US-C01-02 | AC-C01-01.5, 02.4 |
| TB-01 / AT20 | US-C01-01 | AC-C01-01.8 |
