-- Supabase TEST project permission checks (MIGRATIONS.md §3; decision #15, 2026-11-16).
-- Run ONLY against the TEST project, connected as the role migrations will use (normally `postgres`).
-- Everything runs inside one transaction that is ROLLED BACK: nothing is left behind.
--
-- How to run (keep the connection string out of chat and out of git):
--   set the variable in your own terminal, then:
--   psql "$SUPABASE_TEST_DB_URL" -v ON_ERROR_STOP=1 -f supabase_permission_check.sql
-- Each check prints PASS or FAIL. Send back only the printed PASS/FAIL lines.

\set ON_ERROR_STOP 0
BEGIN;

SELECT current_user AS connected_as, current_database() AS database, version();

-- 1. Can this role create a role that bypasses RLS? (needed for app_resolver)
SAVEPOINT s1;
DO $$ BEGIN
  CREATE ROLE zz_check_resolver NOLOGIN BYPASSRLS;
  RAISE NOTICE 'PASS 1: can create a BYPASSRLS role';
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'FAIL 1: cannot create a BYPASSRLS role: %', SQLERRM;
END $$;

-- 2. Can a function be handed to that role as owner? (iam.resolve_login, app.resolve_device_org)
SAVEPOINT s2;
DO $$ BEGIN
  CREATE SCHEMA zz_check_schema;
  CREATE FUNCTION zz_check_schema.f() RETURNS int LANGUAGE sql AS 'SELECT 1';
  EXECUTE 'GRANT USAGE, CREATE ON SCHEMA zz_check_schema TO zz_check_resolver';
  EXECUTE 'GRANT zz_check_resolver TO ' || quote_ident(current_user);
  ALTER FUNCTION zz_check_schema.f() OWNER TO zz_check_resolver;
  RAISE NOTICE 'PASS 2: function ownership can be transferred to the resolver role';
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'FAIL 2: cannot transfer function ownership: %', SQLERRM;
END $$;

-- 3. Is the btree_gist extension available and creatable? (exclusion constraints on date ranges)
SAVEPOINT s3;
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'btree_gist') THEN
    RAISE NOTICE 'PASS 3: btree_gist already installed';
  ELSE
    CREATE EXTENSION btree_gist;
    RAISE NOTICE 'PASS 3: btree_gist can be created';
  END IF;
EXCEPTION WHEN OTHERS THEN
  RAISE NOTICE 'FAIL 3: btree_gist not available: %', SQLERRM;
END $$;

-- 4. Information only: server region setting is not visible from SQL; confirm Mumbai in the Supabase dashboard (D-14).

ROLLBACK;
\echo 'Done. Everything above was rolled back.'
