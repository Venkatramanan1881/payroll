# User story file template (Phase 0)

Every file in `USER_STORIES/` follows this template so stories, acceptance criteria and traces stay consistent.

Source of truth: `docs/requirements/Payroll-App-BRD-v0.8.txt` (BRD, draft; v0.8 changed only pay rounding, so story files written against v0.7 remain valid) and `docs/requirements/Payroll-App-TSD-v0.1.txt` (TSD, draft).
All content is **Provisional** until Approval Gate 1, because both source documents are still drafts.

## Rules

1. Phase 0 only. No database tables, API routes, code, or UI layouts.
2. Never invent a business rule. If the BRD is silent or ambiguous, raise an open question (`OQ-…`) instead of writing an acceptance criterion that assumes an answer.
3. Every story and every acceptance criterion cites the BRD IDs it comes from (for example `PC-R7`, `LA-05`, `BR-04`, `E12`, `AT16`).
4. Settings are referenced by ID with their BRD default. Do not copy a setting's full option list; the BRD is the source.
5. Use the BRD's own terms (punch, session, attendance date, late mark, deduction days, policy set …).
6. Version 1 scope only. If a V1 requirement mentions a V2/V3 item, note the dependency and do not write stories for the later item.
7. Write acceptance criteria as Given / When / Then, observable and testable. Cover the happy path, boundaries, negative or forbidden cases, permission (who may and may not), tenant isolation where relevant, audit where relevant, and recalculation where relevant.
8. Where a BRD worked example (E1–E19) applies, include it as an acceptance criterion with exactly the BRD's numbers.

## File layout

```markdown
# <Module code> <Module name> — User stories (V1)

Status: Provisional (BRD v0.7 draft)
Source sections: BRD §x.y; TSD §z (if relevant)

## Scope
- In scope (V1): …
- Out of scope / later release: … (with BRD reference)

## Stories

### US-<MOD>-01 <short title>
As a <role from BRD §5>, I want <capability>, so that <outcome>.
Source: <BRD IDs>
Settings involved: <IDs with defaults>

Acceptance criteria
- AC-<MOD>-01.1 Given … When … Then …  (Source: …)
- AC-<MOD>-01.2 …

Edge and negative cases
- … (each case either has an AC above or an OQ below)

## Open questions
- OQ-<MOD>-01 <question>. Source: <IDs>. Why it matters: <one line>.

## Trace
| BRD ID | Story | Acceptance criteria |
|---|---|---|
```

`<MOD>` is the BRD module code: V01, MD, C01 … C16, M02. Use `TB` for the BRD §22.2 technical requirements and `NFR` for BRD §15.
