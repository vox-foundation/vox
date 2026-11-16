---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 08
subsystem: kb
tags: [kb_findings, legacy-voxkb, migration, mens, dpo, trust-tier]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "legacy_kb_counts / legacy_kb_entries_page / LEGACY_VOXKB_DDL (03-03), capture_finding (03-04)"
provides:
  - "kb_tools::findings::{LegacyKbReport, migrate_legacy_kb_entries}"
  - "ServerState::with_db_initialized spawns the migration on every DB attach"
  - "MENS KbSignals reads kb_findings_unqueued_reviewed and emits DPO pairs only (kb_findings_dpo_records)"
affects: [03-12, 03-13, phase-09]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/kb_legacy_migration.rs
  modified:
    - crates/vox-orchestrator-mcp/src/kb_tools/findings.rs
    - crates/vox-orchestrator-mcp/src/server_state.rs
    - crates/vox-ml-cli/src/commands/mens/pipeline.rs
key-decisions:
  - "A capture_finding error aborts the pass (logged as a warning at attach, retried next attach) rather than silently skipping a row"
  - "KbSignals marks only ids that landed in an emitted pair; unpaired reviewed rows stay unqueued so a later counterpart can pair"
requirements-completed: [RET-03, GROW-06]
status: complete
commits: 2
plan_head_before: 33dfff891da3a2e6ced92e49de0065380889a0ca
actuals:
  tokens: 12000
  tasks: 2
  commits: 2
duration: 150min
completed: 2026-10-06
---

# Phase 3 Plan 08: Legacy VoxKB migration and MENS DPO-only signals Summary

Legacy `kb_entries` rows are counted and copied into the inbox as pending Untrusted findings on every DB attach (idempotent, copy-only), and MENS `KbSignals` now reads only human-reviewed findings and emits DPO pairs, never SFT.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | Legacy rows counted, then migrated as pending Untrusted findings on attach | 51ddfcd16 |
| 2 | Retarget MENS KbSignals to reviewed findings, DPO only | 30a280f54 |

## What was built

- `migrate_legacy_kb_entries` counts the three legacy tables first, returns early at zero entries, pages `legacy_kb_entries_page` (500, id cursor), skips whitespace-only rows (`skipped_empty`), and calls `capture_finding` with origin `LegacyKbEntry` and `dedupe_scope = legacy id` (dedupe hit -> `already_present`). Provenance carries legacy_id, kb_name, source_signal, source_ref, routing_confidence, accepted, mens_queued, created_at_ms. The `accepted` flag never raises trust. It only reads legacy tables.
- `with_db_initialized` spawns it (info log when any legacy count is non-zero, warn on error, never fatal).
- `kb_findings_dpo_records` (pure) groups reviewed rows by provenance `query`, emits one `{type: dpo, source: kb_findings, prompt, chosen, rejected}` per approved/rejected pair, and returns only used ids. `run_kb_signals_stage` reads `kb_findings_unqueued_reviewed`, writes `mix_sources/kb_signals.jsonl`, marks used ids via `kb_findings_mark_mens_queued`. All legacy-API reads and the SFT branch are gone from vox-ml-cli.

## Verification

- `cargo nextest run -p vox-orchestrator-mcp legacy_kb_`: 7 passed (5 integration tests in `kb_legacy_migration.rs`, 1 unit, plus a matching existing kb_leak test).
- `cargo nextest run -p vox-ml-cli kb_signals_`: 3 passed.
- `cargo check -p vox-ml-cli -p vox-orchestrator-mcp`: clean.
- Acceptance greps: `migrate_legacy_kb_entries` appears once in server_state.rs; `vox-deprecated-since` present; DELETE/DROP count in the migrate fn = 0; no `kb_unqueued_training_entries|kb_mark_mens_queued` in vox-ml-cli; `"sft"` count in `run_kb_signals_stage` = 0; `kb_findings_unqueued_reviewed` count in pipeline.rs = 1.
- Deferred to hosted CI per the lean-verification policy: `vox ci sql-surface-guard --all` (tests seed only via `execute_batch`; `grep -c "connection()\.\(query\|execute\)("` on the new test file = 0).

## Mutation proofs (`grep -c` presence before / during / after)

1. Whitespace skip (Task 1): `if e.content.trim().is_empty() {` 1 -> 0 (replaced by `if false {`, 0 -> 1) -> 1. `legacy_kb_counts_then_migrates_non_empty_rows_as_pending_untrusted` went red (`capture_finding: text and full_text are both empty` unwrap, so the `(1, 1)` report assertion was never reached); restored, green. A first attempt with BSD `sed -i` silently did not apply (during count stayed 1, test stayed green); it was redone with Edit and is the run recorded above.
2. reviewed_by skip (Task 2): `filter(|r| r.reviewed_by.is_some())` 1 -> 0 -> 1. `kb_signals_unreviewed_rows_emit_nothing` went red (assert at pipeline.rs:780); restored, 3/3 green.

## Deviations from Plan

**1. [Process] No separate RED commit.** Tests and implementation were written in the same working session and committed together per task, because each build on this host takes 3 to 9 minutes. The mutation proofs supply the red/green evidence.

No other deviations.

## Known Stubs

None.

## Threat Flags

None. The migration adds no endpoint or file surface beyond the existing `capture_finding` snapshot writes under `.vox/cache/kb-findings/`.

## Notes

- Per D24 the MENS stage still reads the repo DB while GUI/MCP findings land in the user DB; no reviewed rows exist until Phase 9, so the stage emits nothing until then.
- Already-exported SFT data from `mens_queued=1` legacy rows cannot be recalled (03-13 records this in the CHANGELOG).

## Self-Check: PASSED

Files exist (kb_legacy_migration.rs, findings.rs, server_state.rs, pipeline.rs); commits 51ddfcd16 and 30a280f54 are on the branch.
