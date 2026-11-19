-- Tenant isolation checks at database level (TB-01, AT20, NFR-01.1..01.4).
-- Run against a disposable database after all migrations: psql -v ON_ERROR_STOP=1 -f tenant_isolation.sql
-- Any failure raises an exception. Runs inside one transaction and rolls back.

\set ON_ERROR_STOP 1
BEGIN;
-- PostgreSQL 16+ (Supabase): SET ROLE needs membership. Granted inside the transaction, so the ROLLBACK removes it.
GRANT app_api TO CURRENT_USER WITH INHERIT FALSE, SET TRUE;

-- 1. Build check: no unprotected tables (NFR-01.4).
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM app.unprotected_tables) THEN
    RAISE EXCEPTION 'unprotected tables: %', (SELECT string_agg(table_name::text, ', ') FROM app.unprotected_tables);
  END IF;
END $$;

-- 2. Every table with an org_id column outside platform/audit/app.outbox uses exactly the standard policy.
DO $$
DECLARE r record;
BEGIN
  FOR r IN
    SELECT c.oid::regclass AS t
    FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace
    WHERE c.relkind = 'r'
      AND n.nspname NOT IN ('platform','audit','pg_catalog','information_schema')
      AND EXISTS (SELECT 1 FROM pg_attribute a WHERE a.attrelid = c.oid AND a.attname = 'org_id' AND NOT a.attisdropped)
      AND c.oid NOT IN (SELECT table_name FROM app.rls_exempt)
  LOOP
    IF NOT EXISTS (
      SELECT 1 FROM pg_policies p
      WHERE format('%I.%I', p.schemaname, p.tablename)::regclass = r.t
        AND p.policyname = 'tenant_isolation'
        AND p.qual = '(org_id = app.current_org())'
        AND p.with_check = '(org_id = app.current_org())') THEN
      RAISE EXCEPTION 'table % lacks the standard tenant_isolation policy', r.t;
    END IF;
  END LOOP;
END $$;

-- 3. No application role may bypass RLS (the non-login resolver is the documented exception).
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname LIKE 'app\_%' AND rolbypassrls AND rolname <> 'app_resolver') THEN
    RAISE EXCEPTION 'an app role has BYPASSRLS';
  END IF;
  IF EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'app_resolver' AND rolcanlogin) THEN
    RAISE EXCEPTION 'app_resolver must not be able to log in';
  END IF;
END $$;

-- 4. Behaviour: rows of organisation A are invisible and unwritable from organisation B, and from an unset organisation.
INSERT INTO platform.vendor_user (id, email, full_name, vendor_role)
VALUES ('00000000-0000-0000-0000-0000000000f1', 'vendor@test.invalid', 'Vendor', 'full_admin');
INSERT INTO platform.organisation (id, legal_name, display_name, created_by) VALUES
  ('00000000-0000-0000-0000-00000000000a', 'Org A', 'A', '00000000-0000-0000-0000-0000000000f1'),
  ('00000000-0000-0000-0000-00000000000b', 'Org B', 'B', '00000000-0000-0000-0000-0000000000f1');

SET LOCAL ROLE app_api;

SELECT app.set_org('00000000-0000-0000-0000-00000000000a');
INSERT INTO org.branch (org_id, id, code, name) VALUES
  ('00000000-0000-0000-0000-00000000000a', '00000000-0000-0000-0000-0000000000a1', 'A-HQ', 'A head office');
INSERT INTO workforce.employee (org_id, id, employee_code, full_name, joining_date, employment_status) VALUES
  ('00000000-0000-0000-0000-00000000000a', '00000000-0000-0000-0000-0000000000a2', 'A001', 'Employee A', '2026-01-01', 'active');

DO $$ BEGIN
  IF (SELECT count(*) FROM org.branch) <> 1 OR (SELECT count(*) FROM platform.organisation) <> 1 THEN
    RAISE EXCEPTION 'org A cannot see its own rows';
  END IF;
END $$;

SELECT app.set_org('00000000-0000-0000-0000-00000000000b');
DO $$ BEGIN
  IF (SELECT count(*) FROM org.branch) <> 0 THEN RAISE EXCEPTION 'org B sees org A branches'; END IF;
  IF (SELECT count(*) FROM workforce.employee) <> 0 THEN RAISE EXCEPTION 'org B sees org A employees'; END IF;
  IF (SELECT count(*) FROM platform.organisation WHERE id = '00000000-0000-0000-0000-00000000000a') <> 0 THEN
    RAISE EXCEPTION 'org B sees org A organisation row';
  END IF;
  -- update/delete of A's rows from B affect nothing
  UPDATE workforce.employee SET full_name = 'hacked' WHERE id = '00000000-0000-0000-0000-0000000000a2';
  IF FOUND THEN RAISE EXCEPTION 'org B updated an org A employee'; END IF;
  DELETE FROM org.branch WHERE id = '00000000-0000-0000-0000-0000000000a1';
  IF FOUND THEN RAISE EXCEPTION 'org B deleted an org A branch'; END IF;
  -- writing a row tagged with org A while acting as B is refused
  BEGIN
    INSERT INTO org.branch (org_id, code, name) VALUES ('00000000-0000-0000-0000-00000000000a', 'X', 'X');
    RAISE EXCEPTION 'org B inserted a row for org A';
  EXCEPTION WHEN insufficient_privilege THEN NULL;  -- RLS WITH CHECK violation
  END;
  -- referencing org A's employee from org B fails the composite FK (row does not exist for B)
  BEGIN
    INSERT INTO workforce.employee_assignment (org_id, employee_id, kind, valid, branch_id)
    VALUES ('00000000-0000-0000-0000-00000000000b', '00000000-0000-0000-0000-0000000000a2', 'master_branch',
            daterange('2026-01-01', NULL), '00000000-0000-0000-0000-0000000000a1');
    RAISE EXCEPTION 'cross-organisation reference accepted';
  EXCEPTION WHEN foreign_key_violation THEN NULL;
  END;
END $$;

-- unset organisation: fails closed
SELECT set_config('app.org_id', '', true);
DO $$ BEGIN
  IF (SELECT count(*) FROM org.branch) <> 0 OR (SELECT count(*) FROM workforce.employee) <> 0 THEN
    RAISE EXCEPTION 'rows visible with no organisation set';
  END IF;
END $$;

-- 5. Append-only: audit entries cannot be changed.
SELECT app.set_org('00000000-0000-0000-0000-00000000000a');
INSERT INTO audit.audit_entry (org_id, actor_kind, action, target_type)
VALUES ('00000000-0000-0000-0000-00000000000a', 'system_job', 'test', 'test');
DO $$ BEGIN
  BEGIN
    UPDATE audit.audit_entry SET action = 'changed';
    RAISE EXCEPTION 'audit entry was updated';
  EXCEPTION WHEN insufficient_privilege THEN NULL;
  END;
END $$;

RESET ROLE;
ROLLBACK;
\echo 'tenant_isolation: all checks passed'
