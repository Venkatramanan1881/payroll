# C15 Opening balances — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.1 (C15), §5, §9.8, §9.9, §9.15, §10.2, §12 (BR-07), §14 (R25), §16, §19 (AT13, AT18, AT20), §21; TSD none

## Scope
- In scope (V1): entering, uploading, confirming and changing opening balances for leave per type, comp-off (when M02 is on), permissions used this month, and late and early counters for the current period, each with an as-of date (§21 Version 1 "Opening balances for leave, permissions and counters").
- Out of scope / later release:
  - Open advances and open loans with remaining instalments (OB-R2) — V2 with C10 (§4.1 C15, §21 Version 2). AT18's "advances, loans" part applies from V2.

## Stories

### US-C15-01 Enter opening balances per employee
As an HR Manager, I want a screen where I enter each employee's opening balances with an as-of date, so that an organisation, branch or employee starting part-way through a year continues from the right figures.
Source: OB-R1, OB-R2, OB-R4, OB-01, §5 HR Manager, LM-R8, MD-03, AT13, AT20
Settings involved: OB-01 (default HR Manager)

Acceptance criteria
- AC-C15-01.1 Given an organisation, branch or employee starting on the system part-way through a year, When the HR Manager opens the opening balances screen, Then they can enter, per employee: a balance per leave type, comp-off balance, permissions used this month, and late and early counters for the current period, each with an as-of date.  (Source: OB-R1, OB-R2, OB-R4)
- AC-C15-01.2 Given M02 Compensatory off is disabled, When the screen is opened, Then the comp-off balance is not offered.  (Source: MD-03, LM-R8)
- AC-C15-01.3 Given a V1 organisation, When the screen is opened, Then advances and loans are not offered.  (Source: §4.1 C15, §21)
- AC-C15-01.4 Given a balance is entered without an as-of date, When it is saved, Then it is refused.  (Source: OB-R4)
- AC-C15-01.5 Given an HR Manager limited to branch X, When they open the screen, Then they can enter balances only for employees within their scope.  (Source: UM-R2, AC-07, AT13)
- AC-C15-01.6 Given a user who is not allowed by OB-01 (for example a Manager or Employee), When they try to enter opening balances, Then it is refused.  (Source: OB-01, UM-R1)
- AC-C15-01.7 Given a user of organisation A, When they enter or view opening balances, Then they can reach only organisation A's employees.  (Source: AT20, TB-01)

Edge and negative cases
- Validation limits (negative balance, above the maximum, leave type the employee is not eligible for) → OQ-C15-03.
- "Permissions used this month" when the permission limit period (PM-08) is not the calendar month → OQ-C15-01.
- "Current period" for counters when LA-18 / EC-17 reset is not the period → OQ-C15-06.

### US-C15-02 Upload opening balances by spreadsheet
As an HR Manager, I want to upload opening balances for many employees by spreadsheet with validation and an error report, so that setting up a large organisation is quick and accurate.
Source: OB-R3, OB-R4, TB-02, AT20
Settings involved: OB-01 (default HR Manager)

Acceptance criteria
- AC-C15-02.1 Given a spreadsheet with valid rows, When it is uploaded, Then each row's balances are recorded against the matching employee with its as-of date.  (Source: OB-R3, OB-R4)
- AC-C15-02.2 Given a spreadsheet with some invalid rows (unknown employee code, unknown leave type, missing as-of date, non-numeric value), When it is uploaded, Then an error report lists each invalid row with the reason.  (Source: OB-R3)
- AC-C15-02.3 Given a row with an employee code of another organisation or outside the uploader's scope, When it is uploaded, Then the row is rejected and reported, and no data of the other organisation is revealed.  (Source: AT13, AT20, TB-01)
- AC-C15-02.4 Given the uploaded file, When it is stored, Then it is held privately in the organisation's own area and opened only through an access check.  (Source: TB-02)

Edge and negative cases
- Whether valid rows are saved when other rows fail, or the whole file is rejected → OQ-C15-09.

### US-C15-03 Confirm opening balances once
As an HR Manager, I want to confirm opening balances once, after which they are locked, so that everyone works from one agreed starting point.
Source: OB-R5, OB-01, BR-07, R25, §16
Settings involved: OB-01 (default HR Manager)

Acceptance criteria
- AC-C15-03.1 Given OB-01 = HR Manager and entered balances, When the HR Manager confirms, Then the balances are locked and the confirmation is recorded with user and time.  (Source: OB-R5, OB-01, §16)
- AC-C15-03.2 Given OB-01 = HR Manager with Organisation Admin approval, When the HR Manager confirms, Then the balances stay unconfirmed until the Organisation Admin approves.  (Source: OB-01)
- AC-C15-03.3 Given OB-01 requires approval, When the person who entered the balances tries to approve their own confirmation, Then it is refused.  (Source: BR-07)
- AC-C15-03.4 Given balances are confirmed, When R25 is run, Then it shows them as confirmed with their as-of dates.  (Source: R25)
- AC-C15-03.5 Given balances are confirmed, When anyone tries to edit them through the normal entry screen or upload, Then the edit is refused (changes go through US-C15-04).  (Source: OB-R5)

Edge and negative cases
- Whether unconfirmed balances are used in calculations, and from when → OQ-C15-08.
- Whether OB-01's approval applies to entry, confirmation or both → OQ-C15-07.

### US-C15-04 Change an opening balance after confirmation
As an HR Manager, I want to change a confirmed opening balance with a reason when the setting allows it, so that a mistake can be fixed with a trace.
Source: OB-R5, OB-02, G9, §16
Settings involved: OB-02 (default allowed with a reason)

Acceptance criteria
- AC-C15-04.1 Given OB-02 = not allowed, When anyone tries to change a confirmed balance, Then it is refused.  (Source: OB-02)
- AC-C15-04.2 Given OB-02 = allowed with a reason, When the HR Manager changes a confirmed balance without a reason, Then it is refused.  (Source: OB-02, OB-R5)
- AC-C15-04.3 Given OB-02 = allowed with a reason, When the HR Manager changes a confirmed balance with a reason, Then the change is saved and logged with who, when, old value, new value and reason.  (Source: OB-R5, G9, §16)

Edge and negative cases
- A change whose as-of date falls in a locked period, or after the balance has already been used → OQ-C15-04.

### US-C15-05 Opening balances carry into balances and counters
As an Employee, I want my balances and counters to start from my confirmed opening balances, so that I am not penalised or short-changed because the system started mid-year.
Source: AT18, OB-R4, LM-R4, LM-02, PM-06, LA-06, EC-05, CO-R2
Settings involved: LM-02 (default monthly accrual), PM-06 (default 2), LA-06 / EC-05 (default off), LA-07 / LA-08 (default 0, every 3rd)

Acceptance criteria
- AC-C15-05.1 Given a confirmed opening balance of B days for a leave type as of date D, When leave is accrued, taken or lapsed after D, Then the balance equals B plus accruals after D minus leave taken after D, and earlier dates do not change it.  (Source: AT18, OB-R4, LM-02)
- AC-C15-05.2 Given an opening balance, When the employee views their leave balance, Then it includes the opening balance.  (Source: LM-R4, AT18)
- AC-C15-05.3 Given PM-06 = 2 and an opening "permissions used this month" of 1, When the employee applies for a permission later that month, Then 1 permission is available, and a third request in the month follows PM-18.  (Source: AT18, OB-R2, PM-06, PM-18)
- AC-C15-05.4 Given LA-06 on, LA-07 = 0, LA-08 = every 3rd, and an opening late counter of 2 for the current period, When one more late mark is recorded in that period, Then it is counted as the 3rd mark for the period-end penalty.  (Source: AT18, OB-R2, LA-06, LA-08, BR-10)
- AC-C15-05.5 Given M02 on and an opening comp-off balance, When the employee applies for comp-off, Then the opening balance is available to take like leave.  (Source: CO-R2, OB-R2, AT18)
- AC-C15-05.6 Given the same confirmed opening balances and the same later inputs, When balances are recalculated, Then the result is identical.  (Source: AT5)

Edge and negative cases
- Expiry date of opening comp-off (HW-05 counts from the day earned) → OQ-C15-02.
- Accrual for the part-month or part-year around the as-of date (LM-04) → OQ-C15-05.
- Days before the as-of date: calculated, ignored, or locked → OQ-C15-05.

## Open questions
- OQ-C15-01 OB-R2 says "permissions used this month", but PM-08 lets the permission limit period be a week, payroll period, quarter or year. When PM-08 is not the calendar month, what is entered? Source: OB-R2, PM-08. Why it matters: wrong opening usage gives wrong permission limits.
- OQ-C15-02 What expiry date does an opening comp-off balance get, since HW-05 counts from the day earned and that date is before the system started? Source: OB-R2, HW-05, HW-06. Why it matters: comp-off may lapse at once or never.
- OQ-C15-03 What validation applies to opening values: negative leave balances (allowed only up to LM-07?), balances above the maximum (LM-05), leave types the employee is not eligible for (LM-01), counters above the penalty trigger? Source: OB-R3, LM-01, LM-05, LM-07. Why it matters: the error report needs defined rules.
- OQ-C15-04 For a change after confirmation (OB-02), if the as-of date is inside a locked period or the balance has since been used, is the effect recalculated forward from the as-of date or posted as a difference to the next open period (BR-06, PL-R4)? Source: OB-02, OB-R5, BR-03, BR-06. Why it matters: a balance change could otherwise alter a locked period.
- OQ-C15-05 How are the days around the as-of date treated: is accrual for the part-period after the as-of date pro-rated (LM-04), and are punches or days before the as-of date calculated at all? Source: OB-R4, LM-03, LM-04. Why it matters: double or missing accrual in the start month.
- OQ-C15-06 "Late and early counters for the current period": which period is meant in V1 (see OQ-C12-01), and what is entered when LA-18 / EC-17 reset by quarter or year? Source: OB-R2, LA-18, EC-17. Why it matters: count-based penalties depend on it.
- OQ-C15-07 Under OB-01 "HR Manager with Organisation Admin approval", does the approval apply to entry, to confirmation, or both, and do changes under OB-02 also need approval? Source: OB-01, OB-02. Why it matters: permission ACs.
- OQ-C15-08 Are opening balances used in calculations before they are confirmed, or only from confirmation (still effective from the as-of date)? Source: OB-R4, OB-R5. Why it matters: leave requests made during set-up.
- OQ-C15-09 On spreadsheet upload, are valid rows saved when other rows fail, or is the whole file rejected? Source: OB-R3, EM-R7. Why it matters: partial loads can leave mixed data.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| OB-R1 | US-C15-01 | AC-C15-01.1 |
| OB-R2 | US-C15-01, US-C15-05 | AC-C15-01.1–01.3, AC-C15-05.3–05.5; OQ-C15-01, 02, 06 |
| OB-R3 | US-C15-02 | AC-C15-02.1–02.3; OQ-C15-03, 09 |
| OB-R4 | US-C15-01, US-C15-05 | AC-C15-01.4, AC-C15-05.1; OQ-C15-05 |
| OB-R5 | US-C15-03, US-C15-04 | AC-C15-03.1, 03.5, AC-C15-04.2, 04.3 |
| OB-01 | US-C15-01, US-C15-03 | AC-C15-01.6, AC-C15-03.1–03.3; OQ-C15-07 |
| OB-02 | US-C15-04 | AC-C15-04.1–04.3; OQ-C15-04 |
| AT18 | US-C15-05 | AC-C15-05.1–05.5 |
| AT13 | US-C15-01, US-C15-02 | AC-C15-01.5, AC-C15-02.3 |
| AT20, TB-01, TB-02 | US-C15-01, US-C15-02 | AC-C15-01.7, AC-C15-02.3, 02.4 |
| BR-07 | US-C15-03 | AC-C15-03.3 |
| R25 | US-C15-03 | AC-C15-03.4 |
| MD-03, LM-R8 | US-C15-01 | AC-C15-01.2 |
| G9, §16 | US-C15-03, US-C15-04 | AC-C15-03.1, AC-C15-04.3 |
| AT5 | US-C15-05 | AC-C15-05.6 |
