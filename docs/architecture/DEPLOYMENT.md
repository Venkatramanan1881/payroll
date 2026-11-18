# Deployment (Cloud Run + Supabase)

How the API is built, configured and released (TSD §2, §10, §12). Nothing here is run automatically. Each step is a deliberate, approved action, run by a person or by CI with its own credentials.

## 1. Build

```
docker build -f backend/Dockerfile -t asia-south1-docker.pkg.dev/<gcp-project>/payroll/api:<git-sha> .
docker push asia-south1-docker.pkg.dev/<gcp-project>/payroll/api:<git-sha>
```

- The image holds one binary, `api`.
- `api` serves HTTP on `$PORT` (default 8080).
- `api migrate` applies the embedded migrations and exits.
- The image runs as a non-root user.

## 2. Configuration (environment)

| Variable | Source | Notes |
|---|---|---|
| `DATABASE_URL` | Secret Manager | Serving: connect as `api_login` (§3), never `postgres`. Migrating: the owner role. |
| `APP_JWT_SECRET` | Secret Manager | Signs access tokens. 32+ random bytes. |
| `FIREBASE_PROJECT_ID` | plain env | Phone sign-in token audience (D-12). |
| `JOB_SECRET` | Secret Manager | Enables `/internal/jobs/*`. 32+ characters, or the job endpoints stay off. |
| `TRUSTED_PROXY_HOPS` | plain env | `1` behind the Cloud Run front end (PC-05 client IP). |
| `PORT` | Cloud Run | Set by the platform. |

Secrets never go into git, images or chat.

## 3. Database (Supabase, Mumbai)

### Roles

- Migrations run as the project owner (`postgres`).
- After migrations, run `docs/database/supabase_roles.sql` once per project. It creates `api_login`:
  - a login role with no rights of its own;
  - a member of `app_api` and `app_vendor`, so it can switch to them (SET), without inheriting their rights.
- Every API transaction switches to `app_api`, so row-level security always applies.

### Migrations

```
DATABASE_URL=<owner connection> api migrate
```

- Run as a one-off Cloud Run job (or from CI) before releasing an image that needs them.
- Serving instances never migrate.
- The migrations are written to run as a non-superuser owner on PostgreSQL 16+ (0002/0003 grant `app_resolver` membership and schema CREATE only while function ownership moves).

### The current test project

- `izxqgkbiulmbgrbpvfzq` received migrations 0001–0009 through the Supabase console tooling, not through `api migrate`. It therefore has no `_sqlx_migrations` record, and `api migrate` would try to apply everything again.
- Because it holds no data, the simplest fix is to reset the project's application schemas and then run `api migrate`.
- Alternatively, record 0001–0009 as applied (the checksums must match the files in git).
- Either way, it needs explicit approval before it is done.

## 4. Cloud Run service

```
gcloud run deploy payroll-api --image <image> --region asia-south1 \
  --service-account payroll-api@<gcp-project>.iam.gserviceaccount.com \
  --set-secrets DATABASE_URL=api-db-url:latest,APP_JWT_SECRET=jwt-secret:latest,JOB_SECRET=job-secret:latest \
  --set-env-vars FIREBASE_PROJECT_ID=<firebase-project>,TRUSTED_PROXY_HOPS=1 \
  --min-instances 1 --cpu 1 --memory 512Mi
```

`--min-instances 1` keeps punches fast (TSD §19.3).

## 5. Timed jobs (Cloud Scheduler, India time)

| Job | Schedule | Request |
|---|---|---|
| J1 nightly recalculation | `0 3 * * *` | `POST /internal/jobs/nightly-recalculation` |
| J3 approval timeouts | `0 * * * *` | `POST /internal/jobs/approval-timeouts` |

- Each request carries `Authorization: Bearer $JOB_SECRET`.
- Both jobs are idempotent, so a retried or doubled run changes nothing more:
  - J1 writes nothing when inputs are unchanged;
  - J3 escalates once per level.
- The other jobs in TSD §10 (J2, J4–J12) arrive with their slices.
- Planned upgrade: replace the shared secret with Cloud Scheduler OIDC tokens checked for the service's audience.

## 6. Release checklist

1. All tests pass (`cargo test --workspace` against a throwaway PostgreSQL) and clippy is clean.
2. Migrations reviewed; any new one approved (Gate 3 amendment).
3. `api migrate` run against the target database as the owner.
4. Deploy the image; check `/health`.
5. Scheduler jobs point at the new service URL.
