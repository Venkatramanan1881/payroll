-- Reverses 0001. Roles are left in place (shared across databases in the cluster); drop them manually if required.
DROP SCHEMA IF EXISTS audit CASCADE;
DROP SCHEMA IF EXISTS platform CASCADE;
DROP SCHEMA IF EXISTS app CASCADE;
DROP EXTENSION IF EXISTS btree_gist;
