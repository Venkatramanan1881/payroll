# Non-functional and technical-baseline requirements

Status: Provisional (BRD v0.7, TSD v0.1 drafts)
Sources: BRD §15 (N1–N12), §16, §18, §19 (AT20, AT21), §22 (TB-01–TB-09); TSD v0.1

These requirements apply across every module. Each one gets testable acceptance criteria here, which later phases trace to tests.

## 1. Requirements

| ID | Requirement | Acceptance criteria (Provisional) | Release |
|---|---|---|---|
| N1 / TB-01 | Organisation data fully separated, enforced by the database as well as the application. The only exception is time-limited, logged vendor support access (VC-R6). | NFR-01.1 A signed-in user of org A gets "not found" or "forbidden" on every screen, API route, report, export and file link of org B (AT20). NFR-01.2 A request whose organisation is not set returns no client rows (fails closed; TSD §4.1). NFR-01.3 The organisation is taken from the verified sign-in, never from the request body (TSD §4.1). NFR-01.4 A new client table without the database-level organisation rule fails the build (TSD §4.5). | V1 |
| N2 | Sized for hundreds of organisations and up to 2,000 employees each (proposed: 500 organisations in the first two years). | NFR-02.1 A load test at the confirmed target passes the N3 timings. | V1 |
| N3 | A changed day is recalculated within seconds; a 2,000-employee monthly pay run finishes within 5 minutes (proposed). | NFR-03.1 The p95 time from a punch being saved to the day result updating is ≤ the confirmed number of seconds. NFR-03.2 (V2) pay run ≤ 5 min for 2,000 employees. | V1 / V2 |
| N4 | The app records punches without a network and syncs with the original punch time. | Covered in C04 stories. | V1 |
| N5 | Role and scope checks on every screen and API. | NFR-05.1 Every API operation declares its module, permission and data scope; an undeclared operation fails the build (TSD §7). | V1 |
| N6 | Web admin on current desktop browsers; app on Android and iOS. | NFR-06.1 Supported browser and OS versions are listed and pass the UAT script. | V1 |
| N7 | English, Tamil, Hindi for app, payslips and notifications; new languages without code change. | NFR-07.1 Every user-facing string in app, web and notifications displays in all three languages (AT14). NFR-07.2 Adding a language needs only translation data. | V1 |
| N8 | 99.5% availability (proposed). | NFR-08.1 Monthly uptime of API, device gateway and web ≥ the confirmed target. | V1 |
| N9 | Personal, pay, bank, location and document data encrypted in transit and at rest. | NFR-09.1 No plaintext transport. NFR-09.2 Bank and identity numbers encrypted at field level and masked on screen by default (TSD §15). | V1 |
| N10 | Records kept for the period required by law (to confirm). | Blocked: OQ-X-08. | V1 |
| N11 / TB-08 | Daily backups with tested restore; point-in-time recovery once live payroll data is held. | NFR-11.1 A restore is tested and recorded every quarter (TSD §15). | V1 |
| N12 | New device makes and new modules can be added without changing existing modules. | NFR-12.1 A second device adapter can be added without changing punch processing (PC-R1). | V1 |
| TB-02 | Files private, one folder per organisation, opened only through short-lived links after an access check. | NFR-13.1 A file link expires after the confirmed time. NFR-13.2 A link issued to org A cannot open an org B file. | V1 |
| TB-03 / AT21 | Timed work started by Cloud Scheduler; failed runs retried and shown in the vendor health view; every timed job is safe to run twice. | NFR-14.1 Running any timed job twice for the same date leaves results unchanged (AT21). NFR-14.2 A failed run appears in VC-R8. | V1 |
| TB-04 | Devices reach the server over a secure connection, or through an approved relay. | Blocked by Q15 / TSD D1. | V1 |
| TB-05 | Money in exact decimals; rounding only as BR-15 states and where PY-06 says so (BRD v0.8). | NFR-15.1 P1–P17 reproduce the BRD figures to the paisa (AT1). NFR-15.2 Bank + cash = net pay in every case (AT10). NFR-15.3 The code names its rounding mode explicitly on every rounding call; no library default is relied on. | V2 |
| TB-06 | Standard Postgres only; sign-in and files independent of Supabase. | NFR-16.1 No Supabase-specific feature is used (TSD §17). | V1 |
| TB-07 | All data and files in the India region. | NFR-17.1 Every data store and bucket is in an India region. | V1 |
| TB-09 | Test and live separate; client data not copied into test without masking. | NFR-18.1 Separate projects per environment (TSD §16). | V1 |
| §16 | Audit log of every create, change, approval, waiver, lock, payment and settings change, with user and time. | NFR-19.1 Each audited action records actor, organisation, action, target, time, and before/after or a change reference. NFR-19.2 The audit log cannot be edited through the application (TSD §15). | V1 |
| K5 | Identity, bank, location and face data need a privacy review (consent, purpose, access, retention). | NFR-20.1 Consent text for location, selfie and documents is shown and its acceptance recorded (TSD §15). Retention is blocked by OQ-X-08. | V1 |

## 2. Architecture decisions recorded in the BRD/TSD (context, not stories)

Rust backend; Next.js web; Flutter mobile; Postgres on Supabase; Cloud Run; Firebase hosting and push; Cloud Storage; Cloud Scheduler and Cloud Tasks; Mumbai region (BRD §22.1). Sign-in is built into the backend (TSD §8, proposed); Supabase Auth is not used (TB-06).

Pub/Sub and Redis are **not** in the BRD/TSD decisions. See OQ-X-09.
