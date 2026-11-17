# Plans (proposal)

Status: **Proposal for the project owner**, answering Q5 ("generate a plan", 2026-11-16). Nothing here is decided until approved. No prices: invoicing is outside the app (VC-R5).
Sources: BRD v0.8 §3, §4.1, MD-01..MD-04, VC-01, N2; TSD v0.2 §14, §19.

## What a plan controls (MD-02)

1. Which modules are switched on.
2. The employee limit (and VC-01: block or allow-and-alert when reached).
3. Whether the Android and iOS employee app is included.

In V1 almost everything is a **common module that every client gets** (C01–C09, C12–C16). The only optional V1 module is **M02 Compensatory off**. So in V1 plans differ mainly by employee limit, the mobile app, comp-off and notification channels. Later modules attach to plans as they are released.

## Proposed plans

| | **Starter** | **Standard** | **Business** |
|---|---|---|---|
| Intended for | Small shops, cafés, single branch | Multi-branch retail, restaurants, small factories | Factories and chains up to the platform size (N2) |
| Employee limit (active) | 100 | 500 | 2,000 |
| When the limit is reached (VC-01) | Allow and alert the vendor | Allow and-list all options alert the vendor | Allow and alert the vendor |
| Branches | No limit | No limit | No limit |
| Common modules C01–C09, C12–C16 (V1) | ✓ | ✓ | ✓ |
| Punch: biometric device, web check-in with location | ✓ | ✓ | ✓ |
| **Employee mobile app** (punch, selfie, offline, requests) | — | ✓ | ✓ |
| **M02 Comp-off** (earning and balance) | — | ✓ | ✓ |
| Notifications: in-app, push, email | ✓ | ✓ | ✓ |
| Notifications: SMS / WhatsApp (TSD §14: per-message cost) | — | Capped monthly allowance | Capped monthly allowance |
| **V2 when released:** Advances and loans (C10), Payroll (M03), Reimbursements (M04), comp-off double pay | — | Add-on | ✓ |
| **V3 when released:** Overtime pay (M01), Documents and assets (C11), Daily allowance, Projects (M07), Performance (M09) | — | Add-on | Add-on |
| Client-specific payroll rules (V2, vendor-built, 10.3.7) | — | — | ✓ (by agreement) |

Notes:
- **Workers without a smartphone** use the device in every plan (BRD §3), so Starter still covers device-only workforces.
- **Module dependencies** (MD-04) are enforced whatever the plan says: payslips, accounting export, reimbursement payout with salary, statutory, bonus and piece rate all need Payroll.
- **Per-organisation exceptions:** whether an organisation's modules or limit can differ from its plan is still open (OQ-V01-02). This proposal assumes the vendor can grant an add-on to one organisation without changing the plan.
- **Sign-in cost:** employee sign-in codes go through Firebase (D-12) and are billed per SMS. The app keeps a long-lived session to limit this (TSD §8). It applies in every plan that includes the app.

## To decide

1. Approve the three plans, or change names, limits or contents.
2. Whether Starter gets the app as a paid add-on.
3. SMS/WhatsApp allowance per month for Standard and Business.
4. OQ-V01-02: can one organisation's modules or limit differ from its plan?
