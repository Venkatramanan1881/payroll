# C14 Notifications — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.1 (C14), §4.2 (MD-03), §5, §9.3 (UM-R4), §9.14, §12 (BR-10, BR-14), §13.1 (E13), §15 (N7), §19 (AT14, AT20, AT21), §22.2 (TB-01, TB-03); TSD §10 (J2, J3, J7, J9) and §14 for context only

## Scope
- In scope (V1): NT-01 to NT-11; on/off per event, per channel (in-app, email, SMS, WhatsApp) and per recipient; notifications in the recipient's language.
- Out of scope / later release:
  - NT-12 Advance or loan approved, recovered or closed — V2 (C10).
  - NT-13 Document missing or expiring — V3 (C11).
  - NT-14 Payslip released — V2 (M03).
  - NT-05 for overtime, advance, loan and reimbursement requests — those request types arrive in V2/V3 (C10, M01, M04).

## Stories

### US-C14-01 Switch each event on or off per channel and recipient
As an HR Manager, I want to switch each notification event on or off per channel and per recipient, so that the right people get the right messages without noise.
Source: §9.14, §5 HR Manager, AC-04, G9, MD-03, BR-14, AT20, TB-01
Settings involved: per-event channel and recipient switches (§9.14; no defaults given), AC-04 (default HR Manager only)

Acceptance criteria
- AC-C14-01.1 Given event NT-xx (NT-01 to NT-11), When the HR Manager views notification settings, Then each event can be switched on or off separately for in-app, email, SMS and WhatsApp, and separately for each recipient.  (Source: §9.14)
- AC-C14-01.2 Given an event is on for in-app and off for email for the Employee recipient, When the event occurs, Then the employee receives it in-app and not by email.  (Source: §9.14)
- AC-C14-01.3 Given an event is off for a recipient on every channel, When the event occurs, Then that recipient receives nothing.  (Source: §9.14)
- AC-C14-01.4 Given a notification setting is changed, When the change is saved, Then who, when, old value, new value and reason are recorded.  (Source: G9)
- AC-C14-01.5 Given a user without permission to change settings, When they try to change a notification setting, Then the change is refused.  (Source: AC-04, UM-R1)
- AC-C14-01.6 Given the module or rule group behind an event is disabled (for example comp-off requests with M02 off), When the matching action would occur, Then no notification is sent and the event is not offered in settings.  (Source: MD-03, BR-14)
- AC-C14-01.7 Given an event in organisation A, When notifications are sent, Then only recipients in organisation A receive them, and the content contains only organisation A's data.  (Source: AT20, TB-01)
- AC-C14-01.8 Given a V1 organisation, When the HR Manager views notification settings, Then NT-12, NT-13 and NT-14 are not offered.  (Source: §4.1 C10, C11, M03, §21)

Edge and negative cases
- Default on/off per event, channel and recipient (needed for sector templates) → OQ-C14-01.
- Push notifications (TSD §14) are not among the BRD channels → OQ-C14-02.
- SMS / WhatsApp availability per plan, and what happens when a recipient has no email or mobile number → OQ-C14-07.

### US-C14-02 Notifications in the recipient's language
As an Employee, I want notifications in the language I chose, so that I understand them.
Source: N7, UM-R4, OR-03, AT14
Settings involved: OR-03 (default English on)

Acceptance criteria
- AC-C14-02.1 Given an employee who chose Tamil, When any notification is sent to them on any channel, Then its text is in Tamil.  (Source: N7, UM-R4, AT14)
- AC-C14-02.2 Given one event sent to an employee (Hindi) and to their manager (English), When it is sent, Then each recipient receives it in their own language.  (Source: N7, UM-R4)

Edge and negative cases
- A user's chosen language no longer offered by the organisation (OR-03 changed), or a template not yet translated → OQ-C14-08.

### US-C14-03 Attendance event notifications (NT-01, NT-02, NT-03, NT-07, NT-08)
As an Employee, I want to be told about missed punches, late and early marks, nearing penalties, penalties applied and flagged punches, so that I can fix or explain them in time.
Source: NT-01, NT-02, NT-03, NT-07, NT-08, MP-04, E13, BR-10, TB-03, AT21
Settings involved: MP-04 (default on, 30 minutes), LA-06 / EC-05 (default off), LA-08 / EC-07 (default every 3rd)

Acceptance criteria
- AC-C14-03.1 (E13) Given shift 09:00–18:00, MP-04 on with 30 minutes and NT-01 on, When an employee punches in 09:00 with no out, Then a missed punch reminder is sent at 18:30 and again next morning.  (Source: E13, NT-01, MP-04)
- AC-C14-03.2 Given the missed punch reminder job runs twice (or overlaps) for the same day, When reminders go out, Then the employee receives the 18:30 reminder once and the next-morning reminder once.  (Source: TB-03, AT21; TSD J2 context)
- AC-C14-03.3 Given the missing out punch is regularised before the next-morning reminder, When the reminder would be sent, Then it is not sent.  (Source: NT-01, MP-04)
- AC-C14-03.4 Given NT-02 is on, When a late mark or early mark is recorded (for example E2: in 09:25, one late mark), Then the configured recipients are notified with the date and the mark.  (Source: NT-02, E2)
- AC-C14-03.5 Given NT-07 is on and a count-based or minutes-based penalty is applied at period end, When the penalty is applied, Then the employee is notified with the penalty and its reason.  (Source: NT-07, BR-10)
- AC-C14-03.6 Given NT-07 and period-end processing runs twice for the same period, When notifications go out, Then each penalty is notified once.  (Source: TB-03, AT21)
- AC-C14-03.7 Given NT-08 is on, When a punch is flagged for being outside the geo-fence, late sync or clock mismatch, Then the configured recipients are notified with the reason for the flag.  (Source: NT-08, PC-04, PC-07, PC-08)

Edge and negative cases
- When NT-02 is sent (on punch, or after nightly finalisation) and what happens if the mark is later removed by regularisation or waiver → OQ-C14-05.
- What "nearing a penalty" means for NT-03 (how many marks or minutes before the trigger) → OQ-C14-03.
- Overlap with alerts defined in other modules (LA-19 escalation, PC-R10 device offline, LM-31 absconding, RG-07 escalation, VC-02 renewal) → OQ-C14-06.

### US-C14-04 Request, approval and balance notifications (NT-04, NT-05, NT-06)
As a Manager (Approver), I want to be told of requests waiting for me, and as an Employee I want to hear the outcome of my requests and my permission balance, so that nothing waits unnoticed.
Source: NT-04, NT-05, NT-06, PM-06, PM-R4, RG-07, LM-13, PM-17, OD-05, TB-03, AT21
Settings involved: PM-06 (default 2 per month), escalation settings RG-07 / LM-13 / PM-17 / OD-05

Acceptance criteria
- AC-C14-04.1 Given NT-05 is on, When a leave, permission, on-duty, regularisation or comp-off request is submitted, approved, rejected or escalated, Then the configured recipients are notified of that step.  (Source: NT-05, RG-07, LM-13, PM-17, OD-05)
- AC-C14-04.2 Given NT-04 is on and PM-06 = 2, When the employee's first permission of the month is approved, Then they are notified that 1 permission is left; When the second is approved, Then they are notified the permission balance is exhausted.  (Source: NT-04, PM-06)
- AC-C14-04.3 Given NT-06 is on and a manager has pending approvals, When the daily digest runs, Then the manager receives one digest listing all their pending approvals.  (Source: NT-06)
- AC-C14-04.4 Given a manager has no pending approvals, When the digest runs, Then no digest is sent to them.  (Source: NT-06)
- AC-C14-04.5 Given the digest job runs twice for the same day, When digests go out, Then each manager receives one digest that day.  (Source: TB-03, AT21; TSD J9 context)
- AC-C14-04.6 Given a manager, When the digest is built, Then it lists only requests for which they are the approver and no other team's requests.  (Source: AC-02, AT13)

Edge and negative cases
- Delegation (AC-03): does the delegate receive NT-05 / NT-06 while the approver is away → OQ-C14-09.

### US-C14-05 Period notifications (NT-09, NT-10)
As a Payroll Officer (see OQ-C14-04), I want a pre-lock checklist and lock reminder, and to know when an adjustment is posted to the next period, so that periods are locked on time and corrections are visible.
Source: NT-09, NT-10, PL-R2, PL-04, PL-R4, E15, TB-03, AT21
Settings involved: PL-04 (default manual)

Acceptance criteria
- AC-C14-05.1 Given NT-09 is on and a period is approaching its lock, When the reminder is due, Then the configured recipients receive the reminder with the count of pending checklist items.  (Source: NT-09, PL-R2)
- AC-C14-05.2 Given the reminder job runs twice for the same date, When reminders go out, Then each recipient receives one reminder for that date.  (Source: TB-03, AT21; TSD J7 context)
- AC-C14-05.3 Given NT-10 is on and E15 (28 October corrected to present after October was locked), When the one-day arrear is posted to November, Then the configured recipients are notified with the original date (28 October) and the period it was posted to (November).  (Source: NT-10, E15, PL-R4)

Edge and negative cases
- Who receives NT-09 in V1, when the reminder is sent (how many days before lock) → OQ-C14-04.

### US-C14-06 Leave balance low or about to lapse (NT-11)
As an Employee, I want to be warned when my leave balance is low or about to lapse, so that I can plan leave.
Source: NT-11, LM-06, LM-R7, OR-01
Settings involved: LM-06 (default lapse), OR-01 (default April to March)

Acceptance criteria
- AC-C14-06.1 Given NT-11 is on and LM-06 = lapse, When the leave year end approaches and the employee has a balance that will lapse, Then the employee is notified of the balance and the lapse date.  (Source: NT-11, LM-06, OR-01)
- AC-C14-06.2 Given LM-06 = carry forward up to N and the balance is within N, When the year end approaches, Then no lapse warning is sent for that balance.  (Source: NT-11, LM-06)

Edge and negative cases
- What counts as "low" and how far ahead "about to lapse" is → OQ-C14-03.
- Comp-off expiry (HW-05) warning — not named in NT-11 → OQ-C14-03.

## Open questions
- OQ-C14-01 What are the default on/off values per event, channel and recipient for NT-01 to NT-11 (and per sector template)? Source: §9.14, G6. Why it matters: G6 says every setting has a default; §9.14 gives none.
- OQ-C14-02 TSD §14 adds push notifications (Firebase), but BRD §9.14 lists only in-app, email, SMS and WhatsApp. Is push a channel with its own switch, or part of in-app? Source: §9.14; TSD §14. Why it matters: settings screen and costs.
- OQ-C14-03 Thresholds: when is a count "nearing a penalty" (NT-03); when is a leave balance "low" and how far ahead is "about to lapse" (NT-11); does NT-11 also cover comp-off expiry (HW-05)? Source: NT-03, NT-11, HW-05. Why it matters: these are rules not defined anywhere.
- OQ-C14-04 Who receives NT-09 in V1, where the Payroll Officer role belongs to a module that arrives in V2 (see OQ-C12-05), and how many days before the lock or cut-off is it sent? Source: NT-09, §5, PL-04. Why it matters: reminders may go to nobody.
- OQ-C14-05 When is NT-02 sent (at the punch, or after the day is finalised overnight), and is a follow-up sent if the mark is later removed by regularisation, waiver (E14) or recalculation? Source: NT-02, BR-03, E14. Why it matters: employees may act on marks that later disappear.
- OQ-C14-06 Do alerts defined in other modules — LA-19 / EC-17 escalation, PC-R10 / PC-10 device offline, LM-31 absconding, RG-07 / LM-13 / PM-17 escalation, VC-02 renewal, EM-04 confirmation reminder — follow the §9.14 per-channel, per-recipient switches, or are they always on? Source: §9.14, LA-19, PC-10, LM-31, VC-02, EM-04. Why it matters: not in the NT list, so their channels are undefined.
- OQ-C14-07 Are SMS and WhatsApp available to every V1 organisation or only by plan, and what happens when a recipient has no email or mobile number for an enabled channel? Source: §9.14, MD-02, Q5; TSD §14. Why it matters: per-message cost and silent delivery failures.
- OQ-C14-08 Which language is used when the recipient's chosen language is not offered by the organisation (OR-03) or a message has no translation yet? Source: N7, OR-03, UM-R4. Why it matters: AT14 test cases.
- OQ-C14-09 With delegation on (AC-03), does the delegate receive NT-05 and NT-06 for the absent approver? Source: AC-03, NT-05, NT-06. Why it matters: approvals may stall.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| §9.14 channels/recipients | US-C14-01 | AC-C14-01.1–01.3 |
| G9, AC-04 | US-C14-01 | AC-C14-01.4, 01.5 |
| MD-03, BR-14 | US-C14-01 | AC-C14-01.6 |
| AT20, TB-01 | US-C14-01 | AC-C14-01.7 |
| N7, UM-R4, AT14 | US-C14-02 | AC-C14-02.1, 02.2 |
| NT-01, MP-04, E13 | US-C14-03 | AC-C14-03.1–03.3 |
| NT-02, E2 | US-C14-03 | AC-C14-03.4 |
| NT-03 | US-C14-03 | OQ-C14-03 |
| NT-07, BR-10 | US-C14-03 | AC-C14-03.5, 03.6 |
| NT-08 | US-C14-03 | AC-C14-03.7 |
| NT-04, PM-06 | US-C14-04 | AC-C14-04.2 |
| NT-05 | US-C14-04 | AC-C14-04.1 |
| NT-06 | US-C14-04 | AC-C14-04.3–04.6 |
| NT-09 | US-C14-05 | AC-C14-05.1, 05.2 |
| NT-10, E15 | US-C14-05 | AC-C14-05.3 |
| NT-11, LM-06 | US-C14-06 | AC-C14-06.1, 06.2 |
| AT21, TB-03 | US-C14-03, US-C14-04, US-C14-05 | AC-C14-03.2, 03.6, AC-C14-04.5, AC-C14-05.2 |
| NT-12, NT-13, NT-14 | US-C14-01 | AC-C14-01.8 (not offered in V1) |
