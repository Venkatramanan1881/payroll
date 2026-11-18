# Phase 5 Slice 5: Regularisation, Approvals and HR Corrections — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Employees ask to correct a day (missed punch, late arrival, early check-out, wrong punch, whole day); the request is routed (AC-02), approved at one or two levels (RG-05) and, once approved, produces manual punches that reference the approval (BR-01) and recalculates the day (BR-03). HR records direct corrections (RG-10) and bulk-corrects a device outage (RG-11).

**Architecture:** `domain-policy::rg` (typed RG settings, BRD defaults, validated on save). `domain-attendance::regularisation` (submit, route, decide, cancel, HR correction, bulk outage, reads). The approval steps are read back from the audit log (contract: "AuditEntry-backed"), so no new table and no migration. The routing and decision code is written so the Leave and Permission slices can reuse it.

**Spec:** `docs/api/openapi/attendance.yaml` (`attendanceSubmitRegularisation`, `attendanceListRegularisations`, `attendanceListPendingRegularisations`, `attendanceGetRegularisation`, `attendanceApproveRegularisation`, `attendanceRejectRegularisation`, `attendanceCancelRegularisation`, `attendanceRecordHrCorrection`, `attendanceBulkRegulariseOutage`); BRD RG-01..11, AC-02, AC-03, BR-01, BR-03, BR-06, BR-07, E13; STATE_MACHINES SM-REQ R1–R6, R10; PERMISSION_MATRIX ATT-01..06; schema `attendance.regularisation_request`, `iam.approver_rule`, `iam.delegation`.

## RG settings (typed, BRD defaults)

| Key | Shape | Default |
|---|---|---|
| RG-01 | boolean | true |
| RG-02 | `{missed_punch, late_arrival, early_checkout, wrong_punch, whole_day}` booleans | all true |
| RG-03 | integer ≥ 0, or `{overall, per_type: {type: n}}` | 3 overall per calendar month |
| RG-04 | integer days ≥ 0 | 7 |
| RG-05 | `"one" \| "two"` | `"one"` |
| RG-06 | list of request types auto-approved on submit | none |
| RG-07 | `{days, action: escalate\|auto_approve\|auto_reject}` | stored; the timeout job (J3) comes with deployment |
| RG-08 | `"block" \| "hr_approval"` | `"hr_approval"` |
| RG-09 | list of request types needing an attachment | none |
| RG-10, RG-11 | boolean | true |

A switched-off RG group means regularisation is off (409 RULE_GROUP_DISABLED).

## Rules
- **Submit (R1/R2/R3):** RG-01 on and the type on, else 409 RULE_GROUP_DISABLED. Reason required; an attachment where RG-09 asks for one (422). The date must be from today − RG-04 days to today and not locked, else 409 OUTSIDE_REQUEST_WINDOW. The limit counts requests in the date's calendar month, excluding rejected and cancelled: over the limit with RG-08 = block → 409 LIMIT_EXCEEDED; with hr_approval → created in `pending_l2`, routed to HR. RG-06 types are approved on submit. A refused submission creates nothing.
- **Type checks:** missed punch needs at least one proposed time; late arrival needs the in time; early check-out needs the out time; whole day needs both; wrong punch needs the employee's own `target_raw_punch_id`. Proposed times must fall within a day either side of the date.
- **Routing (AC-02):** `iam.approver_rule` for `regularisation`. Defaults: level 1 = reporting manager, level 2 = HR. A named user is assigned directly; the reporting manager's user account, if any; otherwise (HR, department head, no manager) the HR pool. The HR pool is anyone holding the level's grant with the employee in scope.
- **Deciding (R4/R5/R6):** grant ATT-03 for level 1, ATT-04 or ATT-05 for level 2. The assigned approver or their active delegate (AC-03) decides; for the HR pool, any holder with the employee in scope. Someone with the grant who is not the approver gets 403; out of scope is 404. Deciding your own request is 403 SELF_APPROVAL_FORBIDDEN (BR-07), delegates included. Rejecting needs a reason. Repeating the same decision returns 200 with no second effect; the opposite decision is 409 ILLEGAL_TRANSITION.
- **Approval effect:** new manual punches at the proposed times, with source manual, the request's reason, the decider as `manual_entered_by` and the request as `manual_approval_ref`. Their idempotency keys come from the request, so the effect never runs twice. The punch being corrected (wrong punch; for late arrival the day's first in; for early check-out its last out) is left out of the day by the engine, never edited (BR-01). The day is recalculated; a locked date gets no new result (BR-06).
- **Cancel (R10):** the requester, while pending. A repeat returns 200; other states are 409 ILLEGAL_TRANSITION (GAP-SM-09).
- **HR correction (RG-10, ATT-06):** origin `hr_direct`, approved immediately (DP-08 is open, so there is no second person) and the effect applied. Not for your own attendance (G-17). A locked date is 409 PERIOD_LOCKED.
- **Bulk outage (RG-11, ATT-06):** for each employee of the branch (in scope) whose day has no completed pair, an `hr_direct` whole-day correction at the expected shift times. `mark_present` and `use_shift_times` both use shift times. Employees without a timed shift are skipped and counted. Answers 202 with a finished Job.

## Tasks
1. RG settings and their validation on save (unit + API).
2. Submit, routing, type checks, window, limit, auto-approve (domain + API tests).
3. Decide (levels, delegation, BR-07, idempotency), cancel, the effect and the engine exclusion of corrected punches; E13 end to end.
4. HR correction, bulk outage, lists, pending-approval, get with approval steps.
5. Full suite, clippy, review, fixes, merge, push.

## Review focus
1. E13: in 09:00, no out → request 18:05 → manager approves → present; the manual punch carries the approval reference.
2. BR-07: a requester never decides their own request, including as someone's delegate.
3. Two levels: manager approval moves the request to HR; only then is it approved and applied.
4. Approving twice applies the effect once.
5. A late-arrival correction removes the late mark without editing the original punch.

## Not in this slice
The RG-07 timeout and escalation job (J3), notifications (NT-05), attachments upload links, cancel after approval (GAP-SM-09), per-period counters by attendance cycle (calendar month used), pending requests at period lock (OQ-C12-03).
