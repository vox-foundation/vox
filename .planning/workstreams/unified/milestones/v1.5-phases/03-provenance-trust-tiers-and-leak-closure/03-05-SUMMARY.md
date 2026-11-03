---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 05
subsystem: db
tags: [vox-db, vox-search, trust-tier, search_documents, kb_index_state, invariant-test, compile-fail]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "ContextTrustTier/tier_for_curated (03-01), search_documents.trust_tier column + backfill (03-03)"
provides:
  - "IndexDeps.trust_tier / LibraryDocUpsert.trust_tier / KbIndexState.trust_tier typed as ContextTrustTier"
  - "apply_library_document writes trust_tier on insert and conflict; put_kb_index_state aligns every corpus row in one transaction"
  - "upsert_search_document(.., tier: ContextTrustTier) with compile_fail/no_run doctest pair; ingest_markdown_tree(.., tier)"
  - "VoxDb::search_document_tiers() typed read accessor"
  - "tiered_writer_invariant::search_corpus_and_inbox_have_only_tiered_writers"
  - "Removed TRUST_TIER_WIRE / validate_trust_tier_wire and re-exports"
affects: [03-09, 03-11, phase-05, phase-11]
tech-stack:
  added: []
  patterns: ["single production tier decision (tier_for_curated(false) in reindex_at) aligned onto rows by the manifest write", "static source-scan invariant with non-vacuity floor"]
key-files:
  created:
    - crates/vox-db/tests/tiered_writer_invariant.rs
  modified:
    - crates/vox-db/src/store/ops_memory/library.rs
    - crates/vox-db/src/store/ops_memory/search.rs
    - crates/vox-db/src/store/ops_memory/mod.rs
    - crates/vox-db/src/store/mod.rs
    - crates/vox-db/src/lib.rs
    - crates/vox-db/src/research.rs
    - crates/vox-db/src/questioning_telemetry.rs
    - crates/vox-search/src/library/mod.rs
    - crates/vox-search/src/kb_eval/library.rs
    - crates/vox-search/src/ingest.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - crates/vox-cli/src/commands/db_research/retrieval.rs
key-decisions:
  - "put_kb_index_state is the single source of the corpus tier: it writes the manifest and UPDATEs search_documents by corpus in one transaction, so rows the indexer skipped as unchanged cannot drift (Phase 11 must exempt evidence-bearing rows)"
  - "Added VoxDb::search_document_tiers instead of allowlisting raw SQL in tests (sql-surface-guard); also usable by 03-09"
  - "mirror_search_corpus tier is a named const MIRROR_TIER so the file carries a real in-file test (tdd-guard)"
requirements-completed: [TRUST-01]
status: complete
commits: 4
plan_head_before: 4e6bede91eb8832a2b692ce2f5a4cae355fe2497
actuals:
  tokens: 4500
  tasks: 3
  commits: 4
duration: 150min
completed: 2026-10-06
---

# Phase 3 Plan 05: Required tier on every corpus writer Summary

Every `search_documents` writer now requires a `ContextTrustTier` at the type boundary, Library rows take their tier from one corpus-level source that a mutation-proven test keeps in agreement, and a static test pins the set of code allowed to write `search_documents` or `kb_findings`.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | One typed corpus tier from reindex to kb_index_state and every Library row | f029edabc |
| 2 | Required tier on `upsert_search_document`, every caller threaded | 8bb10d9bf |
| 3 | Pin the single-writer invariant | 66dfa1778 |
| fix | Typed `search_document_tiers` accessor replaces raw SQL in tests | 257572cb5 |

## Updated sites

`IndexDeps` construction sites: production `crates/vox-orchestrator-mcp/src/kb_tools/library.rs` (`tier_for_curated(false)`, the only production decision; `kb_status` now serializes `as_db_str()`), eval harness `crates/vox-search/src/kb_eval/library.rs` (`tier_for_curated(false)`), `crates/vox-search/src/library/mod.rs` unit test (Trusted; the rejects-bad-tier assertion was deleted, an invalid tier is no longer expressible), `crates/vox-search/tests/kb_library_index.rs` (`deps()` Trusted, new `deps_with(tier)`), `crates/vox-search/tests/kb_library_chunking.rs` x2, `crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs`, and `LibraryDocUpsert`/`KbIndexState` in `crates/vox-db/tests/kb_library_store.rs` and `library.rs` unit tests.

`upsert_search_document` callers: research-ingest mirror (`research.rs`) passes `Untrusted`; questioning publication dual-write (`questioning_telemetry.rs`) passes `Advisory` (D-06); `ingest_markdown_tree` gained a required `tier` and `vox db mirror-search-corpus` (`MIRROR_TIER`) passes `Untrusted`. No `Trusted` literal in any of those files.

## What was verified

- `cargo nextest run -p vox-search --test kb_library_index --test kb_library_chunking`: 33 passed (incl. `kb_library_tier_rows_agree_with_corpus_tier`); `-E 'test(/kb_library_tier|ingest/)'`: 8 passed.
- `cargo nextest run -p vox-db library` 14 passed; `-E 'test(/tiered_writer|library/)'` 15 passed; `-E 'test(/search_document|questioning|research|tiered_writer/)'` 24 passed; `search_document` 4 passed.
- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_status|stdio_daemon_routing/)'`: 2 passed (`kb_status_before_and_after_index` still sees `"trusted"`). `stdio_daemon_routing_tests` is verified by compilation only (`cargo check --tests`); the name filter does not select any test in that binary.
- `cargo test -p vox-db --doc upsert_search_document`: the `no_run` doctest (line 50) and the `compile_fail` doctest (line 44) both ran and passed.
- `cargo check -p vox-db -p vox-search -p vox-orchestrator-mcp -p vox-cli -p vox-research-shim --tests`: clean.
- `VOX_SKIP_FRESHNESS_CHECK=1 cargo run -p vox-cli -- ci crate-edges`: OK (689 edges); no Cargo.toml touched.
- `ci sql-surface-guard --all` exits 1, but only for two pre-existing 03-04 test files (see Deferred Issues); the two offenders this plan introduced were fixed.
- Acceptance greps: `trust_tier: "` prints nothing under crates; `validate_trust_tier_wire|TRUST_TIER_WIRE` prints nothing; `tier_for_curated` appears once in `kb_tools/library.rs`; the pre-`#[cfg(test)]` part of `library/mod.rs` has 0 tier-variant literals.

## Mutation proofs

1. Alignment step. Presence count of `UPDATE search_documents SET trust_tier = ?1 WHERE corpus` in `library.rs`: before 1, mutated 0 (statement turned into a no-op), restored 1. With the mutation `kb_library_tier_rows_agree_with_corpus_tier` fails (`assertion left == right failed: library:docs/src/a.md`, the unchanged-tree Advisory step) and every other `kb_library_index` test still passes. Re-run after rewriting the test to use `search_document_tiers`: fails again under the mutation.
2. Single-writer invariant. `grep -c 'INSERT INTO search_documents'` in `crates/vox-search/src/ingest.rs`: before 0, mutated 1 (a raw string const appended), restored 0. The invariant test failed naming `vox-search/src/ingest.rs`.

Final writer list (non-test sources containing `into search_documents` / `into kb_findings`): `vox-db/src/store/ops_memory/search.rs`, `kb_findings.rs`, `library.rs`. Non-vacuity floor: more than 50 files scanned.

## Deviations from Plan

**1. [Rule 3 - Blocking] Wire-validator re-export removal done in Task 1's commit.** Deleting the validator from `library.rs` (needed by Task 1) broke the three re-export lists at once, so Task 3 step 1 landed with Task 1. Task 3 commit holds only the invariant test.

**2. [Rule 3 - Blocking] `search_document_tiers` accessor added (not in plan).** The agreement and ingest tests initially read tiers with raw `connection().query`, which `sql-surface-guard` rejects outside vox-db. A typed accessor (with a test) replaced it. Commit 257572cb5.

**3. [Rule 3 - Blocking] tdd-guard on two touched files.** `vox-cli/.../retrieval.rs` and `vox-db/src/research.rs` had no in-file tests (pre-existing), which blocked the commit under `enforce-strict`. Added `MIRROR_TIER` plus a one-assert test, and an in-file `research_ingest_mirror_row_is_untrusted` test (replacing an assertion I had first put in `tests/db_research_tests.rs`, which was reverted).

**4. Rule 1 style.** `put_kb_index_state` now takes a clone of the state to move into the transaction closure; no behavior change.

## Known Stubs

None.

## Deferred Issues

- `ci sql-surface-guard --all` still fails for `crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs:42` and `crates/vox-search/tests/web_capture_test.rs:65` (raw count queries from 03-04). Recorded in `deferred-items.md`; fix needs a typed count accessor or an allowlist decision.

## Threat Flags

None. T-03-05, T-03-37 and T-03-21 are mitigated as specified.

## Self-Check: PASSED

- FOUND: crates/vox-db/tests/tiered_writer_invariant.rs
- FOUND commits: f029edabc, 8bb10d9bf, 66dfa1778, 257572cb5
- `git rev-list --count 4e6bede91..HEAD` = 4 at write time
