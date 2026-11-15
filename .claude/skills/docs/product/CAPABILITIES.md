# Product capability map

Status: Provisional (BRD v0.7 and TSD v0.1 are drafts, not yet approved)
Sources: BRD v0.7 §1–5, §8, §21; TSD v0.1 §2–3
Phase: 0 (requirements analysis). No schema, API or code is implied by this document.

## 1. What the product is

A multi-tenant platform that turns raw punches into attendance results, and from version 2, into pay. Punches are the only raw facts (BRD §1). Every other result is produced by rules that each organisation configures on top of them (BRD §8):

| Layer | Produces | Release |
|---|---|---|
| 1 Punches | Raw, immutable punch events | V1 |
| 2 Schedule | Expected work: shift, roster, weekly offs, holidays | V1 |
| 3 Attendance rules | One result per employee per date | V1 |
| 4 Overlays | Leave, permission, on-duty, comp-off applied to the day | V1 |
| 5 Period totals | Payable days, deduction days, late/early minutes, overtime hours | V1 (exported as payroll input, MD-06) |
| 6 Pay rules | Money | V2 |
| 7 Recoveries and payment | Advances and loans recovered; bank and cash split | V2 |

Each layer reads only the layers below it. A correction recalculates the layers above it while the period is open (BRD §8).

## 2. Two levels of control (BRD G1, §4.2)

- **Vendor:** switches modules per organisation (MD-01), sets the plan and employee limit (MD-02), builds client-specific payroll rules (V2).
- **Organisation HR:** switches each rule group inside an enabled module (MD-05, G2) and sets every option. Settings are versioned and effective-dated (G5) and grouped into policy sets (G3) and payroll groups (G4, V2).

## 3. Actors (BRD §5)

| Actor | Scope | Main capabilities |
|---|---|---|
| Vendor Admin | Platform | Create organisations and plans, switch modules, maintain templates, health view. Cannot change an organisation's results. |
| Vendor support / read-only (VC-05) | Platform | Support access (time-limited, logged and visible to the organisation, VC-R6), read-only views |
| Organisation Admin | Organisation | Users, roles, branches; approves period unlock and support access (VC-03) |
| HR Manager | Organisation, optionally limited to branches | Settings, policy sets, waivers, HR approvals, device enrolment, opening balances, manual attendance on device failure |
| Payroll Officer | Organisation | Locks periods; from V2 also runs payroll, releases payslips, records payments |
| Manager (Approver) | Their team | Approves leave, permission, regularisation, on-duty, comp-off (and from later releases overtime and advances) |
| Supervisor | Their team | Views team attendance; marks daily cash payouts (V3) |
| Employee | Self | Punches, views own results and balances, raises requests |
| Auditor | Organisation, read-only | Results, settings history, audit trail |
| Custom roles | Organisation-defined | Configurable view, edit and approve rights (AC-01, AC-06) |
| eSSL device | Branch | Pushes punches (PC-R1) |

## 4. Module map and release

| Code | Module | Kind | Release | User story file |
|---|---|---|---|---|
| V01 | Vendor console + module control (MD) | Platform | V1 | `USER_STORIES/V01-vendor-console.md` |
| C01 | Organisation setup | Common | V1 | `USER_STORIES/C01-organisation-setup.md` |
| C02 | Employee master | Common | V1 | `USER_STORIES/C02-employee-master.md` |
| C03 | Users, roles and access | Common | V1 | `USER_STORIES/C03-user-access.md` |
| C04 | Punch collection | Common | V1 | `USER_STORIES/C04-punch-collection.md` |
| C05 | Shifts and roster | Common | V1 | `USER_STORIES/C05-shifts-roster.md` |
| C06 | Attendance rules | Common | V1 | `USER_STORIES/C06-attendance-rules.md` |
| C07 | Holidays and weekly offs | Common | V1 | `USER_STORIES/C07-holidays-weekly-offs.md` |
| C08 | Leave | Common | V1 | `USER_STORIES/C08-leave.md` |
| C09 | Permissions and on-duty | Common | V1 | `USER_STORIES/C09-permissions-on-duty.md` |
| C10 | Advances and loans | Common | V2 | — |
| C11 | Documents, assets, on/offboarding | Common | V3 | — |
| C12 | Pay period and lock | Common | V1 (attendance periods); V2 (pay frequencies) | `USER_STORIES/C12-period-lock.md` |
| C13 | Reports | Common | V1, extended each release | `USER_STORIES/C13-reports.md` |
| C14 | Notifications | Common | V1, extended each release | `USER_STORIES/C14-notifications.md` |
| C15 | Opening balances | Common | V1; advances/loans V2 | `USER_STORIES/C15-opening-balances.md` |
| C16 | Dashboards | Common | V1, extended each release | `USER_STORIES/C16-dashboards.md` |
| M01 | Overtime | Optional | V3 (hours identified from V1, in C06) | — |
| M02 | Compensatory off | Optional | V1 earning and balance; V2 double pay | `USER_STORIES/M02-comp-off.md` |
| M03 | Payroll | Optional | V2; daily allowance V3 | — |
| M04 | Reimbursements | Optional | V2 | — |
| M07 | Projects, tasks, time tracking | Optional | V3 | — |
| M09 | Employee performance | Optional | V3 | — |
| M05, M06, M08 | Statutory, bonus, piece rate | Optional | Not scheduled | — |

Cross-cutting: `BUSINESS_RULES.md` (fixed rules and evaluation orders), `NON_FUNCTIONAL.md` (BRD §15, §22.2), `TRACEABILITY.md`, `../requirements/OPEN_QUESTIONS.md`.

## 5. Version 1 scope (BRD §21)

In scope: everything marked V1 above, plus web admin and employee app (Flutter) in English, Tamil and Hindi, offline app punches (PC-R5, N4), and the payroll input export when Payroll is off (MD-06).

Version 1 produces results in **days and minutes only**. Money is produced from V2 (release note 1).

V1 items that depend on later releases (flagged in the story files):
- PL-01 sets the pay period per payroll group, but payroll groups arrive in V2. See OQ-X-04.
- Overtime hours are identified in V1; approval and pay come in V3.
- Comp-off "pay one extra day on expiry" (HW-06) needs Payroll (V2).
- Supervisor cash payouts (DB-R6) need the daily allowance (V3).

## 6. Platforms (BRD §3, §22.1; TSD §2)

| Client | Users | Notes |
|---|---|---|
| Web (Next.js) | Vendor, org admin, HR, payroll, managers, employees (web check-in) | Talks only to the API |
| Mobile (Flutter, Android and iOS) | Employees and managers, where the plan includes it (MD-02) | Offline punch queue |
| eSSL device | Employees without a smartphone | Direct push to the device gateway |
