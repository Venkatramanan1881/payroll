-- Login roles for the API and the jobs on Supabase (DEPLOYMENT.md §3). Run once per project as "postgres", AFTER migrations.
-- The passwords never go into git or chat: pass them as psql variables from your own terminal, e.g.
--   psql "$SUPABASE_DB_URL" -v ON_ERROR_STOP=1 -v api_password="$API_DB_PASSWORD" -v jobs_password="$JOBS_DB_PASSWORD" \
--        -v vendor_password="$VENDOR_DB_PASSWORD" -f supabase_roles.sql
--
-- api_login: what the internet-facing API service connects as. It owns nothing and bypasses nothing; each
--   transaction switches to app_api (SET LOCAL ROLE), so row-level security always applies (TB-01).
-- vendor_login: what the vendor console service connects as (D-22). Member of app_vendor (platform rows of every
--   organisation) and app_api (it seeds a new organisation's roles and first admin under that organisation).
-- jobs_login: what the timed-jobs service connects as. Also a member of app_vendor, which lists active
--   organisations before the jobs work per organisation as app_api. app_vendor reads every tenant's platform rows,
--   so the internet-facing role is deliberately NOT a member of it. Also a member of app_relay, the only role that
--   reads and marks the outbox (notifications-dispatch, ADR-002).

\set ON_ERROR_STOP 1
BEGIN;

DO $$ BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'api_login') THEN
    CREATE ROLE api_login LOGIN NOINHERIT NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'jobs_login') THEN
    CREATE ROLE jobs_login LOGIN NOINHERIT NOBYPASSRLS;
  END IF;
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'vendor_login') THEN
    CREATE ROLE vendor_login LOGIN NOINHERIT NOBYPASSRLS;
  END IF;
END $$;
ALTER ROLE api_login  WITH PASSWORD :'api_password';
ALTER ROLE jobs_login WITH PASSWORD :'jobs_password';
ALTER ROLE vendor_login WITH PASSWORD :'vendor_password';

-- PostgreSQL 16+: SET ROLE needs membership with the SET option; INHERIT FALSE keeps the login roles' own rights at none.
GRANT app_api    TO api_login  WITH INHERIT FALSE, SET TRUE;
REVOKE app_vendor FROM api_login;  -- an earlier version of this script granted it
GRANT app_api    TO jobs_login WITH INHERIT FALSE, SET TRUE;
GRANT app_vendor TO jobs_login WITH INHERIT FALSE, SET TRUE;
GRANT app_relay  TO jobs_login WITH INHERIT FALSE, SET TRUE;
GRANT app_api    TO vendor_login WITH INHERIT FALSE, SET TRUE;
GRANT app_vendor TO vendor_login WITH INHERIT FALSE, SET TRUE;

-- Checks: no login role bypasses RLS, and the internet-facing one cannot become the vendor role.
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname IN ('api_login','jobs_login','vendor_login') AND rolbypassrls) THEN
    RAISE EXCEPTION 'login roles must not bypass RLS';
  END IF;
  IF pg_has_role('api_login', 'app_vendor', 'SET') THEN
    RAISE EXCEPTION 'api_login must not be able to SET ROLE app_vendor';
  END IF;
END $$;

COMMIT;
\echo 'Ready: the API connects as api_login, the jobs service as jobs_login (Supavisor user name: <role>.<project-ref>).'
