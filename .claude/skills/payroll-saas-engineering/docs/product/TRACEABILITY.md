# Requirements traceability (V1)

Status: Provisional (BRD v0.7 draft). Generated from the `## Trace` table in each `USER_STORIES/*.md` file; edit those tables, not this file, then regenerate.

Chain so far: BRD requirement or setting → user story → acceptance criteria. Later phases add the columns domain concept → state transition → database constraint → API operation → implementation → automated test.

Coverage check (15 Nov 2026): all 349 V1 requirement and setting IDs in BRD v0.7 (prefixes VC, MD, OR, EM, UM, AC, PC, SH, PR, BK, LA, EC, EA, MP, DS, RD, RG, HD, LM, LV, PM, OD, PL, NT, OB, DB, HW, CO) are referenced by at least one story. Worked examples E1–E19 and the V1 acceptance tests are referenced. Not referenced, because they are V2/V3: AT7, AT9, AT10, AT11, AT15, BR-12, BR-13, BR-15 (added in BRD v0.8). Cross-cutting NFR and TB items are traced in `NON_FUNCTIONAL.md`.

| BRD ID | File | Story | Acceptance criteria |
|---|---|---|---|
| OR-R1 | C01-organisation-setup | US-C01-01 | AC-C01-01.1, 01.2, 01.3, 01.6, 01.7 |
| OR-R2 | C01-organisation-setup | US-C01-02 | AC-C01-02.1, 02.2, 02.3 |
| OR-R3 | C01-organisation-setup | US-C01-03 | AC-C01-03.1 |
| OR-R4 | C01-organisation-setup | US-C01-04 | AC-C01-04.1, 04.2, 04.3 |
| OR-R5 | C01-organisation-setup | US-C01-02, US-C01-03 | AC-C01-02.2, 03.2 |
| OR-01 | C01-organisation-setup | US-C01-05 | AC-C01-05.1 |
| OR-02 | C01-organisation-setup | US-C01-05 | AC-C01-05.1, 05.2 |
| OR-03 | C01-organisation-setup | US-C01-05 | AC-C01-05.1, 05.3 |
| OR-04 | C01-organisation-setup | US-C01-05 | AC-C01-05.1, 05.4 |
| BR-11 | C01-organisation-setup | US-C01-01 | AC-C01-01.3, 01.4 |
| G3 / G7 | C01-organisation-setup | US-C01-02, US-C01-03 | AC-C01-02.3, 03.3 |
| G6 | C01-organisation-setup | US-C01-02, US-C01-05 | AC-C01-02.1, 05.1 |
| G9 / §16 | C01-organisation-setup | all | AC-C01-01.7, 02.5, 04.3, 05.6 |
| AC-04 | C01-organisation-setup | US-C01-05 | AC-C01-05.5 |
| UM-R2 / AT13 | C01-organisation-setup | US-C01-01, US-C01-02 | AC-C01-01.5, 02.4 |
| TB-01 / AT20 | C01-organisation-setup | US-C01-01 | AC-C01-01.8 |
| EM-R1 | C02-employee-master | US-C02-01 | AC-C02-01.1 |
| EM-R2 | C02-employee-master | US-C02-01 | AC-C02-01.1 |
| EM-R3 | C02-employee-master | US-C02-02 | AC-C02-02.1 |
| EM-R4 | C02-employee-master | US-C02-03 | AC-C02-03.1, 03.2 |
| EM-R5 | C02-employee-master | US-C02-04, US-C02-11 | AC-C02-04.1, 11.1 |
| EM-R6 | C02-employee-master | US-C02-05 | AC-C02-05.1–05.5, 06.1, 07.6, 09.4, 11.3 |
| EM-R7 | C02-employee-master | US-C02-09 | AC-C02-09.1–09.6 |
| EM-R8 | C02-employee-master | US-C02-10 | AC-C02-10.1–10.3 |
| EM-R9 | C02-employee-master | US-C02-07, US-C02-08 | AC-C02-07.2, 07.4, 08.1, 08.3 |
| EM-R10 | C02-employee-master | US-C02-08 | AC-C02-08.4 |
| EM-01 | C02-employee-master | US-C02-01, US-C02-11 | AC-C02-01.2, 01.3, 11.2 |
| EM-02 | C02-employee-master | US-C02-01, US-C02-09, US-C02-10 | AC-C02-01.5, 09.2, 10.2 |
| EM-03 | C02-employee-master | US-C02-12 | AC-C02-12.1–12.5 |
| EM-04 | C02-employee-master | US-C02-04 | AC-C02-04.2 |
| EM-05 | C02-employee-master | US-C02-11 | AC-C02-11.1, 11.2 |
| EM-06 | C02-employee-master | US-C02-07 | AC-C02-07.1, 07.2, 07.3 |
| EM-07 | C02-employee-master | US-C02-07 | AC-C02-07.5 (behaviour in C04) |
| EM-08 | C02-employee-master | US-C02-08 | AC-C02-08.1, 08.2, 08.5 |
| EM-09 | C02-employee-master | US-C02-06 | AC-C02-06.1, 06.2 |
| E17 | C02-employee-master | US-C02-08 | AC-C02-08.1 (costing part) |
| AT16 | C02-employee-master | US-C02-08 | AC-C02-08.1, 08.2 (allowed-branch part) |
| BR-05 / G5 | C02-employee-master | US-C02-02, US-C02-05 | AC-C02-02.2, 05.1, 05.2 |
| BR-06 | C02-employee-master | US-C02-05 | AC-C02-05.4 |
| BR-07 | C02-employee-master | US-C02-12 | AC-C02-12.6 |
| UM-R2 / AT13 | C02-employee-master | US-C02-01, 06, 09 | AC-C02-01.6, 06.3, 09.5 |
| TB-01 / AT20 | C02-employee-master | US-C02-01, 09, 10 | AC-C02-01.4, 09.6, 10.3 |
| VC-01 | C02-employee-master | US-C02-01, US-C02-09 | AC-C02-01.8 |
| UM-R1 | C03-user-access | US-C03-01 | AC-C03-01.2, 01.3, 03.7 |
| UM-R2 | C03-user-access | US-C03-03 | AC-C03-03.1–03.4 |
| UM-R3 | C03-user-access | US-C03-04, US-C03-05 | AC-C03-04.1–04.3, 05.1, 05.4 |
| UM-R4 | C03-user-access | US-C03-06 | AC-C03-06.1–06.3 |
| AC-01 | C03-user-access | US-C03-01 | AC-C03-01.1–01.7 |
| AC-02 | C03-user-access | US-C03-07 | AC-C03-07.1–07.3, 07.6 |
| AC-03 | C03-user-access | US-C03-08 | AC-C03-08.1–08.3, 08.5 |
| AC-04 | C03-user-access | US-C03-09 | AC-C03-09.1–09.5 |
| AC-05 | C03-user-access | US-C03-10 | AC-C03-10.1–10.5 |
| AC-06 | C03-user-access | US-C03-02 | AC-C03-02.1–02.3 |
| AC-07 | C03-user-access | US-C03-03 | AC-C03-03.1–03.4 |
| AC-08 | C03-user-access | US-C03-05 | AC-C03-05.1–05.3, 05.5 |
| §5 roles | C03-user-access | US-C03-01, US-C03-11 | AC-C03-01.1, 11.1, 11.2 |
| BR-07 | C03-user-access | US-C03-07, US-C03-08 | AC-C03-07.4, 07.5, 08.4 |
| AT13 | C03-user-access | US-C03-03, US-C03-10 | AC-C03-03.1, 03.2, 03.3, 03.5, 10.5 |
| TB-01 / AT20 | C03-user-access | US-C03-12, US-C03-02 | AC-C03-12.1–12.7, 02.3 |
| TB-02 | C03-user-access | US-C03-12 | AC-C03-12.4 |
| OR-03 / N7 / AT14 | C03-user-access | US-C03-06 | AC-C03-06.1, 06.2 |
| G9 / §16 | C03-user-access | all | AC-C03-01.6, 03.6, 05.5, 07.7, 08.5, 09.5, 11.3 |
| PC-R1 | C04-punch-collection | US-C04-01, US-C04-02 | AC-C04-01.5, AC-C04-02.1 |
| PC-R2 | C04-punch-collection | US-C04-01 | AC-C04-01.1, AC-C04-01.2 |
| PC-R3 | C04-punch-collection | US-C04-03 | AC-C04-03.2 (OQ-C04-04) |
| PC-R4 | C04-punch-collection | US-C04-07 | AC-C04-07.2, AC-C04-07.3 |
| PC-R5 | C04-punch-collection | US-C04-07, US-C04-08 | AC-C04-07.1, AC-C04-08.1, AC-C04-08.2 |
| PC-R6 | C04-punch-collection | US-C04-02, US-C04-06 | AC-C04-02.1, AC-C04-02.4, AC-C04-06.1 |
| PC-R7 | C04-punch-collection | US-C04-08, US-C04-14 | AC-C04-08.4, AC-C04-14.1, AC-C04-14.2 (OQ-C04-13) |
| PC-R8 | C04-punch-collection | US-C04-03 | AC-C04-03.1, AC-C04-03.3, AC-C04-03.5 |
| PC-R9 | C04-punch-collection | US-C04-05 | AC-C04-05.1 to AC-C04-05.7 |
| PC-R10 | C04-punch-collection | US-C04-04 | AC-C04-04.1 to AC-C04-04.5 |
| PC-R11 | C04-punch-collection | US-C04-09 | AC-C04-09.1 |
| PC-R12 | C04-punch-collection | US-C04-09, US-C04-11 | AC-C04-09.11, AC-C04-11.1 to AC-C04-11.5 |
| PC-R13 | C04-punch-collection | US-C04-07, US-C04-09, US-C04-18 | AC-C04-07.1, AC-C04-09.1, AC-C04-18.1 |
| PC-R14 | C04-punch-collection | US-C04-10 | AC-C04-10.1 to AC-C04-10.4 |
| PC-R15 | C04-punch-collection | US-C04-07, US-C04-09 | AC-C04-07.5, AC-C04-09.1 |
| PC-01 | C04-punch-collection | US-C04-17, US-C04-07, US-C04-09 | AC-C04-17.1 to AC-C04-17.3, AC-C04-07.4, AC-C04-09.12 |
| PC-02 | C04-punch-collection | US-C04-07, US-C04-11 | AC-C04-07.2, AC-C04-11.7 |
| PC-03 | C04-punch-collection | US-C04-10 | AC-C04-10.1, AC-C04-10.2 |
| PC-04 | C04-punch-collection | US-C04-09, US-C04-11 | AC-C04-09.2, AC-C04-09.3, AC-C04-11.4 |
| PC-05 | C04-punch-collection | US-C04-09 | AC-C04-09.8 |
| PC-06 | C04-punch-collection | US-C04-13 | AC-C04-13.1 to AC-C04-13.5 |
| PC-07 | C04-punch-collection | US-C04-08, US-C04-14 | AC-C04-08.5, AC-C04-14.3 |
| PC-08 | C04-punch-collection | US-C04-14 | AC-C04-14.4 (OQ-C04-05) |
| PC-09 | C04-punch-collection | US-C04-16 | AC-C04-16.1 to AC-C04-16.4 |
| PC-10 | C04-punch-collection | US-C04-04 | AC-C04-04.1, AC-C04-04.2 |
| PC-11 | C04-punch-collection | US-C04-05 | AC-C04-05.3, AC-C04-05.4 |
| PC-12 | C04-punch-collection | US-C04-09, US-C04-11 | AC-C04-09.1, AC-C04-11.7 |
| PC-13 | C04-punch-collection | US-C04-09 | AC-C04-09.4, AC-C04-09.5 |
| PC-14 | C04-punch-collection | US-C04-09 | AC-C04-09.6, AC-C04-09.7 |
| PC-15 | C04-punch-collection | US-C04-09 | AC-C04-09.9, AC-C04-09.10 |
| PC-16 | C04-punch-collection | US-C04-10, US-C04-11 | AC-C04-11.2, AC-C04-11.3 |
| PC-17 | C04-punch-collection | US-C04-11, US-C04-16 | AC-C04-11.6, AC-C04-16.1 |
| BR-01 | C04-punch-collection | US-C04-06, US-C04-02, US-C04-05, US-C04-15 | AC-C04-06.1, AC-C04-06.2, AC-C04-02.3, AC-C04-05.6, AC-C04-15.2 |
| BR-11 | C04-punch-collection | US-C04-02, US-C04-06, US-C04-08 | AC-C04-02.1, AC-C04-06.4, AC-C04-08.2 |
| N4 | C04-punch-collection | US-C04-08 | AC-C04-08.1 to AC-C04-08.3 |
| TB-04 | C04-punch-collection | US-C04-01 | AC-C04-01.5 |
| E17 | C04-punch-collection | US-C04-12 | AC-C04-12.1, AC-C04-12.2 |
| E18 | C04-punch-collection | US-C04-09, US-C04-15 | AC-C04-09.2, AC-C04-09.3, AC-C04-09.4, AC-C04-15.1 |
| AT16 | C04-punch-collection | US-C04-12 | AC-C04-12.1, AC-C04-12.6 |
| AT19 | C04-punch-collection | US-C04-09 | AC-C04-09.11 |
| EM-07 | C04-punch-collection | US-C04-11, US-C04-12 | AC-C04-11.5, AC-C04-12.2, AC-C04-12.3 |
| EM-08 | C04-punch-collection | US-C04-12 | AC-C04-12.1, AC-C04-12.4 |
| AT20 / TB-01 | C04-punch-collection | US-C04-01, US-C04-06 | AC-C04-01.3, AC-C04-06.6 |
| SH-R1 | C05-shifts-roster | US-C05-01 | AC-C05-01.1 to AC-C05-01.4 (OQ-C05-01, OQ-C05-02) |
| SH-R2 | C05-shifts-roster | US-C05-03, US-C05-04 | AC-C05-03.1, AC-C05-04.1 to AC-C05-04.3 |
| SH-R3 | C05-shifts-roster | US-C05-05 | AC-C05-05.1, AC-C05-05.2 |
| SH-R4 | C05-shifts-roster | US-C05-06 | AC-C05-06.1 to AC-C05-06.4 |
| SH-01 | C05-shifts-roster | US-C05-01 | AC-C05-01.1, AC-C05-01.2, AC-C05-01.4 |
| SH-02 | C05-shifts-roster | US-C05-01 | AC-C05-01.1 |
| SH-03 | C05-shifts-roster | US-C05-01 | AC-C05-01.1, AC-C05-01.5 |
| SH-04 | C05-shifts-roster | US-C05-01 | AC-C05-01.2 |
| SH-05 | C05-shifts-roster | US-C05-01 | AC-C05-01.3 |
| SH-06 | C05-shifts-roster | US-C05-02 | AC-C05-02.1 to AC-C05-02.3 |
| SH-07 | C05-shifts-roster | US-C05-02 | AC-C05-02.4, AC-C05-02.5 (OQ-C05-03 to OQ-C05-05) |
| SH-08 | C05-shifts-roster | US-C05-07 | AC-C05-07.1 to AC-C05-07.3 |
| SH-09 | C05-shifts-roster | C07 US-C07-02 | see C07 |
| SH-10 | C05-shifts-roster | C07 US-C07-01 | see C07 |
| SH-11 | C05-shifts-roster | US-C05-01 | AC-C05-01.1, AC-C05-01.6 |
| SH-12 | C05-shifts-roster | US-C05-08 | AC-C05-08.1 to AC-C05-08.5 |
| BR-03 | C05-shifts-roster | US-C05-03, US-C05-08 | AC-C05-03.3, AC-C05-08.3 |
| BR-06 | C05-shifts-roster | US-C05-03 | AC-C05-03.4 |
| BR-07 | C05-shifts-roster | US-C05-08 | AC-C05-08.4 |
| G5 / G9 | C05-shifts-roster | US-C05-01, US-C05-03 | AC-C05-01.7, AC-C05-03.2 |
| AT13 / AT20 | C05-shifts-roster | US-C05-03, US-C05-06 | AC-C05-03.7, AC-C05-03.8, AC-C05-06.2 |
| §8 Layers 1–5 | C06-attendance-rules | US-C06-02, US-C06-03 | AC-C06-02.7, AC-C06-03.1 |
| §9.6 steps 1–8 | C06-attendance-rules | US-C06-02 | AC-C06-02.1 |
| §9.6 step 2 / SH-07, SH-06 | C06-attendance-rules | US-C06-04 | AC-C06-04.1, AC-C06-04.2 |
| §9.6 step 3 / PR-02..PR-04 | C06-attendance-rules | US-C06-04 | AC-C06-04.3, AC-C06-04.4 |
| PR-01 | C06-attendance-rules | US-C06-05 | AC-C06-05.1..05.4 |
| PR-05 | C06-attendance-rules | US-C06-05 | AC-C06-05.5 |
| PR-06 | C06-attendance-rules | US-C06-05 | AC-C06-05.6 |
| BK-01..BK-09 | C06-attendance-rules | US-C06-06 | AC-C06-06.1..06.9 |
| LA-01..LA-05 | C06-attendance-rules | US-C06-07 | AC-C06-07.2, 07.3, 07.6, 07.11..07.13, 07.17 |
| LA-06..LA-12 | C06-attendance-rules | US-C06-15 | AC-C06-15.1..15.6, 15.11 |
| LA-13, LA-14 | C06-attendance-rules | US-C06-08 | AC-C06-08.1..08.5 |
| LA-15 | C06-attendance-rules | US-C06-07 | AC-C06-07.4, 07.5 |
| LA-16, LA-17 | C06-attendance-rules | US-C06-07 | AC-C06-07.10, 07.15 |
| LA-18, LA-19 | C06-attendance-rules | US-C06-15 | AC-C06-15.9, 15.10 |
| LA-20, LA-21, LA-22, LA-23 | C06-attendance-rules | US-C06-07, US-C06-01 | AC-C06-07.9, 07.14, 07.7, AC-C06-01.7 |
| EC-01..EC-04, EC-12..EC-17 | C06-attendance-rules | US-C06-09 | AC-C06-09.1..09.12 |
| EC-05..EC-11 | C06-attendance-rules | US-C06-15 | AC-C06-15.7, 15.8, 15.9 |
| EA-01, EA-02 | C06-attendance-rules | US-C06-10 | AC-C06-10.1..10.3 |
| MP-01..MP-05 | C06-attendance-rules | US-C06-11 | AC-C06-11.1..11.6 |
| DS-01..DS-07, DS-09, DS-10 | C06-attendance-rules | US-C06-12 | AC-C06-12.1..12.11 |
| RD-01..RD-04, RD note | C06-attendance-rules | US-C06-13 | AC-C06-13.1..13.5 |
| RG-01..RG-11 | C06-attendance-rules | US-C06-14 | AC-C06-14.1..14.14 |
| BR-02 | C06-attendance-rules | US-C06-03 | AC-C06-03.2, 03.6 |
| BR-03 | C06-attendance-rules | US-C06-03 | AC-C06-03.1, AC-C06-11.8, AC-C06-14.8 |
| BR-04 | C06-attendance-rules | US-C06-16 | AC-C06-16.1..16.4 |
| BR-05 | C06-attendance-rules | US-C06-02 | AC-C06-02.2, 02.3 |
| BR-08 | C06-attendance-rules | US-C06-03 | AC-C06-03.3, 03.4, AC-C06-15.11 |
| BR-09 | C06-attendance-rules | US-C06-09, US-C06-11 | AC-C06-09.5, AC-C06-11.6 |
| BR-10 | C06-attendance-rules | US-C06-15 | AC-C06-15.3, 15.13 |
| BR-11 | C06-attendance-rules | US-C06-04, US-C06-13 | AC-C06-04.6, AC-C06-13.3 |
| BR-14 | C06-attendance-rules | US-C06-01 | AC-C06-01.6, AC-C06-07.13, AC-C06-09.12 |
| Order of checks (late; early mirrored) | C06-attendance-rules | US-C06-07, US-C06-09 | AC-C06-07.1, AC-C06-09.3, 09.4, 09.8 |
| E1 | C06-attendance-rules | US-C06-07 | AC-C06-07.2 |
| E2 | C06-attendance-rules | US-C06-07 | AC-C06-07.3, AC-C06-13.4 |
| E3 | C06-attendance-rules | US-C06-07 | AC-C06-07.4 |
| E4 | C06-attendance-rules | US-C06-08 | AC-C06-08.1 |
| E5 | C06-attendance-rules | US-C06-15 | AC-C06-15.1 |
| E6 | C06-attendance-rules | US-C06-07, US-C06-16 | AC-C06-07.6, AC-C06-16.2 |
| E7 | C06-attendance-rules | US-C06-09 | AC-C06-09.1 |
| E8 | C06-attendance-rules | US-C06-12 | AC-C06-12.2 |
| E9 | C06-attendance-rules | US-C06-09 | AC-C06-09.3 |
| E10 | C06-attendance-rules | US-C06-07 | AC-C06-07.8 |
| E11 | C06-attendance-rules | — (C09 permissions) | — |
| E12 | C06-attendance-rules | US-C06-07 | AC-C06-07.9 |
| E13 | C06-attendance-rules | US-C06-11 | AC-C06-11.1, 11.2 |
| E14 | C06-attendance-rules | US-C06-07 | AC-C06-07.10 |
| E15 | C06-attendance-rules | reference only (C12) | AC-C06-03.5 |
| E16 | C06-attendance-rules | US-C06-09 | AC-C06-09.4 |
| OT §10.1 intro, OT-02, OT-03, OT-07 | C06-attendance-rules | US-C06-17 | AC-C06-17.1..17.6 |
| AT1 | C06-attendance-rules | all stories | every AC citing E1–E14, E16 |
| AT3 | C06-attendance-rules | US-C06-01 | AC-C06-01.6 |
| AT4 | C06-attendance-rules | US-C06-02 | AC-C06-02.4 |
| AT5 | C06-attendance-rules | US-C06-03 | AC-C06-03.2 |
| AT6 | C06-attendance-rules | US-C06-16 | AC-C06-16.4 |
| HD-R1 | C07-holidays-weekly-offs | US-C07-01, US-C07-03 | AC-C07-01.1, AC-C07-01.2, AC-C07-03.4 |
| HD-R2 | C07-holidays-weekly-offs | US-C07-02 | AC-C07-02.1 to AC-C07-02.3 |
| HD-R3 | C07-holidays-weekly-offs | US-C07-03 | AC-C07-03.2, AC-C07-03.4, AC-C07-03.5 |
| HD-01 | C07-holidays-weekly-offs | US-C07-03 | AC-C07-03.1 to AC-C07-03.3 |
| HD-02 | C07-holidays-weekly-offs | US-C07-04 | AC-C07-04.1 to AC-C07-04.3 (OQ-C07-05) |
| HD-03 | C07-holidays-weekly-offs | US-C07-05 | AC-C07-05.1 to AC-C07-05.3 (OQ-C07-06) |
| HD-04 | C07-holidays-weekly-offs | US-C07-06 | AC-C07-06.1, AC-C07-06.2, AC-C07-06.4 |
| HD-05 | C07-holidays-weekly-offs | US-C07-06 | AC-C07-06.3, AC-C07-06.4 |
| SH-09 | C07-holidays-weekly-offs | US-C07-02 | AC-C07-02.1 to AC-C07-02.3 |
| SH-10 | C07-holidays-weekly-offs | US-C07-01 | AC-C07-01.2 |
| BR-03 | C07-holidays-weekly-offs | US-C07-01, US-C07-02 | AC-C07-01.4, AC-C07-02.5 |
| BR-06 | C07-holidays-weekly-offs | US-C07-01 | AC-C07-01.5 |
| G5 / G9 | C07-holidays-weekly-offs | US-C07-01, US-C07-02, US-C07-04 | AC-C07-01.6, AC-C07-02.5, AC-C07-04.3 |
| AT13 / AT20 | C07-holidays-weekly-offs | US-C07-01 | AC-C07-01.7 |
| LM-R1 | C08-leave | US-C08-01 | AC-C08-01.1 |
| LM-R2 | C08-leave | US-C08-02 | AC-C08-02.1–02.8 |
| LM-R3 | C08-leave | US-C08-04, US-C08-05 | AC-C08-04.1, AC-C08-05.1–05.4 |
| LM-R4 | C08-leave | US-C08-05, US-C08-06, US-C08-07 | AC-C08-05.1, AC-C08-06.1, 06.2, AC-C08-07.1 |
| LM-R5 | C08-leave | US-C08-08 | AC-C08-08.2 |
| LM-R6 | C08-leave | US-C08-09 | AC-C08-09.1–09.3 |
| LM-R7 | C08-leave | US-C08-03 | AC-C08-03.1–03.3; OQ-C08-05 |
| LM-R8 | C08-leave | US-C08-06 | AC-C08-06.3 |
| LM-R9 | C08-leave | US-C08-13 | AC-C08-13.1 |
| LM-01 | C08-leave | US-C08-01 | AC-C08-01.1, 01.2; OQ-C08-01, 02 |
| LM-02 | C08-leave | US-C08-02 | AC-C08-02.1, 02.3–02.5; OQ-C08-04 |
| LM-03 | C08-leave | US-C08-02 | AC-C08-02.1, 02.2 |
| LM-04 | C08-leave | US-C08-02 | AC-C08-02.6; OQ-C08-03 |
| LM-05 | C08-leave | US-C08-02 | AC-C08-02.7; OQ-C08-03 |
| LM-06 | C08-leave | US-C08-03 | AC-C08-03.1–03.3; OQ-C08-05 |
| LM-07 | C08-leave | US-C08-04 | AC-C08-04.7; OQ-C08-08 |
| LM-08 | C08-leave | US-C08-04 | AC-C08-04.2 |
| LM-09 | C08-leave | US-C08-04 | AC-C08-04.3 |
| LM-10 | C08-leave | US-C08-04 | AC-C08-04.4 |
| LM-11 | C08-leave | US-C08-04, US-C08-12 | AC-C08-04.5 |
| LM-12 | C08-leave | US-C08-04 | AC-C08-04.6 |
| LM-13 | C08-leave | US-C08-05 | AC-C08-05.1, 05.2, 05.6; OQ-C08-09 |
| LM-14 | C08-leave | US-C08-04 | AC-C08-04.8 |
| LM-15 | C08-leave | US-C08-04 | AC-C08-04.9; OQ-C08-07 |
| LM-16 | C08-leave | US-C08-08 | AC-C08-08.1 |
| LM-17 | C08-leave | US-C08-08 | AC-C08-08.2–08.4; OQ-C08-11 |
| LM-18 | C08-leave | US-C08-08 | AC-C08-08.5; OQ-C08-11 |
| LM-19 | C08-leave | US-C08-08 | AC-C08-08.6 |
| LM-20 | C08-leave | US-C08-08 | AC-C08-08.2, 08.7; OQ-C08-11 |
| LM-21 | C08-leave | US-C08-10, US-C08-11 | AC-C08-10.1, 10.6, AC-C08-11.6 |
| LM-22 | C08-leave | US-C08-10 | AC-C08-10.2 |
| LM-23 | C08-leave | US-C08-10, US-C08-12 | AC-C08-10.3, 10.4, AC-C08-12.2 |
| LM-24 | C08-leave | US-C08-11 | AC-C08-11.7 |
| LM-25 | C08-leave | US-C08-11 | AC-C08-11.1–11.4, 11.8; OQ-C08-15 |
| LM-26 | C08-leave | US-C08-11 | AC-C08-11.1, 11.8; OQ-C08-15 |
| LM-27 | C08-leave | US-C08-10, US-C08-11, US-C08-12 | AC-C08-10.3, 10.4, AC-C08-11.1, 11.5, 11.6, AC-C08-12.1, 12.3; OQ-C08-14 |
| LM-28 | C08-leave | US-C08-09 | AC-C08-09.4–09.6; OQ-C08-12 |
| LM-29 | C08-leave | US-C08-13 | AC-C08-13.3 |
| LM-30 | C08-leave | US-C08-13 | AC-C08-13.1, 13.2; OQ-C08-17 |
| LM-31 | C08-leave | US-C08-13 | AC-C08-13.1, 13.4; OQ-C08-13 |
| LM-32 | C08-leave | US-C08-13 | AC-C08-13.5, 13.6; OQ-C08-16 |
| LV-01 | C08-leave | US-C08-14 | AC-C08-14.1–14.3; OQ-C08-18 |
| LV-02 | C08-leave | C09 (OD) | see C09-permissions-on-duty.md |
| LV-03 | C08-leave | US-C08-14 | AC-C08-14.4, 14.5; OQ-C08-18 |
| LV-04 | C08-leave | US-C08-09, US-C08-15 | AC-C08-09.5, AC-C08-15.1; OQ-C08-19 |
| DS-08 | C08-leave | US-C08-08 | AC-C08-08.1–08.9 |
| DS-10 | C08-leave | US-C08-09 | OQ-C08-12 |
| LA-12 | C08-leave | US-C08-15 | AC-C08-15.1; OQ-C08-19 |
| E5 | C08-leave | US-C08-15 | AC-C08-15.1 |
| E19 | C08-leave | US-C08-13 | AC-C08-13.1 |
| P6 (day counts) | C08-leave | US-C08-10, US-C08-12 | AC-C08-10.2, 10.3, AC-C08-12.2 |
| P7 (day counts) | C08-leave | US-C08-11 | AC-C08-11.1 |
| BR-03 | C08-leave | US-C08-05, 07, 08, 11, 12 | AC-C08-05.10, 07.1, 08.8, 11.8, 12.1 |
| BR-04 | C08-leave | US-C08-11 | AC-C08-11.6 |
| BR-06 | C08-leave | US-C08-07, US-C08-12 | AC-C08-07.2, AC-C08-12.4 |
| BR-07 | C08-leave | US-C08-05 | AC-C08-05.5 |
| BR-08 | C08-leave | US-C08-02, 06, 09, 10, 15 | AC-C08-02.8, 06.4, 09.7, 10.5, 15.2 |
| R6 | C08-leave | US-C08-06 | AC-C08-06.5 |
| R7 | C08-leave | US-C08-08, 09, 10 | AC-C08-08.9, 09.7, 10.5 |
| NT-05, NT-06 | C08-leave | US-C08-05 | AC-C08-05.6, 05.9 |
| NT-11 | C08-leave | US-C08-03 | AC-C08-03.4 |
| AT6 | C08-leave | US-C08-11 | AC-C08-11.6 |
| AT18 | C08-leave | US-C08-02 (reference; owned by C15) | AC-C08-02.10 |
| AT20 | C08-leave | US-C08-01 | AC-C08-01.5 |
| AT21 | C08-leave | US-C08-02, US-C08-03 | AC-C08-02.9, 03.5 |
| PM-R1 | C09-permissions-on-duty | US-C09-02, US-C09-04 | AC-C09-02.1, AC-C09-04.1 |
| PM-R2 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.2–02.15; OQ-C09-04 |
| PM-R3 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.1–05.3 |
| PM-R4 | C09-permissions-on-duty | US-C09-04, US-C09-08 | AC-C09-04.1, AC-C09-08.1; OQ-C09-11 |
| PM-R5 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.1 |
| PM-R6 | C09-permissions-on-duty | US-C09-11 | AC-C09-11.1, 11.2 |
| PM-R7 | C09-permissions-on-duty | US-C09-11 | AC-C09-11.3; OQ-C09-13 |
| PM-01 | C09-permissions-on-duty | US-C09-01 | AC-C09-01.1 |
| PM-02 | C09-permissions-on-duty | US-C09-01 | AC-C09-01.2 |
| PM-03 | C09-permissions-on-duty | US-C09-02, US-C09-06 | AC-C09-02.2, AC-C09-06.1 |
| PM-04 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.3; OQ-C09-09 |
| PM-05 | C09-permissions-on-duty | US-C09-02, US-C09-08 | AC-C09-02.4, 02.5, AC-C09-08.1 |
| PM-06 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.4 |
| PM-07 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.5 |
| PM-08 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.4; OQ-C09-01 |
| PM-09 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.6; OQ-C09-03 |
| PM-10 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.7; OQ-C09-07 |
| PM-11 | C09-permissions-on-duty | US-C09-01 | setting only; paid permission has no deduction (see AC-C09-03.2 for unpaid minutes) |
| PM-12 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.1, 05.7 |
| PM-13 | C09-permissions-on-duty | US-C09-01 | scope note (overtime pay V3) |
| PM-14 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.8 |
| PM-15 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.9 |
| PM-16 | C09-permissions-on-duty | US-C09-04 | AC-C09-04.1–04.3 |
| PM-17 | C09-permissions-on-duty | US-C09-04 | AC-C09-04.4; OQ-C09-06 |
| PM-18 | C09-permissions-on-duty | US-C09-03 | AC-C09-03.1–03.5; OQ-C09-05 |
| PM-19 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.2, 05.5, 05.6; OQ-C09-08 |
| PM-20 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.10 |
| PM-21 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.11 |
| PM-22 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.12 |
| PM-23 | C09-permissions-on-duty | US-C09-01 | AC-C09-01.4; OQ-C09-02 |
| PM-24 | C09-permissions-on-duty | US-C09-01 | AC-C09-01.5; OQ-C09-02 |
| PM-25 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.13 |
| PM-26 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.14 |
| PM-27 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.2–05.4; OQ-C09-07 |
| PM-28 | C09-permissions-on-duty | US-C09-02 | AC-C09-02.1, 02.15 |
| PM-29 | C09-permissions-on-duty | US-C09-07 | AC-C09-07.1, 07.2 |
| PM-30 | C09-permissions-on-duty | US-C09-06 | AC-C09-06.1, 06.4, 06.5 |
| OD-01 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.1, 09.7 |
| OD-02 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.2 |
| OD-03 | C09-permissions-on-duty | US-C09-09 | OQ-C09-12 |
| OD-04 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.3 |
| OD-05 | C09-permissions-on-duty | US-C09-10 | AC-C09-10.1–10.3; OQ-C09-06 |
| OD-06 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.1, 09.4 |
| OD-07 | C09-permissions-on-duty | US-C09-11 | AC-C09-11.2, 11.3; OQ-C09-13 |
| OD-08 | C09-permissions-on-duty | US-C09-11 | AC-C09-11.4 |
| OD-09 | C09-permissions-on-duty | — | V3 (daily allowance); dependency noted in Scope |
| OD-10 | C09-permissions-on-duty | US-C09-11 (M02 US-M02-02) | AC-C09-11.5 |
| OD-11 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.5 |
| OD-12 | C09-permissions-on-duty | US-C09-09 | AC-C09-09.6 |
| LA-15, EC-13 | C09-permissions-on-duty | US-C09-06 | AC-C09-06.1–06.6; OQ-C09-09 |
| E3 | C09-permissions-on-duty | US-C09-06 | AC-C09-06.1 |
| E9 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.1 |
| E10 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.2 |
| E11 | C09-permissions-on-duty | US-C09-03 | AC-C09-03.1, 03.2 |
| E16 | C09-permissions-on-duty | US-C09-11 | AC-C09-11.1 |
| BR-03 | C09-permissions-on-duty | US-C09-04, 05, 07, 10, 11 | AC-C09-04.7, 05.8, 07.3, 10.1, 11.6 |
| BR-06 | C09-permissions-on-duty | US-C09-07 | AC-C09-07.3 |
| BR-07 | C09-permissions-on-duty | US-C09-04, US-C09-10 | AC-C09-04.5, AC-C09-10.4 |
| BR-08 | C09-permissions-on-duty | US-C09-05 | AC-C09-05.9 |
| NT-04 | C09-permissions-on-duty | US-C09-08 | AC-C09-08.2 |
| NT-05 | C09-permissions-on-duty | US-C09-04, US-C09-10 | AC-C09-04.4, AC-C09-10.3 |
| R8 | C09-permissions-on-duty | US-C09-08 | AC-C09-08.3 |
| AT18 | C09-permissions-on-duty | US-C09-08 (reference; owned by C15) | AC-C09-08.4 |
| AT20 | C09-permissions-on-duty | US-C09-01 | AC-C09-01.6 |
| PL-R1 | C12-period-lock | US-C12-01 | AC-C12-01.1, 01.2, 01.3 |
| PL-01 | C12-period-lock | US-C12-01 | AC-C12-01.1, 01.2, 01.3; OQ-C12-01, OQ-C12-09 |
| PL-02 | C12-period-lock | US-C12-02 | AC-C12-02.1–02.3; OQ-C12-09 |
| PL-03 | C12-period-lock | US-C12-02 | AC-C12-02.1–02.3; OQ-C12-02 |
| PL-R2 | C12-period-lock | US-C12-03 | AC-C12-03.1–03.5 |
| PL-R3 | C12-period-lock | US-C12-04 | AC-C12-04.1, 04.5 |
| PL-04 | C12-period-lock | US-C12-04 | AC-C12-04.1, 04.7, 04.8; OQ-C12-08 |
| PL-05 | C12-period-lock | US-C12-04 | AC-C12-04.2–04.4; OQ-C12-03 |
| PL-06 | C12-period-lock | US-C12-05 | AC-C12-05.1–05.4; OQ-C12-07 |
| PL-R4 | C12-period-lock | US-C12-06, US-C12-07 | AC-C12-06.1–06.6, AC-C12-07.2 |
| PL-07 | C12-period-lock | US-C12-06 | AC-C12-06.4; OQ-C12-04 |
| BR-03 | C12-period-lock | US-C12-03, US-C12-06 | AC-C12-03.2, AC-C12-06.8 |
| BR-06 | C12-period-lock | US-C12-04, US-C12-06, US-C12-07 | AC-C12-04.5, AC-C12-06.1, AC-C12-07.3 |
| BR-07 | C12-period-lock | US-C12-05 | AC-C12-05.4 |
| BR-08 | C12-period-lock | US-C12-06 | AC-C12-06.6 |
| BR-10 | C12-period-lock | US-C12-03 | AC-C12-03.5; OQ-C12-06 |
| E15 | C12-period-lock | US-C12-06, US-C12-07 | AC-C12-06.1, AC-C12-07.2 |
| AT5 | C12-period-lock | US-C12-06, US-C12-07 | AC-C12-06.5, AC-C12-07.3 |
| AT8 | C12-period-lock | US-C12-04, US-C12-06 | AC-C12-04.5, AC-C12-06.2 |
| AT13 | C12-period-lock | US-C12-03, US-C12-04, US-C12-07 | AC-C12-03.3, AC-C12-04.6, AC-C12-07.4 |
| AT20 / TB-01 | C12-period-lock | US-C12-01, US-C12-04, US-C12-07 | AC-C12-01.4, AC-C12-04.9, AC-C12-07.5 |
| AT21 / TB-03 | C12-period-lock | US-C12-03, US-C12-04 | AC-C12-03.4, AC-C12-04.8 |
| TB-02 | C12-period-lock | US-C12-07 | AC-C12-07.5 |
| MD-06 | C12-period-lock | US-C12-07 | AC-C12-07.1, 07.6; OQ-C12-11 |
| §21 note 1 | C12-period-lock | US-C12-07 | AC-C12-07.1 |
| NT-09 | C12-period-lock | US-C12-03 | AC-C12-03.4 |
| NT-10 | C12-period-lock | US-C12-06 | AC-C12-06.7 |
| VC-R8 | C12-period-lock | US-C12-04 | AC-C12-04.10 |
| §16 audit | C12-period-lock | US-C12-04, US-C12-05 | AC-C12-04.1, 04.7, AC-C12-05.3 |
| §14 common rule | C13-reports | US-C13-01 | AC-C13-01.1–01.6 |
| UM-R1, UM-R2, AC-01, AC-07 | C13-reports | US-C13-01 | AC-C13-01.2, 01.3 |
| MD-03, AT2 | C13-reports | US-C13-01, US-C13-04 | AC-C13-01.6, AC-C13-04.2 |
| AT13 | C13-reports | US-C13-01 | AC-C13-01.2, 01.3 |
| AT20, TB-01, TB-02 | C13-reports | US-C13-01 | AC-C13-01.5, 01.7 |
| VC-R4 | C13-reports | US-C13-01 | AC-C13-01.8 |
| AT5, BR-06 | C13-reports | US-C13-01, US-C13-06 | AC-C13-01.9, AC-C13-06.2 |
| R1 | C13-reports | US-C13-02 | AC-C13-02.1 |
| R2 | C13-reports | US-C13-02 | AC-C13-02.2 |
| R3, E2, E5 | C13-reports | US-C13-02 | AC-C13-02.3–02.5 |
| R4 | C13-reports | US-C13-02 | AC-C13-02.6 |
| R5 | C13-reports | US-C13-02 | AC-C13-02.7 |
| R6 | C13-reports | US-C13-03 | AC-C13-03.1 |
| R7, LM-27, P7 | C13-reports | US-C13-03 | AC-C13-03.2, 03.3 |
| R8 | C13-reports | US-C13-03 | AC-C13-03.4 |
| R9 | C13-reports | US-C13-04 | AC-C13-04.1, 04.2 |
| R10, EM-R10, EM-08, E17 | C13-reports | US-C13-05 | AC-C13-05.1–05.3 |
| R13, MD-06, E15 | C13-reports | US-C13-06 | AC-C13-06.1, 06.2 |
| R25 | C13-reports | US-C13-07 | AC-C13-07.1 |
| R26, G9 | C13-reports | US-C13-07 | AC-C13-07.2 |
| R27, VC-R6 | C13-reports | US-C13-07 | AC-C13-07.3, 07.4 |
| §5 Auditor | C13-reports | US-C13-07 | AC-C13-07.5 |
| §9.14 channels/recipients | C14-notifications | US-C14-01 | AC-C14-01.1–01.3 |
| G9, AC-04 | C14-notifications | US-C14-01 | AC-C14-01.4, 01.5 |
| MD-03, BR-14 | C14-notifications | US-C14-01 | AC-C14-01.6 |
| AT20, TB-01 | C14-notifications | US-C14-01 | AC-C14-01.7 |
| N7, UM-R4, AT14 | C14-notifications | US-C14-02 | AC-C14-02.1, 02.2 |
| NT-01, MP-04, E13 | C14-notifications | US-C14-03 | AC-C14-03.1–03.3 |
| NT-02, E2 | C14-notifications | US-C14-03 | AC-C14-03.4 |
| NT-03 | C14-notifications | US-C14-03 | OQ-C14-03 |
| NT-07, BR-10 | C14-notifications | US-C14-03 | AC-C14-03.5, 03.6 |
| NT-08 | C14-notifications | US-C14-03 | AC-C14-03.7 |
| NT-04, PM-06 | C14-notifications | US-C14-04 | AC-C14-04.2 |
| NT-05 | C14-notifications | US-C14-04 | AC-C14-04.1 |
| NT-06 | C14-notifications | US-C14-04 | AC-C14-04.3–04.6 |
| NT-09 | C14-notifications | US-C14-05 | AC-C14-05.1, 05.2 |
| NT-10, E15 | C14-notifications | US-C14-05 | AC-C14-05.3 |
| NT-11, LM-06 | C14-notifications | US-C14-06 | AC-C14-06.1, 06.2 |
| AT21, TB-03 | C14-notifications | US-C14-03, US-C14-04, US-C14-05 | AC-C14-03.2, 03.6, AC-C14-04.5, AC-C14-05.2 |
| NT-12, NT-13, NT-14 | C14-notifications | US-C14-01 | AC-C14-01.8 (not offered in V1) |
| OB-R1 | C15-opening-balances | US-C15-01 | AC-C15-01.1 |
| OB-R2 | C15-opening-balances | US-C15-01, US-C15-05 | AC-C15-01.1–01.3, AC-C15-05.3–05.5; OQ-C15-01, 02, 06 |
| OB-R3 | C15-opening-balances | US-C15-02 | AC-C15-02.1–02.3; OQ-C15-03, 09 |
| OB-R4 | C15-opening-balances | US-C15-01, US-C15-05 | AC-C15-01.4, AC-C15-05.1; OQ-C15-05 |
| OB-R5 | C15-opening-balances | US-C15-03, US-C15-04 | AC-C15-03.1, 03.5, AC-C15-04.2, 04.3 |
| OB-01 | C15-opening-balances | US-C15-01, US-C15-03 | AC-C15-01.6, AC-C15-03.1–03.3; OQ-C15-07 |
| OB-02 | C15-opening-balances | US-C15-04 | AC-C15-04.1–04.3; OQ-C15-04 |
| AT18 | C15-opening-balances | US-C15-05 | AC-C15-05.1–05.5 |
| AT13 | C15-opening-balances | US-C15-01, US-C15-02 | AC-C15-01.5, AC-C15-02.3 |
| AT20, TB-01, TB-02 | C15-opening-balances | US-C15-01, US-C15-02 | AC-C15-01.7, AC-C15-02.3, 02.4 |
| BR-07 | C15-opening-balances | US-C15-03 | AC-C15-03.3 |
| R25 | C15-opening-balances | US-C15-03 | AC-C15-03.4 |
| MD-03, LM-R8 | C15-opening-balances | US-C15-01 | AC-C15-01.2 |
| G9, §16 | C15-opening-balances | US-C15-03, US-C15-04 | AC-C15-03.1, AC-C15-04.3 |
| AT5 | C15-opening-balances | US-C15-05 | AC-C15-05.6 |
| DB-R1, AT17 | C16-dashboards | US-C16-01 | AC-C16-01.1, 01.2; OQ-C16-01, 02 |
| DB-R2 | C16-dashboards | US-C16-02 | AC-C16-02.1–02.5 |
| DB-R3 | C16-dashboards | US-C16-03 | AC-C16-03.1–03.4 |
| DB-R4 | C16-dashboards | US-C16-04 | AC-C16-04.1–04.3; OQ-C16-03 |
| DB-R5 | C16-dashboards | US-C16-05 | AC-C16-05.1, 05.2; OQ-C16-05 |
| DB-R6 | C16-dashboards | US-C16-06 | AC-C16-06.1, 06.2 |
| DB-R7 | C16-dashboards | US-C16-07 | AC-C16-07.1–07.4 |
| DB-R8 | C16-dashboards | US-C16-03, -04, -05, -06, -07, -08 | AC-C16-03.3, 03.4, 04.2, 04.3, 05.2, 06.1, 07.3, 08.2, 08.4 |
| DB-01 | C16-dashboards | US-C16-08 | AC-C16-08.1–08.3; OQ-C16-06 |
| DB-02 | C16-dashboards | US-C16-08 | AC-C16-08.4 |
| AC-05 | C16-dashboards | US-C16-07 | AC-C16-07.2 |
| MD-03, AT2 | C16-dashboards | US-C16-08 | AC-C16-08.2 |
| AT13 | C16-dashboards | US-C16-01, -03, -04, -05, -07, -08 | AC-C16-01.2, 03.3, 04.3, 05.2, 07.1, 08.4 |
| AT20, TB-01 | C16-dashboards | US-C16-02, US-C16-08 | AC-C16-02.4, AC-C16-08.5 |
| VC-02 | C16-dashboards | US-C16-02 | AC-C16-02.2 |
| G9 | C16-dashboards | US-C16-08 | AC-C16-08.6 |
| CO-R1 | M02-comp-off | US-M02-02, US-M02-03 | AC-M02-02.1, AC-M02-03.1 |
| CO-R2 | M02-comp-off | US-M02-05 | AC-M02-05.1, 05.2; OQ-M02-08, 09 |
| CO-R3 | M02-comp-off | — | V2 dependency; AC-M02-06.6; OQ-M02-01 |
| CO-R4 | M02-comp-off | US-M02-04 | AC-M02-04.1–04.3; OQ-M02-06, 07 |
| HW-01 | M02-comp-off | US-M02-01, US-M02-02, US-M02-04 | AC-M02-01.3, 01.4, AC-M02-02.1, AC-M02-04.1–04.3 |
| HW-02 | M02-comp-off | US-M02-03 | AC-M02-03.1–03.4; OQ-M02-05 |
| HW-03 | M02-comp-off | US-M02-02 | AC-M02-02.5; OQ-M02-02 |
| HW-04 | M02-comp-off | US-M02-02 | AC-M02-02.1–02.3; OQ-M02-03 |
| HW-05 | M02-comp-off | US-M02-06 | AC-M02-06.1, 06.2; OQ-M02-10 |
| HW-06 (lapse) | M02-comp-off | US-M02-06 | AC-M02-06.1 |
| HW-06 (pay one extra day) | M02-comp-off | — | V2 dependency; AC-M02-06.6; OQ-M02-01 |
| P8 | M02-comp-off | — | V2 dependency; OQ-M02-01 |
| OD-10 | M02-comp-off | US-M02-02 | AC-M02-02.4 |
| LM-R8 | M02-comp-off | US-M02-05 | AC-M02-05.1 |
| LM-25 | M02-comp-off | US-M02-05 | AC-M02-05.4 |
| MD-01, MD-03 | M02-comp-off | US-M02-01 | AC-M02-01.1, 01.2 |
| BR-03 | M02-comp-off | US-M02-02 | AC-M02-02.6; OQ-M02-04 |
| BR-07 | M02-comp-off | US-M02-03 | AC-M02-03.5 |
| BR-08 | M02-comp-off | US-M02-02 | AC-M02-02.7 |
| R9 | M02-comp-off | US-M02-05, US-M02-06 | AC-M02-05.7, 06.5 |
| NT-05 | M02-comp-off | US-M02-03 | AC-M02-03.6 |
| NT-11 | M02-comp-off | US-M02-06 | AC-M02-06.3 |
| AT18 | M02-comp-off | US-M02-05 (reference; owned by C15) | AC-M02-05.6 |
| AT20 | M02-comp-off | US-M02-01 | AC-M02-01.6 |
| AT21 | M02-comp-off | US-M02-06 | AC-M02-06.4 |
| VC-R1 | V01-vendor-console | US-V01-01 | AC-V01-01.1, 01.2, 01.3 |
| VC-R2 | V01-vendor-console | US-V01-04 | AC-V01-04.1, 04.2, 03.9, 08.2 |
| VC-R3 | V01-vendor-console | US-V01-06 | AC-V01-06.1, 06.2 |
| VC-R4 | V01-vendor-console | US-V01-07 | AC-V01-07.1–07.5, 07.8 |
| VC-R5 | V01-vendor-console | US-V01-08 | AC-V01-08.1, 08.4 |
| VC-R6 | V01-vendor-console | US-V01-09 | AC-V01-09.1–09.8 |
| VC-R7 | V01-vendor-console | US-V01-10 | AC-V01-10.1–10.4 (template part only; rules library V2) |
| VC-R8 | V01-vendor-console | US-V01-11 | AC-V01-11.1–11.5 |
| VC-R9 | V01-vendor-console | US-V01-12 | AC-V01-12.1–12.3 |
| VC-01 | V01-vendor-console | US-V01-05 | AC-V01-05.1–05.5 |
| VC-02 | V01-vendor-console | US-V01-08 | AC-V01-08.3 |
| VC-03 | V01-vendor-console | US-V01-09 | AC-V01-09.1–09.4 |
| VC-04 | V01-vendor-console | US-V01-07 | AC-V01-07.6, 07.7 |
| VC-05 | V01-vendor-console | US-V01-13 | AC-V01-13.1–13.5 |
| MD-01 | V01-vendor-console | US-V01-03, US-V01-01 | AC-V01-03.1, 03.8, 01.5 |
| MD-02 | V01-vendor-console | US-V01-02 | AC-V01-02.1–02.4 |
| MD-03 | V01-vendor-console | US-V01-03 | AC-V01-03.2, 03.3, 03.4, 14.3 |
| MD-04 | V01-vendor-console | US-V01-03 | AC-V01-03.7 (testable from V2) |
| MD-05 | V01-vendor-console | US-V01-14 | AC-V01-14.1–14.6 |
| G1 | V01-vendor-console | US-V01-03, US-V01-14 | AC-V01-03.8, 14.6 |
| G2 / AT3 | V01-vendor-console | US-V01-14 | AC-V01-14.1, 14.2 |
| G6 / AT12 | V01-vendor-console | US-V01-01, US-V01-10 | AC-V01-01.1, 01.4, 10.1 |
| G9 | V01-vendor-console | all | AC-V01-01.7, 03.9, 04.1, 05.5, 10.2, 13.5, 14.4 |
| BR-14 / AT2 | V01-vendor-console | US-V01-03 | AC-V01-03.2, 03.3 |
| TB-01 / AT20 | V01-vendor-console | US-V01-01, 06, 09, 11, 12, 13 | AC-V01-01.6, 06.3, 06.4, 09.7, 11.5, 12.3, 13.3, 13.4 |
| TB-03 / AT21 | V01-vendor-console | US-V01-08, US-V01-11 | AC-V01-08.3, 11.2 |
