# Payroll web app

Next.js app with three areas: the organisation admin (`/app`), employee self-service (`/me`) and the vendor console (`/vendor`). TSD §12 and D-42 set the ground rules:
- **No business rules here.** Every rule is decided by the Rust API.
- **Tokens stay in httpOnly cookies.** The browser talks only to this app's route handlers:
  - `/bff/auth/*` for the sign-in steps;
  - `/bff/api/*` to reach `/v1/*` with the session's bearer token.
- **Every string comes from `messages/<locale>.json`.** English is the only language so far.

## Run locally

Use only the throwaway PostgreSQL on port 55432, never Supabase or the PostgreSQL Windows service.

1. **Create and seed a dev database.** The seed makes:
   - one organisation;
   - an HR Manager who signs in with a password and no authenticator step;
   - two branches and a few departments and designations.

   The seed refuses any database that is not on localhost.

   ```sh
   createdb -h localhost -p 55432 -U postgres payroll_dev
   DATABASE_URL=postgres://postgres@localhost:55432/payroll_dev DEV_ADMIN_PASSWORD='<12+ chars>' \
     cargo run -p api --example dev_seed
   ```

2. **Start the API.** `APP_JWT_SECRET` can be any long random string; `FIREBASE_PROJECT_ID` can be any name locally.

   ```sh
   DATABASE_URL=postgres://postgres@localhost:55432/payroll_dev APP_JWT_SECRET='<random>' \
     FIREBASE_PROJECT_ID=dev-local PORT=8080 cargo run -p api
   ```

3. **Start the web app:**

   ```sh
   cd web && cp .env.example .env.local   # API_BASE_URL=http://localhost:8080
   npm install && npm run dev               # http://localhost:3000, sign in as hr@dev.invalid
   ```

Only email-and-password sign-in works locally:
- **Employee mobile sign-in** needs the Firebase web config (`NEXT_PUBLIC_FIREBASE_*`).
- **Vendor Google sign-in** needs the OAuth client id. The API must also run with `VENDOR_GOOGLE_CLIENT_ID` and `VENDOR_GOOGLE_DOMAIN`.
- `NEXT_PUBLIC_*` values are built into the page when the app is built. They are public configuration, not secrets.

## Checks

```sh
npm run lint && npm run typecheck && npm test && npm run build
E2E_EMAIL=hr@dev.invalid E2E_PASSWORD='<seed password>' npm run e2e   # needs the API and `npm run dev` running
```

On Windows, run Vitest from a path that starts with an upper-case drive letter (`D:\...`). From Git Bash's `/d/...` path it loads itself twice and finds no tests.
