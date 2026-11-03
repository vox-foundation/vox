---
phase: 02-library-index-and-kb-tool-surface
verified: 2026-10-06T00:00:00Z
status: passed
score: 5/5 roadmap success criteria verified; both prior gaps closed

behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 5/5 criteria, 2 cross-phase regression gaps open
  gaps_closed:
    - "Phase-1 real-data ratchet tests red after 02-09 re-epoch (real_baseline_shape, real_retriever_meets_real_baseline)"
    - "clippy cloned_ref_to_slice_refs at kb_eval/library.rs:185"
  gaps_remaining: []
  regressions: []
---
# Phase 2: Library Index and KB Tool Surface - Verification Report (re-verification 2)

**Phase Goal:** The curated corpus (Vox docs plus general-knowledge `docs/src/library/`) is indexed incrementally with stable chunk identity. Any MCP host reaches it through one KB tool contract.
**Verified:** 2026-10-06
**Status:** passed
**Re-verification:** Yes - after gap-closure plan 02-12 (commits 86d0ef43a, c1e4b10b1, 72b024c93)

## Summary

Both gaps from the previous report are closed, verified by re-running the checks myself, not from the 02-12 SUMMARY. The Phase-1 real-data ratchet now scores the production `LibraryRetriever` against the epoch-2 baseline and is mutation-proven (per 02-12; the test bodies directly exercise the guarded state). `cargo clippy -p vox-search --all-targets -- -D warnings` exits 0. The baseline, ratchet logic and lexical boosts are byte-identical to `eb47f2888`, so the ratchet was retargeted, not loosened. All five ROADMAP success criteria still hold; no regressions in the prune, chunking, MCP, or eval suites.

## Gap Re-checks

| Gap | Command (scoped, plain cargo) | Result | Status |
|-----|-------------------------------|--------|--------|
| Ratchet tests red | `cargo nextest run -p vox-search -E 'binary(kb_eval_library) \| binary(kb_eval_golden_contract)'` | 11 run, 11 passed, 0 skipped; includes `real_baseline_shape`, `real_retriever_meets_real_baseline`, `degraded_retriever_fails_real_baseline` | CLOSED |
| clippy lint | `cargo clippy -p vox-search --all-targets -- -D warnings` | exit 0 (checked with a direct `$?`, not a pipe) | CLOSED |
| Ratchet not loosened | `git diff --quiet eb47f2888 -- contracts/kb/ crates/vox-search/src/kb_eval/baseline.rs crates/vox-search/src/library/lexical.rs` | exit 0 | CONFIRMED |

Code read (`crates/vox-search/tests/kb_eval_library.rs`): `real_retriever_meets_real_baseline` builds `LibraryRetriever` over the ratchet split at `EVAL_DEPTH`, calls `library::evaluate`, and asserts `check_regression(&report, &real_baseline())` is empty. `degraded_retriever_fails_real_baseline` reverses the Library ranking, forces like-for-like config labels, and asserts a `dev/all` metric drop and no "not comparable" error, so it fails on quality, not on a label mismatch. `real_baseline_shape` asserts `retriever == LIBRARY_RETRIEVER`, `chunker == CHUNKER_VERSION`, `epoch >= 2`, tolerance, `RATCHET_KEYS`, budget and estimator. The `>= 2` pin is a deliberate design choice (an exact pin caused the original gap); config equality is what ties the scored retriever to the committed baseline. `kb_eval/library.rs:185` now uses `std::slice::from_ref(&q)`. The old BM25-scored tests and the epoch-1 pin are removed from `kb_eval_golden_contract.rs`; the retained `real_dir()` helper is used by the five golden-set contract tests.

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Reindex on unchanged tree rewrites zero chunks; one-section edit rewrites one chunk; delete/rename prunes; no orphan `embeddings.source_id` - safely | VERIFIED | Regression run `cargo nextest run -p vox-search -E 'test(/kb_/) \| binary(/kb_/) \| test(/walk_corpus_/) \| test(/tantivy_mirror/) \| test(/chunk_fence/) \| test(/chunk_oversize/)'`: 111/111 passed, covering `kb_reindex_unchanged`, `kb_reindex_one_section`, `kb_reindex_prune`, and the CR-01 safe-prune tests (`kb_reindex_unreadable_subdir_keeps_rows`, `kb_reindex_missing_root_fails_and_keeps_rows`, `kb_reindex_empty_root_keeps_rows`, `walk_corpus_*`). Safe-prune code unchanged since the previous verification. |
| 2 | `#` in fence/frontmatter never chunks; every chunk has `heading_path`; oversize splits in budget; `docs/src/archive/` never indexed | VERIFIED | `chunk_fence_hash_no_split`, `chunk_oversize_split_within_budget`, `kb_archive_excluded` in the 111/111 run. |
| 3 | `vox kb status` reports commit, counts, embed model, manifest, per-file errors; every doc has a card row | VERIFIED | Unchanged since previous verification; embed fields NULL / "none (lexical-only)" by design until Phase 5. |
| 4 | External MCP host: five `vox_kb_*` tools + `kb://doc`, `kb://chunk`; Library sidebar section and search | VERIFIED | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_\|metadata_read_role\|dispatch_timeout::/)'`: 47/47 passed (1 leaky, non-failing). External-host UAT approved by brbrainerd 2026-10-05 (02-09). |
| 5 | Lexical leg = probe choice; writer-less mirror gone; doc-level eval does not regress; qrels re-keyed once | VERIFIED | `tantivy_mirror_removed` in the 111/111 run; `real_retriever_meets_real_baseline` passes against the epoch-2 baseline for the production Library retriever (the eval does not regress against its own committed baseline); baseline file unchanged vs `eb47f2888`. |

**Score:** 5/5 truths verified, 0 behavior-unverified.

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| IDX-01 | SATISFIED | SC1; REQUIREMENTS.md `[x]`, table Complete |
| IDX-02 | SATISFIED | chunking tests; `[x]`, Complete |
| IDX-03 | SATISFIED | manifest + card rows; `[x]`, Complete |
| IDX-04 | SATISFIED | `kb_archive_excluded`; `[x]`, Complete |
| IDX-05 | SATISFIED | `vox kb status\|reindex` via daemon; `[x]`, Complete |
| IDX-06 | SATISFIED | `docs/src/library/`, Library category; `[x]`, Complete |
| IDX-07 | SATISFIED | probe -> Tantivy, mirror removed; `[x]`, Complete |
| RET-01 | SATISFIED | 5 tools + 2 resources; `[x]`, Complete |

REQUIREMENTS.md reconciled: `grep` confirms `[x]` on IDX-01..07 and RET-01 and `Phase 2 | Complete` table rows for all eight. No orphaned IDs (all eight appear in plan frontmatter and map to Phase 2).

## Anti-Patterns

No TBD/FIXME/XXX in `crates/vox-search/tests/kb_eval_library.rs` or `crates/vox-search/src/kb_eval/library.rs`; none previously in `library/*.rs`. Open review warnings from 02-REVIEW.md (0f548efdc: 0 critical; WR-01..07, WR-09 open; IN-10..12 info) are not re-proven here and none falsifies a success criterion: WR-01 non-atomic Tantivy swap, WR-02 per-process single-flight, WR-03 transaction not cancellation-safe, WR-04 DB/Tantivy divergence undetected, WR-05 `source_uri` hard-codes corpus, WR-06 library chunks leak into generic `search_document_chunks` (overlaps Phase 3 RET-02), WR-07 CLI uses shared daemon root, WR-09 CLI deadline vs 600 s reindex. Pre-existing and logged in deferred-items.md: `cargo check -p vox-search --no-default-features` fails (tavily cfg), unrelated to this phase.

## Human Verification

None outstanding (02-01 and 02-09 checkpoints recorded approved).

## Gaps Summary

No gaps. The phase goal is achieved. Carry the open review warnings into Phase 3+ planning (WR-06 especially) and the no-default-features item as tracked debt.

---
_Verified: 2026-10-06_
_Verifier: Claude (gsd-verifier)_
