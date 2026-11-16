---
phase: 02-library-index-and-kb-tool-surface
plan: 10
subsystem: search
tags: [kb, library-index, prune, gap-closure, cr-01, mutation-proof]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-04 index_library with incremental reindex and prune"
provides:
  - "walk_corpus(repo_root) -> (Vec<CorpusFile>, Vec<FileError>) with repo-relative walk errors"
  - "index_library: docs/src is_dir check (Err before any DB access) and prune_safe guard around prune_library_documents_not_in"
  - "Regression tests for unreadable subdir, missing root, empty root with rows, empty root on a fresh DB"
affects: [phase-03]
status: complete
actuals:
  tokens: 9000
  tasks: 3
  commits: 2
plan_head_before: 36f032c0d7c7dac5a2f46e172247f186925d093b
commits: 2
key-files:
  created: []
  modified:
    - crates/vox-search/src/library/corpus.rs
    - crates/vox-search/src/library/mod.rs
    - crates/vox-search/tests/kb_library_index.rs
    - crates/vox-search/tests/kb_library_chunking.rs
    - crates/vox-orchestrator-mcp/src/kb_resources.rs
decisions:
  - "Only the prune is gated on walk completeness; orphan-embedding cleanup and the Tantivy rebuild stay unconditional (gating them would orphan vectors or leave edited docs unsearchable)"
  - "Missing docs/src is a hard Err (absolute repo root allowed in that message: it only reaches vox_kb_reindex); walk errors in errors_json stay repo-relative"
requirements-completed: [IDX-01]
---

# Phase 2 Plan 10: Prune safety on incomplete walks (CR-01) Summary

A reindex no longer reads a partial, empty or missing corpus walk as "everything else was deleted": the walker reports its errors, prune runs only when the walk was complete and not empty-with-rows, and a missing `docs/src` fails the call before any DB access.

## What changed

- `corpus.rs`: `walk_corpus` iterates the walker directly and returns walk errors as `FileError` (path repo-relative via one shared `rel_slash_path` helper, falling back to the corpus root; text is the io error's Display, which carries no host path).
- `mod.rs`: `ensure!(repo_root.join(root).is_dir())` per `KB_CORPUS_ROOTS` right after trust-tier validation; walk errors are folded into `report.errors`; `prune_safe = walk_complete && (docs_seen > 0 || library_chunk_count == 0)`; prune only inside `if prune_safe {`, else a `prune skipped:` `FileError`. Orphan-embedding cleanup, rebuild decision and manifest write remain unconditional.
- Callers: `kb_resources.rs::list_kb_doc_resources` and the four call sites in `kb_library_chunking.rs` take `.0`.

## Tests

- corpus.rs unit: `walk_corpus_reports_missing_root`, `walk_corpus_reports_unreadable_dir` (cfg unix, restores perms before asserting, early return if perms are bypassed); the two existing tests now also assert no errors.
- kb_library_index.rs: `kb_reindex_unreadable_subdir_keeps_rows` (rows, vectors, and a same-run edit stay searchable; no absolute path in errors), `kb_reindex_missing_root_fails_and_keeps_rows` (manifest `indexed_at` unchanged), `kb_reindex_empty_root_keeps_rows`, `kb_reindex_empty_root_fresh_db_ok`.
- `cargo nextest run -p vox-search -E 'test(/walk_corpus_|kb_reindex_unreadable_subdir_keeps_rows|kb_reindex_prune/)'`: 6 run, 6 passed.
- Task 2 filter: 4 run, 4 passed.
- `cargo check -p vox-orchestrator-mcp --tests`: ok.
- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_|list_kb_doc_resources|parse_kb_uri/)'`: 33 run, 33 passed.
- vox-search kb suite (full verify filter): 111 run, 109 passed, 2 failed — both pre-existing, see Deferred Issues.

## Mutation proof

Both mutants applied with a presence check before/during/after, restored with `git checkout --` (`git diff --quiet HEAD -- crates/vox-search/src/library/mod.rs` exits 0 afterwards).

| Mutant | Edit | Guard-token count before/during/after | MUTANT-CR01 count before/during/after | nextest exit | Failing tests |
|---|---|---|---|---|---|
| M1 prune guard | `if prune_safe {` -> `if true /* MUTANT-CR01 prune */ {` | 1 / 0 / 1 | 0 / 1 / 0 | 100 | `kb_reindex_empty_root_keeps_rows`, `kb_reindex_unreadable_subdir_keeps_rows` (both FAIL) |
| M2 root check | `repo_root.join(root).is_dir()` -> `true /* MUTANT-CR01 root */` | 1 / 0 / 1 | 0 / 1 / 0 | 100 | `kb_reindex_missing_root_fails_and_keeps_rows` |

Final tree: `grep -c MUTANT-CR01 mod.rs` = 0. (The first M1 attempt lost its exit status to a pipe; M1 was re-run with output redirected to a file to capture the exact status 100.)

## Deviations from Plan

### Auto-fixed Issues

None in code. Process note: Tasks 1 and 2 were committed separately as planned (the `is_dir` check and its three tests were held back from the Task 1 commit). I did not run a separate red phase before implementing; the tests could not compile against the old `walk_corpus` signature.

### Deferred Issues (out of scope, logged in `deferred-items.md`)

1. `kb_eval_golden_contract::real_baseline_shape` and `real_retriever_meets_real_baseline` fail at HEAD: 02-09 re-epoched the baseline to epoch 2 / Library config while these Phase-1 tests still pin epoch 1 and the BM25 retriever. The plan's `fails_when` (non-zero exit on the kb suite) therefore trips for reasons unrelated to CR-01; all 02-10 and other kb tests pass.
2. `cargo clippy -p vox-search --all-targets -- -D warnings` fails on `clippy::cloned_ref_to_slice_refs` at `crates/vox-search/src/kb_eval/library.rs:185` (02-09 code). With that single lint allowed the crate, including all 02-10 changes, is clean.

## Known Stubs

None.

## Threat Flags

None. T-02-33 to T-02-37 mitigated as planned; T-02-38 accepted.

## Self-Check: PASSED

Files verified present; commits `733282e7b` and `7ea733fd1` exist; `git rev-list --count` from the ledger base = 2.
