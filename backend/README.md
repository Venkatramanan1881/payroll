# Backend (Rust)

Workspace for the Payroll SaaS API. Design docs: `.claude/skills/payroll-saas-engineering/docs/`.

## Tests

Tests create and drop their own databases, so they must only ever point at a **throwaway local PostgreSQL 16**, never Supabase:

```sh
# one-off: a disposable cluster on port 55432 (trust auth, local only)
initdb -D /tmp/pgdev -U postgres -A trust -E UTF8 --locale=C
pg_ctl -D /tmp/pgdev -o "-p 55432 -c listen_addresses=localhost -c max_connections=200" -l /tmp/pgdev.log start

export DATABASE_URL=postgres://postgres@localhost:55432/postgres
cargo test --workspace
```

Migrations live in `crates/db/migrations/` and are embedded as `db::MIGRATOR`.
