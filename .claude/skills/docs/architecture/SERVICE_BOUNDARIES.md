# Service boundaries and data ownership (V1)

Status: Draft for Approval Gate 2
Sources: TSD v0.1 §3, §5, §10; BRD v0.7 §22; `docs/domain/DOMAIN_MODEL.md`
Decisions: ADR-001 (modular backend), ADR-002 (async work)

## 1. Deployable units

| Unit | Runs on | Contains | Why separate |
|---|---|---|---|
| **api** | Cloud Run service | Every bounded context as a module: HTTP, access checks, use cases, persistence | One place for business logic (TSD §3.1) |
| **device-gateway** | Cloud Run service | Device adapters (eSSL first) and the Time Capture write path for RawPunch only | Device traffic must not slow the API; new device makes are new adapters (TSD §3.1, PC-R1, N12) |
| **jobs** | Cloud Run jobs, plus job endpoints on api triggered by Scheduler and Tasks | Nightly finalisation, period end, accrual, expiry, escalations, reminders (TSD §10) | Long-running work outside request limits |
| **web** | Firebase App Hosting / Cloud Run | Next.js UI; no business rules | Client |
| **mobile** | Android and iOS (Flutter) | Punch, offline queue, requests; no business rules | Client |

All three backend units are built from **one Rust workspace** and share the same domain and calculation crates (TSD §5.1). V1 does **not** split contexts into separately deployed services. Each context is kept separable instead (§3), so a context can later move to its own service where scale or operations justify it (ADR-001).

## 2. Workspace layout (from TSD §5.1, mapped to contexts)

| Crate | Holds | Depends on |
|---|---|---|
| `shared` | IDs, money and time value types (exact decimal, UTC plus zone), error types | — |
| `core` | **Pure** calculation engine (DayEngine, PeriodEngine, GeoEvaluator, PolicyResolver, ScheduleResolver logic), settings schema, explanation model | shared |
| `domain-<context>` | One per bounded context: aggregates, invariants, use cases, repository traits | shared, core |
| `db` | Repository implementations, migrations, organisation-scoped transaction set-up (TSD §4.1) | domain-*, shared |
| `api` | HTTP transport, sign-in, operation registry (module, permission, scope: TSD §7) | domain-*, db |
| `gateway` | Device adapters, RawPunch ingestion | domain-time-capture, db |
| `jobs` | Job entry points | domain-*, db |

Rule: `core` contains no database, clock or network access (TSD §3.3).

## 3. Data ownership

Each context owns its aggregates' data. Other contexts:
- **read** through the owning context's query interface (in-process in V1), never by querying its tables directly in domain code;
- **never write** another context's data. Effects across contexts go through a use case of the owning context or a domain event (§4).

| Context | Owns | Main readers |
|---|---|---|
| Platform | Organisation, entitlements, plans, templates, support grants | all (entitlement checks) |
| Identity & Access | Users, roles, scopes, delegations, approver rules | all (authorisation) |
| Organisation Structure | Branches, fences, lists, contractors, org settings | Workforce, Time Capture, Scheduling |
| Workforce | Employee and dated assignments | nearly all |
| Time Capture | Devices, enrolment mappings, RawPunch, PunchFlag | Attendance |
| Scheduling | Shifts, rosters, weekly offs, holidays | Attendance, Leave, Permission |
| Policy | Policy sets, versions, assignments, exemptions | Attendance, Leave, Permission, Period |
| Attendance | AttendanceDay/DayResult, regularisation, waivers, counters | Period, Leave (classification), Reporting |
| Leave & Absence | Leave types, accounts, requests, comp-off, absconding | Attendance (overlays), Period |
| Permission & On-duty | Types, accounts, requests | Attendance (overlays) |
| Period | Periods, totals, adjustments, payroll input exports | Payroll (V2), Reporting |
| Opening Balances | Batches | posts into Leave, Permission, CompOff, Attendance counters through their use cases |
| Notifications | Rules, messages | — |
| Audit | Audit entries | Reporting, Auditor |
| Reporting & Dashboards | Read models only (rebuildable) | — |

## 4. Interaction patterns

| Pattern | Used for | Guarantee |
|---|---|---|
| In-process call within one database transaction | A use case and its own aggregate, its audit entry (TSD §3.3), and its outbox record | Atomic |
| **Transactional outbox → Cloud Tasks** | "Input changed → recalculate (org, employee, date)"; notifications; ledger postings caused by another context's event | The event is written in the same transaction as the change; a relay hands it to Cloud Tasks; consumers are idempotent (TSD §5.5) |
| Cloud Scheduler → job endpoint → one Cloud Task per organisation | Timed work J1–J12 | Each job is safe to run twice (TB-03, AT21) |
| Nightly finalisation J1 | A safety net: re-processes the previous day for every organisation, so a lost task cannot leave a day stale | Idempotent via the input fingerprint |

No distributed transactions. Cross-context consistency is eventual and converges through idempotent recalculation.

### Domain events (V1, internal)
`PunchRecorded`, `PunchFlagDecided`, `RequestApproved` / `RequestCancelled` (by type), `WaiverChanged`, `PolicyVersionActivated`, `RosterChanged`, `HolidayCalendarChanged`, `EmployeeAssignmentChanged`, `OpeningBalanceConfirmed`, `DayResultChanged`, `PeriodLocked`, `PeriodUnlocked`, `AdjustmentPosted`.
Each carries organisation, aggregate ID, the dates affected, and an event ID used for consumer idempotency.

## 5. Tenancy enforcement points

1. Sign-in token → organisation. Device serial number → organisation (gateway).
2. Each request or task runs in one transaction that first sets the organisation; row-level security fails closed (TSD §4.1, TB-01).
3. Operation registry: module entitlement + permission + data scope are checked before the use case runs (TSD §7, N5).
4. Files: the path starts with the organisation; links are short-lived and issued after the same check (TB-02).
5. Task and job payloads carry one organisation (TSD §4.2).

## 6. What is not in V1 infrastructure

- **Pub/Sub:** not in the decided stack (BRD §22.1). Cloud Tasks covers V1 work queues. Revisit via ADR when a second independently deployed service needs to consume events (OQ-X-09, DP-12).
- **Redis:** not in the decided stack. Sessions, rate limits and dashboard summaries are kept in Postgres in V1 (TSD J11). Revisit if load tests (NFR-02.1) show a need.
