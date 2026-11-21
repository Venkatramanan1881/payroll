# Phase 6 Web Slice 8: Organisation settings — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** HR and the Organisation Admin keep the organisation's structure and settings on the web:
- organisation settings (OR-01..04, EM-01..08) as dated, append-only versions;
- branches, each with geo-fences and an allowed web IP list;
- the four lists (departments, designations, categories, employee types);
- contractors.

**Spec:**
- `docs/api/openapi/organisation.yaml`;
- PERMISSION_MATRIX ORG-01..07.

**Decision (owner, 2026-11-21):** web slice 8 follows the 11-slice outline.

## What the API offers today
- **Built:** every organisation.yaml operation except the three below.
- **Not built:**
  - change a branch's time zone (OQ-C01-03 open);
  - settings changes held for Organisation Admin approval (AC-04: list, approve, reject);
  - the employees a contractor supplies.
- **On screen instead:**
  - the time-zone history is read-only;
  - a save is applied directly, as the API does ("HR Manager only");
  - the contractor shows no employees.

## Backend (dev only)
- `dev_seed` adds ORG-01, ORG-04 and ORG-05 to HR's dev-only role. G-11 is open: who maintains branches, lists and contractors, OA or HR. By default only the Organisation Admin holds ORG-01, and no role holds ORG-04 or ORG-05.

## Screens (`/app/organisation`, one menu item, C01)
- **Sub-navigation:** Settings · Branches · Lists · Contractors. Each is readable with ORG-07.
- **Settings:**
  - the organisation's names;
  - the version in force;
  - all versions, newest first;
  - for organisation-wide ORG-06, a new version: every setting, a date it applies from, a reason. The form starts from the version in force. A setting never saved starts empty; the web invents no defaults.
- **Branches:**
  - the list;
  - create, for organisation-wide ORG-01: name, code, address, time zone and the date it applies from.
- **A branch:**
  - details (ORG-01, If-Match) and the time-zone history;
  - **Geo-fences (ORG-02):**
    - add (starts off);
    - change (clears the last test);
    - switch on or off;
    - "Test from here" sends the browser's position and shows inside or outside and the distance.
  - **IP allow list (ORG-03):** replace it with a reason, If-Match on the branch.
- **Lists:**
  - one of the four kinds, retired entries on request;
  - add, rename and retire with a reason (organisation-wide ORG-04, If-Match per row).
- **Contractors:** the same as lists, with a contact instead of a code (organisation-wide ORG-05).

## Tests
- **Playwright (fresh seed):**
  1. HR saves a settings version.
  2. HR creates a branch with a time zone.
  3. HR adds a fence and tests it from a set browser position (inside).
  4. HR switches the fence on and saves an IP list.
  5. HR adds and renames a designation, then retires it.
  6. HR adds a contractor.

## Not in this slice
- Settings-change approvals and the branch time-zone change, which wait on the backend.
- A map picker for fences: coordinates are typed.
- Policy sets: their own screens later.

## Built along the way
- **Plural list paths:** the API now accepts the contract's plural list path names (`/v1/lists/departments`, `employee-types`, …) as well as the stored kinds it already took. Before, it answered "not found" to the contract's names. `tests/setup.rs` checks both forms.

## Review fixes
1. **Clearing a field clears it** (a backend bug that predates this slice):
   - PATCH treated `null` like a missing field, so a cleared branch address, list code or contractor contact kept its old value while the API answered 200.
   - The routes now tell "not sent" apart from `null`, and the update writes NULL only when the field was sent.
   - `tests/setup.rs` clears each of the three.
2. **The IP list follows the branch version:**
   - The IP list's ETag is the branch's. It now comes from the branch read, so saving details and then the IP list no longer answers 412; the browser test does exactly that.
   - Saving the IP list no longer throws away unsaved detail edits.
3. **"Saved" stays on screen:** the settings message lives outside the form, which starts again from a back-dated version once that version is in force.
4. **Fence edits survive a test or a switch:** a fence's edit form starts again only when the fence itself changed.
5. **Fence buttons name their fence** for screen readers.
6. **Accepted, not fixed:** create and change controls for ORG-01, ORG-04, ORG-05 and ORG-06 show for any all-or-scoped grant. The API also requires organisation-wide scope, and `/v1/me` does not say a grant's scope. A branch-scoped holder would see the forms and be refused; this needs `/v1/me` to expose scope.
7. **Still open:** the retire reason for list entries and contractors is sent but not kept by the API.
