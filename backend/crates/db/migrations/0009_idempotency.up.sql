-- 0009 Idempotency keys for POST commands (API_CONVENTIONS §5). Approved 2026-11-17 (Gate 3 amendment).
-- Same key + same body -> stored response; same key + different body -> IDEMPOTENCY_CONFLICT. Rows older than 24 h may be purged.
CREATE TABLE app.idempotency_key (
  org_id          uuid NOT NULL REFERENCES platform.organisation (id),
  key             uuid NOT NULL,
  request_hash    bytea NOT NULL,
  response_status integer,
  response_body   jsonb,
  created_at      timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (org_id, key)
);
SELECT app.register_tenant_table('app.idempotency_key');
