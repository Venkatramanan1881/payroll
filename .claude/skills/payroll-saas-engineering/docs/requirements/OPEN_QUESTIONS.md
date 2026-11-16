# Open questions register

Status: open. This register is the single list of unresolved decisions. A decision is closed only when the answer, the person who decided and the date are recorded here and the BRD/TSD (or an ADR) is updated.

Columns: ID · question · source · blocks (phase or release) · owner · status

## 1. Questions already in the source documents

### BRD v0.8 §20 (v0.8 changed only pay rounding; see its change note)
| ID | Question (short) | Blocks | Status |
|---|---|---|---|
| Q1 | Daily allowance earning logic (DA-02..04) | V3 | Open |
| Q2 | Leave encashment in V2? | V2 | Open |
| Q3 | Bank list formats first | V2 | Open |
| Q4 | Cash payout acknowledgement | V3 | Open |
| Q5 | Modules and limits per plan; which plans include the app | V1 (V01 stories) | Open |
| Q6 | Reimbursements in V2 | V2 | Open |
| Q7 | Tally version and format | V2 | Open |
| Q8 | First 2–3 real client-specific calculations | V2 design | Open |
| Q9 | Absconding default 7 days; working or calendar days | V1 (C08) | Open |
| Q10 | Extra shift detection | V2 | Open |
| Q11 | §15 targets acceptable (N2, N3, N8) | V1 (NFR) | Open |
| Q12 | Manual overtime pay in V2, or move overtime/daily allowance to V2 | V2 | Open |
| Q13 | Employee performance expectations | V3 | Open |
| Q14 | Who signs off the BRD | **Gate 1** | Open |
| Q15 | eSSL HTTPS capability (TB-04) | V1 (C04) | Open |
| Q16 | OTP via Indian SMS provider or Firebase | V1 (C03) | Open |
| Q17 | Confirm BR-15 rounding mode: half up (in v0.8) or half to even | V2 | Open (added in BRD v0.8) |
| Q18 | Bank/cash split: which portion takes the paise difference; is cash paid in whole rupees? | V2 | Open (added in BRD v0.8) |

### TSD v0.1 §20
| ID | Question (short) | Blocks | Status |
|---|---|---|---|
| D1 | eSSL HTTPS and contact rate (same as Q15) | V1 | Open |
| D2 | Sign-in codes provider (same as Q16) | V1 | Open |
| D3 | Which SMS and email provider | V1 | Open |
| D4 | Supabase Mumbai region on chosen plan | V1 (TB-07) | Open |
| D5 | Firebase App Hosting in Mumbai or plain Cloud Run | V1 | Open |
| D6 | Selfie retention (90 days proposed) | V1 | Open |
| D7 | Build pipeline tool | V1 | Open |
| D8 | Point-in-time recovery at launch or at Growth | V1 | Open |

## 2. Cross-cutting questions raised in Phase 0

| ID | Question | Source | Blocks | Status |
|---|---|---|---|---|
| OQ-X-01 | **Where does pay rounding happen?** The worked examples mix rounded and unrounded rates: per-day 576.92 and per-minute 1.20 are shown, but P6 (1,153.85) and P7 (3,461.54) use the unrounded 15,000/26, and P3 uses 1.2019. TB-05 says rounding only where a setting says so, but the only setting is PY-06 (net pay). Is each line amount rounded to 2 decimals, and how? | §13.2, TB-05, PY-06, AT1 | V2 (AT1 for P-examples) | **Closed 2026-11-15** by BRD v0.8: BR-15 (rates unrounded, each line rounded to 2 decimals, totals sum lines, then PY-06), TB-05 reworded, §13.2 working shown, P17 added. Rounding mode confirmation moved to Q17. |
| OQ-X-02 | LA-12's default "casual leave, then loss of pay" names a leave type that organisations define themselves (LM-01). What happens when the organisation has no such type? Is the default set by the sector template? | LA-12, LM-01, G6 | V1 (C06) | Open |
| OQ-X-03 | Policy-set precedence between branch, department, category and shift is not ranked. Which wins when an employee matches assignments at two of these levels? | G3, BR-05 | V1 (C06, domain) | Open |
| OQ-X-04 | PL-01 sets the period per payroll group, but payroll groups arrive in V2. How are V1 attendance periods defined: one per organisation, per branch, or per policy set? | PL-01, §21 | V1 (C12) | Open |
| OQ-X-05 | Order at period end between count/minutes penalties (LA/EC) and absence cuts (LM-21..27) when both reduce the same period's days. Does BR-04 apply across them? | BR-04, BR-10, LM-27 | V1 (C06, C08) | Open |
| OQ-X-06 | Both BRD v0.7 and TSD v0.1 are drafts. Phase 0 outputs stay Provisional until sign-off (Q14). | Headers | **Gate 1** | Open |
| OQ-X-07 | The N3 "within seconds" target needs a number for testing. | N3, Q11 | V1 (NFR) | Open |
| OQ-X-08 | Retention periods: N10 "as required by law", DC-07, D6 selfies, audit log, raw punches. One retention table is needed, reviewed against current legal requirements. | N10, K5, DC-07, D6 | V1 (privacy, database) | Open |
| OQ-X-09 | **Pub/Sub and Redis.** The decided stack (BRD §22.1, TSD §2) uses Cloud Scheduler and Cloud Tasks and has no Pub/Sub or Redis. The skill's defaults list both as "where required". No V1 requirement found so far needs them: recalculation fan-out goes through Cloud Tasks, and sessions, rate limits and dashboard summaries can live in Postgres (TSD §10, J11). Recommendation: leave both out of V1 and add either later through an ADR when a concrete need appears (for example cross-service events once a second backend service owns data). | BRD §22.1, TSD §2, §10 | Architecture (Phase 1) | **Closed 2026-11-15**: out of V1 per ADR-002 (Gate 2 part 1 approval) |

## 2a. Duplicate groups (answer once, close all)

Different writers raised the same question in different modules. One answer closes each group.

| Group | Question | Closes |
|---|---|---|
| G-A | How is a V1 attendance period defined without payroll groups, and what does "payroll period" / "the month" mean in V1 settings? | OQ-X-04, OQ-C12-01, OQ-C09-01, OQ-C08-01, OQ-C08-15, OQ-C09-12, OQ-C13-01, OQ-C15-06, OQ-C16-05 |
| G-B | Leave debit order for penalties and absences: LA-12/EC-11 vs LV-04, and the "casual leave" default | OQ-X-02, OQ-C06-04, OQ-C08-19 |
| G-C | Absent day with no leave: DS-10 vs LM-28 | OQ-C06-06, OQ-C08-12 |
| G-D | Does the Payroll Officer exist in V1 orgs without Payroll? Who locks? | OQ-C12-05, OQ-C16-03 |
| G-E | How is a work-from-home day requested and approved? | OQ-C04-14, OQ-C08-18 (part) |
| G-F | Escalation timers: do they count non-working time, and who is the target? | OQ-C08-09, OQ-C09-06, OQ-M02-05 |
| G-G | Device enrolment ID uniqueness (organisation, branch or device) | OQ-C04-04, OQ-C02-03 |
| G-H | Spreadsheet uploads with bad rows: apply valid rows or reject the file | OQ-C02-04, OQ-C05-08, OQ-C15-09 |
| G-I | Language fallback when a language is switched off or untranslated | OQ-C03-07, OQ-C14-08 |
| G-J | Which time zone and holiday calendar apply when punching at another branch | OQ-C04-03, OQ-C07-01 |
| G-K | Which "cut-off" applies to missed punches and pending requests at lock | OQ-C06-03, OQ-C12-03 |
| G-L | Absconding: working or calendar days | Q9, OQ-C08-17 |
| G-M | Retention periods (punch location, selfies, documents, audit) | OQ-X-08, OQ-C04-17 (part), D6 |
| G-N | Overtime identification in V1 when M01 / OT-01 is off | OQ-C06-16, OQ-C06-17 |

## 2b. Decide first (these shape the domain model in Phase 1)

1. Q14 / OQ-X-06: who signs off the BRD.
2. G-A: the V1 attendance period definition.
3. OQ-X-03: policy-set precedence.
4. G-J: time zone and calendar at another branch.
5. OQ-C04-01 / OQ-C04-02: which flagged punches count before approval, and who approves flags.
6. G-B and G-C: debit order and absent-day handling.
7. OQ-X-09: Pub/Sub and Redis.
8. Q15 / D1: eSSL HTTPS capability (blocks the device gateway design).

The rest can be answered during Phase 1 without blocking it.

## 3. Module questions

Raised in the user story files (`docs/product/USER_STORIES/*.md`, section "Open questions"). They are listed here for review in one place.

<!-- generated from the story files; edit there and regenerate -->

### C01-organisation-setup (8)
Source file: `../product/USER_STORIES/C01-organisation-setup.md`

- OQ-C01-01 Who may create and change branches, departments, designations, categories, employee types and contractors: only the Organisation Admin (§5 names it for branches), the HR Manager, or both? Source: §5, OR-R1, OR-R2. Why it matters: default role permissions under AC-01.
- OQ-C01-02 Can a branch be closed or deactivated, and what happens to employees whose master or allowed branch it is, its devices, its geo-fence and its open periods? Source: OR-R1, EM-R9. Why it matters: branch closure is common in retail and restaurants.
- OQ-C01-03 If a branch's time zone is changed after punches exist, are past punches and results shown in the old or new time zone, and are open days recalculated? Source: OR-R1, BR-11, BR-03. Why it matters: could shift attendance dates.
- OQ-C01-04 Can a department, designation, category, employee type or contractor that is in use (by employees, policy set assignments or exemptions) be removed or renamed, and how are history and reports kept? Source: OR-R2, OR-R3, OR-R5, EM-R6. Why it matters: dated history must stay readable.
- OQ-C01-05 How do employee type "probation" (OR-R3) and status "on probation" (EM-R5) relate: is one derived from the other, and does confirmation (EM-04) change the type? Source: OR-R3, EM-R5, EM-04. Why it matters: rules that exempt or target probationers could read different fields.
- OQ-C01-06 Must an employee of type "contract" be linked to a contractor, and may other types be linked? Source: OR-R4, EM-R2. Why it matters: validation on the employee master and upload.
- OQ-C01-07 Can the financial year and the leave year be set differently, and what happens if OR-01 is changed mid-year (accruals, carry forward, reports)? Source: OR-01, G5. Why it matters: OR-01 is a single setting in the BRD but names two years.
- OQ-C01-08 Which currencies and number formats are offered besides the Indian rupee, and is the default number format Indian grouping (1,00,000) or international (100,000)? Source: OR-04. Why it matters: option list and default format are not stated.

### C02-employee-master (15)
Source file: `../product/USER_STORIES/C02-employee-master.md`

- OQ-C02-01 Can a reporting manager be saved who is the employee themselves, an exited employee, or someone outside the employee's access scope? Source: EM-R2, BR-07, AC-02. Why it matters: approval routing depends on it.
- OQ-C02-02 With EM-01 = automatic with a branch prefix, does an employee's code change on transfer to another branch, on rejoining at another branch, or when the prefix is changed? Source: EM-01, EM-09, EM-05. Why it matters: codes appear on devices, reports and payslips.
- OQ-C02-03 Is the device enrolment ID unique per organisation, per branch or per device? Source: EM-R3, PC-R3. Why it matters: employees working at allowed branches may be enrolled on several devices.
- OQ-C02-04 For a spreadsheet upload with some bad rows (or rows over the employee limit when VC-01 = block), are the good rows applied and the bad ones reported, or is the whole file rejected? Source: EM-R7, VC-01. Why it matters: partial loads can leave a half-updated workforce.
- OQ-C02-05 Which pay details (payroll group, wage type, salary or rate, bank details, bank and cash split) are held in V1 when Payroll (V2) is not yet available, and are they needed for the V1 payroll input export? Source: EM-R4, MD-06, §21. Why it matters: decides V1 employee form and upload template.
- OQ-C02-06 Who receives the probation confirmation reminder, how many days before the end, and does confirmation change status automatically or only when HR acts? Source: EM-04, EM-R5. Why it matters: reminder content and status flow.
- OQ-C02-07 What happens to punches received for an employee after their exit date (stored and flagged, ignored for results)? Source: EM-R5, BR-01. Why it matters: devices may keep an exited employee enrolled.
- OQ-C02-08 Can an employee master change be back-dated into a locked period, and is the limit PL-07 (default 3 past periods)? Source: EM-R6, BR-06, PL-07. Why it matters: back-dated transfers and pay changes are common.
- OQ-C02-09 Can an HR Manager limited to chosen branches transfer an employee to a branch outside their scope? Source: EM-09, UM-R2, AC-07. Why it matters: scope enforcement on transfers.
- OQ-C02-10 Does "reviewed at the time of transfer" (EM-09) mean the transfer cannot be saved until shift, policy set and payroll group are each confirmed or changed, or is it a reminder only? Source: EM-09. Why it matters: blocking versus advisory flow.
- OQ-C02-11 Is EM-06 an organisation-wide setting or chosen per employee, and does "all branches" include branches created later? Source: EM-06, EM-R9. Why it matters: geo-fence and punch acceptance depend on the allowed list.
- OQ-C02-12 How is a day costed when the employee punches in at one branch and out at another, or when a punch at a non-allowed branch is approved; and does R10 "days worked by branch" follow EM-08 or always the branch worked? Source: EM-08, EM-07, EM-R10, E17. Why it matters: branch costing totals.
- OQ-C02-13 Can a custom field that holds data be removed, and is its data kept? Source: EM-R8. Why it matters: data loss.
- OQ-C02-14 For EM-05 "reuse old code": is the earlier employee record reopened (one continuous history) or is a new record created with the same code; and do leave balances, comp-off, counters and dated history carry over? Source: EM-05, EM-R6. Why it matters: history, balances and reports for rejoiners.
- OQ-C02-15 When an employee changes the mobile number used for sign-in (UM-R3) under EM-03 with approval, is the new number used for sign-in only after approval, and is the old number's session ended? Source: EM-03, UM-R3. Why it matters: account takeover risk and lock-out.

### C03-user-access (13)
Source file: `../product/USER_STORIES/C03-user-access.md`

- OQ-C03-01 Can the predefined §5 roles be deleted or renamed, and can their rights be changed beyond the §5 description (for example giving the Auditor edit rights, or the Vendor Admin's "cannot change results" for organisation roles)? Source: AC-01, §5. Why it matters: defines limits on AC-01.
- OQ-C03-02 When AC-06 is switched off, what happens to custom roles already created and the users holding them? Source: AC-06. Why it matters: users could lose all access.
- OQ-C03-03 Can one person hold more than one role (for example Manager and Employee, or HR Manager and Organisation Admin), and how does a Manager or HR Manager use employee self-service for their own punches and requests? Source: §5, UM-R3. Why it matters: affects sign-in method, scope and BR-07 checks.
- OQ-C03-04 Can AC-07 combine branch and department scope (for example branch A and department Bakery only), and does a Manager's scope default to their reporting team? Source: AC-07, UM-R2. Why it matters: AC-07 lists three exclusive-looking options.
- OQ-C03-05 How long is a one-time code valid, how many attempts are allowed, what is the lock-out, what happens when the same mobile number is on employee records in two organisations, and how do employees sign in on the web when the plan excludes the app? Source: UM-R3, MD-02, PC-R11, Q16. Why it matters: testable sign-in limits and multi-employer workers.
- OQ-C03-06 Which roles are "admin users" who sign in with a password (UM-R3) and get the AC-08 second step: all non-Employee roles, or only Organisation Admin, HR Manager, Payroll Officer and Auditor? What password rules, reset and lock-out apply? Source: UM-R3, AC-08. Why it matters: Managers and Supervisors may have only a phone.
- OQ-C03-07 If a language is turned off in OR-03 after users chose it, what language do they then see? Source: OR-03, UM-R4. Why it matters: users must not be left with no language.
- OQ-C03-08 Where does a request go when the routed approver is the requester (BR-07), or when the employee has no reporting manager: to the next level, to HR, or to a named fallback? Source: BR-07, AC-02. Why it matters: such requests would otherwise be stuck.
- OQ-C03-09 How is a "department head" (AC-02) identified: a field on the department, a designation, or a role? Source: AC-02, OR-R2. Why it matters: routing cannot be tested without it.
- OQ-C03-10 Who may set a delegation (the approver, HR, or both), must the delegate have the same access scope, what is the AC-03 default, and what happens to requests pending with the delegate when delegation ends? Source: AC-03. Why it matters: delegation can widen access.
- OQ-C03-11 Under AC-04 "with Organisation Admin approval", who approves when the same person holds both roles (BR-07), and does AC-04 also cover changes to AC-04 itself and to role rights (AC-01)? Source: AC-04, BR-07. Why it matters: one person could otherwise approve their own settings change.
- OQ-C03-12 Is an Employee user created automatically for every employee with a mobile number; what happens to a user's access when the employee exits; and may the HR Manager create and manage users, or only the Organisation Admin? Source: §5, UM-R3, EM-R5. Why it matters: access after exit and user administration rights.
- OQ-C03-13 What are the defaults for each AC-05 item (own punches, day status, counters, permission balance, reason for any deduction)? Source: AC-05, G6. Why it matters: G6 says every setting has a default but AC-05 states none.

### C04-punch-collection (17)
Source file: `../product/USER_STORIES/C04-punch-collection.md`

- OQ-C04-01 Before approval, does a punch flagged under "record and flag" count in the day's result for every flag type? E18 says a geo-fence-flagged punch is "not counted until the manager approves", but EM-07, PC-06 (allow and flag), PC-07 late sync, PC-08 clock mismatch, PC-13 and PC-14 do not say. Source: E18, PC-04, PC-06, PC-07, PC-08, PC-13, PC-14, EM-07. Why it matters: decides whether a day is present or absent (and later paid) while flags are pending.
- OQ-C04-02 Who approves each flagged punch type (E18 says the manager; AC-02 has no "flagged punch" request type), what happens when the approver does not act (no escalation setting like RG-07), and what a rejection does to the day given the punch can never be deleted (BR-01). Source: E18, AC-02, BR-01, PL-05. Why it matters: unresolved flags can block lock (PL-05 default) or silently change attendance.
- OQ-C04-03 When an employee punches at an allowed branch in a different time zone from the master branch, which time zone is used to store the punch, decide the attendance date and measure late and early: the punching branch (BR-11 "branch's time zone") or the master branch whose shift and rules apply (EM-R9)? Source: BR-11, EM-R9, SH-06. Why it matters: can move a punch to another date or create false late marks.
- OQ-C04-04 What happens to a punch from a device not registered to the organisation, or with an enrolment ID not mapped to any employee (stored and flagged for later mapping, or refused); and are enrolment IDs unique per organisation or per device/branch? Source: PC-R2, PC-R3, PC-R6. Why it matters: PC-R6 says every punch is stored, but an unattributed punch cannot be shown to any employee.
- OQ-C04-05 What does PC-08 "device clock mismatch" compare, given a late push legitimately has device time far from receipt time; and does it apply to phones (offline app punches with a changed phone clock) and to web punches? Source: PC-08, PC-07, N4. Why it matters: without a defined reference, every late-synced punch may also be flagged as clock mismatch, or changed phone clocks go undetected.
- OQ-C04-06 What does "block" mean for a biometric device punch under EM-07 = block or PC-06 = block, given the device has already captured it and BR-01 forbids deleting raw punches (stored but ignored, or never stored)? Source: EM-07, PC-06, BR-01, PC-R6. Why it matters: decides whether evidence of the attempt is kept.
- OQ-C04-07 PC-01 lists kiosk and file or API import as sources but no requirement describes them: are they in V1, and what happens to device pushes when the biometric device source is off, or to manual attendance when "entry by HR" is off during a device failure (PC-R9)? Source: PC-01, PC-R9, §21. Why it matters: scope and whether a source switch can make punches disappear.
- OQ-C04-08 Who is the approver BR-01 requires for manual attendance entered by the HR Manager under PC-R9, given BR-07 (nobody approves their own request)? Is the HR Manager's entry self-approved, or does it need a second person? Source: PC-R9, BR-01, BR-07, RG-06. Why it matters: affects the audit trail and whether manual entries count immediately.
- OQ-C04-09 For selfie punches (PC-R4), is the selfie only stored as evidence, or must it be matched to the employee's enrolled face, and with what outcome on no match? Source: PC-R4, PC-02, K5. Why it matters: decides whether a selfie prevents buddy punching or is only a record.
- OQ-C04-10 What happens when the app cannot obtain location (permission refused or no fix) while PC-02 location = on? PC-13 covers web only. Source: PC-02, PC-13, PC-R5. Why it matters: app and web must behave the same at the same spot (AT19), but the app has no rule.
- OQ-C04-11 With PC-15 = either may pass, PC-13 = block and the IP check passing, is a web punch without location accepted or blocked? Source: PC-13, PC-15, PC-05. Why it matters: the two defaults point in opposite directions.
- OQ-C04-12 Who or what sets an employee's enrolment status to "re-enrolment needed" (HR by hand, the device, repeated failures)? Source: PC-R8. Why it matters: decides whether this is a manual field or an automatic signal.
- OQ-C04-13 Is there a limit on how late an offline or device punch may arrive and still be applied (for example beyond PL-07 past periods, or after the employee has exited)? Source: PC-R7, PC-07, PL-07, BR-06. Why it matters: PC-07 only flags; a very old punch could post adjustments beyond the correction window.
- OQ-C04-14 How is a work-from-home day requested and approved (PC-09 "approved WFH days"), since LV-03 only switches WFH on and C09 covers only permissions and on-duty? Source: PC-09, LV-03, PM-R5. Why it matters: PC-09 default depends on an approval that no module defines.
- OQ-C04-15 When a fence is changed or switched on, are punches already flagged under the old fence re-evaluated, or does the change apply only to new punches? Source: PC-R14, G5, BR-03. Why it matters: affects pending approvals and recalculation.
- OQ-C04-16 When OD-07 = app check-in at the on-duty place with location, how is the place checked (no fence exists for it), and is any distance stored? Source: OD-07, PM-R7, PC-17, PC-R13. Why it matters: PC-R13 requires a "distance from the nearest allowed fence" that is meaningless for an on-duty place.
- OQ-C04-17 Which roles may see punch location and selfie photos, and for how long are they kept? Source: K5, N9, N10, AC-05. Why it matters: location and face images are sensitive personal data needing a privacy review.

### C05-shifts-roster (12)
Source file: `../product/USER_STORIES/C05-shifts-roster.md`

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

### C06-attendance-rules (25)
Source file: `../product/USER_STORIES/C06-attendance-rules.md`

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

### C07-holidays-weekly-offs (8)
Source file: `../product/USER_STORIES/C07-holidays-weekly-offs.md`

- OQ-C07-01 Which holiday calendar applies to an employee who punches at an allowed branch: the master branch's calendar (EM-R9 says the employee's own rules apply wherever they punch) or the calendar of the branch worked? And is SH-10 "one per location" the same as one per branch (HD-R1, OR-R1)? Source: HD-R1, SH-10, OR-R1, EM-R9. Why it matters: decides whether the day is a holiday, a working day, or holiday work earning comp-off.
- OQ-C07-02 Are half-day holidays supported? Source: HD-R1. Why it matters: the BRD is silent; some festivals are observed for half a day.
- OQ-C07-03 For SH-09 = from roster, is a minimum number of weekly offs per week required, and what happens in a week where the roster gives none? Source: SH-09, HD-R2. Why it matters: every day without an off becomes a working day and may trigger absence cuts.
- OQ-C07-04 For optional holidays: does the employee's pick need approval, by when must it be made or changed, can it be picked after the date, and is "per year" the calendar year, financial year or leave year (OR-01)? Source: HD-R3, HD-01, OR-01. Why it matters: decides whether an absence on an optional holiday is a holiday or an absence.
- OQ-C07-05 For HD-02 = move to next working day: what if the next working day is also a holiday or weekly off; with weekly offs from the roster (or alternating), is "next working day" worked out per employee, since rostered offs differ by person; and does HD-02 apply to optional holidays? Source: HD-02, SH-09, HD-R3. Why it matters: the moved holiday can land on different dates for colleagues in one branch.
- OQ-C07-06 For HD-03, what counts as "presence" on the working day before and after (half day, approved leave, on-duty, permission, a flagged punch), and what does an unpaid holiday become: absent, loss of pay, or a deduction day? Source: HD-03, §6 Deduction days. Why it matters: decides the pay effect and the BR-04 no-double-penalty check.
- OQ-C07-07 When HD-03 and the sandwich policy (LM-16) are both on and the employee is absent on both sides of a holiday, which applies, and how is a double effect avoided (BR-04)? Source: HD-03, LM-16, LM-17, BR-04. Why it matters: both rules can cut the same holiday.
- OQ-C07-08 For hourly workers with HD-04 or HD-05 = yes, how many hours is a paid holiday or weekly off worth (SH-03 required hours, a fixed number, an average)? Source: HD-04, HD-05, SH-03. Why it matters: the export must carry payable hours, not only days, for hourly workers.

### C08-leave (19)
Source file: `../product/USER_STORIES/C08-leave.md`

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

### C09-permissions-on-duty (13)
Source file: `../product/USER_STORIES/C09-permissions-on-duty.md`

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

### C12-period-lock (11)
Source file: `../product/USER_STORIES/C12-period-lock.md`

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

### C13-reports (5)
Source file: `../product/USER_STORIES/C13-reports.md`

- OQ-C13-01 In V1, what does the "period" filter mean: the attendance period from C12 (see OQ-C12-01), a calendar month, or any date range? Source: §14, PL-01. Why it matters: totals differ when the period is a custom cycle.
- OQ-C13-02 For a locked period that later received corrections, do R1–R8 show the values as locked, or the corrected values with the adjustment noted? Where are V1 adjustments listed, given R23 is V2? Source: BR-06, PL-R4, R23. Why it matters: HR may see two different numbers for the same day.
- OQ-C13-03 When a rule group inside an enabled module is switched off (for example LM-21, LM-24, LA-06), are the related report columns hidden, or shown empty? Source: §14, G2, BR-14. Why it matters: §14 only speaks of disabled modules.
- OQ-C13-04 What are the default report permissions for the standard roles (Manager, Supervisor, Employee, Auditor)? Can a Manager or Employee run R1–R9 for their own team or self? Source: §5, AC-01, AC-05. Why it matters: AC-01 makes it configurable but sector templates need defaults.
- OQ-C13-05 Are reports and exports shown in the user's language, given N7 names only the employee app, payslips and notifications? Source: N7, UM-R4. Why it matters: Tamil/Hindi-speaking HR users.

### C14-notifications (9)
Source file: `../product/USER_STORIES/C14-notifications.md`

- OQ-C14-01 What are the default on/off values per event, channel and recipient for NT-01 to NT-11 (and per sector template)? Source: §9.14, G6. Why it matters: G6 says every setting has a default; §9.14 gives none.
- OQ-C14-02 TSD §14 adds push notifications (Firebase), but BRD §9.14 lists only in-app, email, SMS and WhatsApp. Is push a channel with its own switch, or part of in-app? Source: §9.14; TSD §14. Why it matters: settings screen and costs.
- OQ-C14-03 Thresholds: when is a count "nearing a penalty" (NT-03); when is a leave balance "low" and how far ahead is "about to lapse" (NT-11); does NT-11 also cover comp-off expiry (HW-05)? Source: NT-03, NT-11, HW-05. Why it matters: these are rules not defined anywhere.
- OQ-C14-04 Who receives NT-09 in V1, where the Payroll Officer role belongs to a module that arrives in V2 (see OQ-C12-05), and how many days before the lock or cut-off is it sent? Source: NT-09, §5, PL-04. Why it matters: reminders may go to nobody.
- OQ-C14-05 When is NT-02 sent (at the punch, or after the day is finalised overnight), and is a follow-up sent if the mark is later removed by regularisation, waiver (E14) or recalculation? Source: NT-02, BR-03, E14. Why it matters: employees may act on marks that later disappear.
- OQ-C14-06 Do alerts defined in other modules — LA-19 / EC-17 escalation, PC-R10 / PC-10 device offline, LM-31 absconding, RG-07 / LM-13 / PM-17 escalation, VC-02 renewal, EM-04 confirmation reminder — follow the §9.14 per-channel, per-recipient switches, or are they always on? Source: §9.14, LA-19, PC-10, LM-31, VC-02, EM-04. Why it matters: not in the NT list, so their channels are undefined.
- OQ-C14-07 Are SMS and WhatsApp available to every V1 organisation or only by plan, and what happens when a recipient has no email or mobile number for an enabled channel? Source: §9.14, MD-02, Q5; TSD §14. Why it matters: per-message cost and silent delivery failures.
- OQ-C14-08 Which language is used when the recipient's chosen language is not offered by the organisation (OR-03) or a message has no translation yet? Source: N7, OR-03, UM-R4. Why it matters: AT14 test cases.
- OQ-C14-09 With delegation on (AC-03), does the delegate receive NT-05 and NT-06 for the absent approver? Source: AC-03, NT-05, NT-06. Why it matters: approvals may stall.

### C15-opening-balances (9)
Source file: `../product/USER_STORIES/C15-opening-balances.md`

- OQ-C15-01 OB-R2 says "permissions used this month", but PM-08 lets the permission limit period be a week, payroll period, quarter or year. When PM-08 is not the calendar month, what is entered? Source: OB-R2, PM-08. Why it matters: wrong opening usage gives wrong permission limits.
- OQ-C15-02 What expiry date does an opening comp-off balance get, since HW-05 counts from the day earned and that date is before the system started? Source: OB-R2, HW-05, HW-06. Why it matters: comp-off may lapse at once or never.
- OQ-C15-03 What validation applies to opening values: negative leave balances (allowed only up to LM-07?), balances above the maximum (LM-05), leave types the employee is not eligible for (LM-01), counters above the penalty trigger? Source: OB-R3, LM-01, LM-05, LM-07. Why it matters: the error report needs defined rules.
- OQ-C15-04 For a change after confirmation (OB-02), if the as-of date is inside a locked period or the balance has since been used, is the effect recalculated forward from the as-of date or posted as a difference to the next open period (BR-06, PL-R4)? Source: OB-02, OB-R5, BR-03, BR-06. Why it matters: a balance change could otherwise alter a locked period.
- OQ-C15-05 How are the days around the as-of date treated: is accrual for the part-period after the as-of date pro-rated (LM-04), and are punches or days before the as-of date calculated at all? Source: OB-R4, LM-03, LM-04. Why it matters: double or missing accrual in the start month.
- OQ-C15-06 "Late and early counters for the current period": which period is meant in V1 (see OQ-C12-01), and what is entered when LA-18 / EC-17 reset by quarter or year? Source: OB-R2, LA-18, EC-17. Why it matters: count-based penalties depend on it.
- OQ-C15-07 Under OB-01 "HR Manager with Organisation Admin approval", does the approval apply to entry, to confirmation, or both, and do changes under OB-02 also need approval? Source: OB-01, OB-02. Why it matters: permission ACs.
- OQ-C15-08 Are opening balances used in calculations before they are confirmed, or only from confirmation (still effective from the as-of date)? Source: OB-R4, OB-R5. Why it matters: leave requests made during set-up.
- OQ-C15-09 On spreadsheet upload, are valid rows saved when other rows fail, or is the whole file rejected? Source: OB-R3, EM-R7. Why it matters: partial loads can leave mixed data.

### C16-dashboards (6)
Source file: `../product/USER_STORIES/C16-dashboards.md`

- OQ-C16-01 Which dashboard does a user with a custom role (AC-06), or a user holding more than one role (for example Manager who is also an Employee), land on? Source: DB-R1, AC-06, AT17. Why it matters: AT17 test cases for real users.
- OQ-C16-02 The Auditor role (§5) has no dashboard in DB-R2 to DB-R7. Does the Auditor get a dashboard, and with what widgets? Source: §5, DB-R1, AT17. Why it matters: AT17 says each role lands on its own dashboard.
- OQ-C16-03 Does the Payroll Officer role exist in V1 organisations (Payroll is V2)? If not, do the V1 lock widgets of DB-R4 go to the HR Manager dashboard? Source: DB-R4, §5, PL-04 (see OQ-C12-05). Why it matters: lock work may have no dashboard.
- OQ-C16-04 How current must live widgets be ("who is in now", "today's present, absent, late")? TSD J11 refreshes every 15 minutes. And when during the day is an employee counted absent (before shift start, after the shift window, after finalisation)? Source: DB-R3, DB-R6; TSD §10 J11. Why it matters: Supervisors staff the floor from these numbers.
- OQ-C16-05 What do "late and absence trends" (DB-R5) cover — how many periods, by day, week or period? Is the Employee's "this month's attendance" (DB-R7) the calendar month or the attendance period (OQ-C12-01)? Source: DB-R5, DB-R7, PL-01. Why it matters: undefined widget content.
- OQ-C16-06 What widgets does each role see before the Organisation Admin makes a choice under DB-01, and who chooses the vendor dashboard widgets? Source: DB-01, G6, VC-05. Why it matters: G6 requires a default for every setting.

### M02-comp-off (10)
Source file: `../product/USER_STORIES/M02-comp-off.md`

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

### V01-vendor-console (17)
Source file: `../product/USER_STORIES/V01-vendor-console.md`

- OQ-V01-01 What happens to existing employees when the employee limit is lowered below the current active headcount (nothing until the next addition, a block on further additions only, or something else), and is a limit of zero or "no limit" allowed? Source: VC-01, VC-R2. Why it matters: downgrades must not silently deactivate employees or stop their punches.
- OQ-V01-02 Which modules and limits belong to each plan (BRD Q5), and may an organisation's modules or limit differ from its plan (VC-R1 lists plan, modules and limit separately)? Source: MD-02, VC-R1, Q5. Why it matters: decides whether module control is per plan or per organisation.
- OQ-V01-03 MD-04 says dependencies are "enforced when switching". When the prerequisite (Payroll) is switched off while a dependent module is on, is the switch blocked, or are dependants switched off too? Source: MD-04. Why it matters: different user flows and data effects.
- OQ-V01-04 Does a module change (on, off, plan change) take effect immediately or from a chosen date, and does switching a module off mid-period recalculate already-processed open days? Source: MD-03, VC-R2, AT2, BR-03. Why it matters: AT3 gives effective dates for rule groups but AT2 is silent for modules.
- OQ-V01-05 Can the Organisation Admin or Auditor of an organisation see the vendor's history of plan, module and limit changes for their organisation? Source: VC-R2, §5 (Auditor). Why it matters: transparency versus vendor-only data.
- OQ-V01-06 Which employee statuses count toward the employee limit (active, on probation, on notice; exited presumably not), and does a rejoining employee count as a new employee? Source: VC-01, VC-R3, EM-R5, EM-05. Why it matters: the limit check result depends on it.
- OQ-V01-07 "Last sign-in" in the usage view: of any user, of the Organisation Admin, or per role? Source: VC-R3. Why it matters: defines the usage metric.
- OQ-V01-08 While an organisation is suspended, are app and web punches accepted (VC-R4 says "punches are still stored" but users "can only view and export"), and do timed jobs and recalculation keep running? Source: VC-R4, PC-R5, PC-R11, TB-03. Why it matters: employees without devices may lose attendance, or results may be stale on reactivation.
- OQ-V01-09 For VC-04 automatic suspension, what counts as "not renewed" (billing status value?), and is the suspension on day N or day N+1 after the renewal date? Source: VC-04, VC-R5. Why it matters: automatic suspension must not fire on a paid client.
- OQ-V01-10 Does a change to a sector template affect organisations already created from it, and can the vendor add new sector templates beyond the four in G6? Source: VC-R7, G6. Why it matters: live clients' settings could change without HR action.
- OQ-V01-11 Who in the organisation receives the VC-02 renewal reminder (Organisation Admin?), through which channel, and what are the billing status values? Source: VC-02, VC-R5. Why it matters: reminder routing and the VC-04 trigger.
- OQ-V01-12 During support access, may the vendor user only view, or also change settings and data? §5 says the Vendor Admin "cannot change an organisation's results", while VC-R6 says "with every action logged". Source: VC-R6, §5. Why it matters: decides whether support access is read-only.
- OQ-V01-13 How long is the support access time limit, who sets it, and can the Organisation Admin end it early? Source: VC-R6, VC-03. Why it matters: testable expiry.
- OQ-V01-14 What can the VC-05 "support" vendor role do that "read-only" cannot, and which vendor roles may request support access? Source: VC-05, VC-R6. Why it matters: vendor-side permission matrix.
- OQ-V01-15 What counts as a "device sync failure" in the health view, and when is a period "waiting to be locked" (past period end, past cut-off PL-02)? Source: VC-R8, PC-10, PL-02. Why it matters: health view content is not testable without it.
- OQ-V01-16 Which users inside a chosen organisation receive a vendor announcement (all users, admins only, chosen roles), and through which channel (in-app, email)? Source: VC-R9, C14. Why it matters: reach and channel cost.
- OQ-V01-17 How do vendor users sign in (UM-R3 covers employees and admin users of organisations only)? Is a second step mandatory for them? Source: UM-R3, VC-05. Why it matters: vendor accounts reach every organisation through support access.

## 4. Phase 1/2 gaps

Raised while modelling. Each is listed in full in its source file. Many repeat a module question already above, which is noted in the source file.

- **GAP-SM-01 … GAP-SM-17**: lifecycle gaps. See `../domain/STATE_MACHINES.md` §15.
- **G-01 … G-40**: permission gaps (72 "?" cells). See `../domain/PERMISSION_MATRIX.md` gaps list.
- **DP-01 … DP-12**: model decision points. See `../domain/DOMAIN_MODEL.md` §11 (DP-12 closed by ADR-002).

Impact on Phase 3: permission gaps change **default role grants (seed data)**, not table structure, because role rights are configurable data (AC-01). Lifecycle gaps GAP-SM-06/08/12/16/17 and DP-01/03/05 change structure and are flagged in the data model where they apply.
