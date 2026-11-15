# C05 Shifts and roster — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §6, §7 (G3, G5, G9), §8 (Layer 2), §9.2 (EM-R3, EM-R6, EM-09), §9.5, §9.6 steps 1-2, §12 (BR-03, BR-05, BR-06, BR-07, BR-11), §16 (Schedule), §19 (AT5, AT13, AT20); TSD §5.3 (context only)

## Scope
- In scope (V1): defining shifts of every type in SH-R1 and SH-01 (fixed, flexible, flexible with core hours, roster, split, open, night); shift times, required hours, core hours, flexible band, shift window, night-shift attendance date and half-day split point (SH-02 to SH-07, SH-11); assigning shifts and rosters to employees, teams and branches by screen or spreadsheet (SH-R2, EM-R3); copying and rotating rosters (SH-R3); showing the roster to employees in the app (SH-R4); auto-detecting the shift from the first punch (SH-08); shift change and swap requests (SH-12).
- Out of scope / later release: weekly offs (SH-09) and holiday calendars (SH-10) are specified in C07 (US-C07-01, US-C07-02); C05 only holds them as settings. Extra shift pay and night-shift allowance (SP-01 to SP-07, P13, P15) are M03, version 2. How punches are read into sessions, late and early minutes and day status are C06 (PR, LA, EC, DS); C05 states only what was expected.
- Dependency notes: shifts are schedule inputs (BRD §8 Layer 2). Assigning punches to an attendance date is C06 step 2, using SH-06 and SH-07 defined here.

## Stories

### US-C05-01 Define shifts of every supported type
As an HR Manager, I want to define shifts of each supported type with their times and hours, so that attendance knows what was expected of each employee.
Source: SH-R1, SH-01, SH-02, SH-03, SH-04, SH-05, SH-11, G5, G6, G9, OR-02
Settings involved: SH-01 (shift type, default fixed), SH-02 (start and end, default 09:00 to 18:00), SH-03 (required hours per day or per week, default 8 per day), SH-04 (core hours), SH-05 (flexible band), SH-11 (half-day split point, default middle of shift)

Acceptance criteria
- AC-C05-01.1 Given HR creates a shift without changing defaults, When saved, Then it is a fixed shift 09:00 to 18:00 with 8 required hours per day and the half-day split at the middle of the shift.  (Source: SH-01, SH-02, SH-03, SH-11)
- AC-C05-01.2 Given HR chooses "flexible with core hours", When they save without core hours start and end, Then the save is refused.  (Source: SH-01, SH-04)
- AC-C05-01.3 Given HR chooses "flexible", When they set the flexible band (earliest allowed check-in, latest allowed check-out), Then the shift stores the band.  (Source: SH-05)
- AC-C05-01.4 Given HR chooses "open", When saved, Then the shift has required hours but no set start or end times.  (Source: SH-01)
- AC-C05-01.5 Given a weekly-hours employee, When HR sets required hours per week, Then the requirement is held per week, with the week starting on the organisation's OR-02 day (default Monday).  (Source: SH-03, OR-02)
- AC-C05-01.6 Given SH-11 = a fixed time of 14:00, When a half day applies on that shift, Then the second half starts at 14:00 (as in E12).  (Source: SH-11, E12)
- AC-C05-01.7 Given a shift in use is changed, When HR saves the change, Then a new version with a start date is created, the version in use is not edited, and who, when, old value, new value and reason are recorded.  (Source: G5, G9)
- AC-C05-01.8 Given a user without permission to change settings (AC-01, AC-04), When they try to create or change a shift, Then it is refused.  (Source: AC-01, AC-04)

Edge and negative cases
- How a split shift is described (two or more segments) when SH-02 holds one start and one end: OQ-C05-01.
- What marks a shift as a night shift: OQ-C05-02.

### US-C05-02 Assign the attendance date for night shifts and the shift window
As an HR Manager, I want punches of a shift that crosses midnight to belong to one attendance date chosen by our setting, so that a night worker's day is not split across two calendar dates.
Source: SH-06, SH-07, §6 Attendance date, §9.6 step 2, BR-11, P13 (V2 context); TSD §5.3 (context)
Settings involved: SH-06 (night shift attendance date: default shift start date), SH-07 (shift window: default 3 hours before, 6 after)

Acceptance criteria
- AC-C05-02.1 Given a night shift 22:00 to 06:00 and SH-06 = shift start date, When the employee punches 21:55 on 10 November and 06:05 on 11 November (branch time), Then both punches belong to attendance date 10 November.  (Source: SH-06)
- AC-C05-02.2 Given the same shift and punches and SH-06 = shift end date, When processed, Then both punches belong to attendance date 11 November.  (Source: SH-06)
- AC-C05-02.3 Given a night shift, When attendance is shown or totalled, Then it is grouped by attendance date, never by the calendar date of each punch, so the employee has one day, not two half days.  (Source: SH-06, §6)
- AC-C05-02.4 Given a fixed shift 09:00 to 18:00 and SH-07 = 3 before and 6 after, When a punch is made at 06:30 or at 23:30 that day, Then it falls inside that day's shift window.  (Source: SH-07)
- AC-C05-02.5 Given the same shift and window, When a punch is made at 05:30 or at 00:30 the next day, Then it falls outside that day's shift window.  (Source: SH-07)
- AC-C05-02.6 Given a night shift, When the attendance date is worked out, Then local times are taken in the branch's time zone, from punch times stored in UTC.  (Source: BR-11)

Edge and negative cases
- A punch exactly at a window edge (06:00 or 24:00 in AC-C05-02.4): OQ-C05-03.
- Windows of two consecutive shifts overlap (for example a night shift followed by a morning shift next day): OQ-C05-04.
- Shift window for open shifts with no set times: OQ-C05-05.
- Branch in a different time zone from the master branch: OQ-C04-03 in C04.

### US-C05-03 Assign shifts and rosters to employees
As an HR Manager, I want to assign a fixed shift or a roster to each employee, team or branch for a week or a month, so that every working day has an expected shift.
Source: SH-R2, EM-R3, EM-R6, EM-09, G3, G5, BR-03, BR-05, BR-06, PC-06, §16 (Schedule)
Settings involved: SH-01 (roster), PC-06 (punch with no shift assigned: default allow and flag)

Acceptance criteria
- AC-C05-03.1 Given employees of branch A, When HR builds a roster for a week or for a month per employee, per team or for the branch, Then each employee has a shift for each rostered date.  (Source: SH-R2)
- AC-C05-03.2 Given an employee's shift assignment changes, When saved, Then the change is dated and kept in history with who made it.  (Source: EM-R6, G9)
- AC-C05-03.3 Given a roster change for a date in an open period, When saved, Then the affected days are recalculated.  (Source: BR-03)
- AC-C05-03.4 Given a roster change for a date in a locked period, When saved, Then the locked period is unchanged and only the difference posts to the next open period tagged with the original date.  (Source: BR-06, PL-R4)
- AC-C05-03.5 Given a roster-shift employee with no shift on a date, When they punch, Then PC-06 applies (default allow and flag).  (Source: PC-06)
- AC-C05-03.6 Given an employee transfers master branch, When the transfer is saved, Then their shift is shown for review at the time of transfer.  (Source: EM-09)
- AC-C05-03.7 Given HR scoped to branch A, When they try to roster an employee of branch B, Then it is refused.  (Source: UM-R2, AC-07, AT13)
- AC-C05-03.8 Given a user of organisation X, When they try to view or change rosters of organisation Y, Then nothing is returned or changed.  (Source: TB-01, AT20)

Edge and negative cases
- More than one shift on the same date for one employee (needed later for P15 extra shift): OQ-C05-06.
- Shift assigned by both a policy-set level (G3 allows shift level) and an employee roster: OQ-C05-07.

### US-C05-04 Upload a roster by spreadsheet
As an HR Manager, I want to upload a roster from a spreadsheet, so that I can roster many employees quickly.
Source: SH-R2, EM-R7 (comparable upload), G9
Settings involved: none

Acceptance criteria
- AC-C05-04.1 Given a spreadsheet listing employees, dates and shifts that all exist in the organisation, When HR uploads it, Then the roster is applied exactly as in the file.  (Source: SH-R2)
- AC-C05-04.2 Given a row names an employee outside HR's access scope or outside the organisation, When uploaded, Then that row is not applied.  (Source: AC-07, TB-01, AT20)
- AC-C05-04.3 Given the upload changes rostered days in an open period, When applied, Then the affected days are recalculated.  (Source: BR-03)

Edge and negative cases
- Behaviour when some rows are invalid (reject whole file or apply valid rows with an error report, as EM-R7 does for employees): OQ-C05-08.

### US-C05-05 Copy a roster forward and rotate shifts on a pattern
As an HR Manager, I want to copy a roster to following weeks or months and rotate shifts on a pattern, so that I do not rebuild repeating rosters by hand.
Source: SH-R3, BR-03
Settings involved: none

Acceptance criteria
- AC-C05-05.1 Given a week's roster for a team, When HR copies it forward two weeks, Then the next two weeks have the same shift per employee per weekday.  (Source: SH-R3)
- AC-C05-05.2 Given a rotation pattern of morning, evening, night by week for three employees, When HR applies it for three weeks, Then each employee works each shift once in that order.  (Source: SH-R3)

Edge and negative cases
- Copy into dates that already have roster entries, or are holidays or weekly offs: OQ-C05-09.

### US-C05-06 See my roster in the app
As an Employee, I want to see my roster in the app, so that I know which shift I am working on each day.
Source: SH-R4, MD-02, DB-R6, AC-05
Settings involved: none

Acceptance criteria
- AC-C05-06.1 Given the plan includes the app, When an employee opens their roster, Then they see their own shifts for the published dates.  (Source: SH-R4, MD-02)
- AC-C05-06.2 Given an employee, When they open the roster, Then they cannot see other employees' rosters.  (Source: AT13)
- AC-C05-06.3 Given a Supervisor, When they open their dashboard, Then they see the roster for today and tomorrow for their team.  (Source: DB-R6)
- AC-C05-06.4 Given the plan does not include the app, When the employee signs in, Then the roster is not offered in the app.  (Source: MD-02)

Edge and negative cases
- Whether a roster must be "published" before employees see it, and whether a roster change is notified (no NT event exists): OQ-C05-10.

### US-C05-07 Detect the shift from the first punch for staff without a roster
As an HR Manager, I want the system to pick an employee's shift from their first punch when they have no roster, so that staff who come for any of several shifts are measured against the right one.
Source: SH-08, §9.6 step 2
Settings involved: SH-08 (auto-detect shift: default off)

Acceptance criteria
- AC-C05-07.1 Given SH-08 = off, When an employee without a roster punches, Then no shift is detected and the employee's assigned shift (or PC-06 when none) applies.  (Source: SH-08, PC-06)
- AC-C05-07.2 Given SH-08 = on and the employee has a roster for the date, When they punch, Then the rostered shift applies, not a detected one.  (Source: SH-08 "staff without a roster")
- AC-C05-07.3 Given SH-08 = on, When a shift is detected, Then the day's explanation shows the detected shift and that it was detected.  (Source: BR-08)

Edge and negative cases
- Which shifts are candidates and how the first punch is matched (nearest start, inside window, tie-break): OQ-C05-11.

### US-C05-08 Request a shift change or swap
As an Employee, I want to request a shift change or a swap with a colleague, and have it approved, so that I can adjust my schedule within the rules.
Source: SH-12, AC-02, BR-07, BR-03, NT-05
Settings involved: SH-12 (shift change and swap requests: default off, with approver), AC-02 (approver per request type)

Acceptance criteria
- AC-C05-08.1 Given SH-12 = off, When an employee tries to request a shift change or swap, Then the option is not available.  (Source: SH-12)
- AC-C05-08.2 Given SH-12 = on, When an employee requests a change for a date, Then it goes to the configured approver and the roster is unchanged until approved.  (Source: SH-12, AC-02)
- AC-C05-08.3 Given the approver approves, When saved, Then the roster for that date changes, the change is logged with approver and time, and the affected day is recalculated if the period is open.  (Source: SH-12, BR-03, §16)
- AC-C05-08.4 Given the approver is the requester, When they try to approve their own request, Then it is refused.  (Source: BR-07)
- AC-C05-08.5 Given NT-05 is on, When the request is submitted, approved or rejected, Then the requester is notified.  (Source: NT-05)

Edge and negative cases
- Whether a swap needs the other employee's consent and who approves when the two have different managers; what happens when the approver does not act: OQ-C05-12.

## Open questions
- OQ-C05-01 How is a split (break) shift described: two or more working segments each with start and end, given SH-02 has a single start and end? How do late, early and required hours apply to each segment? Source: SH-R1, SH-01, SH-02. Why it matters: split shifts are common in restaurants and the setting list cannot express them.
- OQ-C05-02 What makes a shift a "night shift" for SH-06: any shift whose end time is earlier than its start (crosses midnight), or an explicit marker (SP-06 later says "the shift is marked as a night shift")? Source: SH-R1, SH-06, SP-06. Why it matters: decides which shifts use the SH-06 date rule and, in V2, the night allowance.
- OQ-C05-03 Are shift-window edges inclusive (is a punch exactly 3 hours before start or exactly 6 hours after end inside the window)? Source: SH-07. Why it matters: boundary punches could move to another day.
- OQ-C05-04 When the windows of two consecutive shifts overlap (for example night shift 22:00-06:00 then a 09:00 shift the next day with 3 h before / 6 h after), which attendance date does a punch in the overlap belong to? Source: SH-07, SH-06, §9.6 step 2. Why it matters: a punch can be claimed by two days, causing a missed punch on one and an extra punch on the other.
- OQ-C05-05 For open shifts (hours only, no set times) and flexible shifts, what is the shift window and how is the attendance date decided? Source: SH-01, SH-07. Why it matters: SH-07 is defined relative to start and end times that open shifts do not have.
- OQ-C05-06 Can a roster hold more than one shift for one employee on one date in V1 (P15 and SP-02 later rely on "a second rostered shift worked the same day")? Source: SH-R2, SP-02, P15. Why it matters: affects V1 roster capability and how two shifts' punches are separated.
- OQ-C05-07 If a shift is set both by a policy-set assignment at shift level (G3) and by an employee's own shift or roster (EM-R3), which wins, and is "shift" in G3 a level at which policies are assigned rather than where shifts are assigned? Source: G3, EM-R3, BR-05. Why it matters: two sources of the expected shift can give different results.
- OQ-C05-08 On roster upload, if some rows are invalid, is the whole file rejected, or are valid rows applied with an error report (as EM-R7 does for employees)? Source: SH-R2, EM-R7. Why it matters: partial application can leave a half-built roster.
- OQ-C05-09 When copying forward or rotating into dates that already have roster entries, or that are holidays or weekly offs, are existing entries overwritten, kept, or reported? Source: SH-R3, SH-09, HD-R1. Why it matters: silent overwrite can undo manual changes.
- OQ-C05-10 Must a roster be published before employees see or are measured against it, and are employees told when their roster changes (no notification event exists in §9.14)? Source: SH-R4, §9.14. Why it matters: last-minute changes can produce late marks the employee could not foresee.
- OQ-C05-11 For SH-08 auto-detect, which shifts are candidates for an employee and how is the first punch matched to one (nearest start, inside window, tie-break)? Source: SH-08. Why it matters: a wrong match creates false late or early marks.
- OQ-C05-12 For a shift swap, must the other employee agree, who approves when the two have different approvers, and what happens if the approver does not act (no escalation setting given)? Source: SH-12, AC-02. Why it matters: a swap changes two employees' rosters at once.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| SH-R1 | US-C05-01 | AC-C05-01.1 to AC-C05-01.4 (OQ-C05-01, OQ-C05-02) |
| SH-R2 | US-C05-03, US-C05-04 | AC-C05-03.1, AC-C05-04.1 to AC-C05-04.3 |
| SH-R3 | US-C05-05 | AC-C05-05.1, AC-C05-05.2 |
| SH-R4 | US-C05-06 | AC-C05-06.1 to AC-C05-06.4 |
| SH-01 | US-C05-01 | AC-C05-01.1, AC-C05-01.2, AC-C05-01.4 |
| SH-02 | US-C05-01 | AC-C05-01.1 |
| SH-03 | US-C05-01 | AC-C05-01.1, AC-C05-01.5 |
| SH-04 | US-C05-01 | AC-C05-01.2 |
| SH-05 | US-C05-01 | AC-C05-01.3 |
| SH-06 | US-C05-02 | AC-C05-02.1 to AC-C05-02.3 |
| SH-07 | US-C05-02 | AC-C05-02.4, AC-C05-02.5 (OQ-C05-03 to OQ-C05-05) |
| SH-08 | US-C05-07 | AC-C05-07.1 to AC-C05-07.3 |
| SH-09 | C07 US-C07-02 | see C07 |
| SH-10 | C07 US-C07-01 | see C07 |
| SH-11 | US-C05-01 | AC-C05-01.1, AC-C05-01.6 |
| SH-12 | US-C05-08 | AC-C05-08.1 to AC-C05-08.5 |
| BR-03 | US-C05-03, US-C05-08 | AC-C05-03.3, AC-C05-08.3 |
| BR-06 | US-C05-03 | AC-C05-03.4 |
| BR-07 | US-C05-08 | AC-C05-08.4 |
| G5 / G9 | US-C05-01, US-C05-03 | AC-C05-01.7, AC-C05-03.2 |
| AT13 / AT20 | US-C05-03, US-C05-06 | AC-C05-03.7, AC-C05-03.8, AC-C05-06.2 |
