---
phase: 02-library-index-and-kb-tool-surface
plan: 12
subsystem: kb-eval
tags: [vox-search, ratchet, clippy, gap-closure, requirements]
requires: [02-09, 02-10]
provides:
  - real-data ratchet scoring the production LibraryRetriever against the epoch-2 baseline
  - vox-search clippy -D warnings clean
  - IDX-02..IDX-07 Complete in workstream REQUIREMENTS.md
affects: [crates/vox-search]
tech-stack:
  added: []
  patterns: [config-aware baseline shape check, mutation-proved ratchet]
key-files:
  created: []
  modified:
    - crates/vox-search/tests/kb_eval_library.rs
    - crates/vox-search/tests/kb_eval_golden_contract.rs
    - crates/vox-search/src/kb_eval/library.rs
    - .planning/workstreams/kb/REQUIREMENTS.md
    - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/deferred-items.md
decisions:
  - "real_baseline_shape pins retriever/chunker config and epoch >= 2 instead of an exact epoch: an exact pin re-breaks on every legitimate re-epoch (this gap's cause); config assertions fail exactly when scored retriever and committed baseline diverge, and epochs only move forward via new_epoch."
metrics:
  duration: ~10 min
  completed: 2026-10-06
status: complete
actuals:
  tokens: 2500
  tasks: 3
  commits: 2
plan_head_before: 3fdb51cd8e5384c8e7eeef1f76b214dd9e090c88
---

# Phase 2 Plan 12: Ratchet retarget and clippy fix Summary

The Phase-1 real-data ratchet now scores the production Library retriever (`kb_eval::library::evaluate` over a `LibraryRetriever` at `EVAL_DEPTH` on `ratchet_split()`, the `gate.rs` path) against the committed epoch-2 baseline, and `cargo clippy -p vox-search --all-targets -- -D warnings` is clean.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 Tracer: retarget ratchet + clippy fix | 86d0ef43a | 11/11 tests pass, clippy exit 0 |
| 2 Mutation proof + kb regression surface | (no file change; evidence below) | M1, M2 killed with exit 100 |
| 3 REQUIREMENTS.md + deferred-items | c1e4b10b1 | 6 IDX checked, 7 table rows Complete |

Red state confirmed first: `real_baseline_shape` and `real_retriever_meets_real_baseline` exited 100 at HEAD (config "bm25-inmem-v1/... != baseline library-tantivy-v1/..." "not comparable" plus 18 metric-drop lines).

## Mutation proof

Plain `cargo nextest run --retries 0`, exit captured directly (no pipe).

**M1 (02-09 first-run boosts: TITLE 2.0, HEADING 1.5 in `library/lexical.rs`)**
- Presence counts before / during / after: `BOOST: f32 = 0.25;` 2 / 0 / 2; `MUTANT-RATCHET` 0 / 2 / 0.
- `test(=real_retriever_meets_real_baseline)`: nextest exit **100**; failing test `vox-search::kb_eval_library real_retriever_meets_real_baseline`.
- Kill came from quality, not config: 0 "not comparable" lines; e.g. `[regression] dev/all doc ndcg@10: 0.548914 -> 0.357935 (drop beyond tolerance 0.010000)` (also `doc recall@5 0.551974 -> 0.342105`, `doc mrr@10 0.607728 -> 0.404856`).
- The pre-plan BM25-scored test could not catch this mutant: BM25 never calls `library::search`.
- Restored with `git checkout -- crates/vox-search/src/library/lexical.rs`.

**M2 (degrade removed: `r.reverse();` -> `r.truncate(r.len()); // MUTANT-RATCHET` in `kb_eval_library.rs`)**
- Presence counts before / during / after: `r.reverse();` 1 / 0 / 1; `MUTANT-RATCHET` 0 / 1 / 0.
- `test(=degraded_retriever_fails_real_baseline)`: nextest exit **100**; failing test `vox-search::kb_eval_library degraded_retriever_fails_real_baseline`, panic "reversed ranking must regress" (unchanged ranking meets the baseline).
- Restored with `git checkout -- crates/vox-search/tests/kb_eval_library.rs`.

After both: `git diff --quiet HEAD -- lexical.rs kb_eval_library.rs` exit 0; `grep -c MUTANT-RATCHET` 0 for both files.

## Verification

- `binary(kb_eval_library) | binary(kb_eval_golden_contract)`: 11 run, 11 passed, 0 skipped (6 + 5).
- kb regression surface (`test(/kb_/) | binary(/kb_/) | test(/walk_corpus_/) | test(/tantivy_mirror/) | test(/chunk_fence/) | test(/chunk_oversize/)`): **111 tests run, 111 passed, 0 skipped/failed** (floor was 80).
- `cargo clippy -p vox-search --all-targets -- -D warnings`: exit 0, 0 `error` lines.
- `git diff --quiet eb47f2888 -- contracts/kb/ crates/vox-search/src/kb_eval/baseline.rs crates/vox-search/src/library/lexical.rs`: exit 0 (baseline, ratchet logic, boosts untouched).
- REQUIREMENTS.md: 6 `[x] IDX-02..07`, 0 unchecked IDX, 7 `IDX-01..07 | Phase 2 | Complete` rows, RET-01 Complete, 11 unrelated Pending rows unchanged. `requirements.mark-complete ... --ws kb` worked directly; no manual-edit fallback needed.
- deferred-items.md: 2 `RESOLVED (02-12, 86d0ef43a)` entries; the 02-05 `--no-default-features` entry left as is.

## Deviations from Plan

**1. [Plan inconsistency, no code impact] Acceptance grep `grep -c 'fn real_\|fn degraded_' kb_eval_golden_contract.rs` prints 1, not 0.** The match is the retained `fn real_dir()` helper, which the five golden-set contract tests use and which the plan itself says stays. All three ratchet tests, `real_baseline`, `real_report` and the BM25 import are gone from that file (`kb_eval::bm25` count 0).

**2. [Formatting] rustfmt expanded the `build_blocking(...)` call in the library.rs unit test onto multiple lines** after the `from_ref` swap (10-line diff instead of one line); still test-only code.

Otherwise: plan executed as written. No auth gates, no stubs, no new threat surface.

## Threat flags

None.

## Self-Check: PASSED

- Commits 86d0ef43a and c1e4b10b1 exist on the branch; `git rev-list --count 3fdb51cd8..HEAD` = 2 before this docs commit.
- Modified files present; MUTANT-RATCHET absent from the tree.
