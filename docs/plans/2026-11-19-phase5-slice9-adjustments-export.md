# Phase 5 Slice 9: Adjustments after Lock and the Payroll Input Export — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** A correction to a locked day never changes that day. Only the difference is posted to the next open period, tagged with the original date (PL-R4, E15, SM-ADJ). A locked period produces the `PayrollInput v1` document (MD-06), stored in Google Cloud Storage and downloaded through short-lived signed links (D-19, TB-02).

**Architecture:**
- `domain-attendance` gains a measuring mode, `measure_locked`, beside `recalculate`. It writes a `for_adjustment` day result that is never current; the existing database triggers already allow exactly that (BR-06).
- `domain-attendance::period::post_adjustment` measures, computes the difference and posts the `period_adjustment`.
- `domain-attendance::period::payroll_input` builds the export document from frozen data only.
- `api::storage`: an `ObjectStore` trait with a Google Cloud Storage implementation. It uses the Cloud Run service account through the metadata server: an access token for uploads, and V4 signed URLs through IAM `signBlob`. No key file exists.
- Tests use an in-memory store.
- No new migration.

**Spec:** `docs/api/openapi/period.yaml` (adjustments, payroll-input-exports, download-link, `PayrollInputV1`), `attendance.yaml` (hr-corrections and recalculations post adjustments for locked dates); BRD PL-R4, PL-07, MD-06, E15; SM-ADJ Q1–Q3; C12 stories US-C12-06/07; DECISIONS D-19, D-20.

## Rules
- **Routes that post adjustments** (OQ-C12-03 is still open, so only the contract's named routes):
  - HR direct correction (RG-10), the BRD's E15 example;
  - HR-requested recalculation (ATT-06, SM-DAY A10). Here a date beyond PL-07 is skipped and counted, not fatal.

  Every other write to a locked date keeps answering PERIOD_LOCKED.
- **Target:** the first period after the locked one that is not locked, row-locked while posting. With none → 409 ILLEGAL_TRANSITION.
- **PL-07 (OQ-C12-04):** source and target more than N periods apart → 409 ADJUSTMENT_WINDOW_EXCEEDED, and nothing is written.
- **What it is measured against (OQ-C12-06):** the date's latest measured result, else the locked one. So repeated corrections add up to "final − locked", and the same inputs twice post nothing (AC-C12-06.5).
- **Difference:** day status from/to, signed late/early minutes, signed deduction lines by reason (as in totals). `days` = the signed change in payable days (minus the change in deductions). An unchanged outcome posts nothing.
- **States:** posted → frozen when the target locks (Q2) → exported when an export carries it (Q3). Unlocking the target returns frozen to posted; exported stays exported (OQ-C12-07).
- **Export (PER-06, G-31):**
  - From a locked period only (OQ-C12-11) → otherwise 409 ILLEGAL_TRANSITION.
  - JSON only; `xlsx` → 422 until the layout is decided.
  - Covers the employees in the caller's scope: frozen totals plus the adjustments targeted at the period. Someone with only an arrear appears with zero totals.
  - Deterministic: exporting twice gives the same employees section (AC-C12-07.3).
  - Stored at `<org_id>/payroll-input/<period_id>/<export_id>.json` with a SHA-256 checksum.
  - The job answers 202 `succeeded` with a 15-minute signed link.
- **Download link (PER-07):** organisation-wide holders see every export; a scoped holder only exports they produced (an export holds its producer's scope). Each link is audited.

## Known limits (shortcuts)
- A measured locked day does not debit leave under DS-10 auto-debit; it is measured as computed.
- A failed commit after an upload leaves an unreferenced file; a bucket lifecycle rule can sweep those.
- A token is fetched from the metadata server per call.
- The Cloud Storage client is checked against Google's V4 signing format by a unit test. It cannot be exercised end to end without a bucket: verify on first deployment.

## Not in this slice
- Other post-lock routes (OQ-C12-03).
- Late or offline punches for locked dates (OQ-C12-10).
- NT-10 notifications.
- The xlsx layout (OQ-C12-11).
- Automatic lock (OQ-C12-08).

## Review focus
1. A locked day's current result and frozen totals never change; adjustments are idempotent and add up.
2. PL-07 and target selection (next open period; concurrency with a lock).
3. Export determinism, scope, tenant isolation of files (path prefix) and links.
4. Signed-URL correctness and that no credential is stored or logged.
