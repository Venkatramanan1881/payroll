# Phase 6 Web Slice 4: Employee self-service — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Employees use the web to:
- see their month and each day;
- ask for corrections (regularisation);
- apply for and cancel leave;
- read their notifications.

This closes the loop with the approver screens of slices 2 and 3.

**Spec:**
- `attendance.yaml`: calendar, day, regularisation submit / list / cancel.
- `leave.yaml`: own balances, requests, submit, cancel.
- `notifications.yaml`: inbox and mark-read.
- PERMISSION_MATRIX self cells: ATT-01 Self Cfg(AC-05), ATT-02, ATT-13, TCP-07 Self Cfg(AC-05), LVE-02, LVE-08 Self.

**Decision (owner, 2026-11-20):** web slice 4 is employee self-service.

## Backend
- **`GET /v1/attendance/employees/{id}/calendar?month=YYYY-MM`:** in the approved contract, not yet built. One CalendarDay per date of the calendar month (OQ-C16-05), with dates without a day carrying only the date.
- **AC-05 employee visibility,** applied when the caller reads through a Self grant (HR or a manager who is also an employee sees what their other grants allow):
  - **Day:** sessions and punches are omitted when "own punches" is off; status when "day status" is off; explanation and deduction reasons when "deduction reason" is off. Omitted, never nulled.
  - **Calendar:** status and marks follow "day status" and "counters".
  - **Punch list and counters:** FORBIDDEN while "own punches" / "counters" is off.
  - This replaces the slice-4 shortcut note in `routes/attendance.rs`.

## Screens (`/me`, one layout)
- **Header:** My details, My attendance, My leave, Notifications (unread count), a link to the admin area for users who also work there, and sign out.
- **My attendance (`/me/attendance`):**
  - A month calendar; each day opens.
  - Below it, my regularisation requests, with cancel while pending.
- **A day (`/me/attendance/{date}`):**
  - The same day view as HR's (shared components), filtered by the API.
  - "Ask for a correction" (ATT-02): type, in/out (next-day option) and reason, posted to `/v1/attendance/regularisation-requests`.
- **My leave (`/me/leave`):** balances, my requests, and apply (LVE-02, the slice-3 form without the employee picker).
- **A leave request (`/me/leave/{id}`):** the shared leave detail; cancel when the API says so.
- **Notifications (`/me/notifications`):**
  - Newest first, with an unread filter.
  - Mark one or all read.
  - A link to the request where one exists (leave, regularisation).

## Tests
- **Backend `self_service.rs`:** calendar shape and refusals; AC-05 hiding for the employee but not for HR.
- **Playwright (fresh seed):**
  1. Asha opens her month and yesterday, asks for a correction and applies for leave.
  2. Meena (HR) approves the leave.
  3. Asha sees the approval in her notifications and marks them read.

## Not in this slice
- **Not built yet:** permission and on-duty requests (their screens follow the same pattern), and attachments (upload links).
- **Settings and sign-in:** the language switch (Tamil and Hindi files do not exist yet), and phone sign-in, which needs Firebase configuration.

## Review fixes
1. **AC-05 is applied by field, in one place.** `AC05_FIELDS` in `routes/attendance.rs` lists the DayResult fields each item hides. A field derived from hidden data goes with it, so nothing gives the hidden data away:
   - **"own punches":** sessions, punches used, gross / break / net minutes and flags.
   - **"day status":** status, day fraction, day effects and flags.
   - **"counters":** late / early marks and minutes, early-arrival and overtime minutes. The day view had kept these.
   - **"deduction reason":** explanation, day effects and flags.
   - **Calendar:** also drops `has_exception` with "day status".
2. **Reading one punch by id follows AC-05:** `GET /v1/punches/{id}` is refused like the list while "own punches" is off. The test now covers it.
3. **Notification links open the right screen:**
   - Inbox items say whom they were for (`recipient_kind`: employee or approver; older items count as employee).
   - An approver's link opens the admin screen (`/app/leave/{id}`, `/app/attendance/regularisations/{id}`); an owner's opens self-service.
4. **Cancel only where the API accepts it:** "My correction requests" offers Cancel only on requests the user filed themself.
5. **Small UI fixes:**
   - The day view leaves out rows for fields the API omitted, instead of showing "Not worked out" or 0.
   - The calendar drops the invalid grid roles; each day's link reads its status and marks aloud.
   - "Wrong punch" is offered only when the employee can see their punches.
