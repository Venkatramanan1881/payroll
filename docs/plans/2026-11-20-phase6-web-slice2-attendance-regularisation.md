# Phase 6 Web Slice 2: Attendance and regularisation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR and managers do the daily attendance work on the web:
- see the day board;
- open one employee's day and see why it came out that way (punches, sessions, explanation, history);
- correct it (HR correction, manual attendance);
- review punch flags;
- decide regularisation requests.

**Spec:**
- `docs/api/openapi/attendance.yaml`: daily, the day, regularisation list / pending / get / approve / reject, and HR corrections.
- `punches.yaml`: `/v1/punches`, punch flags, manual attendance.
- PERMISSION_MATRIX ATT-01, ATT-03..06, TCP-04, TCP-06, TCP-07.

**Decision (owner, 2026-11-20):** web slice 2 is attendance plus regularisation. Leave, shifts and employee self-service follow.

## Screens
- **Daily board (`/app/attendance`, ATT-01):**
  - **Filters:** date (default yesterday in India), branch, department, day status, processing state; sort by code or status.
  - **Rows:** code, name, status, first in, last out (shown in the branch's time zone), net hours, late / early minutes, flags.
  - Each row opens the employee's day.
- **The day (`/app/attendance/{employee}/{date}`):**
  - **Current result:** status, minutes, marks, flags.
  - **Sessions and punches:** punches come from `GET /v1/punches`, joined to `punches_used`, with excluded punches and their reasons.
  - **Day effects:** with their reasons.
  - **The explanation steps,** in order (BR-08).
  - **Superseded results,** collapsed (BRX-03).
  - **Actions:**
    - **HR correction (ATT-06):** type, proposed in / out, reason. The times are entered as wall-clock times in the day's schedule time zone and sent as instants.
    - **Manual attendance (TCP-04):** punch times and a reason, for the employee's current branch.
  - Both send an Idempotency-Key. Afterwards the page reloads from the API; the browser never recalculates.
- **Punch flags (`/app/attendance/flags`, TCP-06):**
  - Open flags (state filter), each with the employee, reason, evidence, punch time and source.
  - Accept (optional comment) or reject (reason required).
- **Regularisations (`/app/attendance/regularisations`):**
  - "Waiting for me" (pending-approval; ATT-03/04/05) and "All" (ATT-01; state and date filters).
  - The detail page shows the request, proposed times, approval steps, and approve (comment) / reject (reason).
  - SELF_APPROVAL_FORBIDDEN, ILLEGAL_TRANSITION and the other refusals are shown as the API gives them.
- **Menus:**
  - **Items:** Attendance (C06, ATT-01 all/scoped), Punch flags (C04, TCP-06 approve or better) and Approvals (C06, any of ATT-03/04/05).
  - Items now name the grant kinds that open them.
  - A user with an `approve` grant lands in `/app`, so a manager reaches the inbox.

## Backend (additive)
- Regularisation requests and punch flags also carry `employee_code` and `employee_name`, so lists need no extra lookups. Recorded in attendance.yaml and punches.yaml.
- **`dev_seed`** adds, through the API in-process:
  - an organisation policy;
  - a 09:00–18:00 shift;
  - **employees:**
    - an employee (Asha) who signs in with a password, for local trials only;
    - a manager employee (Meena), who is the HR user;
  - yesterday's punches for Asha, so the board, the day and the inbox have data.

## Tests
- **Vitest:** the zoned time conversion (wall time in a time zone → instant, including the India +05:30 offset and a DST zone) and the menu kinds.
- **Playwright (local seed):**
  - HR opens the board for yesterday, opens Asha's day, sees punches and explanation, and records an HR correction;
  - Asha submits a regularisation through the BFF;
  - HR approves it from the inbox.
- **Backend:** tests assert the new name fields.

## Not in this slice
- Bulk regularisation and waivers, recalculation runs, the calendar view (`/calendar` is not built in the API), and permission-conversion and held check-in actions.
- Employee self-service submit screens, and attachments (upload links).

## Review fixes
1. **Shifts past midnight:** the HR correction out time and the second manual punch have a "Next day" box, so a night shift's out time lands after midnight instead of on the shift's own morning. The API still checks the order.
2. **No double recording:** after a correction or manual entry succeeds the form is cleared, so a second press cannot record it again under the new Idempotency-Key.
3. **Flags open the right day:** each flag carries `attendance_date`, the day whose current result uses the punch. An overnight punch opens the shift's day; before the day is worked out, the punch's own date is used. Decision times are shown in the punch's zone.
4. **Request times in the right zone:** requests carry `schedule_tz` (the day's zone), and proposed times are shown in it. The board's default date stays India time (noted as a shortcut).
5. **Reject reason no longer becomes a comment:** switching between approve and reject clears the note, so a reject reason never goes out as an approval comment.
6. **Notes survive a reload:** reloading a list keeps the last answer on screen, so notes typed on other flags survive a decision.

Not changed:
- **Approve-only managers:** opening the full day page may be refused (WRK-08), and the page shows the API's refusal.
- **Dev seed:** if it fails partway, the dev database must be dropped and seeded again.
