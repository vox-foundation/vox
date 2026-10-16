---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 12
subsystem: db
tags: [ret-02, ret-03, legacy-voxkb, quarantine, baseline-96, absence-test, d-02, d-03]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "legacy tools and handlers retired (03-10); legacy rows counted and copied, MENS off the legacy CRUD (03-08); legacy DDL split into its own constant (03-03)"
provides:
  - "vox-orchestrator::knowledge_base (KbStore LIKE search, KbRouter, legacy types) and vox-db ops_memory/kb.rs (15 CRUD fns, KbRow/KbEntryRow/KbRuleRow) deleted"
  - "legacy_voxkb_surface_absent extended to the store, router, types, row types and 15 CRUD fn names, mutation-proven"
  - "SCHEMA_LEGACY_VOXKB now lives only in domains/quarantine.rs; fresh default-feature databases no longer create knowledge_bases, kb_entries, kb_routing_rules"
  - "vox_db::LEGACY_VOXKB_DDL re-export kept (test seeding on default builds); baseline digest re-updated, BASELINE_VERSION stays 96"
affects: [03-13, phase-12-inbox-gui]
requirements-completed: [RET-02, RET-03]
tech-stack:
  added: []
  patterns: ["quarantine a retired table by moving its DDL behind the existing feature, never into QUARANTINE_DROP_TABLES", "static absence test with whole-identifier matching that covers CamelCase type names"]
key-files:
  modified:
    - crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs
    - crates/vox-orchestrator/src/lib.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/schema/domains/knowledge.rs
    - crates/vox-db/src/schema/domains/quarantine.rs
    - crates/vox-db/src/schema/domains/mod.rs
    - crates/vox-db/src/schema/manifest.rs
    - contracts/db/baseline-version-policy.yaml
    - .planning/workstreams/kb/phases/03-provenance-trust-tiers-and-leak-closure/deferred-items.md
  deleted:
    - crates/vox-orchestrator/src/knowledge_base/mod.rs
    - crates/vox-orchestrator/src/knowledge_base/store.rs
    - crates/vox-orchestrator/src/knowledge_base/router.rs
    - crates/vox-orchestrator/src/knowledge_base/types.rs
    - crates/vox-db/src/store/ops_memory/kb.rs
key-decisions:
  - "D-03: the three legacy tables are not added to QUARANTINE_DROP_TABLES (precheck_quarantine_tables_empty would abort forever on any database holding rows); only fresh databases stop creating them"
  - "The quarantine module is now always compiled and only baseline_sql() gates what reaches the schema, because the LEGACY_VOXKB_DDL re-export must exist on default builds"
  - "Digest-only re-update: the integer stays 96 because existing databases need no change"
duration: ~3 h wall (build host: rustc stalls, see Issues)
completed: 2026-10-08
status: complete
commits: 2
plan_head_before: de226df89873486e3b4a0281109e73c8674f7f81
actuals:
  tokens: 15100
  tasks: 2
  commits: 2
---

# Phase 3 Plan 12: Delete the legacy VoxKB store and quarantine its DDL Summary

The LIKE-search `KbStore`, the `KbRouter`, the legacy KB types and the whole vox-db legacy CRUD surface are gone from source and a static test keeps them gone; the three legacy tables moved into the quarantine DDL (single copy, still seedable through `vox_db::LEGACY_VOXKB_DDL`), so fresh databases stop creating them while existing databases keep their rows for the 03-08 migration.

## Commits

- `f1454c95f` refactor(03-12): delete the legacy VoxKB store, router and vox-db CRUD (Task 1, tracer)
- `79058f5b9` feat(03-12): quarantine the VoxKB DDL out of the fresh-database baseline (Task 2)

## What was built

- **Task 1 (tracer).** `legacy_voxkb_retired.rs` was extended first: `BANNED` gained `KbStore`, `KbRouter`, `KnowledgeBase`, `KbEntry`, `KbEntrySource`, `KbRoutingRule`, `KbRoutingRuleType`, `KbRow`, `KbEntryRow`, `KbRuleRow` and the 15 CRUD fn names (`kb_create` ... `kb_mark_mens_queued`). `is_ident` now includes uppercase so CamelCase names match as whole identifiers (`KbStoreFactory` and `LegacyKbEntry` never match `KbStore` / `KbEntry`; pinned in `legacy_voxkb_matcher_respects_identifier_boundaries`). The scan still covers only `crates/**/*.rs` and six contract files, so `contracts/kb/` (the Phase 1 eval corpus, which mentions these names) is not scanned. Then the `knowledge_base` module directory and its two lines in vox-orchestrator `lib.rs` were deleted, together with vox-db `ops_memory/kb.rs`, its `mod kb;` line and the three row types in vox-db `lib.rs`. `kb_legacy.rs` (the deprecation-marked readers from 03-03) stays, so `legacy_kb_counts` and `legacy_kb_entries_page` keep working and return zeros when the tables are absent.
- **Task 2.** `SCHEMA_LEGACY_VOXKB` moved verbatim (plus per-table comments in the quarantine file's format) from `knowledge.rs` to `quarantine.rs`; the `legacy_voxkb` fragment left `SCHEMA_FRAGMENTS`; `baseline_sql()` appends the constant inside the existing `#[cfg(feature = "quarantine")]` branch; `vox_db::LEGACY_VOXKB_DDL` re-exports the new path. Two gating tests sit next to the existing ones in `manifest.rs`. The digest was recomputed with `print_baseline_digest` (`0x9c76d9f9...2839`), the integer stays 96, and a ledger line was added in both `manifest.rs` and `baseline-version-policy.yaml`.

## Evidence

**Red before delete (Task 1).** With the extended `BANNED` and the unmodified tree, `legacy_voxkb_surface_absent` FAILED listing `vox-orchestrator/src/lib.rs` (`KnowledgeBase`), the four `knowledge_base/*.rs` files (`KbStore`, `KbRoutingRule`, `KnowledgeBase`), `vox-db/src/lib.rs` and `ops_memory/kb.rs` (`KbRow`) (3 nextest tries); the boundary test passed.

**Red before move (Task 2).** `quarantined_legacy_voxkb_absent_by_default` FAILED against the unmodified manifest (`knowledge_bases DDL must not appear in the default baseline_sql()`).

**Mutation proofs (`grep -c`, before / mutated / restored; edits applied with the Edit tool).**

| Guard | Mutation | Counts | Result |
|-------|----------|--------|--------|
| T-03-35 resurrected store | added `pub struct KbStore;` to vox-orchestrator `lib.rs` | `KbStore` in lib.rs: 0 / 1 / 0 | `legacy_voxkb_surface_absent` FAILED (`lib.rs mentions retired KbStore`); restored, `git diff` of lib.rs is the plain two-line deletion |
| T-03-36 DDL quarantine | registered a `legacy_voxkb` fragment (pointing at the quarantine constant) in the default `SCHEMA_FRAGMENTS` as well | `name: "legacy_voxkb"` fragments: 0 / 1 / 0 | `quarantined_legacy_voxkb_absent_by_default` FAILED; restored, 5/5 green afterwards |

**Acceptance greps.** `rg "KbStore|KbRouter" crates --type rust` prints only the test file; `knowledge_base/` and `ops_memory/kb.rs` absent; `CREATE TABLE IF NOT EXISTS kb_entries` appears once in all of `crates` (in `quarantine.rs`) and not in `knowledge.rs`; `kb_entries|knowledge_bases|kb_routing_rules` do not appear in `migration.rs`; `repository_baseline_integer: 96` once.

**Verification run locally (all with `CARGO_INCREMENTAL=0`).**
- `cargo check -p vox-orchestrator -p vox-orchestrator-mcp -p vox-db -p vox-ml-cli -p vox-cli`: exit 0, no warnings (after Task 1).
- `cargo test -p vox-orchestrator-mcp --test legacy_voxkb_retired`: 2/2 green; `--test kb_legacy_migration` (the 03-08 `legacy_kb_*` tests): 6/6 green, before and after Task 2.
- `cargo test -p vox-db --lib -- quarantine baseline_policy_matches_compiled_schema`: 5/5 green; `--test kb_findings_store`: 8/8 green; `--features quarantine --lib` `quarantined_legacy_voxkb_present_with_feature` and `quarantined_table_present_when_feature_on`: 2/2 green.
- `vox ci crate-edges` (existing stale `target/debug/vox`, `VOX_SKIP_FRESHNESS_CHECK=1`): OK, 689 edges within baseline (the pre-existing "stale baseline edge vox-gui -> vox-db-types" warning is unrelated).
- `--features legacy-import legacy_export_covers_all_baseline_tables`: still FAILS exactly as before this plan (pre-existing, `deferred-items.md` 03-03): the only differences are the five names recorded there; `knowledge_bases`, `kb_entries` and `kb_routing_rules` are present on both sides, so this plan did not widen the gap.

## Verify lines deferred to CI

Per the lean-verification policy these were not run locally: the full `vox-db` and `vox-orchestrator-mcp` test suites; workspace `cargo clippy`; `vox ci check-codex-ssot` as a CLI (the `vox` binary build stalled, see Issues; its logic is text checks plus the same digest compare that `baseline_policy_matches_compiled_schema` runs, and the three `MANIFEST_SNIPPETS` substrings are still present in `manifest.rs`); `cargo test -p vox-ml-cli -p vox-cli` (only `cargo check` was run; no caller of a deleted symbol exists, confirmed by `git grep`); `vox ci sql-surface-guard --all`.

## Deviations from Plan

**1. [Rule 3 - Blocking] `domains::quarantine` was not always compiled.** The plan's context says the module "itself always compiles", but at HEAD `domains/mod.rs` declared it `#[cfg(feature = "quarantine")]`. A re-export of a constant that lives there cannot exist on default builds, which would have broken the `LEGACY_VOXKB_DDL` test seeding the plan requires. Fix: the `cfg` was removed from the `pub mod quarantine;` declaration (with a comment); `baseline_sql()` still appends the DDL only under the feature, so the default schema is unchanged apart from losing the legacy fragment. **Commit:** 79058f5b9.

**2. [Rule 2 - Test hardening] Wider ban list and uppercase identifiers.** The plan named the store, router, row types and CRUD fns; the legacy orchestrator types (`KnowledgeBase`, `KbEntry`, `KbEntrySource`, `KbRoutingRule`, `KbRoutingRuleType`) went onto `BANNED` too, and `is_ident` was widened to `[A-Za-z0-9_]` so `KbStoreFactory` / `LegacyKbEntry` cannot false-positive. Covered by the boundary test. **Commit:** f1454c95f.

**Total deviations:** 2. **Impact:** none on scope; (1) makes the plan's own re-export requirement implementable.

## Issues Encountered

Local builds on this host repeatedly stalled: rustc sat at ~0% CPU for 25+ minutes on `vox_corpus` and then deterministically on `vox_db` (same dep-graph file size every time) under the default incremental profile. Each hung rustc was killed (only my own) and its `target/debug/incremental/<crate>-*` directories deleted. `CARGO_INCREMENTAL=0` with `LEAN_CTX_DISABLED=1` made the identical `cargo check -p vox-db` finish in 1 m 12 s, and every later build used it. A full `cargo build -p vox-cli --bin vox` still stalled in `vox_cli` codegen after 22 minutes, so no fresh `vox` binary exists for the `vox ci` checks above. Recorded in `deferred-items.md`.

## Known Stubs

None.

## Threat Flags

None. T-03-35 (resurrected legacy store) and T-03-36 (legacy rows on upgraded databases) are mitigated as planned: the absence test is mutation-proven, and the DDL is quarantined rather than dropped, with the 03-08 migration tests still green.

## Self-Check: PASSED

- `knowledge_base/` and `ops_memory/kb.rs` absent; `quarantine.rs` carries `SCHEMA_LEGACY_VOXKB`; commits `f1454c95f` and `79058f5b9` found in `git log`; `git rev-list --count de226df89..HEAD` = 2 at the time of writing (code commits only).
