# Error catalog (V1)

Status: Draft for Approval Gate 4

Every error is `application/problem+json` (RFC 9457), as defined by `Problem` in `openapi/common.yaml`:

```json
{ "type": "https://errors.payroll.example/PERIOD_LOCKED", "title": "Period is locked",
  "status": 409, "code": "PERIOD_LOCKED", "detail": "28 Oct 2026 is in a locked period; the change was posted as an adjustment to Nov 2026.",
  "correlation_id": "…", "errors": [ { "field": "…", "code": "…", "message": "…" } ] }
```

`code` is stable and never translated; `title` and `detail` follow the user's language (N7). Responses never include stack traces, SQL, or another organisation's data.

| Code | HTTP | When | Source |
|---|---|---|---|
| `UNAUTHENTICATED` | 401 | Missing, expired or revoked token | TSD §8 |
| `SECOND_FACTOR_REQUIRED` | 401 | Admin sign-in needs the second step (AC-08) | AC-08 |
| `ORGANISATION_CHOICE_REQUIRED` | 409 | Sign-in identity exists in more than one organisation | OQ-C03-05 |
| `OTP_INVALID` | 400 | Not used for employee sign-in since D-12 (Firebase checks the code; a bad Firebase token is `UNAUTHENTICATED`). Kept for any future server-checked code | UM-R3 |
| `RATE_LIMITED` | 429 | Too many sign-in, OTP or punch attempts (`Retry-After` set) | TSD §15 |
| `FORBIDDEN` | 403 | Role has no grant for the operation | AC-01, N5 |
| `NOT_FOUND` | 404 | No such resource **or** outside the user's scope or organisation | TB-01, AC-07 |
| `ORG_SUSPENDED` | 403 | Write attempted while the organisation is suspended | VC-R4 |
| `MODULE_DISABLED` | 403 | Operation's module is off for the organisation | MD-03, BR-14 |
| `RULE_GROUP_DISABLED` | 409 | Operation depends on a rule group or an on/off setting that is off (e.g. regularisation with RG-01 off, shift swaps with SH-12 off) | G2, MD-05 |
| `MODULE_DEPENDENCY_UNMET` | 409 | Vendor switches on a module whose dependency is off, or off a module others depend on (block or cascade: OQ-V01-03) | MD-04 |
| `VALIDATION_FAILED` | 422 | Field errors (listed in `errors`). Where a configured rule forbids the request and no specific code exists (PM-21, PM-22, PM-26, LM-12), the field error's `code` is the setting ID | — |
| `ILLEGAL_TRANSITION` | 409 | Command not allowed from the current state | STATE_MACHINES §0 |
| `SELF_APPROVAL_FORBIDDEN` | 403 | Approver is the requester, including via delegation | BR-07 |
| `PERIOD_LOCKED` | 409 | Change to a date in a locked period where the operation cannot post an adjustment | BR-06 |
| `ADJUSTMENT_WINDOW_EXCEEDED` | 409 | Correction older than PL-07 allows (outcome still OQ-C12-04) | PL-07 |
| `VERSION_FROZEN` | 409 | Edit to a policy or shift version that is in force or superseded | G5 |
| `PRECONDITION_FAILED` | 412 | `If-Match` ETag is stale | API_CONVENTIONS §5 |
| `PRECONDITION_REQUIRED` | 428 | `If-Match` or `Idempotency-Key` missing where required | API_CONVENTIONS §5 |
| `IDEMPOTENCY_CONFLICT` | 409 | Same `Idempotency-Key` reused with a different body | API_CONVENTIONS §5 |
| `LIMIT_EXCEEDED` | 409 | A configured limit is reached: permission count or hours (PM-05..09), regularisation limit (RG-03), leave balance (LM-07), max consecutive days (LM-10) | various |
| `OUTSIDE_REQUEST_WINDOW` | 409 | Request too early or too late (LM-09, LM-11, PM-14, RG-04, OD-04) | various |
| `NOT_ELIGIBLE` | 409 | Employee not eligible (LM-01, PM-25, OD-11) | various |
| `OVERLAPPING_REQUEST` | 409 | Request overlaps another active request where the rules forbid it (PM-20, LM-14) | various |
| `EMPLOYEE_LIMIT_REACHED` | 409 | New employee blocked because VC-01 = block | VC-01 |
| `PUNCH_REFUSED` | 422 | Punch blocked; `rule_ref` is the setting that blocked it (PC-01 source off, PC-02 selfie/location required, PC-04, PC-05, PC-06, PC-09, PC-12, PC-13, PC-14, EM-07, MD-02 plan has no app) and is shown to the employee (E18). The refused attempt is recorded (`attempt_id`) | PC-xx |
| `DEVICE_UNKNOWN` | 401 | Device push from an unregistered serial number | TSD §9.1 |
| `DUPLICATE_PUNCH` | 200 (per item) | Offline batch item already stored; not an error for the batch | TSD §9.3 |
| `PENDING_ITEMS_BLOCK_LOCK` | 409 | Lock refused because PL-05 = block and the checklist has items | PL-05 |
| `UNLOCK_NOT_ALLOWED` | 409 | PL-06 = never | PL-06 |
| `CONFLICTING_SETTINGS` | 409 | Saving settings that conflict where the BRD requires a refusal | K4 |
| `SETTINGS_WARNING` | 200 (warning list) | Settings saved, with warnings (e.g. PD/LA overlap, §10.3.4) | K4 |
| `FILE_TYPE_NOT_ALLOWED` | 422 | Upload type or size not allowed | DC-06, TSD §11 |
| `INTERNAL` | 500 | Unexpected failure; nothing partially committed (one transaction per request) | — |
| `UNAVAILABLE` | 503 | Dependency down; safe to retry with the same `Idempotency-Key` | — |
