# Pending decisions: everything still unanswered

Status as of 2026-11-16, end of Phase 4 (API contracts). Full detail for every ID is in `OPEN_QUESTIONS.md`.
Each line says **what it blocks**. Gates 1–3 were approved with all of these still open. The design keeps a neutral slot for each one, but **Phase 5 code cannot be written for the parts marked Tier 1 without an answer**: the skill forbids guessing business rules in implementation.

## Tier 1: answer before Phase 5 (backend) starts

| # | ID | Question | Blocks |
|---|---|---|---|
| 1 | Q14 | Who signs off BRD v0.8 and TSD v0.1? Both are still "Draft, not yet approved". | Formal baseline for everything built |
| 2 | DP-01 / G-A | How is a V1 attendance period defined without payroll groups: per organisation, per branch, or per policy set? (and what "payroll period" means in V1 settings) | Period generation, lock, counters, exports |
| 3 | DP-03 | When an employee matches policy assignments at two of branch, department, category and shift, which wins? | PolicyResolver, i.e. every day calculation |
| 4 | DP-04 / G-J | Punching at an allowed branch in another time zone: whose time zone and holiday calendar apply? | Attendance date, late/early, holidays |
| 5 | DP-05 / G-16 | Which punch flags count before approval (E18 says geo-fence does not), and who decides each flag? | Day calculation, approval routing |
| 6 | DP-02 / G-B | Leave debit order for penalties and absences: LA-12/EC-11 or LV-04, and what if "casual leave" does not exist? | Penalty recovery, ledger postings |
| 7 | DP-06 / G-C | Absent day with no leave: DS-10 or LM-28 governs? | Day status, deduction days |
| 8 | G-K | Which "cut-off" applies to unresolved missed punches (MP-03) and pending requests at lock? | Missed-punch resolution, lock |
| 9 | DP-11 / G-N | Is overtime identified (and exported) when the Overtime module is off? | Payroll input export |
| 10 | G-D / OQ-C12-05 | Does the Payroll Officer role exist in V1 organisations without Payroll; if not, who locks periods? | Lock permissions, seed roles |
| 11 | Q15 / D1 / TB-04 | Can the eSSL devices in use push over HTTPS, and how often do they contact the server? | Device gateway adapter |
| 12 | Q16 / D2 / D3 | Sign-in codes: Indian SMS provider or Firebase; which SMS and email providers? | Employee sign-in |
| 13 | Q5 | Which modules and limits go in each plan; which plans include the app? | Vendor console seed data |
| 14 | D4 | Is a Mumbai region available on the chosen Supabase plan? (TB-07 data in India) | Environment set-up |
| 15 | Supabase checks | Can the migration role create the `BYPASSRLS` role `app_resolver`, change function owners, and create `btree_gist`? (MIGRATIONS.md §3) | First migration on test |
| 16 | OQ-V01-17 | How do vendor users sign in, and is a second step mandatory for them? (no contract possible until decided) | Vendor console sign-in |
| 17 | GAP-SM-07 | What does "finalised" mean for a day, and when is it reached? (BRD §2 success measure; TSD J1 nightly finalisation) | Day lifecycle, nightly job J1 |

## Tier 2: can be answered during Phase 5 (a default is not assumed; the item stays neutral)

- **V1 behaviour questions:** 197 module questions in `OPEN_QUESTIONS.md` §3, grouped into duplicate sets G-E, G-F, G-G, G-H, G-I, G-L, G-M (one answer closes each set). The most visible: Q9 / G-L (absconding counts working or calendar days), G-E (work-from-home request flow), G-F (escalation timers and non-working days), G-H (bad rows in spreadsheet uploads).
- **Permissions:** G-01 … G-40 (72 "?" cells in PERMISSION_MATRIX). They are seeded as **not granted** until you decide each one.
- **Lifecycles:** GAP-SM-01 … GAP-SM-17 (STATE_MACHINES §15).
- **Other model decision points:** DP-07 (enrolment ID uniqueness: currently per device), DP-08 (does an HR direct correction need a second person?), DP-09 (work-from-home requests), DP-10 (organisation settings dating).
- **Targets:** Q11 / OQ-X-07 (N2, N3 "within seconds" needs a number, N8 99.5%). They are needed before load testing.
- **Retention:** OQ-X-08 / G-M (punch location, selfies (D6: 90 days proposed), documents, audit log).
- **Infrastructure:** D5 (Firebase App Hosting in Mumbai or plain Cloud Run), D7 (build pipeline tool), D8 (point-in-time recovery at launch or later).

## Tier 3: later releases (no V1 impact)

Q1 daily allowance logic (V3) · Q2 leave encashment (V2) · Q3 bank list formats (V2) · Q4 cash payout acknowledgement (V3) · Q6 reimbursements in V2 · Q7 Tally format (V2) · Q8 first client-specific rules (V2) · Q10 extra shift detection (V2) · Q12 overtime pay in V2 or V3 · Q13 employee performance (V3) · **Q17** rounding mode half up vs half even (V2) · **Q18** bank/cash split paise (V2).

## Housekeeping I offered and you have not answered

| # | Item | Why it matters |
|---|---|---|
| H1 | ~~Run `git init`~~ | **Done by you** (commit "Upto phase 3"). |
| H2 | ~~Move `docs/`~~ | **Done 2026-11-16 at your request:** now `.claude/skills/payroll-saas-engineering/docs/` (beside SKILL.md). |
| H3 | Update TSD v0.1: its header says it accompanies BRD v0.7, and §5.3 says rounding happens only where a BRD *setting* says so (BR-15 is now a rule) | The TSD is out of step with BRD v0.8 |
| H4 | Delete BRD v0.7 | Kept only because approved artefacts cite it |

## New from Phase 4 (API contracts)

**Approvals needed with Gate 4** (things Phase 4 added to already-approved work):
- **Edit to approved migration 0005 (Gate 3, committed in "Upto phase 3"):** the regularisation `decided_basis` CHECK changed from `manual` to `approver`, the same vocabulary as SM-REQ and the leave, permission and on-duty tables. The shared contract `common.yaml` changed the same way. Re-validated in the full up/down/up run.
- **Small fixes to approved documents:** event name `OpeningBalancesConfirmed` → `OpeningBalanceConfirmed` (STATE_MACHINES, matching SERVICE_BOUNDARIES); 16 events the contracts use added to the SERVICE_BOUNDARIES §4 event list; `policy.yaml` rule-group edits for HW now also require CMP-01.
- **Migration 0006 (Gate 3 amendment):** 4 tables and 4 columns the contracts need. Validated on a throwaway database.
- **Two new lifecycles (Gate 2 amendment):** SM-CONTACT (employee contact-detail changes, EM-03) and SM-SETCHG (settings changes awaiting Organisation Admin approval, AC-04). STATE_MACHINES §16.
- **Permission matrix lacks "view" rows** for many reads. The contract writers mapped reads to the nearest code (e.g. PLT-10 for vendor reads, IAM-01 for user lists, PER-08 for period lists). Confirm, or add view rows (relates to G-14, G-33).

**New open questions from the contracts:**
- OQ-M02-02 / CMP-02, CMP-03: prior approval to work on a holiday or weekly off (HW-03) has no request type yet. Exempted, not designed.
- DP-10 overlap: period settings (PL-03..07) have their own endpoint, separate from organisation settings versions. Decide whether period settings are versioned with organisation settings.
- A settings schema (the JSON shape of every rule group, TSD §5.4) is needed before Phase 5. This is design work, not a decision for you, but nothing exists yet.
- Phase 4 needed no new error codes beyond `MODULE_DEPENDENCY_UNMET` (added).
