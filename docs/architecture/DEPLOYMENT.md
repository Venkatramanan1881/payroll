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
| `DATABASE_URL` | Secret Manager | API service: `api_login`; jobs service: `jobs_login` (§3); never `postgres`. Migrating: the owner role. Use Supavisor **session mode** (port 5432) or a direct connection: transaction mode (6543) breaks prepared statements. |
| `APP_JWT_SECRET` | Secret Manager | Signs access tokens. 32+ random bytes. |
| `FIREBASE_PROJECT_ID` | plain env | Phone sign-in token audience (D-12). |
| `JOB_SECRET` | Secret Manager | **Jobs service only.** Enables `/internal/jobs/*`. 32+ characters (surrounding whitespace is trimmed), or the job endpoints stay off and a line is logged. |
| `TRUSTED_PROXY_HOPS` | plain env | `1` behind the Cloud Run front end (PC-05 client IP). |
| `PORT` | Cloud Run | Set by the platform. |

Secrets never go into git, images or chat.

## 3. Database (Supabase, Mumbai)

### Roles

- Migrations run as the project owner (`postgres`).
- After migrations, run `docs/database/supabase_roles.sql` once per project. It creates two login roles, neither with rights of its own (members with SET, without inheriting):
  - `api_login` for the internet-facing API: a member of `app_api` only.
  - `jobs_login` for the timed-jobs service: a member of `app_api` and `app_vendor`. `app_vendor` reads every tenant's platform rows, so only this private service holds it.
- Every request transaction switches to `app_api`, so row-level security always applies.

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

`--min-instances 1` keeps punches fast (TSD §19.3). The API service gets **no** `JOB_SECRET`, so `/internal/jobs/*` answers 404 there.

The jobs run from a second service built from the same image, reachable only by Cloud Scheduler:

```
gcloud run deploy payroll-jobs --image <image> --region asia-south1 \
  --service-account payroll-jobs@<gcp-project>.iam.gserviceaccount.com \
  --set-secrets DATABASE_URL=jobs-db-url:latest,APP_JWT_SECRET=jwt-secret:latest,JOB_SECRET=job-secret:latest \
  --set-env-vars FIREBASE_PROJECT_ID=<firebase-project>,TRUSTED_PROXY_HOPS=1 \
  --allow-unauthenticated --min-instances 0 --timeout 1800
```

Until the OIDC upgrade (§5), the job secret is the only gate: Cloud Run IAM would claim the `Authorization` header the secret travels in. The API service never holds the secret, so a leak of the public service exposes nothing here.

## 5. Timed jobs (Cloud Scheduler, India time)

| Job | Schedule | Request |
|---|---|---|
| J1 nightly recalculation | `0 3 * * *` | `POST /internal/jobs/nightly-recalculation` |
| J3 approval timeouts | `0 * * * *` | `POST /internal/jobs/approval-timeouts` |

- Requests go to the `payroll-jobs` service and carry the job secret as `Authorization: Bearer $JOB_SECRET`.
- Set the scheduler's attempt deadline to 30 minutes (`--attempt-deadline 1800s`), matching the service timeout. One request walks every organisation in turn: J1 uses one short transaction per employee, J3 one per organisation with a savepoint per request, and one organisation's failure is reported without stopping the rest.
- When a run nears the deadline, fan out one Cloud Task per organisation (marked `shortcut:` in the code).
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
5. Scheduler jobs point at the `payroll-jobs` service URL.
