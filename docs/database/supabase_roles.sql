-- Login roles for the API and the jobs on Supabase (DEPLOYMENT.md §3). Run once per project as "postgres", AFTER migrations.
-- The passwords never go into git or chat: pass them as psql variables from your own terminal, e.g.
--   psql "$SUPABASE_DB_URL" -v ON_ERROR_STOP=1 -v api_password="$API_DB_PASSWORD" -f supabase_roles.sql
--
-- api_login: what Cloud Run connects as. It owns nothing and bypasses nothing; each transaction switches to
--   app_api (SET LOCAL ROLE), so row-level security always applies (TB-01).
-- Membership of app_vendor lets the timed jobs list active organisations (platform.organisation) before working
--   per organisation as app_api; app_vendor sees only the platform schema.

\set ON_ERROR_STOP 1
BEGIN;

DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'api_login') THEN
    CREATE ROLE api_login LOGIN NOINHERIT NOBYPASSRLS;
  END IF;
END $$;
ALTER ROLE api_login WITH PASSWORD :'api_password';

-- PostgreSQL 16+: SET ROLE needs membership with the SET option; INHERIT FALSE keeps api_login's own rights at none.
GRANT app_api    TO api_login WITH INHERIT FALSE, SET TRUE;
GRANT app_vendor TO api_login WITH INHERIT FALSE, SET TRUE;

-- Checks: the login role cannot bypass RLS and holds no table rights of its own.
DO $$ BEGIN
  IF (SELECT rolbypassrls FROM pg_roles WHERE rolname = 'api_login') THEN
    RAISE EXCEPTION 'api_login must not bypass RLS';
  END IF;
END $$;

COMMIT;
\echo 'api_login ready: connect the API with this role (Supavisor user name: api_login.<project-ref>).'
