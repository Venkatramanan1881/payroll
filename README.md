# Payroll SaaS

Multi-tenant attendance, leave and payroll platform. Rust backend (Axum, SQLx, PostgreSQL).

| Folder | What it holds |
|---|---|
| `docs/` | Project documentation: requirements (BRD, TSD, decisions, open questions), product, domain, database, API contracts, plans |
| `backend/` | Application code: Rust crates `db`, `domain-iam`, `domain-workforce`, `api` |
| `web/` | Next.js web app (admin, self-service, vendor console); see `web/README.md` |
| `.claude/skills/payroll-saas-engineering/` | The engineering skill (process and guardrails) used to build this project |

The Cargo workspace is at the repository root (`Cargo.toml`); migrations live in `backend/crates/db/migrations/` and are embedded as `db::MIGRATOR`.

## Tests

Tests create and drop their own databases, so they must only ever point at a **throwaway local PostgreSQL 16**, never Supabase:

```sh
# one-off: a disposable cluster on port 55432 (trust auth, local only)
initdb -D /tmp/pgdev -U postgres -A trust -E UTF8 --locale=C
pg_ctl -D /tmp/pgdev -o "-p 55432 -c listen_addresses=localhost -c max_connections=200" -l /tmp/pgdev.log start

export DATABASE_URL=postgres://postgres@localhost:55432/postgres
cargo test --workspace
```

API contract checks (need `pip install openapi-spec-validator pyyaml`): `python docs/api/check_contracts.py` and `python docs/api/check_enum_drift.py`.
