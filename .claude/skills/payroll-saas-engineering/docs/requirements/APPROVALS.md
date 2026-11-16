# Approval gate log

| Gate | Scope | Decision | By | Date | Conditions |
|---|---|---|---|---|---|
| 1 Requirements | V1 user stories, acceptance criteria, business rules catalogue, NFRs, traceability (`docs/product/`) | Approved | Project owner (in session) | 2026-11-15 | The blocking questions in `OPEN_QUESTIONS.md` §2b were **not** answered at approval. Phase 1 models each one as an explicit open decision point (`DP-nn` in `docs/domain/DOMAIN_MODEL.md`) instead of assuming an answer. BRD v0.7 and TSD v0.1 remain drafts (Q14). Pub/Sub and Redis remain out of the stack as decided in BRD §22.1, pending OQ-X-09. |
| 2 Domain (part 1) | `docs/domain/DOMAIN_MODEL.md`, `GLOSSARY.md`, `docs/architecture/SERVICE_BOUNDARIES.md`, ADR-001, ADR-002 | Approved | Project owner (in session) | 2026-11-15 | Decision points DP-01..DP-12 remain open. ADR-002's exclusion of Pub/Sub and Redis is accepted as part of this approval. State machines and permission matrix were still being written at approval time and are **not** covered; they need their own approval (Gate 2 part 2). |
| 2 Domain (part 2) | `docs/domain/STATE_MACHINES.md`, `docs/domain/PERMISSION_MATRIX.md` | Approved | Project owner (in session) | 2026-11-15 | GAP-SM-01..17 and permission gaps G-01..G-40 remain open. Permission gaps affect default role grants (seed data) only. SM-DAY "Exception" grouping accepted as a modelling choice. |

## Source document changes after approval

| Date | Change | Effect on approved gates |
|---|---|---|
| 2026-11-15 | BRD v0.7 → v0.8 at the project owner's request: pay rounding fixed (BR-15 added, TB-05 reworded, §13.2 working shown, P17 and Q17 added). v0.7 kept for traceability. | None. The change touches V2 pay only; no V1 story, rule, domain object or table depends on it. No re-approval needed. |

| Gate | Scope | Decision | By | Date | Conditions |
|---|---|---|---|---|---|
| 3 Database | `docs/database/` (DATA_MODEL, migrations 0001–0005, DATA_DICTIONARY, MIGRATIONS, tests/tenant_isolation.sql) | Approved | Project owner (in session) | 2026-11-15 | Supabase deployment checks in MIGRATIONS.md §3 still to be run before the first test-environment migration. "?" permission cells seeded as not granted. |
