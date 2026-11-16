# C12 Pay period and lock — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §4.1, §4.2 (MD-06), §5, §6, §8, §9.12, §12 (BR-03, BR-06, BR-07, BR-10), §13.1 (E15), §16, §19 (AT5, AT8, AT13, AT20, AT21), §21 (Version 1, note 1), §22.2 (TB-01, TB-02, TB-03); TSD §10 (J7) for context only

## Scope
- In scope (V1): attendance periods; attendance cut-off; pre-lock checklist; locking and (where enabled) unlocking an attendance period; posting post-lock corrections as differences to the next open period; the attendance totals / payroll input export produced while Payroll is off (MD-06, release plan "Attendance period and lock, with the attendance totals export", note 1).
- Out of scope / later release:
  - Pay frequencies per payroll group (PL-R1 daily / weekly / bi-weekly, PL-01 "set per payroll group", PG-01) — V2 with Payroll groups (§4.1 C12, §10.3.2). How V1 periods are defined without payroll groups is OQ-C12-01.
  - Advance and loan recoveries in the payroll input export (MD-06, AL-R8) — V2 with C10.
  - Comp-off payouts (HW-06 double pay) in the payroll input — V2.
  - Pay run lock step and "closed pay run never changes" for pay runs (§10.3.1 step 1, BR-06 second half) — V2 with M03.
  - Payroll group filter on the export — V2 (payroll groups do not exist in V1).

## Stories

### US-C12-01 Attendance periods are generated
As an HR Manager, I want attendance periods to be generated for my organisation, so that attendance totals, counters and the lock are worked out per period.
Source: PL-R1, PL-01, §4.1 C12, §21 Version 1
Settings involved: PL-01 (default monthly, calendar month)

Acceptance criteria
- AC-C12-01.1 Given an organisation with PL-01 at its default (monthly, calendar month), When periods are generated, Then each period runs from the 1st to the last day of a calendar month (for example 1 to 31 October).  (Source: PL-01)
- AC-C12-01.2 Given PL-01 is a custom monthly cycle (for example 26th to 25th), When periods are generated, Then each period runs from the 26th of one month to the 25th of the next, with no gap and no overlap between consecutive periods.  (Source: PL-01)
- AC-C12-01.3 Given a V1 organisation, When HR opens the period setting, Then daily, weekly and bi-weekly frequencies and payroll-group assignment are not offered.  (Source: §4.1 C12, PL-R1, §21 Version 2)
- AC-C12-01.4 Given a user of organisation A, When they view periods, Then they see only organisation A's periods.  (Source: AT20, TB-01)

Edge and negative cases
- Which unit a V1 period setting belongs to (whole organisation, branch, or policy set) when PL-01 is "set per payroll group" and payroll groups are V2 → OQ-C12-01.
- Change of PL-01 while periods already exist (for example calendar month to 26th–25th) — the BRD does not say how the transition period is formed → OQ-C12-09.

### US-C12-02 Attendance cut-off and days after the cut-off
As an HR Manager, I want to set an attendance cut-off date within the period and decide how the days after it are treated, so that period totals can be produced before the period ends.
Source: PL-02, PL-03, PL-R4, BR-06
Settings involved: PL-02 (no default given), PL-03 (default assumed present and corrected next period)

Acceptance criteria
- AC-C12-02.1 Given PL-03 = assumed present, When period totals are produced at the cut-off, Then each working day after the cut-off is counted as present in this period's totals.  (Source: PL-03)
- AC-C12-02.2 Given PL-03 = assumed present and a day after the cut-off later turns out to be present, When the actual result is finalised, Then no adjustment is posted to the next period.  (Source: PL-03, PL-R4 — difference is zero)
- AC-C12-02.3 Given PL-03 = held until actuals arrive, When period totals are produced at the cut-off, Then the days after the cut-off are not counted as present in this period's totals.  (Source: PL-03)

Edge and negative cases
- A day assumed present that later turns out absent (or half day, late, on unpaid leave): how and when the difference is posted, and whether it waits for lock → OQ-C12-02.
- Under "held until actuals arrive", in which period's totals the held days finally appear → OQ-C12-02.
- PL-02 has no default; whether a period with no cut-off set behaves as "cut-off = last day" → OQ-C12-09.

### US-C12-03 Pre-lock checklist
As a Payroll Officer, I want a pre-lock checklist of everything still unresolved in the period, so that I can clear it before I lock.
Source: PL-R2, BR-10, NT-09, TB-03, AT21; TSD §10 J7 (context)
Settings involved: none in C12 (lists items produced under RG, LM, PM, OD, MP settings)

Acceptance criteria
- AC-C12-03.1 Given an open period, When the Payroll Officer opens its pre-lock checklist, Then it lists pending requests, flagged days and unresolved missed punches for employees in the period, each with employee and date.  (Source: PL-R2)
- AC-C12-03.2 Given an item on the checklist is resolved (for example a regularisation is approved), When the checklist is next produced, Then that item no longer appears.  (Source: PL-R2, BR-03)
- AC-C12-03.3 Given the Payroll Officer's access is limited to branch X, When they open the checklist, Then only items for employees within their scope appear.  (Source: UM-R2, AC-07, AT13)
- AC-C12-03.4 Given the timed job that produces the checklist runs twice for the same date, When the checklist is viewed, Then each item appears once and no reminder is sent twice.  (Source: TB-03, AT21, NT-09)
- AC-C12-03.5 Given count-based and minutes-based penalties, When the checklist still has unsettled regularisations for the period, Then those penalties are not yet treated as final for the period.  (Source: BR-10)

Edge and negative cases
- Whether the Payroll Officer role exists in a V1 organisation (Payroll module is V2) and who else may use the checklist → OQ-C12-05.
- When the lock reminder is sent and to whom → see C14 OQ-C14-04.

### US-C12-04 Lock an attendance period
As a Payroll Officer, I want to lock an attendance period, so that its attendance is frozen and the totals I export never change afterwards.
Source: PL-R3, PL-04, PL-05, BR-06, AT8, §16
Settings involved: PL-04 (default manual by the Payroll Officer), PL-05 (default block the lock)

Acceptance criteria
- AC-C12-04.1 Given PL-04 = manual and the checklist is empty, When the Payroll Officer locks October, Then October's attendance is frozen for every employee in the period and the lock is recorded with user and time in the audit log.  (Source: PL-R3, PL-04, §16)
- AC-C12-04.2 Given PL-05 = block and the checklist has at least one pending item, When the Payroll Officer tries to lock, Then the lock is refused and the pending items are shown.  (Source: PL-05)
- AC-C12-04.3 Given PL-05 = allow, treating pending days as absent, When the Payroll Officer locks with pending items, Then each pending day is counted as absent in the locked totals.  (Source: PL-05)
- AC-C12-04.4 Given PL-05 = allow, treating pending days as present and correcting next period, When the Payroll Officer locks with pending items, Then each pending day is counted as present in the locked totals.  (Source: PL-05)
- AC-C12-04.5 Given October is locked, When any input for a date in October arrives or is changed (approval, punch, settings change), Then October's day results and totals stay exactly as locked.  (Source: BR-06, PL-R3, AT8)
- AC-C12-04.6 Given a user without the lock permission (for example a Manager or Employee), When they try to lock a period, Then the action is refused.  (Source: §5, UM-R1, AC-01, AT13)
- AC-C12-04.7 Given PL-04 = automatic on a date, When that date is reached, Then the period is locked by the system and recorded in the audit log.  (Source: PL-04, §16)
- AC-C12-04.8 Given automatic lock, When the timed job runs twice for the same date, Then the period is locked once, with one lock record.  (Source: TB-03, AT21)
- AC-C12-04.9 Given organisation A locks October, When organisation B's October is viewed, Then B's period is unaffected.  (Source: AT20, TB-01)
- AC-C12-04.10 Given a Vendor Admin, When they view the vendor health view, Then they can see that a period is waiting to be locked but cannot lock or change it.  (Source: VC-R8, §5 Vendor Admin)

Edge and negative cases
- PL-04 = automatic and PL-05 = block with pending items on the lock date → OQ-C12-08.
- What happens to a request still pending at lock under the two "allow" options (is it closed, or decided later as a post-lock correction), and how this fits RG-04 "before lock" → OQ-C12-03.
- Whether the Payroll Officer role exists in V1 → OQ-C12-05.
- Periods must be locked in order (can November be locked while October is open?) → OQ-C12-09.

### US-C12-05 Unlock a period (only where allowed)
As an Organisation Admin, I want to approve an unlock of a period with a reason when the setting allows it, so that a serious error can be fixed under control.
Source: PL-06, §5 Organisation Admin, BR-07, §16
Settings involved: PL-06 (default never)

Acceptance criteria
- AC-C12-05.1 Given PL-06 = never, When any user views a locked period, Then no unlock action is offered and any unlock attempt is refused.  (Source: PL-06)
- AC-C12-05.2 Given PL-06 = with Organisation Admin approval and a reason, When an unlock is requested without a reason, Then the request is refused.  (Source: PL-06)
- AC-C12-05.3 Given PL-06 allows unlock and a reasoned request, When the Organisation Admin approves, Then the period is unlocked and the request, approval, reason, user and time are recorded in the audit log.  (Source: PL-06, §16)
- AC-C12-05.4 Given the Organisation Admin raised the unlock request, When they try to approve it themselves, Then the approval is refused.  (Source: BR-07)

Edge and negative cases
- BR-06 says a locked period never changes, but PL-06 permits unlock; what happens to adjustments already posted from that period and to an export already taken → OQ-C12-07.

### US-C12-06 Post-lock correction posts only the difference
As an HR Manager, I want a correction to a locked date to post only the difference to the next open period, tagged with the original date, so that the locked period never changes and the employee is still treated correctly.
Source: PL-R4, BR-06, E15, AT8, PL-07, NT-10, BR-08, AT5
Settings involved: PL-07 (default 3 past periods)

Acceptance criteria
- AC-C12-06.1 (E15) Given 28 October was marked absent and October is locked, When on 5 November the day is corrected to present, Then October is unchanged and the November payroll input carries a one-day arrear tagged 28 October.  (Source: E15, PL-R4, BR-06)
- AC-C12-06.2 Given a correction to a locked date, When it is applied, Then exactly one adjustment is created, holding only the difference between the locked result and the corrected result.  (Source: AT8, PL-R4)
- AC-C12-06.3 Given the period immediately after the locked one is also locked, When a correction is applied, Then the adjustment is posted to the first open period.  (Source: PL-R4 "next open period")
- AC-C12-06.4 Given a correction whose original date lies within the last PL-07 (default 3) past periods, When it is applied, Then it is accepted as an adjustment.  (Source: PL-07)
- AC-C12-06.5 Given a correction is reprocessed with the same inputs, When the adjustment is recalculated, Then the same single adjustment results and no duplicate is created.  (Source: AT5, AT8)
- AC-C12-06.6 Given an adjustment is posted, When it is viewed, Then it shows the original date, the difference, and the explanation (what changed, rule, version, approval).  (Source: BR-08, PL-R4)
- AC-C12-06.7 Given an adjustment is posted, When notifications are on for NT-10, Then the configured recipients are notified.  (Source: NT-10)
- AC-C12-06.8 Given a correction to a date in an open period, When it is applied, Then the day and totals are recalculated in place and no adjustment is created.  (Source: BR-03, §8)

Edge and negative cases
- Correction to a date older than PL-07 past periods → OQ-C12-04.
- A second, different correction to the same locked date (difference against the locked result or against the result after the first adjustment) → OQ-C12-06.
- Whether a correction to a locked date also re-runs that period's count-based / minutes-based penalties (BR-10) and posts their difference → OQ-C12-06.
- Late-synced punches (PC-R7, PC-07) arriving for a locked date: are they a correction that posts a difference → OQ-C12-10.

### US-C12-07 Attendance totals / payroll input export (Payroll off)
As a Payroll Officer (or HR Manager, see OQ-C12-05), I want to export each period's payroll input when the Payroll module is off, so that pay can be calculated outside the app from the same attendance results.
Source: MD-06, §8 Layer 5–6, §21 Version 1 and note 1, E15, R13, AT5, AT13, AT20, TB-02
Settings involved: none specific

Acceptance criteria
- AC-C12-07.1 Given Payroll is off (always in V1), When the export is produced for a period, Then for each employee it contains payable days, deduction days, late minutes, early minutes and overtime hours identified, in days and minutes, with no money amounts.  (Source: MD-06, §21 note 1, §6 Deduction days)
- AC-C12-07.2 Given adjustments have been posted to the period, When the export is produced, Then each adjustment appears with its original date tagged (for example the one-day arrear tagged 28 October in the November export).  (Source: E15, PL-R4)
- AC-C12-07.3 Given a locked period, When the export is produced twice, Then both exports have identical content.  (Source: BR-06, AT5)
- AC-C12-07.4 Given a user limited to branch X, When they produce the export, Then it contains only employees within their scope.  (Source: UM-R2, AC-07, AT13)
- AC-C12-07.5 Given a user of organisation A, When they open the export file link, Then it contains only organisation A's data, and a link issued to them cannot be used to read another organisation's file.  (Source: AT20, TB-01, TB-02)
- AC-C12-07.6 Given a V1 organisation, When the export is produced, Then advance and loan recoveries are not included (C10 is V2).  (Source: MD-06, AL-R8, §4.1 C10)
- AC-C12-07.7 Given a user without export permission, When they try to export, Then it is refused.  (Source: UM-R1, AC-01)

Edge and negative cases
- File format and column layout of the export (spreadsheet assumed from §14, not stated in MD-06) → OQ-C12-11.
- Whether the export may be taken for an open (unlocked) period → OQ-C12-11.
- Overtime hours included only as "identified" (computed) in V1, since approval is V3 (§10.1, §21 note 2) — stated, no further rule needed.

## Open questions
- OQ-C12-01 How is a V1 attendance period defined when PL-01 is "set per payroll group" and payroll groups arrive in V2: one period setting for the whole organisation, per branch, or per policy set? Source: PL-01, PL-R1, §4.1 C12, PG-01. Why it matters: decides who is in a period, what is locked together, and LA-18 / PM-08 "payroll period" counter resets in V1.
- OQ-C12-02 Under PL-03 "assumed present", when a day after the cut-off turns out absent (or half day, late, unpaid leave), is the difference posted to the next period as a PL-R4 adjustment tagged with the original date, and does that require the period to be locked first? Under "held until actuals arrive", in which period's totals do the held days appear? Source: PL-02, PL-03, PL-R4. Why it matters: determines whether employees are overpaid or held, and how the export shows it.
- OQ-C12-03 RG-04 allows regularisation only "before lock". Under PL-05 "allow, treating pending as absent / present", what happens to the requests still pending at lock (auto-closed, rejected, or decided later as post-lock corrections)? After lock, which routes can create a correction to a locked date (HR direct correction RG-10, leave approved afterwards LM-11/LM-23, other)? Source: PL-05, RG-04, RG-10, LM-11, LM-23, E15. Why it matters: E15 assumes a correction after lock, but the regularisation window closes at lock.
- OQ-C12-04 PL-07 limits corrections to N past periods (default 3). What happens to a correction for a date older than that: blocked, allowed with a higher approval, or recorded without an adjustment? Source: PL-07, PL-R4. Why it matters: defines the forbidden case and its message.
- OQ-C12-05 Does the Payroll Officer role exist in V1 organisations, where the Payroll module is not available? If not, who locks periods, sees the pre-lock checklist and takes the export (HR Manager?) Source: §5, PL-04, MD-06. Why it matters: permission ACs for lock, checklist and export depend on it.
- OQ-C12-06 For a second, different correction to the same locked date, is the new difference measured against the locked result or against the result already adjusted? Does a post-lock correction re-run that period's period-end penalties (BR-10, for example a late mark waived after the 3rd-mark penalty) and post their difference? Source: PL-R4, BR-06, BR-10, AT8. Why it matters: risk of double or missed adjustments.
- OQ-C12-07 PL-06 permits unlock, but BR-06 says a locked period never changes. If unlock is used, what happens to adjustments already posted from that period to later periods, and to an export already taken? Source: PL-06, BR-06, PL-R4. Why it matters: conflict between a setting and a fixed rule.
- OQ-C12-08 With PL-04 = automatic and PL-05 = block, what happens on the lock date if pending items remain: lock skipped and retried, lock postponed, or someone alerted? Source: PL-04, PL-05, NT-09. Why it matters: automatic lock may silently never happen.
- OQ-C12-09 Period housekeeping: must periods be locked in date order; what is the default when PL-02 (cut-off) is not set; how is the transition period formed when PL-01 changes (for example calendar month to 26th–25th)? Source: PL-01, PL-02, G5. Why it matters: gaps or overlaps in periods break totals.
- OQ-C12-10 A late-synced or out-of-order punch (PC-R7, PC-07) for a date in a locked period: is it treated as a post-lock correction that posts a difference, or only flagged? Source: PC-R7, PC-07, BR-06, PL-R4. Why it matters: offline app punches can arrive days later.
- OQ-C12-11 Export details: file format and columns, whether it can be taken for an open period (as a draft) or only after lock, and how it relates to report R13 (same content or different). Source: MD-06, R13, §21 Version 1. Why it matters: this is the client's input to external payroll in V1.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| PL-R1 | US-C12-01 | AC-C12-01.1, 01.2, 01.3 |
| PL-01 | US-C12-01 | AC-C12-01.1, 01.2, 01.3; OQ-C12-01, OQ-C12-09 |
| PL-02 | US-C12-02 | AC-C12-02.1–02.3; OQ-C12-09 |
| PL-03 | US-C12-02 | AC-C12-02.1–02.3; OQ-C12-02 |
| PL-R2 | US-C12-03 | AC-C12-03.1–03.5 |
| PL-R3 | US-C12-04 | AC-C12-04.1, 04.5 |
| PL-04 | US-C12-04 | AC-C12-04.1, 04.7, 04.8; OQ-C12-08 |
| PL-05 | US-C12-04 | AC-C12-04.2–04.4; OQ-C12-03 |
| PL-06 | US-C12-05 | AC-C12-05.1–05.4; OQ-C12-07 |
| PL-R4 | US-C12-06, US-C12-07 | AC-C12-06.1–06.6, AC-C12-07.2 |
| PL-07 | US-C12-06 | AC-C12-06.4; OQ-C12-04 |
| BR-03 | US-C12-03, US-C12-06 | AC-C12-03.2, AC-C12-06.8 |
| BR-06 | US-C12-04, US-C12-06, US-C12-07 | AC-C12-04.5, AC-C12-06.1, AC-C12-07.3 |
| BR-07 | US-C12-05 | AC-C12-05.4 |
| BR-08 | US-C12-06 | AC-C12-06.6 |
| BR-10 | US-C12-03 | AC-C12-03.5; OQ-C12-06 |
| E15 | US-C12-06, US-C12-07 | AC-C12-06.1, AC-C12-07.2 |
| AT5 | US-C12-06, US-C12-07 | AC-C12-06.5, AC-C12-07.3 |
| AT8 | US-C12-04, US-C12-06 | AC-C12-04.5, AC-C12-06.2 |
| AT13 | US-C12-03, US-C12-04, US-C12-07 | AC-C12-03.3, AC-C12-04.6, AC-C12-07.4 |
| AT20 / TB-01 | US-C12-01, US-C12-04, US-C12-07 | AC-C12-01.4, AC-C12-04.9, AC-C12-07.5 |
| AT21 / TB-03 | US-C12-03, US-C12-04 | AC-C12-03.4, AC-C12-04.8 |
| TB-02 | US-C12-07 | AC-C12-07.5 |
| MD-06 | US-C12-07 | AC-C12-07.1, 07.6; OQ-C12-11 |
| §21 note 1 | US-C12-07 | AC-C12-07.1 |
| NT-09 | US-C12-03 | AC-C12-03.4 |
| NT-10 | US-C12-06 | AC-C12-06.7 |
| VC-R8 | US-C12-04 | AC-C12-04.10 |
| §16 audit | US-C12-04, US-C12-05 | AC-C12-04.1, 04.7, AC-C12-05.3 |
