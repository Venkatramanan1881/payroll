# Business rules catalogue

Status: Provisional (BRD v0.7 draft)
Phase: 0. This file catalogues **fixed** rules, the **evaluation orders** and the **precedence** rules. Configurable settings are not copied here. The BRD is their single source, and §4 below indexes them.

ID scheme: rules keep their BRD IDs. Rules this analysis derives from BRD text get a `BRX-nn` ID with their source, and they need confirmation at Gate 1.

## 1. Fixed business rules (BRD §12, not configurable)

| ID | Rule | Release | Story coverage |
|---|---|---|---|
| BR-01 | Raw punches are never edited or deleted. A correction is a new punch marked manual, with a reason and an approver. | V1 | C04, C06 |
| BR-02 | All calculation is rule-based and repeatable. No AI judgement in the calculation path. | V1 | C06 |
| BR-03 | Any new or changed input for an open period recalculates the affected days and totals. | V1 | C04, C06, C08, C09 |
| BR-04 | No double penalty: one shortfall produces at most one pay effect; where two rules apply, the larger applies once. | V1 (day effects), V2 (money) | C06, C08 |
| BR-05 | The most specific policy assignment wins, using the version in force on the date. | V1 | C06 |
| BR-06 | A locked period and a closed pay run never change. A later correction posts only the difference to the next open period. | V1 (periods), V2 (pay runs) | C12 |
| BR-07 | Nobody approves their own request. | V1 | C03, C08, C09, C06 (RG) |
| BR-08 | Every pay-affecting result stores its explanation: punches, rule, version, approval. | V1 | C06 |
| BR-09 | A missing check-out is a missed punch, never an early check-out. | V1 | C06 |
| BR-10 | Count-based and minutes-based penalties are applied at period end, after regularisation is settled. | V1 | C06 |
| BR-11 | Times are stored in UTC with the branch's time zone. Exact times are stored; rounding affects totals only. | V1 | C04, C06 |
| BR-12 | Net pay is never negative. | V2 | — |
| BR-13 | The daily allowance is never part of the salary figure; a deduction set to come from the allowance never reaches the salary. | V3 | — |
| BR-14 | A disabled module or rule has no effect on any result. | V1 | V01, C06 |
| BR-15 | Money rounding: rates are never rounded inside a calculation; each amount is rounded to 2 decimals (round half up) when it becomes a line; totals are the sum of the lines; net pay is then rounded by PY-06; bank + cash always equals net pay (how the difference is assigned: Q18). Added in BRD v0.8; mode to confirm (Q17). | V2 | — (worked examples P1–P17 under AT1) |

## 2. Fixed rules stated inside settings sections

| ID | Rule | Source | Release |
|---|---|---|---|
| LM-27 | Excess-absence Y is on top of the days' normal effect. Informed and uninformed parts are counted, deducted and reported separately, and a day counts in one part only. A day whose leave is approved afterwards moves from uninformed to informed. An uninformed day is never cut under both LM-21 and LM-26; the larger applies. | §9.8 | V1 (days) |
| OD-12 | On-duty never uses the permission limit. | §9.9 | V1 |
| RD (note) | Rounding is applied to totals only, never to individual punches, and never before late or early minutes are calculated. | §9.6 | V1 |
| MP-05 (note) | Auto check-out always leaves the day flagged. | §9.6 | V1 |
| PC-R15 | Location is read only at the moment of a punch; no tracking between punches. | §9.4 | V1 |
| EM-R9 | Attendance follows the employee's own policy set wherever they punch (and from V2, their own payroll group). | §9.2 | V1 |
| MD-03 | A disabled module is hidden, its rules are not evaluated, its data is kept. | §4.2 | V1 |
| MD-04 | Module dependencies are enforced when switching. | §4.2 | V1 (checks), V2 (Payroll dependants) |
| G5 | A settings version in use is never edited; a change creates a new version with a start date. | §7 | V1 |
| G9 | Every settings change records who, when, old value, new value and reason. | §7 | V1 |
| OT (note) | Computed and approved overtime are stored separately; only approved overtime goes to payroll. | §10.1 | V1 stores computed; V3 approval |
| PD (note) | When PD-01 is on, the system warns if LA-06, LA-10, EC-05 or EC-09 are also on for the same group. | §10.3.4 | V2 |
| SP-11 | A weekly off that is not earned is unpaid. | §10.3.6 | V2 |
| SP-03 (note) | An extra shift paid under SP-03 is not also paid as overtime. | §10.3.6 | V2 |
| DA-06 | Daily allowance payout is worked out the next morning for the previous day. | §10.3.3 | V3 |

## 3. Evaluation orders (fixed)

### 3.1 Daily attendance process (BRD §9.6)
1. Resolve the policy set in force on that date (BR-05).
2. Assign punches to the attendance date using the shift window (SH-07; night shifts per SH-06).
3. Clean: drop repeat punches (PR-02, PR-03), merge very short gaps (PR-04).
4. Read punches into sessions using the configured mode (PR-01). Do not infer direction from parity alone.
5. Validate. Apply the missed-punch rule or flag the day (MP-01..03, BR-09).
6. Calculate gross hours, breaks, net hours, late minutes and early minutes.
7. Apply overlays: leave, holiday, weekly off, on-duty, permission.
8. Set the day status (DS). Send exceptions to regularisation and recalculate on approval.

Rounding (RD), where on, is applied to totals after step 6's late and early minutes are fixed.

### 3.2 Late arrival checks (BRD §12; early check-out uses the same order, mirrored)
1. Day covered by leave, on-duty, holiday, weekly off or an exemption: not late.
2. Expected start moved for half-day leave or an approved permission (LA-20).
3. Within grace, with grace uses remaining (LA-03, LA-04): on time.
4. Approved regularisation or bulk waiver (LA-16, LA-17): waived.
5. Conversion to a permission, if enabled and balance available (LA-15, PM-30).
6. Compensation by extra work, if enabled and satisfied (LA-13, LA-14): waived.
7. Slab action applied (LA-05); minutes passed to the money deduction rule if PD-01 is on (V2).
8. If the result is a late mark, it is counted for period-end penalties (LA-06..LA-12, BR-10).

### 3.3 Period end (BRD BR-10, §9.12)
1. Regularisation settled (RG-04 window, PL-05 handling of pending items).
2. Count-based and minutes-based penalties applied (LA-06..12, EC-05..11).
3. Excess absence and uninformed absence cuts applied (LM-21..27).
4. Period totals produced: payable days, deduction days, late and early minutes, overtime hours (§8 layer 5).
5. Lock (PL-04). After lock, only difference postings to the next open period (BR-06, PL-R4).

The order of steps 2 and 3 relative to each other is not stated in the BRD. See OQ-X-05.

### 3.4 Pay run (BRD §10.3.1), V2, recorded for completeness
Lock → pull inputs → calculate → review exceptions → approve → release payslips and lists → record payments and close.

## 4. Precedence

| Topic | Rule | Source |
|---|---|---|
| Policy assignment | Most specific wins, in this order: **employee > shift > department > category > branch > organisation** (D-03, 2026-11-16). | G3, BR-05, D-03 |
| Settings version | The version in force on the attendance date applies. | G5, BR-05 |
| Two penalties for one shortfall | The larger applies once. | BR-04, LM-27 |
| Disabled rule or module | No effect at all. | G2, BR-14, MD-03 |
| Exemption | An exempted employee, group or date is not evaluated by that rule group. | G7, LA-22 |

## 5. Index of configurable rule groups (source: BRD)

| Prefix | Rule group | BRD § | Release |
|---|---|---|---|
| VC | Vendor console | 4.3 | V1 |
| MD | Module control | 4.2 | V1 |
| OR | Organisation | 9.1 | V1 |
| EM | Employee master | 9.2 | V1 |
| AC | Access | 9.3 | V1 |
| PC | Punch collection | 9.4 | V1 |
| SH | Shifts and roster | 9.5 | V1 |
| PR, BK, LA, EC, EA, MP, DS, RD, RG | Attendance rules | 9.6 | V1 |
| HD | Holidays and weekly offs | 9.7 | V1 |
| LM, LV | Leave | 9.8 | V1 |
| PM, OD | Permissions and on-duty | 9.9 | V1 |
| PL | Period and lock | 9.12 | V1 / V2 |
| NT | Notifications | 9.14 | V1+ |
| OB | Opening balances | 9.15 | V1 |
| DB | Dashboards | 9.16 | V1 |
| HW | Comp-off | 10.2 | V1 / V2 |
| OT | Overtime | 10.1 | V1 hours / V3 |
| AL, PY, PG, DA, PD, CW, SP, CR, AE, RB, DC | Later modules | 9.10, 9.11, 10.3, 10.4 | V2 / V3 |

## 6. Derived rules needing confirmation

| ID | Derived rule | Derived from |
|---|---|---|
| BRX-01 | A raw punch carries no inherent direction; direction is assigned by the reading mode during processing and can change on recalculation without editing the punch. | §6 "Punch", PR-01, BR-01 |
| BRX-02 | Attendance (V1) outputs days, minutes, marks and flags only; it never outputs money. | §8, release note 1 |
| BRX-03 | A recalculated day keeps its previous result and explanation; the new result supersedes it, it does not overwrite it. | BR-08, O3, AT5 |
| BRX-04 | A flagged punch is stored in every case; whether it counts before approval depends on the flag (E18 shows a geo-fence flag that does not count until approved). | PC-04, E18; see C04 open questions |
