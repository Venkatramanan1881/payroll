# Phase 5 Slice 17: Notifications (C14) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** In-app notifications for request decisions (NT-05), a daily pending-approvals digest (NT-06), period-end penalties (NT-07) and posted adjustments (NT-10). Also the rules that switch them, the caller's inbox, and push device registration.

**Decisions (user, 2026-11-20):**
- **D-34:** in-app on by default, other channels off.
- **D-35:** only in-app is delivered.
- **D-36:** NT-05/06/07/10 first.
- **D-37:** slice split (18 = reports, dashboards and audit; 19 = remaining vendor views).

**Spec:** `docs/api/openapi/notifications.yaml`; BRD §9.14 NT-01..NT-11; PERMISSION_MATRIX NTF-01; schema 0004 (`notify.notification_rule`, `notify.notification`), 0006 (`read_at`, `notify.push_device`); ADR-002 outbox. No new migration.

## Rules
- **Rules (NTF-01):**
  - GET lists every sendable (event, recipient) × channel, with stored switches over the D-34 defaults, and an ETag for the set.
  - PATCH needs If-Match and a reason; it is audited before and after. An unknown rule, or switching on any channel other than in-app → 422 (D-35).
  - Recipient kinds: NT-05 employee and approver; NT-06 approver; NT-07 and NT-10 employee.
- **Events → notifications:**
  - Every approval step (all request types, rejections included) emits one `ApprovalStep` outbox event from the shared `approval::step`.
  - The relay job (`POST /internal/jobs/notifications-dispatch`) runs per organisation. As `app_relay` it reads that organisation's undispatched `ApprovalStep`, `PeriodLocked` and `AdjustmentPosted` events; it writes notifications as `app_api` and marks the events dispatched in the same transaction. Other event types are left for their consumers.
  - **NT-05:**
    - The requester hears of every decision on their request, except their own actions.
    - The named approver a request now waits on hears that it waits.
    - A request waiting on the HR pool notifies no one yet (OQ-C14-01).
  - **NT-07:** one notification per penalty recorded by the period's latest lock.
  - **NT-10:** one per posted adjustment.
  - **NT-06:** `POST /internal/jobs/approvals-digest` sends one digest per approver per day with a count of waiting requests.
  - Each notification is sent once (dedupe key) and only when its in-app rule is on. The text is English until translations exist (OQ-C14-08); the language column records the user's language.
- **Inbox:** the caller's own in-app notifications, newest 200, with an unread filter, an event filter and `unread_count`. Mark-read takes ids (others' ids are ignored) or all.
- **Push devices:** register (one live row per token; it moves to the newest user); unregister own (any other id → 204 with no change). Both audited. Nothing is pushed yet (D-35).
- **Deployment:**
  - `jobs_login` becomes a member of `app_relay` (`supabase_roles.sql`, to be re-run).
  - Two scheduler entries are added (DEPLOYMENT.md §5).

## Not in this slice
- **Waiting on open questions:**
  - NT-01, NT-02, NT-03, NT-04, NT-08, NT-09 and NT-11 (OQ-C14-03..05).
  - HR-pool recipients.
  - Delegate copies (OQ-C14-09).
- **Not built yet:**
  - Email, SMS, WhatsApp and push delivery.
  - Translations.
  - AC-04 approval of notification settings (G-08).
  - Inbox paging.

## Review focus
1. The relay: role switching inside one transaction, only handled events marked, idempotence (dedupe, SKIP LOCKED), and tenant isolation (`org_id = $1` plus RLS on the inserts).
2. Recipients: no one notified of their own action, nothing leaking across employees, and rules honoured.
3. The inbox and push devices act only on the caller's own rows.
