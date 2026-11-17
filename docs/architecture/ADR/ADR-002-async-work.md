# ADR-002: Background work through a transactional outbox and Cloud Tasks; no Pub/Sub or Redis in V1

Status: Accepted 2026-11-15 (Gate 2 part 1). Closes OQ-X-09.
Date: 2026-11-15

## Context
Every input change must recalculate the affected days (BR-03), within seconds (N3). BRD §22.1 records Cloud Scheduler for timed work and Cloud Tasks for queued work as decided. A change saved without its follow-up recalculation would leave results stale. Retries must never duplicate effects (TSD §5.5, AT21).

## Decision
- Every use case that changes a recalculation input writes an **outbox record** in the same database transaction as the change.
- A relay turns outbox records into Cloud Tasks, one per (organisation, employee, date) or per notification.
- Consumers are idempotent: recalculation compares input fingerprints, and ledger postings are keyed by their cause.
- Nightly finalisation (J1) re-processes the previous day as a safety net.
- Pub/Sub and Redis are not used in V1.

## Alternatives
- **Enqueue directly to Cloud Tasks after commit, with no outbox.** Rejected: a crash between commit and enqueue loses the recalculation.
- **Pub/Sub for domain events.** Not needed while all consumers are in one deployable. Revisit when a separately deployed service consumes events.
- **Redis for queues, cache or rate limits.** No V1 requirement needs it; Postgres covers these at V1 scale. Revisit after load tests.

## Consequences
- One extra table and relay process.
- Recalculation is at-least-once and converges, which matches BR-02 and BR-03.
