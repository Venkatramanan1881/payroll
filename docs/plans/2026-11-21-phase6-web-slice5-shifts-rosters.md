# Phase 6 Web Slice 5: Shifts, rosters and holidays — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR sets up the work schedule on the web:
- shifts and their dated versions;
- weekly-off patterns;
- each employee's schedule;
- the roster week by week;
- holiday calendars.

Employees see their own next two weeks.

**Spec:**
- `docs/api/openapi/scheduling.yaml`: shifts, shift versions (create, patch draft, publish), weekly-off patterns, employee schedules, roster entries (list, set), holiday calendars and holidays, `/v1/me/roster`.
- PERMISSION_MATRIX SCH-01..05.

**Decision (owner, 2026-11-21):** web slice 5 is shifts and rosters (slice order from the 11-slice outline).

No backend change: every operation used here is already built.

## Screens
- **Shifts (`/app/shifts`, SCH-03 to see, SCH-01 to change):**
  - **List:** code, name, the version in force (type, start–end).
  - **Weekly-off patterns (SCH-04):** listed on the same page:
    - create: fixed days, alternating day and weeks, or from roster;
    - retire.
- **New shift (`/app/shifts/new`):**
  - **Fields:** code, name, effective from, and the shift definition.
  - **Definition form:** a form for the contract's ShiftDefinition. It shows the fields of the chosen shift type (fixed / night: start–end; flexible: required minutes per day or week, earliest in, latest out; flexible with core: plus core hours; open: required minutes), plus the window, half-day split and missed-punch cutoff.
  - **Not offered:** split-shift segments (OQ-C05-01) and "shift end date" attendance for nights, which the API refuses.
- **A shift (`/app/shifts/{id}`):**
  - Its versions, with state.
  - **A draft:** opens an edit form (PATCH with If-Match) with a Publish button (published versions are frozen, G5).
  - **"New version":** copies the current definition onto a new effective date.
- **Employee page:** a "Schedule" card (SCH-03) lists the dated schedules and assigns a new one (SCH-02): fixed shift or roster, weekly-off pattern, from-date and reason.
- **Roster (`/app/roster`, SCH-03 / SCH-02):**
  - **Grid:** employees in scope (first 50, searchable) × a week; each cell is a shift, a weekly off, or nothing.
  - **Saving:** "Save" sends only the changed cells, as one `roster-entries/set`, with a reason.
- **Holidays (`/app/holidays`, SCH-05):**
  - Calendars: create, rename, retire.
  - **A calendar (`/app/holidays/{id}`):** its holidays for a year: add, change (with a reason), remove (with a reason).
- **My schedule (`/me/schedule`):** the next 14 days from `/v1/me/roster` (shift, times, weekly off, holiday).
- **Menus:**
  - Shifts and Roster (C05, SCH-03 all/scoped).
  - Holidays (C07, SCH-05).
  - "My schedule" in self-service.

## Tests
- **Vitest:** the definition form's body builder (only the shown fields; empty optional times become null when editing).
- **Playwright (fresh seed):**
  1. HR creates a night shift, edits the draft and publishes it.
  2. HR creates a weekly-off pattern and assigns Asha a fixed schedule with it.
  3. HR sets a roster cell.
  4. HR adds a holiday.
  5. Asha sees her schedule.

## Not in this slice
- **Not built in the API:** shift version approval (approve / reject), roster upload, copy-forward and rotation, shift-change requests, and optional holidays.
- **Open questions:** split-shift segments (OQ-C05-01).

## Review fixes
1. **Editing a draft never nulls a defaulted column.**
   - The window minutes, half-day split, missed-punch cutoff and "required per" are NOT NULL with a default. Left empty, they keep their value instead of failing with a database error.
   - The half-day select has no blank option.
   - The form's hint says which fields keep their value.
2. **"New version" lets the server copy.** It sends only the new date (`definition: {}`), and the API copies the shift's latest version, as `create_version` does. Before, a partial copy of the version in force was laid over the latest one, so a type change could mix two versions' fields and fail a CHECK.
3. **Roster cells say what the API can do:**
   - A saved cell no longer offers "Unset", since the API has no clear.
   - A saved cell whose shift is retired still shows that shift's code.
4. **No lost or hidden roster edits:**
   - Week arrows and search are disabled while changes are pending, with a "Discard changes" button.
   - Cells are looked up in a map, and the loading skeleton is not shown next to an error.
5. **Schedule history shows names for every viewer:** shifts and patterns are read with `include_retired=true`, whether or not the viewer may assign.
6. **Publish sends only the saved draft:** it is disabled while the form has unsaved changes.
7. **Irreversible actions ask first:**
   - Removing a holiday, retiring a calendar and retiring a pattern each ask for confirmation.
   - Retiring a pattern also blocks a double click.

Not changed: calendars outside the user's SCH-05 scope still appear in the list. The API refuses changes to them, and the page shows that refusal.
