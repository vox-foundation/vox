---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 16
subsystem: retrieval
tags: [trust-01, d-08, wr-09, library-index, git-gate, gap-closure, docs]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "Library indexer and kb_index_state (03-05), Tavily gate and source sanitization (03-14), Untrusted agent graph rows (03-15)"
provides:
  - "IndexDeps::is_committed_clean: a Library file is stamped the corpus ceiling only when git tracks it at HEAD with no staged or working-tree change, otherwise Untrusted"
  - "put_kb_index_state writes only the manifest (no corpus-wide re-stamp); kb_index_state.trust_tier is the corpus ceiling"
  - "VoxDb::set_library_document_tier; tier re-evaluated for unchanged (hash-skip) and unreadable docs on every reindex"
  - "committed_clean_docs (GitExec ls-tree HEAD minus diff HEAD, fail-closed); IndexReport::docs_untrusted and the docs_untrusted key in vox_kb_reindex"
  - "Retrieval SSOT and CHANGELOG describe all four Phase 3 gap closures"
affects: [phase-07-injection, phase-11-evidence-rows]
requirements-completed: [TRUST-01]
tech-stack:
  added: []
  patterns: ["decide a derived trust tier per item on every run, including on the unchanged-skip path", "fail-closed git probe through the central GitExec helper"]
key-files:
  created: []
  modified:
    - crates/vox-search/src/library/mod.rs
    - crates/vox-db/src/store/ops_memory/library.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
    - crates/vox-search/src/kb_eval/library.rs
    - crates/vox-search/tests/kb_library_chunking.rs
    - crates/vox-search/tests/kb_library_index.rs
    - docs/src/architecture/search-retrieval-ssot-2026.md
    - CHANGELOG.md
key-decisions:
  - "D-08 read against HEAD: in git ls-tree HEAD and absent from git diff HEAD; a staged-but-uncommitted new file is Untrusted (stricter, fail-closed)"
  - "Reverses the 03-05 'rows always agree with kb_index_state' invariant; replaced by 'every walked file has its tier re-evaluated every run'"
  - "Subdirectory checkouts match nothing, so every file is Untrusted (fail-closed), documented on committed_clean_docs"
duration: ~1.5 h wall, mostly cold cargo check and waiting on shared build
completed: 2026-10-09
status: complete
commits: 2
plan_head_before: 28f2c981fff866698df52b8c7d10d8fc00ccbe66
actuals:
  tokens: 9000
  tasks: 3
  commits: 2
---

# Phase 3 Plan 16: Git-gated Library tier and SSOT/CHANGELOG update Summary

A `docs/src` Library document is now Trusted only when git has it at HEAD with no staged or working-tree modification; every other document (untracked, staged, modified, or any file when git is unavailable) is stored Untrusted, on first index and on every re-index, and the manifest write can no longer lift it.

## Commits

- `726641cc2` fix(03-16): Library docs are Trusted only when committed and unmodified (D-08) (Tasks 1 and 2)
- `609fdcc34` docs(03-16): retrieval SSOT and CHANGELOG describe the four trust-gap closures (Task 3)

## What was built

- **Task 1 (tracer).** `IndexDeps::is_committed_clean` (`&dyn Fn(&str) -> bool + Sync`); `index_library` computes `tier = ceiling if clean else Untrusted` per walked file and writes it in `LibraryDocUpsert`. The corpus-wide `UPDATE search_documents ... WHERE corpus = ?2` in `put_kb_index_state` is deleted. `committed_clean_docs(repo_root)` in `kb_tools/library.rs` runs `ls-tree -r -z --name-only --full-name HEAD -- docs/src` and `diff -z --name-only --no-renames HEAD -- docs/src` through `GitExec` and returns the first set minus the second; any error returns the empty set. `reindex_at` passes `clean.contains`. All five other `IndexDeps` literal sites get `is_committed_clean: &|_: &str| true` (kb_eval with a comment).
- **Task 2.** `VoxDb::set_library_document_tier(corpus, rel_path, tier)` (single-row, parameterized, no-op on unknown pair). The hash-skip path now calls it before counting `docs_unchanged`; the read-error path demotes to Untrusted. `IndexReport::docs_untrusted` counts walked files whose tier is Untrusted (changed, unchanged, or unreadable); `report_json` exposes it.
- **Task 3.** SSOT tier table (Trusted, Advisory, Untrusted rows), a D-07/D-07a agent-graph-rows paragraph, rewritten "Where the tier is stored", the "Known gap: Tavily" bullet replaced by four injection-rule bullets (auto-preamble exclusion and Tavily skip, explicit Tavily `[tier:untrusted]`, agent graph rows with memory leg unchanged, research-source sanitization), and the web-capture bullet note. CHANGELOG trust-tier bullet extended.

## Tests and evidence (nextest, `CARGO_INCREMENTAL=0`, scratch `CARGO_TARGET_DIR`)

- `cargo nextest run -p vox-orchestrator-mcp --lib -E 'test(/kb_tools::library::tests::/)'`: 19/19, including new `kb_reindex_git_gate_trusts_only_committed_unmodified_docs`, `kb_reindex_git_gate_non_checkout_is_untrusted`, `kb_reindex_git_gate_retiers_unchanged_docs`; existing `kb_reindex_single_flight`, `kb_reindex_uses_vox_crypto_hasher`, `kb_reindex_report` (with `docs_untrusted` in its key list) unchanged and green.
- `cargo nextest run -p vox-search --test kb_library_index --test kb_library_chunking`: 34/34, including new `kb_library_tier_follows_the_commit_gate_per_file`; `kb_library_tier_rows_agree_with_corpus_tier` passes unchanged.
- `cargo nextest run -p vox-db -E 'test(/set_library_document_tier|kb_index_state|library_document_file_hash/)'`: 3/3, including new `set_library_document_tier_is_corpus_and_path_scoped` (also asserts the manifest write does not re-stamp).
- `cargo check -p vox-search -p vox-db -p vox-orchestrator-mcp --tests`: clean (no `error[E`).
- Doc lint: `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/search-retrieval-ssot-2026.md`: "no hard errors".
- Acceptance greps: re-stamp statement gone; `is_committed_clean` present in both files; 5 `GitExec` mentions; no `Command::new("git")` in the touched files; no `Cargo.toml` diff (no new crate edge); no "Known gap: Tavily" or "aligned onto the rows by" in the SSOT; D-07, D-07a, D-08 and `agent_fact` present.

## Mutation proofs (originals saved to scratchpad, restored by copy; hash-verified identical after; `grep -c` counts shown as before / mutated / restored)

| Mutation | Count | Red tests |
|---|---|---|
| (a) `reindex_at` classifier `\|rel\| clean.contains(rel)` -> `\|_rel\| true` | 1 / 0 / 1 | all three `kb_reindex_git_gate_*` (trusts_only_committed, non_checkout, retiers) |
| (b) restore the corpus-wide `UPDATE ... WHERE corpus = ?2` in `put_kb_index_state` | 0 / 1 / 0 | both Task 1 `kb_reindex_git_gate_*` tests, `kb_reindex_git_gate_retiers_unchanged_docs`, and `set_library_document_tier_is_corpus_and_path_scoped` (4/4 red) |
| (c) delete the skip-path `set_library_document_tier` call (count of the identifier in mod.rs) | 2 / 1 / 2 | `kb_library_tier_follows_the_commit_gate_per_file`, `kb_reindex_git_gate_retiers_unchanged_docs` (2/2 red) |

After restoration the full set above was re-run green.

## Deviations from Plan

**1. [Rule 3 - Blocking] Tasks 1 and 2 landed in one commit.** They edit the same four files and Task 1's tracer needs `docs_untrusted` / `set_library_document_tier` only for Task 2, but the skip-path re-tier and the report field are interleaved in `index_library` and the MCP tests; splitting would have required a throwaway intermediate state. Task 2's tests and mutation (c) are recorded separately above. Commit count is therefore 2, not 3.

**2. [Plan test fixture] vox-db test row collision.** The plan's "same rel_path in another corpus" cannot exist: `apply_library_document` keys `source_uri` on `kb_library:<rel_path>` regardless of corpus, so a second corpus with the same path overwrites the row. The test uses a distinct path in the other corpus and asserts that `set_library_document_tier(KB_LIBRARY_CORPUS, <other corpus path>)` is a no-op, which pins the corpus scope.

**3. [Plan command] Doc lint path.** `--paths` takes paths relative to `docs/src`, so the plan's `docs/src/architecture/...` argument errors; ran with `architecture/search-retrieval-ssot-2026.md`.

**4. [Minor] `docs_untrusted` also counts an unreadable file whose computed tier would have been Trusted**, because it is demoted. Matches the stated intent (operators see every Untrusted doc).

**Total deviations:** 4 (none change production behavior beyond the plan).

## Known behavior change for hosted CI

Any test or fixture that reindexes a non-git temp directory with `reindex_at`/`vox_kb_reindex` now gets Untrusted Library rows (the fail-closed case). The kb_tools, kb_library and vox-db suites above pass; `stdio_daemon_routing_tests`, `kb_status_inbox`, kb_eval and workspace suites were not run locally (lean policy) and belong to hosted CI. The `stdio_daemon_routing_tests` site builds `IndexDeps` directly with an always-clean classifier, so it keeps Trusted rows.

## Known Stubs

None.

## Threat Flags

None. T-03-58, T-03-59, T-03-60 mitigated (mutations a, b, c); T-03-61 mitigated (read-only `ls-tree`/`diff`, fixed arguments, through GitExec); T-03-62 mitigated (fixture refuses to run when `GIT_DIR`/`GIT_WORK_TREE` is set, hooks/signing/identity neutralized per command, temp dir only); T-03-63 mitigated (`docs_untrusted` in the report, SSOT documents D-08).

## Self-Check: PASSED

- Commits `726641cc2` and `609fdcc34` exist; `git rev-list --count 28f2c981f..HEAD` was 2 before this SUMMARY commit (`plan_head_before` is the persisted ledger value).
- Modified files exist; SUMMARY and mutation counts above come from the logged runs.
