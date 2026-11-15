# C06 Attendance rules — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §8 (layers), §9.6 (C06: daily process, PR, BK, LA, EC, EA, MP, DS, RD, RG), §10.1 intro and OT-02, OT-03, OT-07 (overtime hours identification only), §12 (BR-02, BR-03, BR-04, BR-05, BR-08, BR-09, BR-10, BR-11, BR-14 and the order of checks for a late arrival), §13.1 (E1–E14, E16), §19 (AT1, AT3, AT4, AT5, AT6)

## Scope
- In scope (V1):
  - The 8-step daily process for each employee and attendance date (§9.6 steps 1–8), as Layer 3 of §8, reading only Layers 1–2 and applying Layer 4 overlays.
  - Policy set resolution by most specific assignment and version in force (G3, G5, BR-05); rule-group master switches and exemptions (G2, G7, BR-14).
  - Punch reading (PR), breaks (BK), late arrival (LA), early check-out (EC), early arrival (EA), missed punches (MP), day status (DS), rounding (RD), regularisation (RG).
  - Period-end count-based and minutes-based penalties (LA-06..LA-12, EC-05..EC-11), expressed in days, minutes and leave debits only (BR-10).
  - Deterministic recalculation with stored explanation (BR-02, BR-03, BR-08, AT5).
  - Identification of overtime hours (computed only) per §10.1 intro, OT-02, OT-03, OT-07 and release plan note 2.
  - Output of C06: day status, hours, late/early minutes, marks, flags, deduction days and an explanation. C06 never produces a money amount.
- Out of scope / later release:
  - Money deduction for late and early minutes (PD-01..PD-09, §10.3.4) — V2. C06 only passes the minutes (order of checks step 7, PD-07). Reference only.
  - Overtime approval, caps, overtime rounding as a pay rule, outcome and pay (OT-04, OT-05, OT-06, OT-08..OT-11, OT-R2, OT-R3) — V3.
  - Deductions from the daily allowance (DA, PD-06) — V3.
  - Raw punch capture, geo-fence, device handling (C04); shift and roster definitions (C05); holiday and weekly-off calendars (C07); leave, sandwich, uninformed and excess absence (C08); permission and on-duty requests and limits (C09). C06 consumes their results as overlays (§9.6 step 7).
  - Period lock and correction after lock (C12, BR-06, E15). C06 recalculates open periods only; E15 is reference only.
  - Notifications delivery (C14: NT-01, NT-02, NT-03, NT-07) and reports (C13: R1–R4). C06 raises the events and results they use.

## Stories

### US-C06-01 Configure attendance rule groups in effective-dated policy sets
As an HR Manager, I want to set every C06 rule group and option inside named, effective-dated policy sets, so that each group of employees is evaluated by its own rules and every change is traceable.
Source: G2, G3, G5, G7, G9, MD-05, BR-05, BR-14, AC-04, AT3, AT4
Settings involved: all PR, BK, LA, EC, EA, MP, DS, RD, RG settings with their BRD defaults; AC-04 (default HR Manager only)

Acceptance criteria
- AC-C06-01.1 Given a new organisation, When no C06 setting has been changed, Then every C06 setting holds its BRD default (for example PR-01 = paired, LA-03 = 10, EC-02 = 5, DS-04 = 15 minutes, RD-01 = off, RG-01 = on).  (Source: G6, §9.6)
- AC-C06-01.2 Given a policy set version in use, When the HR Manager changes any C06 setting, Then the version in use is not edited and a new version with a start date is created.  (Source: G5)
- AC-C06-01.3 Given a C06 setting change, When it is saved, Then the settings history records who, when, old value, new value and reason.  (Source: G9)
- AC-C06-01.4 Given AC-04 = HR Manager with Organisation Admin approval, When the HR Manager saves a C06 change, Then the change does not take effect until the Organisation Admin approves it.  (Source: AC-04)
- AC-C06-01.5 Given a user whose role does not have edit rights on settings (for example Manager, Employee, Auditor), When the user tries to change a C06 setting, Then the change is refused; the Auditor can view the settings history read-only.  (Source: AC-01, §5)
- AC-C06-01.6 Given a rule group (for example late arrival, LA-01) is switched off with an effective date, When days on or after that date are processed, Then the rule group is not evaluated and has no effect on status, marks, minutes or deduction days; days before that date keep the result of the earlier version.  (Source: G2, BR-14, AT3)
- AC-C06-01.7 Given an exemption for a category, department, employee type, employee or date on a rule group, When an exempted employee-date is processed, Then that rule group is not applied to it and the explanation names the exemption.  (Source: G7, LA-22, EC-17, BR-08)
- AC-C06-01.8 Given a user of organisation A, When they try to view or change a policy set of organisation B by any screen, API, report or export, Then access is refused.  (Source: N1, TB-01, AT20)
- AC-C06-01.9 Given the Vendor Admin, When they open an organisation's attendance results, Then they cannot change them.  (Source: §5)

Edge and negative cases
- Rule group off part-way through a period: AC-C06-01.6.
- Settings conflicts with a V2 money rule (PD with LA-06/LA-10): warning belongs to PD (§10.3.4, V2); reference only.

### US-C06-02 Run the daily process with policy resolution
As an HR Manager, I want every employee-date processed through the same fixed 8 steps using the policy set in force that date, so that results are consistent and match each employee's own rules.
Source: §9.6 steps 1–8, §8 Layers 1–5, BR-05, G3, EM-R9, AT4, TB-03, AT21
Settings involved: policy set assignment levels (G3); SH-07 (default 3 h before, 6 h after)

Acceptance criteria
- AC-C06-02.1 Given an employee-date, When it is processed, Then the steps run in this order: resolve policy set; assign punches to the attendance date; clean; read into sessions; validate and apply the missed-punch rule or flag; calculate gross hours, breaks, net hours, late minutes, early minutes; apply overlays (leave, holiday, weekly off, on-duty, permission); set the day status and send exceptions to regularisation.  (Source: §9.6 steps 1–8)
- AC-C06-02.2 Given policy sets assigned at organisation, branch, department, category, shift and employee level, When the policy set is resolved for an employee-date, Then the most specific assignment wins.  (Source: G3, BR-05)
- AC-C06-02.3 Given a policy set version that changes on the 15th, When the 14th and the 15th are processed, Then the 14th uses the earlier version and the 15th uses the new version.  (Source: BR-05, G5)
- AC-C06-02.4 Given two employees with identical punches in different policy sets, When their days are processed, Then each result matches the employee's own policy set.  (Source: AT4)
- AC-C06-02.5 Given an employee punching at an allowed branch other than the master branch, When the day is processed, Then the employee's own policy set is used.  (Source: EM-R9)
- AC-C06-02.6 Given the daily processing job for a date, When it is run twice, Then the results are unchanged after the second run.  (Source: TB-03, AT21)
- AC-C06-02.7 Given C06 processing, When it reads inputs, Then it reads only punches (Layer 1), schedule (Layer 2) and overlays (Layer 4), and it produces days, minutes, marks, flags and an explanation, never a money amount.  (Source: §8, §21 note 1)

Edge and negative cases
- No policy set assignable at any level: organisation level is the least specific; a missing organisation default is not addressed by the BRD beyond G6 defaults — covered by AC-C06-01.1.
- Disabled module overlays (for example M02 off): BR-14, AC-C06-01.6.

### US-C06-03 Recalculate deterministically and keep the explanation
As a Payroll Officer, I want any new or changed input in an open period to recalculate the affected days and totals with the same result every time and a stored explanation, so that every figure can be trusted and explained.
Source: BR-02, BR-03, BR-08, BR-06, PC-R7, §8, §16, AT5, N3, O3
Settings involved: none specific

Acceptance criteria
- AC-C06-03.1 Given an open period, When a new punch, a late or out-of-order punch, an approved regularisation, a leave, permission or on-duty decision, a roster change or a policy set version change affects a day, Then that day and the period totals that depend on it are recalculated.  (Source: BR-03, PC-R7, §8)
- AC-C06-03.2 Given the same punches, schedule, overlays and policy set versions, When a day is reprocessed, Then the result is identical to the earlier one.  (Source: BR-02, AT5)
- AC-C06-03.3 Given any day result, When it is stored, Then it records status, hours, late and early minutes, marks, flags, the punches used, the rule and policy set version applied and any approval, so that each figure has an explanation.  (Source: BR-08, §16)
- AC-C06-03.4 Given a day is recalculated, When the new result is stored, Then the previous result and its explanation remain available, and the audit log records the change with the triggering input, user (or system) and time.  (Source: BR-08, §16 audit log)
- AC-C06-03.5 Given a locked period, When a new input arrives for a date in it, Then C06 does not change that period's results; the correction is handed to the period-lock adjustment process (C12).  (Source: BR-06, PL-R4; E15 reference only)
- AC-C06-03.6 Given the calculation path, When any result is produced, Then it comes from configured rules only, with no AI judgement.  (Source: BR-02)

Edge and negative cases
- Performance: a changed day is recalculated within seconds (N3, proposed; NFR file).

### US-C06-04 Assign punches to the attendance date and clean them
As an HR Manager, I want punches placed on the right attendance date and cleaned of repeats and tiny gaps, so that sessions are built from the correct facts without altering any raw punch.
Source: §9.6 steps 2–3, SH-07, SH-06, PR-02, PR-03, PR-04, BR-01, BR-11, PC-R6
Settings involved: SH-07 (default 3 h before start, 6 h after end); SH-06 (default shift start date); PR-02 (default 60 seconds); PR-03 (default first); PR-04 (default 5 minutes)

Acceptance criteria
- AC-C06-04.1 Given a shift 09:00–18:00 and SH-07 default, When punches fall between 06:00 and 24:00 of that date, Then they are assigned to that attendance date.  (Source: SH-07, §9.6 step 2)
- AC-C06-04.2 Given a night shift crossing midnight and SH-06 = shift start date, When its punches are assigned, Then all punches inside its window belong to the shift start date.  (Source: SH-06)
- AC-C06-04.3 Given two punches 40 seconds apart and PR-02 = 60 seconds, When cleaned, Then they count as one punch, and with PR-03 = first the earlier punch is the one kept.  (Source: PR-02, PR-03)
- AC-C06-04.4 Given an out punch followed by an in punch 3 minutes later and PR-04 = 5 minutes, When cleaned, Then the time is treated as continuous work.  (Source: PR-04)
- AC-C06-04.5 Given cleaning drops or merges punches, When the result is stored, Then every raw punch remains stored unchanged; only the day result records which punches were used.  (Source: BR-01, PC-R6, §16)
- AC-C06-04.6 Given punches at a branch in its own time zone, When times are compared with the shift, Then exact punch times stored in UTC are compared using the branch's time zone, with no rounding of punches.  (Source: BR-11, RD note)

Edge and negative cases
- Punch outside every shift window: not assigned to a working date; handling of punches on a weekly off, holiday or with no shift follows PC-06 (C04).
- Punch exactly at the window edge: BRD does not say whether the window is inclusive — covered by OQ-C06-09 (time granularity and boundaries).

### US-C06-05 Read punches into sessions using the configured reading mode
As an HR Manager, I want punches read into in/out sessions by the configured reading mode, so that direction never depends on counting odd or even punches alone.
Source: PR-01, PR-05, PR-06, SH-07, §9.6 step 4, BR-09
Settings involved: PR-01 (default paired); PR-05 (default 16 hours); PR-06 (default 12)

Acceptance criteria
- AC-C06-05.1 Given PR-01 = first-in/last-out, When a day has punches 09:00, 13:00, 14:00, 18:00, Then one session 09:00–18:00 is read.  (Source: PR-01)
- AC-C06-05.2 Given PR-01 = paired, When a day has punches 09:00, 13:00, 14:00, 18:00, Then sessions 09:00–13:00 and 14:00–18:00 are read, the direction of each punch being taken from the sequence within the shift window.  (Source: PR-01, SH-07)
- AC-C06-05.3 Given PR-01 = direction-tagged, When punches carry in/out tags, Then each punch's direction is its tag.  (Source: PR-01)
- AC-C06-05.4 Given any reading mode, When direction is decided, Then it is never decided by odd/even punch position alone; it comes from PR-01, the shift window and the sequence.  (Source: PR-01, SH-07)
- AC-C06-05.5 Given PR-05 = 16 hours, When a session would be longer than 16 hours, Then it is treated as a missed punch (US-C06-11), not as worked hours.  (Source: PR-05)
- AC-C06-05.6 Given PR-06 = 12, When a day has 13 punches after cleaning, Then the day is flagged.  (Source: PR-06)

Edge and negative cases
- Direction-tagged with two consecutive "in" tags: OQ-C06-15.
- First-in/last-out combined with break types that need break punches: OQ-C06-14.

### US-C06-06 Calculate breaks
As an HR Manager, I want break time handled by the configured break type, so that net hours reflect our break policy.
Source: BK-01..BK-09, §9.6 step 6
Settings involved: BK-01 (default fixed auto-deduct); BK-02 (default unpaid); BK-03 (default 60 minutes); BK-04; BK-05; BK-06 (default deduct from hours); BK-07 (default flag); BK-08 (default off); BK-09 (default off)

Acceptance criteria
- AC-C06-06.1 Given BK-01 = fixed auto-deduct, BK-02 = unpaid, BK-03 = 60, When an employee is in 09:00 and out 18:00, Then gross hours are 9 h 00 and net hours are 8 h 00.  (Source: BK-01, BK-02, BK-03, §13.1 assumptions)
- AC-C06-06.2 Given BK-02 = paid, When a break is taken, Then the break minutes are not deducted from net hours.  (Source: BK-02)
- AC-C06-06.3 Given BK-01 = flexible with cap and BK-05 cap 60 minutes, When punched breaks total 75 minutes, Then 15 minutes are excess and are treated per BK-06 (default: deducted from hours).  (Source: BK-05, BK-06)
- AC-C06-06.4 Given BK-06 = flag only, When there is excess break time, Then net hours are not reduced and the day is flagged.  (Source: BK-06)
- AC-C06-06.5 Given BK-06 = use permission balance, When there is excess break time, Then the permission balance (C09) is used for it.  (Source: BK-06)
- AC-C06-06.6 Given BK-01 = window with BK-04 window 13:00–14:00 and BK-07 = flag, When a break is punched 15:00–15:30, Then the day is flagged and no deduction beyond the normal break rule is made; with BK-07 = deduct, the break is deducted; with BK-07 = both, it is deducted and flagged.  (Source: BK-04, BK-07)
- AC-C06-06.7 Given BK-08 on with X minutes after Y hours, When no break is punched and Y hours are worked, Then X minutes are deducted; with BK-08 off, nothing is auto-deducted for a missing break punch (except under BK-01 = fixed auto-deduct).  (Source: BK-08)
- AC-C06-06.8 Given BK-09 on with lunch and tea breaks, When each break is evaluated, Then each uses its own rule.  (Source: BK-09)
- AC-C06-06.9 Given BK-01 = none, When the day is processed, Then no break is deducted.  (Source: BK-01)

Edge and negative cases
- BK-05 limit on number of breaks exceeded: BRD gives no outcome other than the cap — OQ-C06-11 also covers excess-break routing; number-of-breaks outcome included in OQ-C06-11.
- BK-06 = count as early-check-out minutes: OQ-C06-11.
- Excess break and LA-13 compensation on the same day: OQ-C06-01.

### US-C06-07 Evaluate late arrival for the day
As an HR Manager, I want each late arrival evaluated in the BRD's fixed order of checks, so that the employee gets exactly one consistent outcome per day.
Source: LA-01..LA-05, LA-15, LA-16, LA-17, LA-20..LA-23, order of checks for a late arrival (§12), BR-04, PM-19, PM-30, E1, E2, E3, E6, E10, E12, E14
Settings involved: LA-01 (default on); LA-02 (default shift start); LA-03 (default 10); LA-04 (default unlimited); LA-05 (default 11–120 = late mark, above 120 = half day); LA-15 (default suggest); LA-16 (default on); LA-17 (default on); LA-20 (default yes); LA-21 (default core-hours start where core hours exist, otherwise not applicable); LA-22; LA-23 (default no)

Acceptance criteria
- AC-C06-07.1 Given a late arrival, When it is evaluated, Then the checks run in this order and stop at the first that decides: (1) day covered by leave, on-duty, holiday, weekly off or exemption: not late; (2) expected start moved for half-day leave or approved permission; (3) within grace with grace uses remaining: on time; (4) approved regularisation or bulk waiver: waived; (5) conversion to a permission if enabled and balance available; (6) compensation by extra work if enabled and satisfied: waived; (7) slab action applied, minutes passed on for the money rule (PD-01, V2); (8) if the result is a late mark, it is counted for period-end penalties.  (Source: §12 order of checks)
- AC-C06-07.2 (E1) Given the §13.1 defaults, When the employee is in at 09:08, Then the result is on time, no mark.  (Source: E1, LA-03)
- AC-C06-07.3 (E2) Given the §13.1 defaults, When in 09:25, out 18:00, Then late minutes are 25, slab 11 to 120 applies, and the result is present with one late mark.  (Source: E2, LA-02, LA-05)
- AC-C06-07.4 (E3) Given LA-15 = automatic and 2 permissions left, When in 09:25, Then a 30-minute permission (09:00 to 09:30) is used, and the result is present, no late mark, 1 permission left.  (Source: E3, LA-15, PM-30)
- AC-C06-07.5 Given LA-15 = suggest (default), When an employee is late, Then the conversion is suggested to the employee and is not applied until the employee takes it up; until then the slab action stands.  (Source: LA-15)
- AC-C06-07.6 (E6) Given the §13.1 defaults and LA-23 = no, When in 11:30, out 18:00, Then late minutes are 150, slab above 120 applies, the result is half day present and half day absent, and it is not counted as a late mark.  (Source: E6, LA-05, LA-23)
- AC-C06-07.7 Given LA-23 = yes, When a slab cuts the day to half or full day, Then the day also counts as a late mark.  (Source: LA-23)
- AC-C06-07.8 (E10) Given an approved permission 10:00 to 11:00 and PM-19 default, When out 10:00 and back in 11:25, Then the extra 25 minutes count as late minutes and the result is present with one late mark.  (Source: E10, PM-19)
- AC-C06-07.9 (E12) Given first-half leave approved and SH-11 set to a fixed time so the second half starts 14:00, When in 14:20, out 18:00, Then late minutes are 20 against the adjusted start and the result is half day leave, half day present, one late mark.  (Source: E12, LA-20, SH-11, order step 2)
- AC-C06-07.10 (E14) Given LA-17 on and heavy rain on one date, When HR waives late arrival for one location for that date, Then all late marks for that date and location are cleared, the reason is logged, and the affected days and counters are recalculated.  (Source: E14, LA-17, BR-03, BR-08)
- AC-C06-07.11 Given LA-02 = end of grace and LA-03 = 10, When in 09:25, Then late minutes are 15.  (Source: LA-02)
- AC-C06-07.12 Given LA-04 = 2 uses per period and both uses already taken, When the employee arrives at 09:08, Then the arrival is not treated as on time under grace (order step 3 fails).  (Source: LA-04, order step 3)
- AC-C06-07.13 Given LA-01 = off, When an employee arrives late, Then no late minutes, marks or slab actions are produced.  (Source: LA-01, BR-14)
- AC-C06-07.14 Given a flexible shift with core hours and LA-21 default, When the employee arrives after core-hours start, Then late minutes are measured from core-hours start; without core hours, late is not applicable.  (Source: LA-21)
- AC-C06-07.15 Given LA-16 on, When the employee raises a late-arrival regularisation with a reason from the list and it is approved, Then the late is waived (order step 4) and the day recalculated.  (Source: LA-16, RG-02, order step 4)
- AC-C06-07.16 Given a day covered by full-day leave, on-duty, holiday or weekly off, When a punch shows a late time, Then no late is recorded.  (Source: order step 1, PM-R6)
- AC-C06-07.17 Given a late slab with action "deduct actual minutes" or "quarter day", When applied, Then the result is recorded as minutes or a quarter deduction day, never as money.  (Source: LA-05, §9.6 note)

Edge and negative cases
- Within grace after uses run out — what action, since 1–10 minutes are below the default first slab: OQ-C06-10.
- Second-level timing (09:10:30): OQ-C06-09.
- Slab "hold check-in until the manager approves" with a device punch: OQ-C06-07.
- Escalation alerts (LA-19) and counter reset (LA-18): US-C06-15.

### US-C06-08 Compensate late arrival by working extra
As an Employee, I want late minutes made up by extra work within the allowed window to waive the late, so that I am not penalised when I complete my time.
Source: LA-13, LA-14, order of checks step 6, E4
Settings involved: LA-13 (default off, 60 minutes); LA-14 (default same day)

Acceptance criteria
- AC-C06-08.1 (E4) Given LA-13 on and the §13.1 defaults, When in 09:25, out 18:30, Then 25 late minutes are covered by 30 extra minutes, the result is present with no late mark, and net hours are 8 h 05 (09:25 to 18:30 = 9 h 05, less the 60-minute break).  (Source: E4, LA-13)
- AC-C06-08.2 Given LA-13 on with a maximum of 60 minutes, When the employee is 75 minutes late and works 75 extra minutes, Then compensation is not satisfied for the full lateness and the slab action applies.  (Source: LA-13, order step 6)
- AC-C06-08.3 Given LA-13 on and LA-14 = same day, When late minutes are not made up the same day, Then the late is not waived by extra work on another day.  (Source: LA-14)
- AC-C06-08.4 Given LA-13 off (default), When in 09:25, out 18:30, Then the result is present with one late mark (as E2).  (Source: LA-13, E2)
- AC-C06-08.5 Given LA-14 = same week, When the week has not yet ended, Then the late result for the day is provisional and the day is recalculated when the week's extra work is known.  (Source: LA-14, BR-03)

Edge and negative cases
- Partial make-up (25 late, 10 extra) when the maximum is not exceeded: BRD says "if enabled and satisfied" — not satisfied means the slab applies on the full late minutes; covered by AC-C06-08.2 logic. Whether the residual minutes (15) or the full 25 go to the slab is not stated — OQ-C06-22.
- Extra minutes also counted as overtime or offsetting excess break: OQ-C06-01, OQ-C06-02.

### US-C06-09 Evaluate early check-out for the day
As an HR Manager, I want early check-outs evaluated in the mirrored order of checks, so that early exits are marked consistently and never confused with a missed punch.
Source: EC-01..EC-04, EC-12..EC-17, order of checks (mirrored), BR-09, PM-R6, E7, E9, E16
Settings involved: EC-01 (default on); EC-02 (default 5); EC-03 (default unlimited); EC-04 (default 6–120 = early mark, above 120 = half day); EC-12 (default off); EC-13 (default suggest); EC-14 (default off); EC-15 (default on for flexible, off for fixed); EC-16 (default on); EC-17 (same choices as LA-18..LA-22, set independently)

Acceptance criteria
- AC-C06-09.1 (E7) Given the §13.1 defaults, When in 09:00, out 17:20, Then early minutes are 40, slab 6 to 120 applies, and the result is present with one early mark.  (Source: E7, EC-04)
- AC-C06-09.2 Given EC-02 = 5, When out at 17:56, Then the exit is within grace and no early mark is recorded.  (Source: EC-02)
- AC-C06-09.3 (E9) Given an approved permission 16:00 to 18:00, When out at 16:02, Then the result is present, no early mark, permission time counts as worked, 1 permission left.  (Source: E9, PM-12, order step 2 mirrored)
- AC-C06-09.4 (E16) Given approved on-duty 14:00 to 18:00 at a client site, When in 09:00, out 13:55 and no further punch, Then the result is present, full day, no early mark, and the on-duty hours count as worked.  (Source: E16, PM-R6, order step 1 mirrored)
- AC-C06-09.5 Given an in punch and no out punch, When the day is processed, Then it is a missed punch (US-C06-11), never an early check-out.  (Source: BR-09)
- AC-C06-09.6 Given EC-15 on, When required net hours are already done before an early exit, Then no early mark is recorded.  (Source: EC-15)
- AC-C06-09.7 Given EC-14 on with "early mark" and no prior approval, When the employee leaves early, Then an early mark is recorded; with "half day", the day is a half day; with "flag for HR", the day is flagged.  (Source: EC-14)
- AC-C06-09.8 Given EC-12 on, When the employee had approved early arrival the same day covering the early minutes, Then the early check-out is waived.  (Source: EC-12, order step 6 mirrored)
- AC-C06-09.9 Given EC-13 = automatic and permission balance available, When the employee leaves early, Then the early check-out is converted to a permission and no early mark is recorded.  (Source: EC-13, PM-30)
- AC-C06-09.10 Given EC-16 bulk waiver on, When HR waives early check-out for a date and location or shift, Then all early marks for that date and location are cleared with the reason logged.  (Source: EC-16, BR-08)
- AC-C06-09.11 Given EC-04 slab action options, When an early slab is configured, Then "hold check-in until the manager approves" is not available.  (Source: EC-04)
- AC-C06-09.12 Given EC-01 = off, When an employee leaves early, Then no early minutes, marks or slab actions are produced.  (Source: EC-01, BR-14)

Edge and negative cases
- E16 net hours with a fixed 13:00–14:00 break and a 13:55 out before on-duty starts: OQ-C06-19.
- Early minutes from BK-06 = count as early-check-out minutes: OQ-C06-11.

### US-C06-10 Count early arrival
As an HR Manager, I want early check-in time counted as configured, so that arriving early does not add hours unless our policy allows it.
Source: EA-01, EA-02, OT-07, EC-12
Settings involved: EA-01 (default shift start); EA-02 (default 60)

Acceptance criteria
- AC-C06-10.1 Given EA-01 = shift start (default) and shift 09:00, When in 08:30, out 18:00, Then worked time counts from 09:00.  (Source: EA-01)
- AC-C06-10.2 Given EA-01 = punch time and EA-02 = 60, When in 07:30, Then worked time counts from 08:00 (at most 60 early minutes count).  (Source: EA-01, EA-02)
- AC-C06-10.3 Given EA-01 = punch time only when approved, When in 08:30 without approval, Then worked time counts from 09:00; with approval, from 08:30.  (Source: EA-01)

Edge and negative cases
- Early arrival as overtime: OT-07 default no; US-C06-16.

### US-C06-11 Handle missed punches
As an Employee, I want a missed punch flagged and reminded so that I can correct it, and as an HR Manager I want an unresolved one settled by rule, so that no day stays open forever.
Source: MP-01..MP-05, BR-09, BR-01, §9.6 step 5, E13, PR-05
Settings involved: MP-01 (default flag for regularisation); MP-02 (default flag for regularisation); MP-03 (default count completed pairs only); MP-04 (default on, 30 minutes); MP-05 (default off)

Acceptance criteria
- AC-C06-11.1 (E13) Given in 09:00 and no out, When MP-04 is on with 30 minutes, Then a reminder is sent at 18:30 and next morning; when the employee requests 18:05 and the manager approves, the result is present.  (Source: E13, MP-04, NT-01)
- AC-C06-11.2 (E13) Given the missed check-out is still unresolved at the cut-off, When the cut-off passes, Then MP-03 applies (default: count completed pairs only, absent if none).  (Source: E13, MP-03)
- AC-C06-11.3 Given MP-01 = flag for regularisation, When a day has an odd number of punches that cannot be read into complete sessions, Then the day is flagged and sent to regularisation.  (Source: MP-01, §9.6 step 8)
- AC-C06-11.4 Given MP-02 = half day, When a day has a single punch, Then the day is a half day; with absent, the day is absent.  (Source: MP-02)
- AC-C06-11.5 Given MP-05 = at shift end, When the check-out is missing, Then an auto check-out at shift end is applied and the day is always left flagged.  (Source: MP-05)
- AC-C06-11.6 Given a missing check-out, When any MP option is applied, Then no early minutes or early mark are produced.  (Source: BR-09)
- AC-C06-11.7 Given an approved missed-punch correction, When it is recorded, Then it is a new punch marked manual with reason and approver; the raw punches are unchanged.  (Source: BR-01)
- AC-C06-11.8 Given a late out-of-order punch that completes the pair arrives, When it is received in an open period, Then the day is recalculated and the flag cleared if resolved.  (Source: PC-R7, BR-03)

Edge and negative cases
- Meaning of "the cut-off": OQ-C06-03.
- MP-01 "auto-close at shift end" versus MP-05 auto check-out: OQ-C06-13.
- "Count completed pairs only" under shift-based status: OQ-C06-12.

### US-C06-12 Set the day status
As a Payroll Officer, I want each day's status set from the configured basis after overlays, so that payable days and deduction days are correct.
Source: DS-01..DS-07, DS-09, DS-10, §9.6 steps 7–8, E8, BR-04, LM-28
Settings involved: DS-01 (default shift-based for fixed, roster and split; hours-based for flexible and open); DS-02 (default required hours for hours-based; 75% for hybrid); DS-03 (default 50%); DS-04 (default 15 minutes); DS-05 (default half day); DS-06 (default off); DS-07 (default absent); DS-09 (default off); DS-10 (default pending for 3 days, then loss of pay)

Acceptance criteria
- AC-C06-12.1 Given DS-01 = shift-based, When there is a valid in and out, Then the day is present and late and early rules decide any deduction.  (Source: DS-01a)
- AC-C06-12.2 (E8) Given a flexible shift, 8 required hours, hours-based status, When in 10:15, out 18:30 with a 60-minute break, Then net hours are 7 h 15 and the shortfall is 45 minutes, above the 15-minute tolerance; with DS-05 = half day the result is half day; with DS-05 = deduct actual minutes the result is full day less 45 minutes; no separate late or early penalty.  (Source: E8, DS-01b, DS-04, DS-05)
- AC-C06-12.3 Given hours-based status and 8 required hours, When net hours are 7 h 50, Then the 10-minute shortfall is below DS-04 and the day is full.  (Source: DS-04)
- AC-C06-12.4 Given hours-based status, When net hours are below DS-03 (4 h 00 for 8 required hours) and DS-07 = absent, Then the day is absent; with DS-07 = pay in proportion to hours, the day is recorded in proportion to hours.  (Source: DS-03, DS-07)
- AC-C06-12.5 Given hybrid status with DS-02 = 75% of 8 required hours, When net hours are 5 h 30, Then the day is reduced below full.  (Source: DS-01c, DS-02)
- AC-C06-12.6 Given DS-06 on, When the configured basis yields a quarter-day result, Then a quarter-day status is used; with DS-06 off, no quarter-day status is used.  (Source: DS-06)
- AC-C06-12.7 Given DS-09 on for chosen employees, When they have no punches, Then the day is present.  (Source: DS-09)
- AC-C06-12.8 Given DS-10 default, When a working day is absent with no leave applied, Then it is held as pending for 3 days, then becomes loss of pay unless leave is applied and approved in that time.  (Source: DS-10)
- AC-C06-12.9 Given overlays (leave, holiday, weekly off, on-duty, permission), When the day status is set, Then overlays are applied before the status, and a day covered by full-day leave, holiday or weekly off takes that status.  (Source: §9.6 steps 7–8)
- AC-C06-12.10 Given a status with deduction days, When stored, Then it is in days (full, half, quarter) or minutes, with the rule that produced it, never as money.  (Source: §9.6 note, BR-08, §21 note 1)
- AC-C06-12.11 Given a day status of hours-based basis, When late or early events occurred, Then they are reported but carry no separate penalty.  (Source: DS-01b)

Edge and negative cases
- Hybrid: what "reduces the day" means and interplay with slab cuts: OQ-C06-08.
- DS-10 calendar vs working days; pending at lock: OQ-C06-05.
- DS-10 vs LM-28 overlap: OQ-C06-06.
- DS-05 = make up within the week: day result is provisional until week end and recalculated (BR-03); covered by AC-C06-03.1.

### US-C06-13 Round totals without touching punches or late/early minutes
As an HR Manager, I want optional rounding applied only to totals, so that exact times are preserved and late and early minutes are never distorted.
Source: RD-01..RD-04, RD note, BR-11
Settings involved: RD-01 (default off); RD-02 (default 15); RD-03 (default nearest); RD-04 (default both)

Acceptance criteria
- AC-C06-13.1 Given RD-01 off (default), When net hours are 8 h 05, Then they are stored as 8 h 05.  (Source: RD-01)
- AC-C06-13.2 Given RD-01 on, RD-02 = 15, RD-03 = nearest, RD-04 = net hours, When net hours are 8 h 05, Then the rounded total is 8 h 00; with RD-03 = up, 8 h 15; with RD-03 = down, 8 h 00.  (Source: RD-02, RD-03, RD-04)
- AC-C06-13.3 Given rounding on, When a day is processed, Then individual punch times are never rounded, the exact times remain stored, and late and early minutes are calculated from exact times before any rounding.  (Source: RD note, BR-11)
- AC-C06-13.4 Given rounding on, When in 09:25 (E2), Then late minutes remain 25 regardless of RD-02.  (Source: RD note, E2)
- AC-C06-13.5 Given a rounded total, When stored, Then both the exact and rounded totals and the rounding setting version are in the explanation.  (Source: BR-08, BR-11)

Edge and negative cases
- RD-04 overtime versus OT-06 overtime rounding: OQ-C06-17.
- Whether DS thresholds compare against the exact or rounded net hours: OQ-C06-23.

### US-C06-14 Regularise attendance exceptions
As an Employee, I want to request a correction for a missed punch, late arrival, early check-out, wrong punch or whole day, and as a Manager I want to approve it, so that genuine exceptions are fixed before the period is locked.
Source: RG-01..RG-11, §9.6 step 8, BR-01, BR-03, BR-07, BR-08, E13, LA-16, EC-16
Settings involved: RG-01 (default on); RG-02 (all types listed); RG-03 (default 3 overall); RG-04 (default 7 days and before lock); RG-05 (default one level, manager); RG-06; RG-07 (default escalate after 2 days); RG-08 (default HR approval only); RG-09; RG-10 (default on); RG-11 (default on)

Acceptance criteria
- AC-C06-14.1 Given RG-01 on and a flagged day, When the employee submits a request of an enabled type with a mandatory reason within 7 days and before lock, Then it goes to the approver.  (Source: RG-02, RG-04, RG-09)
- AC-C06-14.2 Given RG-04 = 7 days, When the employee submits on the 8th day after the date, or after the period is locked, Then the request is refused.  (Source: RG-04)
- AC-C06-14.3 Given RG-03 = 3 overall and RG-08 = HR approval only, When the employee raises a 4th request in the period, Then it goes to HR approval only; with RG-08 = block, it is refused.  (Source: RG-03, RG-08)
- AC-C06-14.4 Given RG-05 = two levels, When the manager approves, Then the request goes to HR and takes effect only after HR approves.  (Source: RG-05)
- AC-C06-14.5 Given RG-07 default, When the approver does not act within 2 days, Then the request is escalated.  (Source: RG-07)
- AC-C06-14.6 Given a request type listed in RG-06, When submitted, Then it is auto-approved.  (Source: RG-06)
- AC-C06-14.7 Given an approver who is also the requester, When they try to approve their own request, Then it is refused.  (Source: BR-07)
- AC-C06-14.8 Given an approved missed-punch or wrong-punch request, When it takes effect, Then a new manual punch with reason and approver is recorded, raw punches are unchanged, and the day is recalculated.  (Source: BR-01, BR-03)
- AC-C06-14.9 Given RG-10 on, When HR corrects a day directly with a reason, Then the correction is logged and the day is recalculated.  (Source: RG-10, BR-08)
- AC-C06-14.10 Given RG-11 on, When HR bulk-regularises a device outage for a date and branch, Then all affected days are corrected and recalculated with the reason logged.  (Source: RG-11, PC-R9)
- AC-C06-14.11 Given RG-01 off, When an employee tries to raise a regularisation, Then it is not available.  (Source: RG-01, BR-14)
- AC-C06-14.12 Given a rejected request, When the decision is recorded, Then the day result is unchanged and the decision is in the audit log.  (Source: §16)
- AC-C06-14.13 Given a manager, When they view regularisation requests, Then they see only their team's within their access scope.  (Source: §5, UM-R2, N5)
- AC-C06-14.14 Given an attachment mandatory for a type, When submitted without one, Then the request is refused.  (Source: RG-09)

Edge and negative cases
- RG-07 = auto-approve and BR-07: auto-approval is by rule, not by the requester; no conflict.
- Request after a pending item at lock: PL-05 (C12).

### US-C06-15 Apply period-end late and early penalties
As a Payroll Officer, I want count-based and minutes-based late and early penalties applied once at period end after regularisation is settled, so that penalties reflect final marks and are recovered in the configured order.
Source: LA-06..LA-12, LA-18, LA-19, EC-05..EC-11, EC-17, BR-10, BR-04, E5, TB-03
Settings involved: LA-06 (default off); LA-07 (default 0); LA-08 (default every 3rd); LA-09 (default half day); LA-10 (default off); LA-11 (default X = 60, deduct actual excess); LA-12 (default casual leave, then loss of pay); LA-18 (default payroll period); LA-19 (default on, 3 and 5); EC-05..EC-11 (same defaults, set independently); EC-10 (default separate)

Acceptance criteria
- AC-C06-15.1 (E5) Given LA-06 on, LA-07 = 0, LA-08 = every 3rd, LA-09 = half day, When there are 4 late marks in the period, Then one penalty of half a day is applied at the 3rd mark; it is recovered from casual leave, and if the casual-leave balance is zero, 0.5 day loss of pay.  (Source: E5, LA-06..LA-09, LA-12)
- AC-C06-15.2 Given LA-06 on with LA-07 = 2 and every 3rd, When there are 5 late marks, Then one penalty is applied (at the 5th mark).  (Source: LA-07, LA-08)
- AC-C06-15.3 Given count-based or minutes-based penalties on, When a day is still within its regularisation window or has a pending request, Then no penalty is applied for the period until regularisation is settled at period end.  (Source: BR-10)
- AC-C06-15.4 Given LA-10 on with X = 60 and deduct actual excess, When total late minutes in the period are 85, Then 25 minutes are deducted.  (Source: LA-11)
- AC-C06-15.5 Given LA-11 = a half day for every Y minutes with X = 60 and Y = 30, When total late minutes are 120, Then the excess is 60 minutes and two half days are deducted. (Treatment of a part block, for example 65 minutes, is not defined: OQ-C06-25.)  (Source: LA-11)
- AC-C06-15.6 Given LA-12 = permission balance first, When a penalty is due, Then it is recovered from the permission balance before the next option; with "none (report only)", it is reported and nothing is recovered.  (Source: LA-12)
- AC-C06-15.7 Given EC-10 = separate (default), When the period has 2 late and 2 early marks with every 3rd, Then no penalty is triggered on either counter.  (Source: EC-10)
- AC-C06-15.8 Given EC-10 = one combined counter, When the period has 2 late and 2 early marks with every 3rd, Then one penalty is triggered.  (Source: EC-10)
- AC-C06-15.9 Given LA-18 = payroll period, When a new period starts, Then late counters reset to zero; with calendar month, quarter or year, they reset on that boundary.  (Source: LA-18, EC-17)
- AC-C06-15.10 Given LA-19 on with 3 and 5, When an employee reaches the 3rd late mark, Then the manager is alerted; at the 5th, HR is alerted.  (Source: LA-19, NT-03)
- AC-C06-15.11 Given a penalty applied, When stored, Then it records the marks or minutes counted, the rule version, and the recovery used (leave type debited or loss of pay days); it is never a money amount produced by C06.  (Source: BR-08, §9.6 note)
- AC-C06-15.12 Given the period-end penalty job, When it is run twice for the same period, Then penalties are not applied twice.  (Source: TB-03, AT21)
- AC-C06-15.13 Given a late mark later waived by an approved regularisation before period end, When penalties run, Then it is not counted.  (Source: BR-10, order step 4)
- AC-C06-15.14 Given opening balances for late and early counters with an as-of date, When the first period is evaluated, Then counting starts from those balances.  (Source: OB-R2, OB-R4)

Edge and negative cases
- LA-12 default casual leave when no such leave type exists: OQ-C06-04.
- LA-09 / EC-08 "fixed amount" and LA-12 "fixed fine" in V1: OQ-C06-18.
- LA-06 and LA-10 both on for the same late minutes: OQ-C06-20.
- Combined counter with different LA and EC penalty settings: OQ-C06-21.

### US-C06-16 Never apply two pay effects for one shortfall
As a Payroll Officer, I want each shortfall to produce at most one pay effect, so that employees are never penalised twice for the same minutes.
Source: BR-04, AT6, LA-23, K4, PD note (V2 reference)
Settings involved: LA-23 (default no); EC equivalents via EC-17

Acceptance criteria
- AC-C06-16.1 Given a single shortfall to which two C06 rules would apply (for example a late slab half day and a hybrid hours floor on the same late minutes), When the day is processed, Then the larger effect applies once and the explanation names both rules and which one was applied.  (Source: BR-04, BR-08)
- AC-C06-16.2 (E6) Given LA-23 = no, When a slab cuts the day to half day, Then the same lateness is not also counted toward period-end count penalties.  (Source: E6, LA-23)
- AC-C06-16.3 Given hours-based status (DS-01b), When the day is reduced for a shortfall, Then late and early events of that day carry no separate penalty.  (Source: DS-01b, E8)
- AC-C06-16.4 Given the automated test suite, When all C06 cases run, Then no test case produces two pay effects for one shortfall.  (Source: AT6)

Edge and negative cases
- Comparing a day-fraction penalty with a minutes penalty: OQ-C06-20.
- Money deduction PD-01 alongside day-count penalties: V2 warning (PD note); reference only.

### US-C06-17 Identify overtime hours (computed only)
As a Payroll Officer, I want overtime hours identified from attendance each day, so that computed overtime is available in the period totals and export before overtime approval and pay arrive.
Source: §10.1 intro, OT-02, OT-03, OT-07, OT-R2 (computed kept separately), MD-06, §21 note 2, RD-04
Settings involved: OT-02 (default after shift end); OT-03 (default 30); OT-07 (default no)

Acceptance criteria
- AC-C06-17.1 Given OT-02 = after shift end, OT-03 = 30, shift end 18:00, When out at 18:40, Then overtime is identified for the day (whether 40 or 10 minutes count is OQ-C06-24).  (Source: OT-02, OT-03)
- AC-C06-17.2 Given OT-03 = 30, When out at 18:20, Then no overtime is identified.  (Source: OT-03)
- AC-C06-17.3 Given OT-02 = beyond daily required hours (8), When net hours are 8 h 45, Then overtime is identified against 8 required hours, not against shift end (amount subject to OQ-C06-24).  (Source: OT-02)
- AC-C06-17.4 Given OT-07 = no (default), When in 08:00 for a 09:00 shift, Then the hour before shift start is not identified as overtime.  (Source: OT-07)
- AC-C06-17.5 Given identified overtime, When stored, Then it is held as computed overtime only, separately from any approved overtime, and is included as overtime hours in the period totals and payroll input export.  (Source: OT-R2, §8 Layer 5, MD-06)
- AC-C06-17.6 Given V1, When overtime is identified, Then no approval, cap, outcome or money is produced.  (Source: §10.1 intro, §21 note 2)

Edge and negative cases
- Whether identification runs when OT-01 is off or M01 is not enabled: OQ-C06-16.
- Beyond weekly hours basis: identified at week end; day results recalculated (BR-03).
- Extra minutes used for LA-13 compensation also counted as overtime: OQ-C06-02.
- Overtime rounding RD-04 vs OT-06: OQ-C06-17.

## Open questions
- OQ-C06-01 When BK-06 = deduct from hours and LA-13 compensation is on, do extra minutes after shift end first make good the excess break deduction or the late minutes, or both? Source: BK-06, LA-13, BR-04. Why it matters: the same extra minutes could be counted twice or not at all.
- OQ-C06-02 Are extra minutes used to satisfy LA-13 (or approved early arrival for EC-12) also identified as overtime? Source: LA-13, EC-12, OT-02, OT-10 (V3). Why it matters: the same minutes could both waive a late and count as overtime.
- OQ-C06-03 What is "the cut-off" in MP-03: the PL-02 attendance cut-off date, the end of the RG-04 request window, or period lock? Source: MP-03, PL-02, RG-04, E13. Why it matters: decides when a missed punch is settled automatically.
- OQ-C06-04 LA-12 / EC-11 default is "casual leave, then loss of pay": what applies when the organisation has no casual leave type (or it is not eligible for the employee), and how does LA-12 relate to LV-04 leave debit order? Source: LA-12, EC-11, LV-04, LM-01. Why it matters: the default cannot run as written for every organisation.
- OQ-C06-05 DS-10 "pending for 3 days": calendar or working days, counted from the absent date or from processing, and what happens if the period is locked while a day is still pending? Source: DS-10, PL-05. Why it matters: decides when loss of pay is applied.
- OQ-C06-06 DS-10 and LM-28 both define what an absent day without leave becomes (loss of pay / auto-debit leave). Which setting governs, and does the DS-10 pending hold apply before LM-28? Source: DS-10, LM-28. Why it matters: two settings for one outcome can conflict.
- OQ-C06-07 LA-05 action "hold check-in until the manager approves" for a device punch: is the punch stored but not counted until approval, what is the day status meanwhile, and what is the result if the manager rejects or never acts? Source: LA-05, PC-R6, BR-01. Why it matters: a device cannot hold a punch; the outcome is undefined.
- OQ-C06-08 Hybrid status (DS-01c): what does "too few net hours still reduces the day" become (half day, deduct actual minutes, DS-05 options?), and when a late slab cut and the hours floor both apply, is it confirmed that the larger applies once under BR-04? Source: DS-01c, DS-02, DS-05, LA-05, BR-04. Why it matters: DS-05 is defined for hours-based only.
- OQ-C06-09 Exact times include seconds: how are late, early and shortfall minutes counted (for example in at 09:10:30 with 10 minutes' grace), and are shift-window and slab boundaries inclusive? Source: BR-11, LA-03, LA-05, SH-07. Why it matters: decides on-time vs late at the boundary.
- OQ-C06-10 When grace uses are exhausted (LA-04 / EC-03), an arrival 1–10 minutes late falls below the default first slab (11–120): is it a late mark, no action, or are slabs expected to be reconfigured? Source: LA-04, LA-05, EC-03, EC-04. Why it matters: LA-04 has no effect with default slabs otherwise.
- OQ-C06-11 BK-06 = count as early-check-out minutes: do these minutes go through EC grace and slabs and create an early mark, and what is the outcome when the BK-05 number-of-breaks cap is exceeded? Source: BK-05, BK-06, EC-02, EC-04. Why it matters: an excess break could create an early mark on a day with no early exit.
- OQ-C06-12 MP-03 / MP-01 "count completed pairs only" under shift-based status: is the day present (a valid in and out exists) or reduced by hours worked, given BR-09 forbids treating it as early check-out? Source: MP-01, MP-03, DS-01a, BR-09. Why it matters: same punches could give full day or partial day.
- OQ-C06-13 Is MP-01/MP-02 "auto-close at shift end" the same as MP-05 auto check-out at shift end, and is an auto-closed day also always left flagged? Source: MP-01, MP-02, MP-05. Why it matters: two overlapping settings may disagree.
- OQ-C06-14 With PR-01 = first-in/last-out, intermediate punches are not read: how are break types that need punched breaks (window, flexible, flexible with cap, BK-07) measured? Source: PR-01, BK-01, BK-04, BK-05, BK-07. Why it matters: the combination can be configured but has no defined result.
- OQ-C06-15 With PR-01 = direction-tagged, what happens when tags are inconsistent (two "in" in a row, or an "out" first)? Source: PR-01, MP-01. Why it matters: decides between flagging, pairing by sequence, or a missed punch.
- OQ-C06-16 In V1, is overtime identified when OT-01 is off (default) or M01 is not enabled, given MD-06 requires overtime hours in the export but MD-03/BR-14 say a disabled module's rules are not evaluated? Which policy holds OT-02, OT-03, OT-07 in V1? Source: §10.1 intro, OT-01, MD-03, MD-06, BR-14, §21 note 2. Why it matters: decides whether V1 produces overtime hours by default.
- OQ-C06-17 For identified overtime, does RD-04 (rounding applied to overtime, default nearest 15 when RD-01 on) or OT-06 (default down to 30 minutes) apply in V1? Source: RD-04, OT-06. Why it matters: two rounding rules for one total.
- OQ-C06-18 LA-09 / EC-08 "fixed amount" and LA-12 / EC-11 "fixed fine" are money, but attendance produces no money and Payroll is V2: are these options hidden in V1, or recorded as a count exported for payroll? Source: LA-09, LA-12, EC-08, EC-11, §21 note 1, MD-06. Why it matters: a V1 option cannot produce money.
- OQ-C06-19 E16: is the fixed 13:00–14:00 break still deducted when the employee leaves at 13:55 for on-duty starting 14:00, what are the net hours, and is the 5-minute gap treated as early minutes within EC-02 grace or ignored? Source: E16, BK-01, EC-02, PM-R6. Why it matters: the BRD states the status but not the hours.
- OQ-C06-20 When count-based (LA-06) and minutes-based (LA-10) penalties are both on, or a minutes-based excess overlaps a slab "deduct actual minutes", how is the "larger" effect determined under BR-04 when one is in day fractions and the other in minutes? Source: LA-06, LA-10, LA-05, BR-04. Why it matters: comparison needs a conversion the BRD does not define for V1.
- OQ-C06-21 With EC-10 = one combined counter, which penalty settings apply to the combined count (LA-06..LA-09 or EC-05..EC-08), and which recovery order (LA-12 or EC-11)? Source: EC-10, EC-11, LA-12. Why it matters: the settings may differ.
- OQ-C06-22 When LA-13 compensation is partial (for example 25 late, 10 extra minutes), does the slab apply to the full 25 late minutes or to the 15 not made up? Source: LA-13, LA-05, order step 6. Why it matters: changes the slab and the minutes passed to PD.
- OQ-C06-23 When RD-01 is on with RD-04 = net hours, do DS-02, DS-03 and DS-04 thresholds compare against exact or rounded net hours? Source: RD-04, DS-02..DS-04, RD note. Why it matters: rounding could turn a shortfall into a full day or the reverse.
- OQ-C06-24 OT-03 "minimum minutes before any overtime counts": once the 30-minute minimum is reached, do all minutes count (18:40 out = 40) or only those beyond it (= 10)? Source: OT-03. Why it matters: changes every identified overtime total.
- OQ-C06-25 LA-11 / EC-09 "a quarter or half day for every Y minutes": how is a part block of excess minutes treated (ignored, counted as a full block)? Source: LA-11, EC-09 (PD-04 defines this only for money). Why it matters: decides the deduction days at period end.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
| §8 Layers 1–5 | US-C06-02, US-C06-03 | AC-C06-02.7, AC-C06-03.1 |
| §9.6 steps 1–8 | US-C06-02 | AC-C06-02.1 |
| §9.6 step 2 / SH-07, SH-06 | US-C06-04 | AC-C06-04.1, AC-C06-04.2 |
| §9.6 step 3 / PR-02..PR-04 | US-C06-04 | AC-C06-04.3, AC-C06-04.4 |
| PR-01 | US-C06-05 | AC-C06-05.1..05.4 |
| PR-05 | US-C06-05 | AC-C06-05.5 |
| PR-06 | US-C06-05 | AC-C06-05.6 |
| BK-01..BK-09 | US-C06-06 | AC-C06-06.1..06.9 |
| LA-01..LA-05 | US-C06-07 | AC-C06-07.2, 07.3, 07.6, 07.11..07.13, 07.17 |
| LA-06..LA-12 | US-C06-15 | AC-C06-15.1..15.6, 15.11 |
| LA-13, LA-14 | US-C06-08 | AC-C06-08.1..08.5 |
| LA-15 | US-C06-07 | AC-C06-07.4, 07.5 |
| LA-16, LA-17 | US-C06-07 | AC-C06-07.10, 07.15 |
| LA-18, LA-19 | US-C06-15 | AC-C06-15.9, 15.10 |
| LA-20, LA-21, LA-22, LA-23 | US-C06-07, US-C06-01 | AC-C06-07.9, 07.14, 07.7, AC-C06-01.7 |
| EC-01..EC-04, EC-12..EC-17 | US-C06-09 | AC-C06-09.1..09.12 |
| EC-05..EC-11 | US-C06-15 | AC-C06-15.7, 15.8, 15.9 |
| EA-01, EA-02 | US-C06-10 | AC-C06-10.1..10.3 |
| MP-01..MP-05 | US-C06-11 | AC-C06-11.1..11.6 |
| DS-01..DS-07, DS-09, DS-10 | US-C06-12 | AC-C06-12.1..12.11 |
| RD-01..RD-04, RD note | US-C06-13 | AC-C06-13.1..13.5 |
| RG-01..RG-11 | US-C06-14 | AC-C06-14.1..14.14 |
| BR-02 | US-C06-03 | AC-C06-03.2, 03.6 |
| BR-03 | US-C06-03 | AC-C06-03.1, AC-C06-11.8, AC-C06-14.8 |
| BR-04 | US-C06-16 | AC-C06-16.1..16.4 |
| BR-05 | US-C06-02 | AC-C06-02.2, 02.3 |
| BR-08 | US-C06-03 | AC-C06-03.3, 03.4, AC-C06-15.11 |
| BR-09 | US-C06-09, US-C06-11 | AC-C06-09.5, AC-C06-11.6 |
| BR-10 | US-C06-15 | AC-C06-15.3, 15.13 |
| BR-11 | US-C06-04, US-C06-13 | AC-C06-04.6, AC-C06-13.3 |
| BR-14 | US-C06-01 | AC-C06-01.6, AC-C06-07.13, AC-C06-09.12 |
| Order of checks (late; early mirrored) | US-C06-07, US-C06-09 | AC-C06-07.1, AC-C06-09.3, 09.4, 09.8 |
| E1 | US-C06-07 | AC-C06-07.2 |
| E2 | US-C06-07 | AC-C06-07.3, AC-C06-13.4 |
| E3 | US-C06-07 | AC-C06-07.4 |
| E4 | US-C06-08 | AC-C06-08.1 |
| E5 | US-C06-15 | AC-C06-15.1 |
| E6 | US-C06-07, US-C06-16 | AC-C06-07.6, AC-C06-16.2 |
| E7 | US-C06-09 | AC-C06-09.1 |
| E8 | US-C06-12 | AC-C06-12.2 |
| E9 | US-C06-09 | AC-C06-09.3 |
| E10 | US-C06-07 | AC-C06-07.8 |
| E11 | — (C09 permissions) | — |
| E12 | US-C06-07 | AC-C06-07.9 |
| E13 | US-C06-11 | AC-C06-11.1, 11.2 |
| E14 | US-C06-07 | AC-C06-07.10 |
| E15 | reference only (C12) | AC-C06-03.5 |
| E16 | US-C06-09 | AC-C06-09.4 |
| OT §10.1 intro, OT-02, OT-03, OT-07 | US-C06-17 | AC-C06-17.1..17.6 |
| AT1 | all stories | every AC citing E1–E14, E16 |
| AT3 | US-C06-01 | AC-C06-01.6 |
| AT4 | US-C06-02 | AC-C06-02.4 |
| AT5 | US-C06-03 | AC-C06-03.2 |
| AT6 | US-C06-16 | AC-C06-16.4 |
