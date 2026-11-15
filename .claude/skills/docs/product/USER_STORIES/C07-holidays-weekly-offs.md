# C07 Holidays and weekly offs — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §6, §7 (G5, G7, G9), §8 (Layer 2), §9.1 (OR-R1, OR-01, OR-02), §9.2 (EM-R4, EM-R9), §9.5 (SH-09, SH-10), §9.6 step 7, §9.7, §12 (BR-03, BR-06, order of checks for a late arrival), §19 (AT13, AT20)

## Scope
- In scope (V1): holiday calendars per branch with national, festival and optional holidays (HD-R1, SH-10, OR-R1); weekly offs fixed, alternating or from the roster (HD-R2, SH-09); optional holiday choice by employees (HD-R3, HD-01); a holiday falling on a weekly off (HD-02); presence before and after a paid holiday (HD-03); whether daily-wage and hourly workers are paid for holidays and weekly offs (HD-04, HD-05), produced in V1 as payable days in the attendance totals export (MD-06).
- Out of scope / later release: turning payable days into money is M03, version 2. Earned weekly off (SP-08 to SP-11, P16) is a V2 pay rule. Work on a holiday or weekly off earning comp-off is M02 (HW-01, CO-R1); punches on such days are C04 (PC-06). Sandwich policy is C08 (LM-16 to LM-20). Applying holiday and weekly-off overlays to the day status is C06 step 7.
- Dependency notes: rostered weekly offs come from the roster in C05 (SH-R2, SH-09 "from roster").

## Stories

### US-C07-01 Maintain a holiday calendar per branch
As an HR Manager, I want a holiday calendar for each branch listing national, festival and optional holidays, so that each branch's expected working days are correct.
Source: HD-R1, SH-10, OR-R1, G5, G9, BR-03, BR-06, §16 (Schedule), §9.6 step 7, §12 order of checks (1)
Settings involved: SH-10 (holiday calendar: one per location)

Acceptance criteria
- AC-C07-01.1 Given branch A, When HR adds 2 October as a national holiday and a festival holiday on another date, Then both appear in branch A's calendar with their type.  (Source: HD-R1, OR-R1)
- AC-C07-01.2 Given branches A and B have different calendars, When a date is a holiday only in A's calendar, Then it is a holiday for employees whose calendar is A's and a normal working day for those whose calendar is B's.  (Source: HD-R1, SH-10)
- AC-C07-01.3 Given a date is a holiday for an employee, When the day is processed, Then the employee is not late or early on that day.  (Source: §12 order of checks step 1)
- AC-C07-01.4 Given HR adds, moves or removes a holiday in an open period, When saved, Then the affected employees' days are recalculated.  (Source: BR-03)
- AC-C07-01.5 Given the change touches a locked period, When saved, Then the locked period is unchanged and only the difference posts to the next open period tagged with the original date.  (Source: BR-06, PL-R4)
- AC-C07-01.6 Given a calendar change, When saved, Then who, when, old value, new value and reason are recorded.  (Source: G9, §16 audit log)
- AC-C07-01.7 Given HR scoped to branch A, When they try to change branch B's calendar, Then it is refused; a user of another organisation cannot see or change it.  (Source: UM-R2, AC-07, AT13, TB-01, AT20)

Edge and negative cases
- Employee punching at an allowed branch on a date that is a holiday there but not at the master branch (or the reverse): OQ-C07-01.
- Half-day holidays: OQ-C07-02.

### US-C07-02 Set weekly offs as fixed days, alternating days or from the roster
As an HR Manager, I want to set weekly offs as fixed days, alternating days such as the 2nd and 4th Saturday, or from the roster, so that each employee's rest days match our practice.
Source: HD-R2, SH-09, SH-R2, OR-02, G3, BR-03, §12 order of checks (1)
Settings involved: SH-09 (weekly offs: fixed days / alternating / from roster)

Acceptance criteria
- AC-C07-02.1 Given SH-09 = fixed days with Sunday, When any Sunday comes, Then it is a weekly off for the employees under that setting.  (Source: SH-09, HD-R2)
- AC-C07-02.2 Given SH-09 = alternating with the 2nd and 4th Saturday and Sunday fixed, When November 2026 is processed, Then 14 and 28 November (2nd and 4th Saturdays) and every Sunday are weekly offs, and 7 and 21 November are working days.  (Source: SH-09)
- AC-C07-02.3 Given SH-09 = from roster, When the roster marks 12 November as an off day for an employee, Then 12 November is that employee's weekly off and other employees are unaffected.  (Source: SH-09, SH-R2)
- AC-C07-02.4 Given a date is a weekly off for an employee, When the day is processed, Then the employee is not late or early on that day.  (Source: §12 order of checks step 1)
- AC-C07-02.5 Given the weekly-off setting changes, When saved, Then it applies from its start date as a new version, earlier days are not changed by the new version, and open-period days from the start date are recalculated.  (Source: G5, BR-03)

Edge and negative cases
- A roster-based employee whose roster has no off day in a week: OQ-C07-03.

### US-C07-03 Choose optional holidays
As an Employee, I want to pick my optional holidays from the branch's list up to the allowed number, so that I can take the festivals that matter to me.
Source: HD-R3, HD-01, HD-R1, OR-01, AC-05
Settings involved: HD-01 (optional holidays per year: default 0)

Acceptance criteria
- AC-C07-03.1 Given HD-01 = 0, When an employee looks for optional holidays, Then none can be chosen.  (Source: HD-01)
- AC-C07-03.2 Given HD-01 = 2 and the branch lists five optional holidays, When the employee picks two, Then both become holidays for that employee only.  (Source: HD-R3, HD-01)
- AC-C07-03.3 Given HD-01 = 2 and the employee has already picked two this year, When they try to pick a third, Then it is refused with the reason.  (Source: HD-01)
- AC-C07-03.4 Given an employee, When they pick, Then only the optional holidays of their own calendar are offered, not national or festival holidays and not other branches' lists.  (Source: HD-R1, HD-R3)
- AC-C07-03.5 Given an optional holiday is not picked by an employee, When that date is processed, Then it is a normal working day for that employee.  (Source: HD-R3)

Edge and negative cases
- Whether a pick needs approval, the deadline to pick or change, picking after the date, and which year HD-01 counts in: OQ-C07-04.

### US-C07-04 Handle a holiday that falls on a weekly off
As an HR Manager, I want to choose whether a holiday that falls on a weekly off stays as it is or moves to the next working day, so that our holiday practice is followed automatically.
Source: HD-02, SH-09, BR-03
Settings involved: HD-02 (no change / move to next working day: default no change)

Acceptance criteria
- AC-C07-04.1 Given HD-02 = no change, Sunday is a fixed weekly off and a holiday falls on Sunday 4 October, When October is processed, Then 4 October is one non-working day and Monday 5 October is a normal working day.  (Source: HD-02)
- AC-C07-04.2 Given HD-02 = move to next working day and the same dates, When October is processed, Then Monday 5 October is the holiday for the affected employees.  (Source: HD-02)
- AC-C07-04.3 Given HD-02 changes, When saved, Then the change applies from its start date and is recorded in the settings history.  (Source: G5, G9)

Edge and negative cases
- The next working day is itself a holiday or weekly off: OQ-C07-05.
- Weekly offs from the roster, where the "next working day" differs per employee: OQ-C07-05.
- Optional holiday falling on a weekly off: OQ-C07-05.

### US-C07-05 Require presence around a paid holiday
As an HR Manager, I want to be able to pay a holiday only when the employee is present on the working day before and after, so that holidays are not used to extend absence.
Source: HD-03, LM-16 (C08 context)
Settings involved: HD-03 (presence on working day before and after: default off)

Acceptance criteria
- AC-C07-05.1 Given HD-03 = off, When an employee is absent the working day before a holiday, Then the holiday is still a paid holiday.  (Source: HD-03)
- AC-C07-05.2 Given HD-03 = on, When the employee is present on the working day before and the working day after the holiday, Then the holiday is paid.  (Source: HD-03)
- AC-C07-05.3 Given HD-03 = on, When the employee is absent on the working day before or the working day after, Then the holiday is not paid and the day's explanation names HD-03 as the reason.  (Source: HD-03, BR-08)

Edge and negative cases
- What counts as "presence" (half day, approved leave, on-duty, permission) and what an unpaid holiday becomes (absent, loss of pay, deduction day): OQ-C07-06.
- Interaction with the sandwich policy (LM-16) when both are on: OQ-C07-07.

### US-C07-06 Decide whether daily-wage and hourly workers are paid for holidays and weekly offs
As an HR Manager, I want to set whether daily-wage and hourly workers are paid for holidays and for weekly offs, so that the payable days we export match our pay practice.
Source: HD-04, HD-05, EM-R4, MD-06, BRD §21 note 1
Settings involved: HD-04 (paid for holidays: default no), HD-05 (paid for weekly offs: default no)

Acceptance criteria
- AC-C07-06.1 Given HD-04 = no and a daily-wage worker has one holiday in the week, When the period totals are produced, Then the holiday is not counted as a payable day for that worker.  (Source: HD-04, MD-06)
- AC-C07-06.2 Given HD-04 = yes, When the same week is produced, Then the holiday is counted as a payable day.  (Source: HD-04)
- AC-C07-06.3 Given HD-05 = no and an hourly worker has one weekly off, When the period totals are produced, Then the weekly off is not counted as payable; given HD-05 = yes, Then it is counted.  (Source: HD-05)
- AC-C07-06.4 Given a monthly-salaried employee, When the period totals are produced, Then HD-04 and HD-05 do not change their result.  (Source: HD-04, HD-05 "daily-wage and hourly workers")

Edge and negative cases
- How an hourly worker's paid holiday or weekly off is expressed in hours: OQ-C07-08.

## Open questions
- OQ-C07-01 Which holiday calendar applies to an employee who punches at an allowed branch: the master branch's calendar (EM-R9 says the employee's own rules apply wherever they punch) or the calendar of the branch worked? And is SH-10 "one per location" the same as one per branch (HD-R1, OR-R1)? Source: HD-R1, SH-10, OR-R1, EM-R9. Why it matters: decides whether the day is a holiday, a working day, or holiday work earning comp-off.
- OQ-C07-02 Are half-day holidays supported? Source: HD-R1. Why it matters: the BRD is silent; some festivals are observed for half a day.
- OQ-C07-03 For SH-09 = from roster, is a minimum number of weekly offs per week required, and what happens in a week where the roster gives none? Source: SH-09, HD-R2. Why it matters: every day without an off becomes a working day and may trigger absence cuts.
- OQ-C07-04 For optional holidays: does the employee's pick need approval, by when must it be made or changed, can it be picked after the date, and is "per year" the calendar year, financial year or leave year (OR-01)? Source: HD-R3, HD-01, OR-01. Why it matters: decides whether an absence on an optional holiday is a holiday or an absence.
- OQ-C07-05 For HD-02 = move to next working day: what if the next working day is also a holiday or weekly off; with weekly offs from the roster (or alternating), is "next working day" worked out per employee, since rostered offs differ by person; and does HD-02 apply to optional holidays? Source: HD-02, SH-09, HD-R3. Why it matters: the moved holiday can land on different dates for colleagues in one branch.
- OQ-C07-06 For HD-03, what counts as "presence" on the working day before and after (half day, approved leave, on-duty, permission, a flagged punch), and what does an unpaid holiday become: absent, loss of pay, or a deduction day? Source: HD-03, §6 Deduction days. Why it matters: decides the pay effect and the BR-04 no-double-penalty check.
- OQ-C07-07 When HD-03 and the sandwich policy (LM-16) are both on and the employee is absent on both sides of a holiday, which applies, and how is a double effect avoided (BR-04)? Source: HD-03, LM-16, LM-17, BR-04. Why it matters: both rules can cut the same holiday.
- OQ-C07-08 For hourly workers with HD-04 or HD-05 = yes, how many hours is a paid holiday or weekly off worth (SH-03 required hours, a fixed number, an average)? Source: HD-04, HD-05, SH-03. Why it matters: the export must carry payable hours, not only days, for hourly workers.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| HD-R1 | US-C07-01, US-C07-03 | AC-C07-01.1, AC-C07-01.2, AC-C07-03.4 |
| HD-R2 | US-C07-02 | AC-C07-02.1 to AC-C07-02.3 |
| HD-R3 | US-C07-03 | AC-C07-03.2, AC-C07-03.4, AC-C07-03.5 |
| HD-01 | US-C07-03 | AC-C07-03.1 to AC-C07-03.3 |
| HD-02 | US-C07-04 | AC-C07-04.1 to AC-C07-04.3 (OQ-C07-05) |
| HD-03 | US-C07-05 | AC-C07-05.1 to AC-C07-05.3 (OQ-C07-06) |
| HD-04 | US-C07-06 | AC-C07-06.1, AC-C07-06.2, AC-C07-06.4 |
| HD-05 | US-C07-06 | AC-C07-06.3, AC-C07-06.4 |
| SH-09 | US-C07-02 | AC-C07-02.1 to AC-C07-02.3 |
| SH-10 | US-C07-01 | AC-C07-01.2 |
| BR-03 | US-C07-01, US-C07-02 | AC-C07-01.4, AC-C07-02.5 |
| BR-06 | US-C07-01 | AC-C07-01.5 |
| G5 / G9 | US-C07-01, US-C07-02, US-C07-04 | AC-C07-01.6, AC-C07-02.5, AC-C07-04.3 |
| AT13 / AT20 | US-C07-01 | AC-C07-01.7 |
