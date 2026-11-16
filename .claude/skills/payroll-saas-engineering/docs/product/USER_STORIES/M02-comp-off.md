# M02 Compensatory off — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §10.2 (CO-R1, CO-R2, CO-R4, HW-01..HW-06); related §4.1, §4.2 (MD-01, MD-03), §6 (comp-off), §9.7, §9.8 (LM-R8, LM-25), §9.9 (OD-10), §9.15 (OB-R2), §12, §13 (P8 — V2), §14 (R9), §21

## Scope
- In scope (V1): switching how work on a holiday or weekly off is treated (HW-01); earning a comp-off from such work, with optional prior approval and approval before credit; half-day and full-day comp-off by hours worked; employee choice where allowed; comp-off balance shown with leave and taken like leave; expiry with the "lapse" outcome; on-duty on a holiday or weekly off earning comp-off (OD-10); register R9 (earned, taken, expired).
- Out of scope / later release:
  - CO-R3 and HW-06 = pay one extra day's wage (double pay for the worked day), P8 — V2 with Payroll (§21 "Comp-off double pay on expiry"). Dependency recorded only: in V1 only the lapse outcome is deliverable (see OQ-M02-01). R9 "paid" column is V2.
  - HW-01 = overtime as a paid outcome — overtime approval and pay are V3 (M01); V1 identifies overtime hours only (§21 note 2).
  - Comp-off opening balances (OB-R2, AT18) — owned by C15; referenced only.

## Stories

### US-M02-01 Configure treatment of work on a holiday or weekly off
As an HR Manager, I want to choose how work on a holiday or weekly off is treated and how comp-off is earned and expires, so that the rule matches our policy.
Source: MD-01, MD-03, MD-05, HW-01, HW-02, HW-03, HW-04, HW-05, HW-06, G2, G5, G9
Settings involved: HW-01 (default comp-off), HW-02 (default on), HW-03 (default off), HW-04 (default 4 and 8 hours), HW-05 (default 60 days), HW-06 (default pay one extra day's wage — V2 option; see OQ-M02-01)

Acceptance criteria
- AC-M02-01.1 Given the Vendor Admin has not enabled M02 for the organisation, When any user looks for comp-off settings, balances or reports, Then they are hidden and no comp-off is earned.  (Source: MD-01, MD-03, BR-14)
- AC-M02-01.2 Given M02 is disabled after comp-offs were earned, When it is enabled again, Then the earlier comp-off data is restored.  (Source: MD-03, AT2)
- AC-M02-01.3 Given HW-01 = ignore, When an employee works on a holiday or weekly off, Then no comp-off is earned.  (Source: HW-01)
- AC-M02-01.4 Given HW-01 = overtime, When an employee works on a holiday or weekly off, Then no comp-off is earned and the hours are reported as overtime hours identified.  (Source: HW-01, §21 note 2)
- AC-M02-01.5 Given an HW setting is changed, When it is saved, Then a new version with a start date is created and the change records who, when, old value, new value and reason.  (Source: G5, G9)
- AC-M02-01.6 Given a user from organisation A, When the user views comp-off settings, balances or R9, Then nothing from organisation B is visible or changeable.  (Source: N1, AT20)

Edge and negative cases
- HW-06 default is a V2 behaviour — OQ-M02-01.

### US-M02-02 Earn comp-off for work on a holiday or weekly off
As an Employee, I want a comp-off earned when I work on a holiday or weekly off, so that I get a paid day off for it.
Source: CO-R1, HW-01, HW-03, HW-04, OD-10, §6 (comp-off), PC-06
Settings involved: HW-01 (default comp-off), HW-03 (default off), HW-04 (default 4 and 8 hours), OD-10 (default yes)

Acceptance criteria
- AC-M02-02.1 Given HW-01 = comp-off and HW-04 = 4 and 8, When an employee works 8 hours or more on a weekly off, Then a full-day comp-off is earned (credited after approval, US-M02-03).  (Source: CO-R1, HW-04)
- AC-M02-02.2 Given HW-04 = 4 and 8, When the employee works at least 4 but less than 8 hours on a holiday, Then a half-day comp-off is earned.  (Source: HW-04)
- AC-M02-02.3 Given HW-04 = 4 and 8, When the employee works less than 4 hours on a holiday, Then no comp-off is earned.  (Source: HW-04)
- AC-M02-02.4 Given OD-10 = yes, When an employee has approved on-duty on a holiday or weekly off, Then the on-duty time earns comp-off as normal work does; given OD-10 = no, Then it does not.  (Source: OD-10)
- AC-M02-02.5 Given HW-03 = on, When the employee has prior approval to work on that day, Then comp-off can be earned.  (Source: HW-03)
- AC-M02-02.6 Given the hours worked on that day change (regularisation, late punch) while the period is open, When the day is recalculated, Then the earned comp-off is recalculated to match.  (Source: BR-03, PC-R7)
- AC-M02-02.7 Given a comp-off is earned, When it is recorded, Then it stores the worked date, hours, rule version and punches used.  (Source: BR-08)

Edge and negative cases
- HW-03 = on and the employee works without prior approval — OQ-M02-02.
- HW-04 measured on net hours or gross hours; hours on a split day across midnight — OQ-M02-03.
- Recalculation that reduces a comp-off already taken — OQ-M02-04.
- Day that is both a holiday and a weekly off (HD-02 = no change) — earns once; no extra rule in BRD — OQ-M02-03 (asked together).

### US-M02-03 Approve comp-off before credit
As a Manager (Approver), I want to approve a comp-off before it is credited, so that only genuine holiday work earns a day off.
Source: CO-R1, HW-02, BR-07, NT-05, §16
Settings involved: HW-02 (default on), AC-02

Acceptance criteria
- AC-M02-03.1 Given HW-02 = on, When a comp-off is earned, Then it is pending and not in the balance until approved.  (Source: CO-R1, HW-02)
- AC-M02-03.2 Given HW-02 = on, When the approver approves it, Then the comp-off is credited to the balance by a ledger entry and the approval records actor and time.  (Source: HW-02, §16)
- AC-M02-03.3 Given HW-02 = on, When the approver rejects it, Then nothing is credited and the rejection records actor, time and reason.  (Source: HW-02, §16)
- AC-M02-03.4 Given HW-02 = off, When a comp-off is earned, Then it is credited without approval by a ledger entry recorded as credited by the system.  (Source: HW-02)
- AC-M02-03.5 Given a comp-off earned by the approver themself, When the approver tries to approve it, Then the action is refused.  (Source: BR-07)
- AC-M02-03.6 Given NT-05 is on, When a comp-off is approved or rejected, Then the employee is notified.  (Source: NT-05)

Edge and negative cases
- Escalation or auto-decision when the approver does not act (no HW setting) — OQ-M02-05.

### US-M02-04 Choose between pay and comp-off
As an Employee, I want to choose on the day between pay and comp-off where my organisation allows, so that I get the outcome I prefer.
Source: CO-R4, HW-01
Settings involved: HW-01 = employee chooses (default comp-off)

Acceptance criteria
- AC-M02-04.1 Given HW-01 = employee chooses, When the employee works on a holiday or weekly off and chooses comp-off, Then a comp-off is earned as in US-M02-02.  (Source: CO-R4, HW-01)
- AC-M02-04.2 Given HW-01 = employee chooses, When the employee chooses pay, Then no comp-off is earned for that day.  (Source: CO-R4)
- AC-M02-04.3 Given HW-01 is not "employee chooses", When the employee works on such a day, Then no choice is offered.  (Source: CO-R4)
- AC-M02-04.4 Given a choice is made, When it is saved, Then it records the employee and time.  (Source: §16)

Edge and negative cases
- What the "pay" choice produces in V1, with Payroll in V2 and overtime pay in V3 — OQ-M02-06.
- Employee does not choose — OQ-M02-07.
- Whether the choice can be changed after the day — OQ-M02-07 (asked together).

### US-M02-05 See and take comp-off like leave
As an Employee, I want my comp-off balance shown with my leave and to take it like leave, so that I use it the same way.
Source: CO-R2, LM-R8, LM-R4, LM-25, R9, DB-R7
Settings involved: LM-25 (default comp-off excluded from the informed count)

Acceptance criteria
- AC-M02-05.1 Given approved comp-offs, When the employee views balances, Then the comp-off balance appears alongside leave.  (Source: CO-R2, LM-R8)
- AC-M02-05.2 Given a comp-off balance, When the employee requests comp-off for a day and it is approved, Then the balance is debited by a ledger entry and the day is treated as paid time off.  (Source: CO-R2, §6, LM-R4)
- AC-M02-05.3 Given a comp-off request is cancelled after approval, When the cancellation takes effect, Then the balance is restored by a credit ledger entry.  (Source: LM-R4, §16)
- AC-M02-05.4 Given LM-25 default, When comp-off days are taken in a month, Then they are not counted in the excess informed leave count.  (Source: LM-25)
- AC-M02-05.6 Given a confirmed comp-off opening balance with an as-of date, When the balance is shown, Then it includes the opening balance.  (Source: OB-R2, AT18 — owned by C15)
- AC-M02-05.7 Given an HR Manager limited to chosen branches, When report R9 is run, Then it shows comp-off earned, taken and expired for those branches only, and exports to spreadsheet.  (Source: R9, §14)

Edge and negative cases
- Which LM settings apply to comp-off requests (half day LM-08, notice LM-09, approval LM-13, negative balance LM-07) — OQ-M02-08.
- Which comp-off is used first when several are held — OQ-M02-09.

### US-M02-06 Expire unused comp-off (lapse)
As an HR Manager, I want an unused comp-off to expire after the set number of days and lapse, so that balances do not build up indefinitely.
Source: HW-05, HW-06 (lapse option only), NT-11, R9, AT21; dependency CO-R3, P8 (V2)
Settings involved: HW-05 (default 60 days), HW-06 = lapse

Acceptance criteria
- AC-M02-06.1 Given HW-05 = 60 days and HW-06 = lapse, When a comp-off is not taken within 60 days, Then it is removed from the balance by a lapse ledger entry, and nothing else is produced.  (Source: HW-05, HW-06)
- AC-M02-06.2 Given HW-05 = 60 days, When the comp-off is taken on day 60 or earlier, Then it does not expire.  (Source: HW-05)
- AC-M02-06.3 Given NT-11 is on, When a comp-off is about to lapse, Then the employee is notified.  (Source: NT-11)
- AC-M02-06.4 Given expiry has run for a date, When it runs again for the same date, Then no further entry is made.  (Source: AT21)
- AC-M02-06.5 Given a comp-off has lapsed, When R9 is run, Then it appears as expired.  (Source: R9)
- AC-M02-06.6 Given HW-06 = pay one extra day's wage in V1, When a comp-off expires, Then no payout is produced in V1 (V2 dependency, CO-R3, P8); the V1 outcome is open in OQ-M02-01.  (Source: HW-06, CO-R3, §21)

Edge and negative cases
- Expiry counted from the worked date or the credit date; calendar days — OQ-M02-10.
- Half-day comp-off partly used before expiry — OQ-M02-09.

## Open questions
- OQ-M02-01 HW-06 default is "pay one extra day's wage", which is V2. In V1, is the default lapse, is the payout option hidden, or are expired comp-offs exported as a payout day in the payroll input (MD-06) for later payment? What happens to comp-offs that expire in V1 when Payroll is switched on in V2? Source: HW-06, CO-R3, P8, MD-06, §21. Why it matters: a V1 organisation on the default would expect a payout the system cannot make.
- OQ-M02-02 HW-03 = on: when an employee works on a holiday or weekly off without prior approval, is comp-off denied, flagged for approval, or treated under PC-06? Source: HW-03, PC-06. Why it matters: no outcome is stated for the common case.
- OQ-M02-03 HW-04: hours are net hours or gross hours? Is overtime-style rounding (RD) applied? A day that is both a holiday and a weekly off, or a night shift crossing into one: earns once, and on which date? Source: HW-04, RD-04, SH-06, HD-02. Why it matters: decides whether half or full day is earned.
- OQ-M02-04 If the worked day is corrected after the comp-off was taken (fewer hours, or day found not worked), is the balance allowed to go negative, is the taken day converted to another leave or LOP, or is the correction blocked? Source: BR-03, CO-R2. Why it matters: a correction can remove a credit that is already used.
- OQ-M02-05 HW-02 approval: who approves (AC-02), and what happens if they do not act (no escalation setting for comp-off)? Source: HW-02, AC-02. Why it matters: credits can stay pending until they expire.
- OQ-M02-06 CO-R4 "pay" choice in V1: Payroll is V2 and overtime pay V3. Does choosing pay record the hours as overtime hours identified, or a paid-holiday-work day in the payroll input export, or is the "employee chooses" option hidden in V1? Source: CO-R4, HW-01, §21 notes 1 and 2. Why it matters: the choice has no V1 effect otherwise.
- OQ-M02-07 HW-01 = employee chooses and the employee does not choose: is comp-off the fallback (the HW-01 default), pay, or does the day stay pending? Until when must the choice be made, and can it be changed? Source: HW-01, CO-R4. Why it matters: no fallback is stated.
- OQ-M02-08 "Taken like leave" (CO-R2): which leave settings apply to a comp-off request — half day (LM-08), notice (LM-09), approval and escalation (LM-13), negative balance (LM-07), sandwich (LM-19)? Source: CO-R2, LM-07..LM-13, LM-19. Why it matters: comp-off is not a leave type in LM-01, so none of these is clearly inherited.
- OQ-M02-09 When an employee holds several comp-offs with different expiry dates, which is used first (earliest expiring?) and how is a half day taken from a full-day credit tracked for expiry? Source: CO-R2, HW-05. Why it matters: the wrong order lapses comp-offs the employee meant to use.
- OQ-M02-10 HW-05 expiry of 60 days: counted from the worked date or the credit (approval) date; calendar days or working days? Source: HW-05, P8. Why it matters: a slow approval can shorten or lengthen the usable window.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| CO-R1 | US-M02-02, US-M02-03 | AC-M02-02.1, AC-M02-03.1 |
| CO-R2 | US-M02-05 | AC-M02-05.1, 05.2; OQ-M02-08, 09 |
| CO-R3 | — | V2 dependency; AC-M02-06.6; OQ-M02-01 |
| CO-R4 | US-M02-04 | AC-M02-04.1–04.3; OQ-M02-06, 07 |
| HW-01 | US-M02-01, US-M02-02, US-M02-04 | AC-M02-01.3, 01.4, AC-M02-02.1, AC-M02-04.1–04.3 |
| HW-02 | US-M02-03 | AC-M02-03.1–03.4; OQ-M02-05 |
| HW-03 | US-M02-02 | AC-M02-02.5; OQ-M02-02 |
| HW-04 | US-M02-02 | AC-M02-02.1–02.3; OQ-M02-03 |
| HW-05 | US-M02-06 | AC-M02-06.1, 06.2; OQ-M02-10 |
| HW-06 (lapse) | US-M02-06 | AC-M02-06.1 |
| HW-06 (pay one extra day) | — | V2 dependency; AC-M02-06.6; OQ-M02-01 |
| P8 | — | V2 dependency; OQ-M02-01 |
| OD-10 | US-M02-02 | AC-M02-02.4 |
| LM-R8 | US-M02-05 | AC-M02-05.1 |
| LM-25 | US-M02-05 | AC-M02-05.4 |
| MD-01, MD-03 | US-M02-01 | AC-M02-01.1, 01.2 |
| BR-03 | US-M02-02 | AC-M02-02.6; OQ-M02-04 |
| BR-07 | US-M02-03 | AC-M02-03.5 |
| BR-08 | US-M02-02 | AC-M02-02.7 |
| R9 | US-M02-05, US-M02-06 | AC-M02-05.7, 06.5 |
| NT-05 | US-M02-03 | AC-M02-03.6 |
| NT-11 | US-M02-06 | AC-M02-06.3 |
| AT18 | US-M02-05 (reference; owned by C15) | AC-M02-05.6 |
| AT20 | US-M02-01 | AC-M02-01.6 |
| AT21 | US-M02-06 | AC-M02-06.4 |
