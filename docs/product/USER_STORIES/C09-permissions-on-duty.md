# C09 Permissions and on-duty — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §9.9 (PM-R1..PM-R7, PM-01..PM-30, OD-01..OD-12); related §6 (permission, on-duty), §8, §9.6 (LA-15, LA-20, EC-13, EC-17), §9.4 (PC-17), §9.14 (NT-04, NT-05, NT-06), §9.15 (OB-R2), §12 (BR-03, BR-04, BR-07, BR-08, order of checks), §13 (E3, E9, E10, E11, E16), §14 (R8), §21

## Scope
- In scope (V1): permission types, limits and eligibility; applying for a permission with checks at the time of application; requests over the limit; approval and escalation; matching a permission against punches and over-stay; conversion of a late arrival or early check-out into a permission; cancelling or changing a permission; permission balance and usage; applying for on-duty, its approval, and its effect on attendance; register R8 and notifications NT-04, NT-05, NT-06.
- Out of scope / later release:
  - Daily allowance on an on-duty day (OD-09) — Daily allowance, V3 (§21). Setting recorded; no story.
  - Overtime pay for permission or on-duty time (PM-13, OD-08) — overtime pay is V3 (M01); V1 only decides whether the time is included in overtime hours identified (§21 note 2).
  - Comp-off earned by on-duty on a holiday or weekly off (OD-10) — written in M02-comp-off.md (US-M02-02); referenced here.
  - Money value of unpaid permission minutes (E11 "60 minutes are deducted") — Payroll M03, V2; V1 produces the minutes for the payroll input (MD-06, §21 note 1).
  - Permissions-used-this-month opening balance (OB-R2, AT18) — owned by C15; referenced only.

## Stories

### US-C09-01 Configure permission types, limits and eligibility
As an HR Manager, I want to set permission types with their own limits, timing and eligibility, so that permissions follow our policy without manual checking.
Source: PM-01, PM-02, PM-03, PM-04, PM-05, PM-06, PM-07, PM-08, PM-09, PM-10, PM-11, PM-12, PM-13, PM-23, PM-24, PM-25, PM-26, G2, G5, G9
Settings involved: PM-01 (default on), PM-02 (default Personal only), PM-03 (default 30-minute blocks), PM-04 (default 30 and 120 minutes), PM-05 (default by count), PM-06 (default 2), PM-07 (default 4 hours), PM-08 (default calendar month), PM-09 (default 1 per day, no weekly cap), PM-10 (default any), PM-11 (default paid), PM-12 (default yes), PM-13 (default no), PM-23 (default no), PM-24 (default full), PM-25 (default all confirmed and probation staff), PM-26 (default off)

Acceptance criteria
- AC-C09-01.1 Given PM-01 = off, When an employee opens the request screen, Then permission cannot be requested and no permission rule affects any day.  (Source: PM-01, G2, BR-14)
- AC-C09-01.2 Given PM-02 defines two named types, When each is configured, Then each type keeps its own rules.  (Source: PM-02)
- AC-C09-01.3 Given a permission setting is changed, When it is saved, Then a new version with a start date is created and the change records who, when, old value, new value and reason.  (Source: G5, G9)
- AC-C09-01.4 Given PM-23 = no, When a new limit period starts, Then unused permissions do not carry forward; given PM-23 = yes up to N, Then up to N unused permissions carry forward.  (Source: PM-23)
- AC-C09-01.5 Given PM-24 = full, When an employee joins mid-period, Then the full limit applies for that period.  (Source: PM-24)
- AC-C09-01.6 Given a user from organisation A, When the user lists permission types or permissions, Then nothing from organisation B is visible or changeable through any screen, report or export.  (Source: N1, AT20)

Edge and negative cases
- PM-08 = payroll period while Payroll (V2) is off — OQ-C09-01.
- PM-24 = pro-rated formula — OQ-C09-02.
- PM-23 carry forward by count only, or also hours when PM-05 = by hours — OQ-C09-02 (asked together).

### US-C09-02 Apply for a permission
As an Employee, I want to apply for a permission from the web or the app and be told at once if it breaks a limit or timing rule, so that I do not wait on a request that cannot be approved.
Source: PM-R1, PM-R2, PM-03, PM-04, PM-05, PM-06, PM-07, PM-09, PM-10, PM-14, PM-15, PM-20, PM-21, PM-22, PM-25, PM-26, PM-28
Settings involved: as US-C09-01, plus PM-14 (default before or after within 2 days), PM-15 (default 0), PM-20 (default block), PM-21 (default allow), PM-22 (default no cap), PM-28 (default mandatory)

Acceptance criteria
- AC-C09-02.1 Given an eligible employee within all limits, When the employee submits a permission from the web or the app with a reason from the list, Then it is recorded as pending with the approver set by AC-02.  (Source: PM-R1, PM-28, AC-02)
- AC-C09-02.2 Given PM-03 = 30-minute blocks, When the employee requests 45 minutes, Then the request is refused or must be changed to a whole number of 30-minute blocks.  (Source: PM-03)
- AC-C09-02.3 Given PM-04 = 30 and 120 minutes, When a permission of 30 or 120 minutes is requested, Then it is accepted; When 150 minutes is requested, Then it is refused.  (Source: PM-04)
- AC-C09-02.4 Given PM-05 = by count, PM-06 = 2, PM-08 = calendar month, When an employee with 2 permissions already used in the month applies for a third, Then the over-limit rule PM-18 applies (US-C09-03).  (Source: PM-05, PM-06, PM-08, PM-18, E11)
- AC-C09-02.5 Given PM-05 = both, When either the count or the hours limit is reached first, Then the limit is treated as reached.  (Source: PM-05)
- AC-C09-02.6 Given PM-09 = 1 per day, When a second permission is requested for the same day, Then it is refused.  (Source: PM-09)
- AC-C09-02.7 Given PM-10 = start of shift, When a permission in the middle of the shift is requested, Then it is refused.  (Source: PM-10)
- AC-C09-02.8 Given PM-14 = before or after within 2 days, When the permission is requested 2 days after the date, Then it is accepted; When 3 days after, Then it is refused. Given PM-14 = before only, Then any request on or after the day is refused.  (Source: PM-14)
- AC-C09-02.9 Given PM-15 = N hours, When the request is made less than N hours before the permission start, Then it is refused.  (Source: PM-15)
- AC-C09-02.10 Given PM-20 = block, When a permission is requested on a day that has half-day leave, Then it is refused.  (Source: PM-20)
- AC-C09-02.11 Given PM-21 = block, When a permission is requested on a working day next to leave, a holiday or a weekly off, Then it is refused.  (Source: PM-21)
- AC-C09-02.12 Given PM-22 = N, When a permission would make N + 1 consecutive working days with a permission, Then it is refused.  (Source: PM-22)
- AC-C09-02.13 Given PM-25 excludes the employee (for example notice period), When the employee applies, Then the request is refused.  (Source: PM-25)
- AC-C09-02.14 Given PM-26 = on and the date is a blackout date, When the employee applies, Then the request is refused.  (Source: PM-26)
- AC-C09-02.15 Given PM-28 = mandatory, When no reason is chosen, Then the request cannot be submitted.  (Source: PM-28)

Edge and negative cases
- Weekly cap in PM-09: which week (calendar week, shift week) — OQ-C09-03.
- Whether pending requests count toward the limit when a further request is made — OQ-C09-04.

### US-C09-03 Handle a request over the limit
As an Employee, I want to know what happens when I have used up my permissions, so that I can choose leave or accept unpaid time instead.
Source: PM-18, E11, MD-06
Settings involved: PM-18 (default block)

Acceptance criteria
- AC-C09-03.1 (E11, block) Given the limit is 2 and PM-18 = block, When the employee requests a third permission in the month, Then the request is refused and the employee can apply for half-day leave.  (Source: PM-18, E11)
- AC-C09-03.2 (E11, unpaid minutes) Given the limit is 2 and PM-18 = allow with HR approval as unpaid minutes, When the employee requests a third permission of 60 minutes and HR approves it, Then 60 minutes are recorded as unpaid (deduction) minutes for the payroll input; before HR approves, nothing is deducted.  (Source: PM-18, E11, MD-06)
- AC-C09-03.3 Given PM-18 = allow with HR approval, When the third permission is approved by a manager only, Then it stays pending until HR approves.  (Source: PM-18)
- AC-C09-03.4 Given PM-18 = convert to half-day leave, When the over-limit request is made, Then a half-day leave request is raised instead, under the C08 leave rules.  (Source: PM-18, LM-08)
- AC-C09-03.5 Given PM-18 = debit leave by the hour, When the over-limit permission is approved, Then the leave balance is debited for the permission hours by a ledger entry.  (Source: PM-18, §16)

Edge and negative cases
- Debit leave by the hour: which leave type, and how hours convert to days (shift hours? 8?) — OQ-C09-05.
- Convert to half-day leave when the half-day leave request is refused or the type has no half day (LM-08 off) — OQ-C09-05 (asked together).

### US-C09-04 Approve or reject permissions
As a Manager (Approver), I want to approve or reject permission requests, with escalation if I do not act, so that requests are settled before the day is finalised.
Source: PM-R1, PM-16, PM-17, BR-07, NT-05, NT-06, §16, AC-07
Settings involved: PM-16 (default manager), PM-17 (default escalate after 24 hours)

Acceptance criteria
- AC-C09-04.1 Given PM-16 = manager, When the manager approves, Then the permission becomes approved, the usage is counted against the limit at that moment, and the approval records actor and time.  (Source: PM-16, PM-R4, §16)
- AC-C09-04.2 Given PM-16 = auto-approve within the limit, When a request within all limits is submitted, Then it is approved automatically and recorded as approved by the system, with the time.  (Source: PM-16)
- AC-C09-04.3 Given PM-16 = manager then HR, When the manager approves, Then it stays pending with HR.  (Source: PM-16)
- AC-C09-04.4 Given PM-17 = escalate after 24 hours, When the approver has not acted 24 hours after submission, Then the request is escalated and NT-05 notifies; given auto-approve or auto-reject, Then the system acts and the action is recorded with the time.  (Source: PM-17, NT-05)
- AC-C09-04.5 Given a user's own permission request, When that user tries to approve it, Then the action is refused.  (Source: BR-07)
- AC-C09-04.6 Given an approver outside the employee's access scope, When the approver opens the request, Then it is not visible and cannot be acted on.  (Source: AC-07, AT13)
- AC-C09-04.7 Given an approval changes a day in an open period, When it is saved, Then that day and the period totals are recalculated.  (Source: BR-03)

Edge and negative cases
- Whether the PM-17 24-hour timer counts non-working hours — OQ-C09-06.
- Escalation target — OQ-C09-06 (asked together).

### US-C09-05 Match a permission against punches and handle over-stay
As an HR Manager, I want approved permissions checked against punches where required, with over-stay handled as configured, so that permission time is counted correctly and abuse is caught.
Source: PM-R3, PM-12, PM-19, PM-27, §12 order of checks (step 2), LA-20, E9, E10
Settings involved: PM-12 (default yes), PM-19 (default excess counts as late or early minutes), PM-27 (default yes for middle of shift, no for start and end), LA-20 (default yes)

Acceptance criteria
- AC-C09-05.1 (E9) Given a fixed shift 09:00 to 18:00, a permission limit of 2 per month with none used, and an approved permission 16:00 to 18:00, When the employee punches out at 16:02, Then the day is present, there is no early mark, the permission time counts as worked, and 1 permission is left.  (Source: E9, PM-12, order of checks step 2)
- AC-C09-05.2 (E10) Given an approved permission 10:00 to 11:00 with PM-27 = yes for middle of shift and PM-19 = excess counts as late or early minutes, When the employee punches out at 10:00 and back in at 11:25, Then the extra 25 minutes count as late minutes and the day is present with one late mark.  (Source: E10, PM-19, PM-27, LA-05)
- AC-C09-05.3 Given PM-27 = yes for a middle-of-shift permission, When there is no out and in punch around the permission, Then the permission is not treated as matched.  (Source: PM-27, PM-R3)
- AC-C09-05.4 Given PM-27 = no for a start-of-shift permission, When the employee's first punch is within the approved permission end, Then approval alone moves the expected start and no late mark arises.  (Source: PM-27, order of checks step 2)
- AC-C09-05.5 Given PM-19 = whole absence becomes half-day leave, When the employee over-stays, Then the whole absence is treated as half-day leave under the C08 rules.  (Source: PM-19)
- AC-C09-05.6 Given PM-19 = flag only, When the employee over-stays, Then the day is flagged and no late or early minutes are added for the excess.  (Source: PM-19)
- AC-C09-05.7 Given PM-12 = no, When status is calculated, Then permission time is not counted as worked hours.  (Source: PM-12)
- AC-C09-05.8 Given a punch for a matched permission arrives late (out-of-order sync), When it is received in an open period, Then the day is recalculated.  (Source: PC-R7, BR-03)
- AC-C09-05.9 Given any result from a permission match, When the day is viewed, Then it shows the permission, the punches used and the rule version.  (Source: BR-08)

Edge and negative cases
- PM-27 matching when PM-10 = any and the requested time touches shift start or end — OQ-C09-07.
- PM-27 = yes but punches are missing: is the permission void, the day flagged, or missed-punch rules (MP-01) applied — OQ-C09-07 (asked together).
- PM-19 = use a second permission when none is left — OQ-C09-08.
- Over-stay excess and BR-04 (excess minutes and a slab cut on the same shortfall) — handled by the C06 order of checks; not repeated here.

### US-C09-06 Convert a late arrival or early check-out into a permission
As an Employee, I want a late arrival or early check-out converted into a permission where the organisation allows, so that I avoid a late or early mark when I have permissions left.
Source: PM-30, LA-15, EC-13, §12 order of checks (step 5), E3, PM-03
Settings involved: LA-15 (default suggest), EC-13 (default suggest), PM-30 (default yes), PM-03 (default 30-minute blocks)

Acceptance criteria
- AC-C09-06.1 (E3) Given LA-15 = automatic, a fixed shift 09:00 to 18:00, grace 10 minutes and 2 permissions left, When the employee punches in at 09:25, Then a 30-minute permission (09:00 to 09:30) is used, the day is present with no late mark, and 1 permission is left.  (Source: E3, LA-15, PM-30, PM-03)
- AC-C09-06.2 Given LA-15 = automatic and 0 permissions left, When the employee is late beyond grace, Then no conversion is made and the later checks in the order of checks apply.  (Source: order of checks step 5)
- AC-C09-06.3 Given LA-15 = suggest, When the employee is late beyond grace with a permission left, Then the employee is offered the conversion, and it is made only if the employee accepts.  (Source: LA-15)
- AC-C09-06.4 Given PM-30 = no, When a late arrival is converted, Then the permission count is not reduced.  (Source: PM-30)
- AC-C09-06.5 Given EC-13 = automatic and a permission left, When the employee checks out early beyond grace, Then a permission is used and no early mark is recorded.  (Source: EC-13, PM-30)
- AC-C09-06.6 Given LA-15 = off, When the employee is late, Then no conversion is offered or made.  (Source: LA-15)

Edge and negative cases
- LA-15 = suggest and the employee does not respond before period end — OQ-C09-09.
- Whether a converted permission must also pass PM-04, PM-09, PM-10, PM-25 (for example lateness above PM-04 maximum) — OQ-C09-09 (asked together).

### US-C09-07 Cancel or change a permission
As an Employee, I want to cancel or change my permission before it starts, and HR to be able to do it afterwards, so that my usage reflects what really happened.
Source: PM-29, BR-03, BR-06
Settings involved: PM-29 (default by the employee before the start time; by HR afterwards)

Acceptance criteria
- AC-C09-07.1 Given an approved permission that has not started, When the employee cancels it, Then the usage is returned to the limit and the cancellation records actor and time.  (Source: PM-29, §16)
- AC-C09-07.2 Given a permission whose start time has passed, When the employee tries to cancel or change it, Then the action is refused; When HR does it with a reason, Then it is accepted and logged.  (Source: PM-29, §16)
- AC-C09-07.3 Given a cancellation or change affects an open period, When it is saved, Then the day is recalculated; given the period is locked, Then the locked period is unchanged and only the difference posts to the next open period.  (Source: BR-03, BR-06)

Edge and negative cases
- Whether a change needs fresh approval — OQ-C09-10.

### US-C09-08 See permission balance and usage
As an Employee, I want to see how many permissions or hours I have used and have left, so that I can plan.
Source: PM-R4, AC-05, NT-04, DB-R7, R8
Settings involved: AC-05 (permission balance on / off), NT-04

Acceptance criteria
- AC-C09-08.1 Given AC-05 permission balance = on, When the employee opens the balance, Then used and remaining permissions for the current limit period are shown, by count, by hours, or both, per PM-05.  (Source: PM-R4, AC-05, PM-05)
- AC-C09-08.2 Given NT-04 is on, When the employee uses the last permission or a permission is used, Then the employee is notified on the configured channels.  (Source: NT-04)
- AC-C09-08.3 Given an HR Manager limited to chosen branches, When report R8 is run, Then it lists permissions and on-duty for those branches only and exports to spreadsheet.  (Source: R8, §14, UM-R2)
- AC-C09-08.4 Given a permissions-used opening balance confirmed with an as-of date, When the employee's usage is shown, Then it includes the opening usage.  (Source: OB-R2, AT18 — owned by C15)

Edge and negative cases
- AC-05 = off: whether the employee still sees remaining count on the request screen — OQ-C09-11.

### US-C09-09 Apply for on-duty
As an Employee, I want to apply for on-duty for some hours, a half day, a full day or several days from the same screen as permissions, so that official work away from the workplace counts as worked.
Source: PM-R5, OD-01, OD-02, OD-03, OD-04, OD-06, OD-11, OD-12, LV-02
Settings involved: OD-01 (default on), OD-02 (default all on), OD-03 (default none), OD-04 (default before or after within 2 days), OD-06 (default purpose and place mandatory, proof optional), OD-11 (default all)

Acceptance criteria
- AC-C09-09.1 Given OD-01 = on and an eligible employee, When the employee submits on-duty from the permissions screen with purpose and place, Then it is recorded as pending with the approver.  (Source: PM-R5, OD-01, OD-06)
- AC-C09-09.2 Given OD-02 = several days off, When the employee requests on-duty for more than one day, Then it is refused; the same applies to each of hours, half day, full day when off.  (Source: OD-02)
- AC-C09-09.3 Given OD-04 = before or after within 2 days, When on-duty is requested 3 days after the date, Then it is refused.  (Source: OD-04)
- AC-C09-09.4 Given OD-06 proof = mandatory, When no proof is attached, Then the request cannot be submitted.  (Source: OD-06)
- AC-C09-09.5 Given OD-11 excludes the employee's category, When the employee applies, Then the request is refused.  (Source: OD-11)
- AC-C09-09.6 Given an employee with 0 permissions left, When the employee applies for on-duty, Then the permission limit is not checked and on-duty does not reduce it.  (Source: OD-12)
- AC-C09-09.7 Given OD-01 = off, When the employee opens the request screen, Then on-duty is not offered and has no effect on any day.  (Source: OD-01, BR-14)

Edge and negative cases
- OD-03 limit reached: block, HR approval, or other — OQ-C09-12.
- OD-03 "per month": calendar month or attendance period — OQ-C09-12 (asked together).
- Cancel or change of on-duty (PM-29 is for permissions) — OQ-C09-10.

### US-C09-10 Approve or reject on-duty
As a Manager (Approver), I want to approve or reject on-duty requests, with escalation if I do not act, so that official work is recognised in time.
Source: OD-05, BR-07, NT-05, §16
Settings involved: OD-05 (default manager, escalate after 24 hours)

Acceptance criteria
- AC-C09-10.1 Given OD-05 = manager, When the manager approves, Then the on-duty becomes approved with actor and time recorded, and the covered days are recalculated.  (Source: OD-05, BR-03, §16)
- AC-C09-10.2 Given OD-05 = manager then HR, When the manager approves, Then it stays pending with HR.  (Source: OD-05)
- AC-C09-10.3 Given OD-05 default, When the approver has not acted 24 hours after submission, Then the request is escalated and NT-05 notifies.  (Source: OD-05, NT-05)
- AC-C09-10.4 Given a user's own on-duty request, When that user tries to approve it, Then the action is refused.  (Source: BR-07)

Edge and negative cases
- OD-05 auto-approve / auto-reject: after how long (the BRD gives a time only for the default) — OQ-C09-06.
- Whether the 24-hour timer counts non-working hours — OQ-C09-06.

### US-C09-11 On-duty counts as worked
As an Employee, I want approved on-duty time counted as worked with no late or early mark for the time it covers, so that I am not penalised for official work.
Source: PM-R6, PM-R7, OD-07, OD-08, OD-10, OD-12, PC-17, §6 (on-duty), §12 order of checks (step 1), E16
Settings involved: OD-07 (default not required), OD-08 (default no), OD-10 (default yes), PC-17 (default not applied, location still captured)

Acceptance criteria
- AC-C09-11.1 (E16) Given a fixed shift 09:00 to 18:00 and an approved on-duty 14:00 to 18:00 at a client site, When the employee punches in at 09:00 and out at 13:55 with no further punch, Then the day is present, full day, with no early mark, and the on-duty hours count as worked.  (Source: E16, PM-R6)
- AC-C09-11.2 Given an approved full-day on-duty and OD-07 = not required, When there are no punches that day, Then the day is present and not late, early or missed punch.  (Source: PM-R6, OD-07, order of checks step 1)
- AC-C09-11.3 Given OD-07 = app check-in at the place with location, When the employee checks in from the app at the on-duty place, Then the check-in stores the location; and with PC-17 = not applied, the geo-fence is not applied.  (Source: PM-R7, OD-07, PC-17, PC-R13)
- AC-C09-11.4 Given OD-08 = no, When overtime hours are identified for the day, Then on-duty time is not included; given yes, Then it is included.  (Source: OD-08, §21 note 2)
- AC-C09-11.5 Given OD-10 = yes and approved on-duty on a holiday or weekly off with M02 on, When the day is calculated, Then it is treated as work on that day for comp-off (see US-M02-02).  (Source: OD-10, HW-01)
- AC-C09-11.6 Given approved on-duty is cancelled or rejected after the day was calculated, When the period is open, Then the day is recalculated from its punches.  (Source: BR-03)

Edge and negative cases
- OD-07 = app check-in or start and end punches required but missing — OQ-C09-13.
- Partial on-duty plus a gap (for example E16 out at 13:55 vs on-duty start 14:00) — E16 states the result; no further rule.

## Open questions
- OQ-C09-01 PM-08 = payroll period: payroll periods per payroll group (PL-01) depend on Payroll, which is V2. In V1, is "payroll period" the attendance period (C12 V1), or is the option hidden? Same for LA-18. Source: PM-08, PL-01, §21. Why it matters: decides when the permission limit resets.
- OQ-C09-02 PM-24 = pro-rated: what formula (days in service / days in period, rounded how for a count limit of 2)? Does PM-23 carry forward apply to counts, hours, or both? Source: PM-24, PM-23, PM-05. Why it matters: joiners' limits and carried amounts are undefined.
- OQ-C09-03 PM-09 weekly cap: which week (calendar week Monday to Sunday, shift week, roster week)? Source: PM-09. Why it matters: the cap gives different answers per definition.
- OQ-C09-04 Do pending permission requests count toward PM-06 / PM-07 / PM-09 when the next request is checked? Source: PM-R2, PM-05. Why it matters: otherwise several pending requests can exceed the limit once approved.
- OQ-C09-05 PM-18 = debit leave by the hour: which leave type (LV-04 order?), and how hours convert to days (shift required hours SH-03?). PM-18 = convert to half-day leave: what if the half-day leave is refused or the type has LM-08 off? Source: PM-18, LV-04, SH-03, LM-08. Why it matters: no conversion rule exists.
- OQ-C09-06 Escalation timers PM-17 and OD-05: do they count non-working hours, weekly offs and holidays? To whom is a request escalated? OD-05 gives no N for auto-approve / auto-reject. Source: PM-17, OD-05, AC-02. Why it matters: a request made on Saturday could be escalated or auto-decided before anyone is at work.
- OQ-C09-07 PM-27 matching: with PM-10 = any, how is "middle" vs "start" vs "end" decided for the PM-27 default? When required punches are missing, is the permission void, the day flagged for regularisation, or MP-01 applied? Source: PM-27, PM-10, MP-01. Why it matters: decides the day status in a common case.
- OQ-C09-08 PM-19 = use a second permission: what happens when no permission is left or the excess is below PM-04 minimum (30 minutes)? Source: PM-19, PM-04, PM-06. Why it matters: no fallback is stated.
- OQ-C09-09 LA-15 / EC-13 = suggest: what happens if the employee does not respond before the day or period is finalised? Must a converted permission also satisfy PM-04 (for example 150 minutes late vs 120-minute maximum), PM-09, PM-10, PM-25? Source: LA-15, EC-13, PM-30, PM-04. Why it matters: conversion may otherwise exceed limits.
- OQ-C09-10 Does changing a permission need fresh approval? Who may cancel or change on-duty, and until when (PM-29 covers permissions only)? Source: PM-29, OD-05. Why it matters: no rule for on-duty cancellation.
- OQ-C09-11 With AC-05 permission balance = off, may the employee still see remaining count when applying (PM-R4 says balance is visible)? Source: AC-05, PM-R4. Why it matters: PM-R4 and AC-05 can conflict.
- OQ-C09-12 OD-03 limit: what happens when it is reached (block, HR approval, other)? Is "month" the calendar month or the attendance period? Source: OD-03. Why it matters: a limit with no over-limit action cannot be enforced.
- OQ-C09-13 OD-07 = app check-in at the place, or start and end punches, when the required punches are missing: is on-duty void, the day flagged, or treated as approved anyway? Source: OD-07, PM-R7. Why it matters: decides whether the day counts as worked.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| PM-R1 | US-C09-02, US-C09-04 | AC-C09-02.1, AC-C09-04.1 |
| PM-R2 | US-C09-02 | AC-C09-02.2–02.15; OQ-C09-04 |
| PM-R3 | US-C09-05 | AC-C09-05.1–05.3 |
| PM-R4 | US-C09-04, US-C09-08 | AC-C09-04.1, AC-C09-08.1; OQ-C09-11 |
| PM-R5 | US-C09-09 | AC-C09-09.1 |
| PM-R6 | US-C09-11 | AC-C09-11.1, 11.2 |
| PM-R7 | US-C09-11 | AC-C09-11.3; OQ-C09-13 |
| PM-01 | US-C09-01 | AC-C09-01.1 |
| PM-02 | US-C09-01 | AC-C09-01.2 |
| PM-03 | US-C09-02, US-C09-06 | AC-C09-02.2, AC-C09-06.1 |
| PM-04 | US-C09-02 | AC-C09-02.3; OQ-C09-09 |
| PM-05 | US-C09-02, US-C09-08 | AC-C09-02.4, 02.5, AC-C09-08.1 |
| PM-06 | US-C09-02 | AC-C09-02.4 |
| PM-07 | US-C09-02 | AC-C09-02.5 |
| PM-08 | US-C09-02 | AC-C09-02.4; OQ-C09-01 |
| PM-09 | US-C09-02 | AC-C09-02.6; OQ-C09-03 |
| PM-10 | US-C09-02 | AC-C09-02.7; OQ-C09-07 |
| PM-11 | US-C09-01 | setting only; paid permission has no deduction (see AC-C09-03.2 for unpaid minutes) |
| PM-12 | US-C09-05 | AC-C09-05.1, 05.7 |
| PM-13 | US-C09-01 | scope note (overtime pay V3) |
| PM-14 | US-C09-02 | AC-C09-02.8 |
| PM-15 | US-C09-02 | AC-C09-02.9 |
| PM-16 | US-C09-04 | AC-C09-04.1–04.3 |
| PM-17 | US-C09-04 | AC-C09-04.4; OQ-C09-06 |
| PM-18 | US-C09-03 | AC-C09-03.1–03.5; OQ-C09-05 |
| PM-19 | US-C09-05 | AC-C09-05.2, 05.5, 05.6; OQ-C09-08 |
| PM-20 | US-C09-02 | AC-C09-02.10 |
| PM-21 | US-C09-02 | AC-C09-02.11 |
| PM-22 | US-C09-02 | AC-C09-02.12 |
| PM-23 | US-C09-01 | AC-C09-01.4; OQ-C09-02 |
| PM-24 | US-C09-01 | AC-C09-01.5; OQ-C09-02 |
| PM-25 | US-C09-02 | AC-C09-02.13 |
| PM-26 | US-C09-02 | AC-C09-02.14 |
| PM-27 | US-C09-05 | AC-C09-05.2–05.4; OQ-C09-07 |
| PM-28 | US-C09-02 | AC-C09-02.1, 02.15 |
| PM-29 | US-C09-07 | AC-C09-07.1, 07.2 |
| PM-30 | US-C09-06 | AC-C09-06.1, 06.4, 06.5 |
| OD-01 | US-C09-09 | AC-C09-09.1, 09.7 |
| OD-02 | US-C09-09 | AC-C09-09.2 |
| OD-03 | US-C09-09 | OQ-C09-12 |
| OD-04 | US-C09-09 | AC-C09-09.3 |
| OD-05 | US-C09-10 | AC-C09-10.1–10.3; OQ-C09-06 |
| OD-06 | US-C09-09 | AC-C09-09.1, 09.4 |
| OD-07 | US-C09-11 | AC-C09-11.2, 11.3; OQ-C09-13 |
| OD-08 | US-C09-11 | AC-C09-11.4 |
| OD-09 | — | V3 (daily allowance); dependency noted in Scope |
| OD-10 | US-C09-11 (M02 US-M02-02) | AC-C09-11.5 |
| OD-11 | US-C09-09 | AC-C09-09.5 |
| OD-12 | US-C09-09 | AC-C09-09.6 |
| LA-15, EC-13 | US-C09-06 | AC-C09-06.1–06.6; OQ-C09-09 |
| E3 | US-C09-06 | AC-C09-06.1 |
| E9 | US-C09-05 | AC-C09-05.1 |
| E10 | US-C09-05 | AC-C09-05.2 |
| E11 | US-C09-03 | AC-C09-03.1, 03.2 |
| E16 | US-C09-11 | AC-C09-11.1 |
| BR-03 | US-C09-04, 05, 07, 10, 11 | AC-C09-04.7, 05.8, 07.3, 10.1, 11.6 |
| BR-06 | US-C09-07 | AC-C09-07.3 |
| BR-07 | US-C09-04, US-C09-10 | AC-C09-04.5, AC-C09-10.4 |
| BR-08 | US-C09-05 | AC-C09-05.9 |
| NT-04 | US-C09-08 | AC-C09-08.2 |
| NT-05 | US-C09-04, US-C09-10 | AC-C09-04.4, AC-C09-10.3 |
| R8 | US-C09-08 | AC-C09-08.3 |
| AT18 | US-C09-08 (reference; owned by C15) | AC-C09-08.4 |
| AT20 | US-C09-01 | AC-C09-01.6 |
