---
phase: 07-trial-identity-contracts-observational-baseline
plan: 04
subsystem: storage-contract
tags: [vox-db, tier-a, store-01, human-gate, schema-contract]
requires: []
provides:
  - "docs/src/architecture/research-trial-tier-a-schema-2026.md: approved Tier A contract (trials domain, baseline 96, four tables, loud stamped-without-tables guard)"
affects: [07-06, 07-07, 07-09, 12-09]
status: complete
actuals:
  tasks: 3
  commits: 2
plan_head_before: 4b8e840c298dfdeed520073222682bf0b446bfbe
key-files:
  created:
    - docs/src/architecture/research-trial-tier-a-schema-2026.md
key-decisions:
  - "STORE-01 gate (D-04) approved by the user on 2026-10-12: item-batch run grain, model_benchmark_results option A (separate writer with a documented boundary)"
requirements-completed: []
---

# Phase 7 Plan 04: STORE-01 Tier A contract and human gate. Summary

The canonical trial schema contract is written, its DDL checks out in sqlite, and the user approved it at the STORE-01 human gate. The migration in 07-06 is now unblocked.

## Accomplishments

- **Task 1 (409a3a5d0):** contract doc for the `trials` domain at baseline 96.
  - Defines `trial_campaigns`, `trial_campaigns_log`, `trial_runs` and `trial_samples`.
  - No CHECK constraints, no triggers, no content columns. Money is stored as integer micros, and NULL means Missing.
  - Includes Phase 12's `provenance_json` column plus indexed scalars.
  - Adds a `BaselineStampedWithoutTables` guard.
  - The DDL fence ran on `sqlite3 :memory: -bail`, and ran cleanly twice in a row, so it is idempotent.
  - The docs lint passed.
- **Task 2 (checkpoint:decision):** presented the decision to the user. The user answered `approve-recommended` directly in chat.
- **Task 3:** the Decision record in section 12 now reads `approved` / `item-batch` / `option-a` / `user, 2026-10-12`. Verification:
  - the `Decision status: approved` grep returns 1;
  - the `: pending$` grep returns 0;
  - there is exactly one Run grain line and one model_benchmark_results line.

## Deviations

- **Task 3 was recorded by the orchestrator session, not the executor.** The executor declined to attest an approval that reached it only as a relayed message. It was right to refuse a relayed claim. The orchestrator received the user's answer first-hand through its own question to the user, so it wrote the record. The Approved-by line says this.
- **STORE-01 is not marked complete.** 07-06 and 07-07 deliver the migration and the persistence.

## Next

07-06 (migration at baseline 96) can now run. Its precondition greps for the three approved lines. Apply the 12-09 amendments in `12-09-AMENDMENTS.md`; items 6 and 7 are confirmed by this choice.
