# C08 Leave — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §9.8 (LM-R1..LM-R9, LM-01..LM-32, LV-01..LV-04); related §6, §7, §8, §9.6 (DS-08, DS-10, LA-12), §9.12 (PL-R4), §9.14 (NT-05, NT-06, NT-11), §9.15 (OB-R2), §12, §13 (E19, P6, P7), §14 (R6, R7), §19, §21

## Scope
- In scope (V1): leave types; credit by grant or accrual; year-end carry forward and lapse; leave requests (full and half day) and approval, single and bulk; balances kept as a ledger and visible to the employee; sandwich policy; classification of absent days (informed leave, unpaid leave, uninformed absence); uninformed absence rule and excess absence rule expressed as deduction days; absconding flag and hold; leave and attendance interplay (punches on a leave day, work from home, leave debit order); leave reports R6 and R7 and leave notifications.
- Out of scope / later release:
  - Turning deduction days into money (P6, P7 rupee amounts) — Payroll M03, V2 (§21 note 1). V1 produces deduction days for the payroll input (MD-06).
  - Encashment at year end (LM-06 "encash", LM-R7) — needs Payroll (V2); see OQ-C08-05 and BRD Q2.
  - Releasing a held salary (LM-31 "hold pay") as a pay action — Payroll M03, V2; V1 records the hold only (see OQ-C08-13).
  - Leave opening balances (OB-R2, AT18) — owned by the C15 Opening balances stories; referenced here only.
  - Comp-off earning and expiry — M02 stories (C08 only shows the comp-off balance next to leave, LM-R8).
  - Offboarding started from LM-32 — C11, V3; until then HR starts it directly (§21 note 4).

## Stories

### US-C08-01 Define leave types
As an HR Manager, I want to define my organisation's leave types as paid or unpaid with eligibility rules, so that each employee gets only the leave types that apply to them.
Source: LM-R1, LM-01, G5, G9, BR-14
Settings involved: LM-01 (no default stated)

Acceptance criteria
- AC-C08-01.1 Given the HR Manager creates a leave type with a name, paid or unpaid, and eligibility by category, employee type, payroll group or after probation, When it is saved, Then the type is available only to employees who meet the eligibility.  (Source: LM-R1, LM-01)
- AC-C08-01.2 Given an employee does not meet a leave type's eligibility, When the employee opens the leave request, Then that type is not offered and a request for it cannot be submitted.  (Source: LM-01)
- AC-C08-01.3 Given a leave type is in use, When the HR Manager changes it, Then a new version with a start date is created, the earlier version is unchanged, and the change records who, when, old value, new value and reason.  (Source: G5, G9)
- AC-C08-01.4 Given a user with no settings right (AC-04), When the user tries to create or change a leave type, Then the action is refused.  (Source: AC-04, N5)
- AC-C08-01.5 Given two organisations, When an HR Manager of organisation A lists leave types, Then no leave type of organisation B is visible or changeable through any screen, report or export.  (Source: N1, AT20)

Edge and negative cases
- Eligibility "by payroll group" while payroll groups are a V2 item — OQ-C08-01.
- Employee moves out of an eligible category while holding a balance — OQ-C08-02.

### US-C08-02 Credit leave by grant or accrual
As an HR Manager, I want leave credited automatically by the method I set per leave type, so that balances are correct without manual entry.
Source: LM-R2, LM-02, LM-03, LM-04, LM-05, §16 (balances and ledgers), AT21
Settings involved: LM-02 (default monthly accrual), LM-03 (default end of period), LM-04 (default pro-rated), LM-05 (default no cap)

Acceptance criteria
- AC-C08-02.1 Given a leave type with LM-02 = monthly accrual and LM-03 = end, When a month ends, Then the month's accrual is added to each eligible employee's balance as a ledger entry dated at the end of that month.  (Source: LM-02, LM-03)
- AC-C08-02.2 Given LM-03 = start, When a period starts, Then the accrual for that period is credited at its start.  (Source: LM-03)
- AC-C08-02.3 Given LM-02 = fixed grant at start of year, When the leave year starts, Then the full grant is credited once as a ledger entry.  (Source: LM-02)
- AC-C08-02.4 Given LM-02 = quarterly accrual, When a quarter ends (LM-03 = end), Then the quarter's accrual is credited once.  (Source: LM-02, LM-03)
- AC-C08-02.5 Given LM-02 = earned by days worked at 1 day per 20 days worked, When an employee's worked-day count reaches 20, Then 1 day is credited.  (Source: LM-02)
- AC-C08-02.6 Given LM-04 = none and an employee joins mid-period, When that period's credit runs, Then the employee receives no credit for that period; given LM-04 = full, Then the employee receives the full period credit.  (Source: LM-04)
- AC-C08-02.7 Given LM-05 sets a maximum balance and a credit would take the balance above it, When the credit runs, Then the balance stops at the maximum.  (Source: LM-05)
- AC-C08-02.8 Given any credit, When it is posted, Then it is a new ledger entry recording leave type, days, reason (grant, accrual, pro-rated credit), date and the rule version used; existing entries are never overwritten.  (Source: LM-R2, §16, BR-08)
- AC-C08-02.9 Given the credit job for a date has already run, When it runs again for the same date, Then no additional credit is posted.  (Source: AT21)
- AC-C08-02.10 Given an employee's leave opening balance was confirmed with an as-of date, When credits run, Then they start from that as-of date and build on the opening balance.  (Source: OB-R4, AT18 — owned by C15)

Edge and negative cases
- Pro-rating formula for LM-04 = pro-rated — OQ-C08-03.
- What counts as a "day worked" for LM-02 earned-by-days-worked — OQ-C08-04.
- Whether a credit skipped by the LM-05 cap is recorded in the ledger as lapsed — OQ-C08-03 (asked together).

### US-C08-03 Year-end carry forward and lapse
As an HR Manager, I want year-end processing to carry forward or lapse leave as configured, so that balances start the new year correctly.
Source: LM-R7, LM-06, NT-11
Settings involved: LM-06 (default lapse), NT-11

Acceptance criteria
- AC-C08-03.1 Given LM-06 = lapse, When year-end processing runs, Then the remaining balance of that type is removed by a lapse ledger entry and the new year starts at zero before new credits.  (Source: LM-R7, LM-06)
- AC-C08-03.2 Given LM-06 = carry forward up to N days, When year-end processing runs with a balance above N, Then N days are carried forward and the rest lapses, each as its own ledger entry.  (Source: LM-06)
- AC-C08-03.3 Given LM-06 = carry forward up to N days and a balance at or below N, When year-end processing runs, Then the whole balance is carried forward and nothing lapses.  (Source: LM-06)
- AC-C08-03.4 Given NT-11 is on, When leave is about to lapse, Then the employee is notified on the configured channels.  (Source: NT-11)
- AC-C08-03.5 Given year-end processing has run, When it is run again for the same year, Then balances do not change.  (Source: AT21)

Edge and negative cases
- LM-06 = encash in V1 — OQ-C08-05.
- Leave year start date (calendar year or other) is not defined — OQ-C08-06.

### US-C08-04 Apply for leave
As an Employee, I want to apply for full-day or half-day leave from the web or the app, with the rules checked at once, so that I know immediately whether my request can go forward.
Source: LM-R3, LM-07, LM-08, LM-09, LM-10, LM-11, LM-12, LM-14, LM-15, AC-02
Settings involved: LM-07 (default no), LM-08 (default on), LM-09 (default 0), LM-10 (default no cap), LM-11 (default within 3 days), LM-12 (default off), LM-14 (default allow), LM-15 (default allow), SH-11 (default middle of shift)

Acceptance criteria
- AC-C08-04.1 Given an eligible employee with sufficient balance, When the employee submits a full-day leave request from the web or the app, Then the request is recorded as pending with the approver set by AC-02, and the balance does not change until approval.  (Source: LM-R3, LM-R4, AC-02)
- AC-C08-04.2 Given LM-08 = on for the type, When the employee requests a half day, Then the request is accepted as 0.5 day; given LM-08 = off, Then a half-day request for that type is refused.  (Source: LM-08)
- AC-C08-04.3 Given LM-09 = N days for the type, When the request is made fewer than N days before the first leave day, Then it is refused with the reason shown.  (Source: LM-09)
- AC-C08-04.4 Given LM-10 = N, When a request covers more than N consecutive days, Then it is refused.  (Source: LM-10)
- AC-C08-04.5 Given LM-11 = within 3 days, When a request is made for a past date 3 days or less after it, Then it is accepted; When it is made 4 days after, Then it is refused. Given LM-11 = not allowed, Then any request for a past date is refused.  (Source: LM-11)
- AC-C08-04.6 Given LM-12 = on with N days, When a request is longer than N days and has no supporting document, Then it cannot be submitted.  (Source: LM-12)
- AC-C08-04.7 Given LM-07 = no, When the request would take the balance below zero, Then it is refused; given LM-07 = up to N days, Then it is accepted only while the resulting balance is not below minus N.  (Source: LM-07)
- AC-C08-04.8 Given LM-14 = block, When one continuous stretch combines two leave types, Then the request is refused; given LM-14 = allow, Then it is accepted.  (Source: LM-14)
- AC-C08-04.9 Given an employee is serving a notice period and LM-15 = block, When the employee requests leave inside it, Then the request is refused; given LM-15 = allow, Then it is accepted.  (Source: LM-15)
- AC-C08-04.10 Given an employee, When the employee tries to request leave for another employee, Then the request is refused.  (Source: N5, AC-01)

Edge and negative cases
- LM-15 = extends the notice period: by how many days and calendar or working — OQ-C08-07.
- Whether pending requests count against the balance when checking LM-07 for a second request — OQ-C08-08.
- Request window for a leave date in a locked period — covered by AC-C08-12.4 (lock rule BR-06).

### US-C08-05 Approve or reject leave, singly or in bulk
As a Manager (Approver), I want to approve or reject leave requests one at a time or in bulk, with escalation when I do not act, so that requests are never stuck.
Source: LM-R3, LM-13, BR-07, NT-05, NT-06, §16 (audit log), AC-03
Settings involved: LM-13 (default manager, escalate after 2 days), AC-02, AC-03, NT-05, NT-06

Acceptance criteria
- AC-C08-05.1 Given a pending request and LM-13 = manager, When the manager approves it, Then it becomes approved, the balance is debited by a ledger entry at that moment, and the approval records the actor and the time.  (Source: LM-R3, LM-R4, LM-13, §16)
- AC-C08-05.2 Given LM-13 = manager then HR, When the manager approves, Then the request stays pending with HR and the balance is not debited until HR approves.  (Source: LM-13)
- AC-C08-05.3 Given a pending request, When the approver rejects it, Then it becomes rejected, no ledger entry is posted, and the rejection records actor, time and reason.  (Source: LM-R3, §16)
- AC-C08-05.4 Given several pending requests selected together, When the approver approves them in bulk, Then each request is approved and audited individually, with the same actor and time.  (Source: LM-R3, §16)
- AC-C08-05.5 Given a request raised by a user who is also an approver, When that user tries to approve or reject it (singly or inside a bulk action), Then the action is refused for that request.  (Source: BR-07)
- AC-C08-05.6 Given LM-13 escalation after 2 days, When the approver has not acted 2 days after submission, Then the request is escalated and NT-05 notifies the escalation; given auto-approve or auto-reject, Then the request is approved or rejected automatically and recorded as done by the system, with the time.  (Source: LM-13, NT-05)
- AC-C08-05.7 Given AC-03 = on and the approver has delegated, When the delegate acts, Then the action is recorded against the delegate.  (Source: AC-03, §16)
- AC-C08-05.8 Given a manager whose access scope does not include the employee (AC-07), When the manager opens the request, Then it is not visible and cannot be acted on.  (Source: AC-07, AT13)
- AC-C08-05.9 Given NT-06 is on, When the day's digest is sent, Then the manager receives the list of pending leave requests.  (Source: NT-06)
- AC-C08-05.10 Given an approval changes a day in an open period, When it is saved, Then the affected days and period totals are recalculated.  (Source: BR-03, §8)

Edge and negative cases
- Whether the LM-13 escalation timer counts non-working days — OQ-C08-09.
- Who receives an escalation (next level, HR, department head) — OQ-C08-09 (asked together).
- Auto-approve when balance has meanwhile become insufficient — OQ-C08-08.

### US-C08-06 See leave balances and the ledger
As an Employee, I want to see my leave balances, including comp-off when that module is on, updated the moment a request is approved or cancelled, so that I always know what I have left.
Source: LM-R4, LM-R8, DB-R7, R6, §16, MD-03
Settings involved: none in C08

Acceptance criteria
- AC-C08-06.1 Given a request is approved, When the employee opens the balance, Then the debit is already shown.  (Source: LM-R4)
- AC-C08-06.2 Given an approved request is cancelled, When the employee opens the balance, Then the days are back, shown as a separate credit ledger entry; the original debit is not deleted.  (Source: LM-R4, §16)
- AC-C08-06.3 Given M02 Comp-off is enabled for the organisation, When the employee views balances, Then the comp-off balance appears alongside the leave types; given M02 is disabled, Then no comp-off balance appears.  (Source: LM-R8, MD-03)
- AC-C08-06.4 Given an employee, When the employee views the ledger, Then each entry shows date, type, days, reason and, for request entries, the request it belongs to.  (Source: §16, BR-08)
- AC-C08-06.5 Given an HR Manager limited to branch A, When the HR Manager runs report R6, Then only employees in branch A are included, and the report exports to spreadsheet.  (Source: R6, §14, UM-R2)
- AC-C08-06.6 Given an employee, When the employee tries to view another employee's balance, Then access is refused.  (Source: N5, AT13)

Edge and negative cases
- Balance shown while a request is pending (available vs booked) — OQ-C08-08.

### US-C08-07 Cancel approved or pending leave
As an Employee, I want to cancel a leave request, so that my balance returns when I no longer need the leave.
Source: LM-R4, BR-03, BR-06
Settings involved: none stated

Acceptance criteria
- AC-C08-07.1 Given an approved leave request is cancelled, When the cancellation takes effect, Then the balance is restored by a credit ledger entry and the affected days are recalculated while the period is open.  (Source: LM-R4, BR-03)
- AC-C08-07.2 Given the leave date is in a locked period, When the cancellation is made, Then the locked period is unchanged and only the difference is posted to the next open period, tagged with the original date.  (Source: BR-06, PL-R4)
- AC-C08-07.3 Given any cancellation, When it is saved, Then it records actor, time and reason in the audit log.  (Source: §16)

Edge and negative cases
- Who may cancel, until when, and whether cancelling approved leave needs approval — OQ-C08-10.

### US-C08-08 Apply the sandwich policy
As an HR Manager, I want weekly offs and holidays that fall between leave or absent days counted as leave or absence when the sandwich policy is on, so that the organisation's policy is applied the same way for everyone.
Source: LM-R5, LM-16, LM-17, LM-18, LM-19, LM-20, DS-08, §6 (sandwich policy), R7, BR-03
Settings involved: LM-16 (default off), LM-17 (default either), LM-18 (default off), LM-19 (default all), LM-20 (default same leave type)

Acceptance criteria
- AC-C08-08.1 Given LM-16 = off, When a weekly off falls between two leave days, Then the weekly off stays a weekly off.  (Source: LM-16, G2)
- AC-C08-08.2 Given LM-16 = on, LM-17 = leave, LM-20 = same leave type, and a weekly off between two days of approved casual leave, When the period is calculated, Then the weekly off is counted as casual leave and debited from the casual leave balance by a ledger entry.  (Source: LM-R5, LM-17, LM-20)
- AC-C08-08.3 Given LM-16 = on, LM-17 = uninformed absence, When a holiday falls between two uninformed absent days, Then the holiday is counted as absence; given LM-17 = leave, Then the same holiday is not sandwiched.  (Source: LM-17)
- AC-C08-08.4 Given LM-16 = on, LM-17 = either, When a weekly off falls between a leave day and an uninformed absent day, Then the weekly off is sandwiched.  (Source: LM-17)
- AC-C08-08.5 Given LM-16 = on and LM-18 = off, When leave is on one side of a weekly off only, Then the weekly off is not sandwiched; given LM-18 = on, Then it is sandwiched.  (Source: LM-18)
- AC-C08-08.6 Given LM-19 excludes a leave type, When that type is on both sides, Then the day between is not sandwiched.  (Source: LM-19)
- AC-C08-08.7 Given LM-20 = unpaid, When a day is sandwiched, Then it is counted as an unpaid day (a deduction day in the payroll input) and no leave balance is debited.  (Source: LM-20, MD-06)
- AC-C08-08.8 Given a sandwiched day, When a side day changes (leave cancelled, absence regularised to present), Then the sandwiched day is recalculated and any balance debit is reversed by a ledger entry while the period is open.  (Source: BR-03, §16)
- AC-C08-08.9 Given sandwiching is applied, When report R7 is run, Then the sandwich days applied are listed per employee.  (Source: R7)

Edge and negative cases
- LM-17 = uninformed absence vs LM-18 "leave on one side only": does LM-18 also cover an uninformed absent day on one side — OQ-C08-11.
- LM-20 = same leave type when the two sides are different types, or one side is an uninformed absence — OQ-C08-11 (asked together).
- Sandwiched day debited when the balance is insufficient and LM-07 = no — OQ-C08-11 (asked together).
- Several weekly offs or holidays in a row between the sides — covered by "days between" in §6; each counts (AC-C08-08.2 applies to each).

### US-C08-09 Classify absent days
As an HR Manager, I want every absent day classified as informed leave, unpaid leave or uninformed absence, and absent days without leave handled as configured, so that deduction days and reports are right.
Source: LM-R6, LM-28, LV-04, DS-10, §6 (informed leave, uninformed absence, deduction days)
Settings involved: LM-28 (default loss of pay), LV-04 (no default stated), DS-10 (default pending for 3 days, then loss of pay)

Acceptance criteria
- AC-C08-09.1 Given an absent day covered by an approved paid leave request (approved before or after the day), When the day is classified, Then it is informed leave.  (Source: LM-R6, §6)
- AC-C08-09.2 Given an absent day covered by an approved unpaid leave type, When the day is classified, Then it is unpaid leave and counts as a deduction day.  (Source: LM-R6, LM-R1, §6)
- AC-C08-09.3 Given an absent day with no approved leave request, When the day is classified, Then it is an uninformed absence.  (Source: LM-R6, §6)
- AC-C08-09.4 Given LM-28 = loss of pay, When an absent day is not covered by leave, Then it is one loss-of-pay day in the payroll input.  (Source: LM-28, MD-06)
- AC-C08-09.5 Given LM-28 = auto-debit with LV-04 order casual leave then earned leave, When an absent day is not covered by leave and casual leave balance is 0 and earned leave balance is 1, Then 1 day of earned leave is debited by a ledger entry marked auto-debit.  (Source: LM-28, LV-04)
- AC-C08-09.6 Given LM-28 = auto-debit and no listed type has balance, When the day is processed, Then it is loss of pay.  (Source: LM-28, §6 LOP)
- AC-C08-09.7 Given any classification, When report R7 or the payroll input is produced, Then each day shows its classification and the rule that produced it.  (Source: BR-08, R7)

Edge and negative cases
- DS-10 (pending N days / auto-debit / LOP) and LM-28 (LOP / auto-debit) cover the same event with different defaults — OQ-C08-12.
- Whether an auto-debited day is informed leave or stays uninformed for LM-21, LM-26 and LM-30 — OQ-C08-12 (asked together).

### US-C08-10 Apply the uninformed absence rule
As an HR Manager, I want each uninformed absent day to cut a configured number of days, reversed if leave is approved later, so that the organisation's absence policy is applied without manual work.
Source: LM-21, LM-22, LM-23, LM-27, BR-04, P6 (day counts only), K1, R7
Settings involved: LM-21 (default off), LM-22 (default 2), LM-23 (default yes)

Acceptance criteria
- AC-C08-10.1 Given LM-21 = off, When an employee has an uninformed absent day, Then only the day itself is a deduction day (LM-28 applies).  (Source: LM-21, G2)
- AC-C08-10.2 (P6, day counts) Given LM-21 = on and LM-22 = 2, When an employee has one absent day with no approved leave, Then the deduction days for that day are 2: the day itself and one extra day.  (Source: LM-22, P6)
- AC-C08-10.3 (P6, reversal) Given AC-C08-10.2 and LM-23 = yes, When a leave request for that day is approved later, Then the extra day is reversed, and the day itself follows the approved leave.  (Source: LM-23, P6, LM-27)
- AC-C08-10.4 Given LM-23 = no, When a leave request for that day is approved later, Then the extra day stays cut, and the day moves from the uninformed count to the informed count.  (Source: LM-23, LM-27)
- AC-C08-10.5 Given the extra cut, When the payroll input and report R7 are produced, Then the extra day is shown as its own line, separate from the loss-of-pay day, with the rule and version.  (Source: §6 deduction days, BR-08, R7)
- AC-C08-10.6 Given a sector template that switches LM-21 on, When an organisation is created from it, Then LM-21 is on for that organisation, and the HR Manager can switch it off.  (Source: LM-21, G6)

Edge and negative cases
- LM-21 together with LM-26 on the same day — covered by US-C08-11 (AC-C08-11.6).
- Label of K1 (labour-law review before switching on) — a vendor/HR process, not system behaviour; no AC.

### US-C08-11 Apply the excess absence rule, informed and uninformed parts
As an HR Manager, I want approved leave and uninformed absence beyond a monthly threshold to cut extra days, counted and reported separately, so that excess absence is handled consistently.
Source: LM-24, LM-25, LM-26, LM-27, BR-04, P7 (day counts only), R7, AT6
Settings involved: LM-24 (default off), LM-25 (default off; X = 3, Y = 1 for each day beyond X; all leave types, comp-off excluded), LM-26 (default off; X = 3, Y = 1 for each day beyond X)

Acceptance criteria
- AC-C08-11.1 (P7, day counts) Given LM-25 on with X = 3, Y = 1 for each day beyond X; LM-26 on with X = 2, Y = 1 for each day beyond X; LM-21 off; and in the month 5 approved paid leave days and 3 uninformed absent days, When the month is calculated, Then: informed part = 2 deduction days; loss of pay = 3 days; uninformed part = 1 deduction day; total = 6 deduction days, shown as three separate lines: excess informed leave (2), loss of pay (3), excess uninformed absence (1).  (Source: LM-25, LM-26, LM-27, P7)
- AC-C08-11.2 Given LM-25 with Y as one flat cut, When the count passes X by any number of days, Then Y days are cut once.  (Source: LM-25)
- AC-C08-11.3 Given LM-25 with 2 or 3 slabs, When the count falls in a slab, Then that slab's X and Y apply.  (Source: LM-25)
- AC-C08-11.4 Given LM-25 default leave types, When an employee takes comp-off days in the month, Then those days are not counted in the informed part.  (Source: LM-25)
- AC-C08-11.5 Given an uninformed day whose leave is later approved, When the month is recalculated, Then the day leaves the uninformed count and joins the informed count, never both.  (Source: LM-27)
- AC-C08-11.6 Given LM-21 and LM-26 both on, When an uninformed day would be cut under both, Then it is cut under one rule only and the larger total applies; the deduction days are never the sum of both.  (Source: LM-27, BR-04, AT6)
- AC-C08-11.7 Given LM-24 = on and LM-26 = off, When the month has uninformed days beyond X, Then no excess uninformed cut is made.  (Source: LM-24, G2)
- AC-C08-11.8 Given the excess absence cut, When it is produced, Then it is applied after the month's days are known and recalculated when any counted day changes while the period is open.  (Source: LM-25, LM-26, BR-03)

Edge and negative cases
- "Larger total applies" in LM-27: compared per day or over the month — OQ-C08-14.
- "In the month" when the attendance period is a custom cycle (PL-01, for example 26th to 25th) — OQ-C08-15.
- Whether half-day leave counts as 0.5 toward X — OQ-C08-15 (asked together).

### US-C08-12 Leave approved after the day
As a Manager (Approver), I want leave approved after the event to change the day from uninformed absence to informed leave and undo the related cuts, so that the employee is treated as if the leave had been approved on time, as the settings allow.
Source: LM-11, LM-23, LM-27, §6 (informed leave), BR-03, BR-06, PL-R4, PL-07
Settings involved: LM-11 (default within 3 days), LM-23 (default yes), PL-07 (default 3)

Acceptance criteria
- AC-C08-12.1 Given an uninformed absent day in an open period, When a leave request for it is approved after the day, Then the day becomes informed leave, the leave balance is debited by a ledger entry, and the day and period totals are recalculated.  (Source: §6, LM-27, BR-03)
- AC-C08-12.2 Given LM-23 = yes and an LM-21 extra cut on that day, When the leave is approved, Then the extra cut is reversed.  (Source: LM-23, P6)
- AC-C08-12.3 Given LM-24 on, When the leave is approved, Then the day moves from the LM-26 count to the LM-25 count for that month.  (Source: LM-27)
- AC-C08-12.4 Given the day is in a locked period, When the leave is approved, Then the locked period is unchanged and only the difference is posted to the next open period, tagged with the original date.  (Source: BR-06, PL-R4)

Edge and negative cases
- Approval after lock beyond PL-07 past periods — handled by C12 (PL-07); not repeated here.

### US-C08-13 Flag absconding
As an HR Manager, I want an employee flagged as absconding after a set number of continuous uninformed absent days, with an alert and a pay hold, so that I can decide whether to bring them back or start offboarding.
Source: LM-R9, LM-29, LM-30, LM-31, LM-32, E19, §6 (absconding), Q9
Settings involved: LM-29 (default off), LM-30 (default 7), LM-31 (default hold pay on)

Acceptance criteria
- AC-C08-13.1 (E19) Given LM-29 = on and LM-30 = 7, When an employee has 7 continuous uninformed absent days, Then the employee is flagged as absconding, HR is alerted, and pay is held until HR returns the employee to active or starts offboarding.  (Source: LM-30, LM-31, LM-32, E19)
- AC-C08-13.2 Given LM-29 = on and LM-30 = 7, When an employee has 6 continuous uninformed absent days and then a present day, Then no absconding flag is raised.  (Source: LM-30)
- AC-C08-13.3 Given LM-29 = off, When an employee has 7 continuous uninformed absent days, Then no flag is raised.  (Source: LM-29, G2)
- AC-C08-13.4 Given LM-31 hold pay = off, When the trigger is reached, Then the employee is flagged and HR alerted, but no hold is set.  (Source: LM-31)
- AC-C08-13.5 Given an employee flagged as absconding, When HR returns the employee to active, Then the flag is cleared and the hold released, recording actor, time and reason.  (Source: LM-32, §16)
- AC-C08-13.6 Given an employee flagged as absconding, When HR starts offboarding, Then the employee's status records that offboarding was started, with actor and time.  (Source: LM-32, §16, §21 note 4)

Edge and negative cases
- Leave approved afterwards for some of the counted days after the flag is raised — OQ-C08-16.
- LM-30 working days or calendar days; do weekly offs and holidays break or extend the run — OQ-C08-17 (BRD Q9).
- Meaning of "hold pay" in V1 while Payroll is off — OQ-C08-13.

### US-C08-14 Punches on a leave day and work from home
As an HR Manager, I want punches on a day of approved leave and work-from-home days handled as configured, so that attendance and leave never contradict each other.
Source: LV-01, LV-03, PC-09, PC-17, BR-01
Settings involved: LV-01 (default flag), LV-03 (default on, web punch), PC-09 (default only on approved days)

Acceptance criteria
- AC-C08-14.1 Given LV-01 = ignore, When an employee punches on a day of approved full-day leave, Then the punches are stored and the day stays leave.  (Source: LV-01, BR-01)
- AC-C08-14.2 Given LV-01 = flag, When an employee punches on a day of approved full-day leave, Then the day is flagged for leave cancellation and the leave stays until someone acts.  (Source: LV-01)
- AC-C08-14.3 Given LV-01 = auto-cancel when full hours are worked, When the punches give the full required hours, Then the leave is cancelled, the balance is restored by a credit ledger entry and the day becomes present; When the hours are short of full, Then the leave stays.  (Source: LV-01, LM-R4)
- AC-C08-14.4 Given LV-03 = on with web punch, When an employee works from home on an approved WFH day, Then attendance comes from the web punch; given LV-03 = present by default, Then the day is present without punches.  (Source: LV-03, PC-09)
- AC-C08-14.5 Given LV-03 = off, When an employee tries a work-from-home punch, Then it is not accepted as WFH.  (Source: LV-03)

Edge and negative cases
- LV-01 for half-day leave days — OQ-C08-18.
- How a WFH day is requested and approved (no request flow in the BRD) — OQ-C08-18 (asked together).
- Who acts on an LV-01 flag — OQ-C08-18 (asked together).

### US-C08-15 Debit leave for penalties in a set order
As an HR Manager, I want penalties that are recovered from leave to use the leave debit order I set, so that the right balances are used first.
Source: LV-04, LA-12, EC-11, E5
Settings involved: LV-04 (list, no default stated), LA-12 (default casual leave, then loss of pay)

Acceptance criteria
- AC-C08-15.1 (E5, leave part) Given LA-06 on with every 3rd late mark = half day, LA-12 = casual leave then loss of pay, and 4 late marks in the period, When the period-end penalty is applied, Then one penalty of half a day is applied at the 3rd mark and 0.5 day is debited from casual leave by a ledger entry; if the casual leave balance is zero, Then 0.5 day loss of pay instead.  (Source: E5, LA-12, LV-04, BR-10)
- AC-C08-15.2 Given a penalty debit from leave, When the ledger is viewed, Then the entry names the penalty and the late or early marks that produced it.  (Source: BR-08, §16)

Edge and negative cases
- Relationship between LV-04 and the leave types listed in LA-12 / EC-11 when they differ — OQ-C08-19.

## Open questions
- OQ-C08-01 Leave-type eligibility "by payroll group" (LM-01): payroll groups arrive with Payroll in V2 (G4, M03). Is this criterion hidden in V1, or do payroll groups exist in V1 as a label only? Source: LM-01, G4, §21. Why it matters: decides whether an eligibility option is usable in V1.
- OQ-C08-02 When an employee stops meeting a leave type's eligibility (category change, probation ends), is the existing balance kept, frozen or lapsed? Source: LM-01. Why it matters: balances could be lost or kept without a rule.
- OQ-C08-03 LM-04 pro-rating formula: by calendar days, working days, or whole months in service; how are fractions rounded (to 0.5 day)? And when LM-05 caps a credit, is the uncredited amount recorded in the ledger? Source: LM-04, LM-05. Why it matters: joiners' and leavers' balances differ by formula; audit needs to show capped credits.
- OQ-C08-04 LM-02 "earned by days worked": what counts as a worked day — half days (0.5?), on-duty days (PM-R6 says worked), paid holidays, weekly offs, permission days, leave days? Does the counter carry over the remainder (for example 25 days = 1 credit + 5 carried)? Source: LM-02, PM-R6. Why it matters: changes the credit for every employee on that method.
- OQ-C08-05 LM-06 = encash in V1, when Payroll is V2: is the option hidden in V1, or are encashable days exported in the payroll input (MD-06)? Source: LM-06, LM-R7, Q2, §21. Why it matters: a V1 organisation could pick an option with no effect.
- OQ-C08-06 Leave year: calendar year, financial year (April to March), or set per organisation? Joining-date anniversary? Source: LM-02, LM-06, LM-R7. Why it matters: decides when year-end processing and fixed grants run.
- OQ-C08-07 LM-15 = extends the notice period: by the number of leave days, calendar or working? Is the employee master notice end date updated automatically? Source: LM-15, C02. Why it matters: changes the exit date.
- OQ-C08-08 Do pending requests reserve balance (so a second request is checked against balance minus pending)? What does an auto-approve (LM-13) do if the balance is no longer enough? Source: LM-07, LM-13, LM-R4. Why it matters: prevents balances going negative unnoticed.
- OQ-C08-09 LM-13 escalation after N days: are non-working days, weekly offs and holidays counted, and to whom does it escalate (next level, HR, department head)? Source: LM-13, AC-02. Why it matters: decides when and to whom requests move.
- OQ-C08-10 Leave cancellation: who may cancel (employee, approver, HR), until when (before the leave starts, before lock), and does cancelling approved leave need approval? Source: LM-R4 (PM-29 covers permissions only). Why it matters: no cancellation rule exists for leave.
- OQ-C08-11 Sandwich policy: (a) with LM-17 = uninformed absence or either, does LM-18 (one side only) also apply when an uninformed absent day is on one side, or only when leave is? (b) LM-20 = same leave type when the two sides are different leave types or one side is uninformed absence — which type is debited? (c) if the type's balance is insufficient and LM-07 = no, is the day unpaid? Source: LM-17, LM-18, LM-20, LM-07. Why it matters: these cases occur every month and have no stated result.
- OQ-C08-12 DS-10 (absent with no leave: LOP / auto-debit / pending N days, default pending 3 days then LOP) and LM-28 (LOP / auto-debit, default LOP) govern the same event with different defaults. Which wins, or is DS-10's "pending" a waiting period before LM-28 applies? Is an auto-debited day informed leave or still uninformed for LM-21, LM-26 and LM-30? Source: DS-10, LM-28, LM-21, LM-26, LM-30. Why it matters: decides deduction days and absconding counts.
- OQ-C08-13 LM-31 "hold pay" in V1 when Payroll is V2: is the hold a flag on the employee in the payroll input export (MD-06) and dashboards, and who sees it? Source: LM-31, MD-06, §21. Why it matters: V1 cannot hold pay; the export consumer must know.
- OQ-C08-14 LM-27 "the larger total applies" when LM-21 and LM-26 both cut an uninformed day: compared per day, or the month's LM-21 total against the month's LM-26 total? Source: LM-27, BR-04. Why it matters: gives different deduction days with the same data.
- OQ-C08-15 Excess absence counts "in the month" (LM-25, LM-26): calendar month, or the attendance period when it is a custom cycle (PL-01)? Do half-day leave and half-day absence count as 0.5? Source: LM-25, LM-26, PL-01. Why it matters: changes when the threshold is passed.
- OQ-C08-16 After an absconding flag, if leave is approved later for some counted days, is the flag cleared automatically or does it stay until HR decides under LM-32? Source: LM-32, LM-27. Why it matters: affects the pay hold.
- OQ-C08-17 LM-30 / Q9: are the N continuous days working days or calendar days, and does a weekly off or holiday in the run break it, extend it, or count? Is 7 the right default? Source: LM-30, Q9. Why it matters: decides who gets flagged.
- OQ-C08-18 LV-01 for half-day leave days; who acts on an LV-01 flag; and how is a WFH day (PC-09 "approved WFH days", LV-03) requested and approved? Source: LV-01, LV-03, PC-09. Why it matters: no request flow or owner is defined.
- OQ-C08-19 LV-04 (leave debit order for absences and penalties) and LA-12 / EC-11 (penalty recovery order with "chosen leave types in order") both define an order for penalties. Which applies to penalties? Source: LV-04, LA-12, EC-11. Why it matters: two settings for one behaviour can conflict.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| LM-R1 | US-C08-01 | AC-C08-01.1 |
| LM-R2 | US-C08-02 | AC-C08-02.1–02.8 |
| LM-R3 | US-C08-04, US-C08-05 | AC-C08-04.1, AC-C08-05.1–05.4 |
| LM-R4 | US-C08-05, US-C08-06, US-C08-07 | AC-C08-05.1, AC-C08-06.1, 06.2, AC-C08-07.1 |
| LM-R5 | US-C08-08 | AC-C08-08.2 |
| LM-R6 | US-C08-09 | AC-C08-09.1–09.3 |
| LM-R7 | US-C08-03 | AC-C08-03.1–03.3; OQ-C08-05 |
| LM-R8 | US-C08-06 | AC-C08-06.3 |
| LM-R9 | US-C08-13 | AC-C08-13.1 |
| LM-01 | US-C08-01 | AC-C08-01.1, 01.2; OQ-C08-01, 02 |
| LM-02 | US-C08-02 | AC-C08-02.1, 02.3–02.5; OQ-C08-04 |
| LM-03 | US-C08-02 | AC-C08-02.1, 02.2 |
| LM-04 | US-C08-02 | AC-C08-02.6; OQ-C08-03 |
| LM-05 | US-C08-02 | AC-C08-02.7; OQ-C08-03 |
| LM-06 | US-C08-03 | AC-C08-03.1–03.3; OQ-C08-05 |
| LM-07 | US-C08-04 | AC-C08-04.7; OQ-C08-08 |
| LM-08 | US-C08-04 | AC-C08-04.2 |
| LM-09 | US-C08-04 | AC-C08-04.3 |
| LM-10 | US-C08-04 | AC-C08-04.4 |
| LM-11 | US-C08-04, US-C08-12 | AC-C08-04.5 |
| LM-12 | US-C08-04 | AC-C08-04.6 |
| LM-13 | US-C08-05 | AC-C08-05.1, 05.2, 05.6; OQ-C08-09 |
| LM-14 | US-C08-04 | AC-C08-04.8 |
| LM-15 | US-C08-04 | AC-C08-04.9; OQ-C08-07 |
| LM-16 | US-C08-08 | AC-C08-08.1 |
| LM-17 | US-C08-08 | AC-C08-08.2–08.4; OQ-C08-11 |
| LM-18 | US-C08-08 | AC-C08-08.5; OQ-C08-11 |
| LM-19 | US-C08-08 | AC-C08-08.6 |
| LM-20 | US-C08-08 | AC-C08-08.2, 08.7; OQ-C08-11 |
| LM-21 | US-C08-10, US-C08-11 | AC-C08-10.1, 10.6, AC-C08-11.6 |
| LM-22 | US-C08-10 | AC-C08-10.2 |
| LM-23 | US-C08-10, US-C08-12 | AC-C08-10.3, 10.4, AC-C08-12.2 |
| LM-24 | US-C08-11 | AC-C08-11.7 |
| LM-25 | US-C08-11 | AC-C08-11.1–11.4, 11.8; OQ-C08-15 |
| LM-26 | US-C08-11 | AC-C08-11.1, 11.8; OQ-C08-15 |
| LM-27 | US-C08-10, US-C08-11, US-C08-12 | AC-C08-10.3, 10.4, AC-C08-11.1, 11.5, 11.6, AC-C08-12.1, 12.3; OQ-C08-14 |
| LM-28 | US-C08-09 | AC-C08-09.4–09.6; OQ-C08-12 |
| LM-29 | US-C08-13 | AC-C08-13.3 |
| LM-30 | US-C08-13 | AC-C08-13.1, 13.2; OQ-C08-17 |
| LM-31 | US-C08-13 | AC-C08-13.1, 13.4; OQ-C08-13 |
| LM-32 | US-C08-13 | AC-C08-13.5, 13.6; OQ-C08-16 |
| LV-01 | US-C08-14 | AC-C08-14.1–14.3; OQ-C08-18 |
| LV-02 | C09 (OD) | see C09-permissions-on-duty.md |
| LV-03 | US-C08-14 | AC-C08-14.4, 14.5; OQ-C08-18 |
| LV-04 | US-C08-09, US-C08-15 | AC-C08-09.5, AC-C08-15.1; OQ-C08-19 |
| DS-08 | US-C08-08 | AC-C08-08.1–08.9 |
| DS-10 | US-C08-09 | OQ-C08-12 |
| LA-12 | US-C08-15 | AC-C08-15.1; OQ-C08-19 |
| E5 | US-C08-15 | AC-C08-15.1 |
| E19 | US-C08-13 | AC-C08-13.1 |
| P6 (day counts) | US-C08-10, US-C08-12 | AC-C08-10.2, 10.3, AC-C08-12.2 |
| P7 (day counts) | US-C08-11 | AC-C08-11.1 |
| BR-03 | US-C08-05, 07, 08, 11, 12 | AC-C08-05.10, 07.1, 08.8, 11.8, 12.1 |
| BR-04 | US-C08-11 | AC-C08-11.6 |
| BR-06 | US-C08-07, US-C08-12 | AC-C08-07.2, AC-C08-12.4 |
| BR-07 | US-C08-05 | AC-C08-05.5 |
| BR-08 | US-C08-02, 06, 09, 10, 15 | AC-C08-02.8, 06.4, 09.7, 10.5, 15.2 |
| R6 | US-C08-06 | AC-C08-06.5 |
| R7 | US-C08-08, 09, 10 | AC-C08-08.9, 09.7, 10.5 |
| NT-05, NT-06 | US-C08-05 | AC-C08-05.6, 05.9 |
| NT-11 | US-C08-03 | AC-C08-03.4 |
| AT6 | US-C08-11 | AC-C08-11.6 |
| AT18 | US-C08-02 (reference; owned by C15) | AC-C08-02.10 |
| AT20 | US-C08-01 | AC-C08-01.5 |
| AT21 | US-C08-02, US-C08-03 | AC-C08-02.9, 03.5 |
