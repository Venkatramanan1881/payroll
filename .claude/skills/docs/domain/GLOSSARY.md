# Domain glossary

Status: Draft for Approval Gate 2
Business terms come from BRD v0.7 §6 and are not repeated here: punch, session, attendance date, grace, geo-fence, late mark, early mark, permission, on-duty, master branch, allowed branches, absconding, client-specific rule, regularisation, informed leave, uninformed absence, sandwich policy, LOP, deduction days, comp-off, payroll group, monthly salary, daily allowance, day divisor, piece rate, advance, loan, policy set and period lock. Use those meanings exactly.

This file defines the terms the domain model adds.

| Term | Meaning | Not to be confused with |
|---|---|---|
| **Organisation** | The tenant. Every client record belongs to exactly one. | Branch |
| **Module entitlement** | Whether the vendor has switched a module on for an organisation, from a date (MD-01). | Rule-group switch |
| **Rule group** | A set of settings with one on/off switch inside a module, for example LA (late arrival) or PM (permissions) (MD-05, G2). | Module |
| **Policy version** | An immutable, dated snapshot of a policy set's rule-group settings (G5). | Policy set (the named container) |
| **Policy assignment** | A dated link from a policy set to a level and target (organisation, branch, department, category, shift or employee) (G3). | Payroll group membership |
| **Raw punch** | The immutable stored evidence of one punch event, with no inferred direction. | A session (pairing done during processing) |
| **Manual punch** | A raw punch whose source is manual, with reason, entering user and approval (BR-01). | Editing a punch (never allowed) |
| **Punch flag** | A reason a punch needs a decision, for example outside the fence or late sync. | Day exception |
| **Punch attempt (refused)** | A punch the system refused (for example no location with PC-13 = block). Audited, but not a raw punch. | Rejected flag (the punch exists but is excluded) |
| **Expected schedule** | What was expected of an employee on a date: shift version and window, or weekly off, or holiday. | Roster (the planning artefact) |
| **Attendance day** | The record for one employee and one attendance date. Holds its result history. | Calendar date |
| **Day result** | One calculated interpretation of an attendance day, with inputs, outputs and explanation. Append-only; one is current. | Day status (one field of it) |
| **Day status** | The classification of the day: present, half day, absent, leave, holiday, weekly off, on-duty … (DS). | Processing state of the day |
| **Day effect** | A fraction of a deduction day caused by a rule on that day, with its reason. | Period penalty |
| **Explanation** | The ordered record of the rules, versions, inputs and approvals behind a result (BR-08). | Audit entry (records who did what) |
| **Input fingerprint** | A hash of every input to a day result. The same fingerprint means the same result (BR-02). | — |
| **Period counter set** | Running late and early marks, minutes and grace uses for an employee in a counter period (LA-18). | Attendance period |
| **Period penalty** | A deduction applied at period end from counts or minutes (LA-06..12, BR-10). | Slab action (applied on the day) |
| **Attendance period** | The date range that is locked as a unit and exported (C12). | Counter period, leave year |
| **Period totals** | Layer-5 figures per employee for a period, frozen at lock. | Payroll result (V2) |
| **Period adjustment** | The difference between a locked result and a later correction, posted to the next open period with the original date (PL-R4). | Unlock |
| **PayrollInput v1** | The versioned contract that hands period totals and adjustments to payroll or an export (MD-06). | Payslip |
| **Ledger** | The append-only entries whose sum is a balance (leave, comp-off, permission usage). | A stored balance number |
| **Approval service** | The shared routing, levels, delegation, escalation and BR-07 check used by every request. | A request's own lifecycle |
| **Outbox record** | A follow-up action stored in the same transaction as a change, later handed to the work queue. | Audit entry |
| **Decision point (DP)** | An unanswered business question with a defined slot in the model. | Assumption (never used) |
