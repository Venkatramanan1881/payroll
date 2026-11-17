# Migration plan (V1)

Status: Draft for Approval Gate 3

## 1. Files and order

Location: **`backend/crates/db/migrations/`** (moved from `docs/database/migrations/` on 2026-11-17, Phase 5). SQLx reversible format `<version>_<name>.up.sql` / `.down.sql`, embedded by `db::MIGRATOR`. `.gitattributes` pins them to LF line endings, because SQLx checksums the exact bytes and a line-ending change would make an applied migration look modified.

| Version | Creates | Depends on |
|---|---|---|
| 0001 foundation | extension `btree_gist`; roles; `app` helpers, outbox, RLS build check; `platform` (vendor) tables; `audit.audit_entry` | — |
| 0002 identity_org_workforce | `org`, `iam`, `workforce`; `iam.resolve_login` | 0001 |
| 0003 timecap_sched_policy | `timecap`, `sched`, `policy`; `app.resolve_device_org`; FK `org.branch → sched.holiday_calendar` | 0002 |
| 0004 leave_perm_notify | `leave`, `perm`, `notify`, `app.approval_step` | 0002 |
| 0005 attendance_period_openbal | `attendance`, `period`, `openbal`; lock triggers | 0003, 0004 |
| 0006 api_contract_gaps | Found in Phase 4: `workforce.contact_change_request`, `org.settings_change_request`, `period.unlock_request`, `notify.push_device`; columns `platform.organisation.support_access_needs_approval` (VC-03), `notify.notification.read_at`, `leave.comp_off_credit.employee_choice` (CO-R4), `sched.shift_change_request.decided_basis`; `contact_change` request type. **Gate 3 amendment, approval pending** | 0005 |
| 0007 day_finalised_at_lock | Decision D-17: removes `finalised` from `attendance_day.processing_state` (finalised = locked). Approved with Gate 4 | 0005 |
| 0008 decisions_d08_d09 | D-08: `sched.shift_version.missed_punch_cutoff` (default 02:30). D-09: overtime columns on `day_result` and `period_totals` nullable (NULL = not identified). Approved 2026-11-17 | 0007 |
| 0009 idempotency | `app.idempotency_key` for POST commands (API_CONVENTIONS §5). Approved 2026-11-17 | 0001 |

Version order is apply order. 0006, 0007 and 0008 validated 2026-11-16 the same way (98 tables, isolation test passed, full down/up). Validated 2026-11-15 on a throwaway PostgreSQL 16 cluster: all up (94 tables, 15 schemas), RLS build check 0 rows, `tests/tenant_isolation.sql` passed, all down in reverse order, all up again.

## 2. Reversibility

- Every `.up.sql` has a `.down.sql`. On a database with no data, each pair is checked up → down → up.
- On a database with real data, down migrations **destroy data** (DROP SCHEMA … CASCADE). They are for development and test only. In live, a mistake is fixed by a new forward migration, never by running a down migration.
- Roles created in 0001 and 0002 are not dropped by the down files: they are cluster-wide and may be shared.

## 3. How migrations run

| Environment | How | Approval |
|---|---|---|
| Local / CI | Disposable database, all up files, `tests/tenant_isolation.sql`, then down/up round trip | None (automated) |
| Test (separate Supabase project: TB-09) | Pipeline step on merge | Code review of the migration |
| **Live** | Pipeline step after manual approval (TSD §16) | **Explicit approval for each release.** Before running: confirm the target project ID and database name, take a backup or confirm point-in-time recovery, and run against test first |

**Deployment check before the first test-environment run:** migrations ran as a superuser in validation. Under a non-superuser migration role (Supabase `postgres`), confirm three things: it can create the `BYPASSRLS` role `app_resolver`; `ALTER FUNCTION … OWNER TO app_resolver` works (the role needs CREATE on schemas `iam` and `app`); and `CREATE EXTENSION btree_gist` is allowed.

Never: run SQL by hand in a live console, run a down migration on live, or run any migration against a database whose identity has not been confirmed.

## 4. Seed and reference data

Seed data is applied by the application's set-up use cases, not by migrations, so it is audited and per organisation:

| Data | When | Source |
|---|---|---|
| Plans, sector templates (Factory, Supermarket, Restaurant and bakery, Office and services) | Vendor set-up | G6, VC-R7; content from Q5 |
| Per organisation at creation (VC-R1): system roles (BRD §5), **default role grants** from `docs/domain/PERMISSION_MATRIX.md`, organisation settings v1, default policy set, lists, leave and permission types | Copied from the sector template | G6 |
| Notification rules | At creation | OQ-C14-01 (defaults not yet defined) |

**"?" permission cells (G-01..G-40) are seeded as not granted.** They are opened per organisation once decided. This is least privilege; nobody gets a right the BRD hasn't given.

Test data: synthetic only, never real employee data (TB-09).

## 5. Planned later migrations (not in V1)

- **Partition by month** for `timecap.raw_punch`, `attendance.day_result` and `audit.audit_entry` when they grow large (TSD §4.6). This means a new partitioned table plus a batch copy and a swap in a maintenance window. Primary keys will then need the partition column, so the referencing foreign keys get the time column at that point. Planned as its own reviewed migration with a rollback.
- V2 schemas: `payroll`, `loans`, `reimburse`. V2 also adds `payroll_group` to `period.attendance_cycle.owner_kind`.

## 6. Data-changing operations covered by constraints

| Operation | Guard |
|---|---|
| Editing a raw punch, a day result, a ledger entry or an audit entry | Append-only triggers plus no UPDATE/DELETE grants |
| Writing a result for a locked date | Trigger on `attendance.day_result` against `period.lock_coverage` |
| Overlapping dated assignments, versions or periods | Gist exclusion constraints |
| Cross-organisation references | Composite foreign keys plus RLS |
| Editing a policy or shift version in force | Edit-guard triggers (G5) |

## 7. Backup and recovery

Daily backups from day one; point-in-time recovery once live payroll data is held (TB-08, TSD D8). A restore is tested every quarter into a separate project (NFR-11.1), and the tenant isolation test is run on the restored copy.
