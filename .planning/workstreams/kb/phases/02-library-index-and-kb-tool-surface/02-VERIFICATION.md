---
phase: 02-library-index-and-kb-tool-surface
verified: 2026-10-05T00:00:00Z
status: gaps_found
score: 5/5 roadmap success criteria verified; 2 cross-phase regression gaps open
covered_files:
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-01-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-01-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-02-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-02-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-03-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-03-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-04-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-04-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-05-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-05-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-06-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-06-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-07-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-07-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-08-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-08-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-09-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-09-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-10-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-10-SUMMARY.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-11-PLAN.md
  - .planning/workstreams/kb/phases/02-library-index-and-kb-tool-surface/02-11-SUMMARY.md
  - .planning/workstreams/kb/REQUIREMENTS.md
  - contracts/kb/retrieval-baseline.v1.json
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-orchestrator-mcp/src/kb_resources.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
  - crates/vox-search/src/library/chunk.rs
  - crates/vox-search/src/library/corpus.rs
  - crates/vox-search/src/library/lexical.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/tests/kb_eval_golden_contract.rs
  - crates/vox-search/src/kb_eval/library.rs
covered_digest: "v1:sha256:5a459485cbbc558470f2e601e0d0996aef8509a0a38bb11c14a0f3712c982d38"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "SC1 safe prune (CR-01): walk errors surfaced, prune gated, missing docs/src hard-fails, mutation-proven tests"
    - "WR-08: vox_kb_status removed from HTTP read role"
    - "WR-02 (partial): explicit 600 s dispatch timeout for vox_kb_reindex"
  gaps_remaining: []
  regressions:
    - "Phase-1 real-data ratchet tests red at HEAD (introduced by 02-09 epoch-2 re-baseline)"
    - "clippy -D warnings red for vox-search (02-09 code)"
gaps:
  - truth: "Phase 2 does not leave the Phase-1 retrieval ratchet red: tests that pin the committed baseline pass after the baseline is re-epoched"
    status: failed
    reason: "02-09 (6a9a6b4be) re-epoched contracts/kb/retrieval-baseline.v1.json to epoch 2 / library-tantivy-v1 config but did not update the Phase-1 tests that pin it. Verified by running them at HEAD: kb_eval_golden_contract::real_baseline_shape (asserts epoch == 1) and ::real_retriever_meets_real_baseline (scores the in-memory BM25 retriever against the Library baseline; reports 'config bm25-inmem-v1/... != baseline library-tantivy-v1/...: metrics are not comparable' plus ~18 metric drops) both FAIL after 3 retries. The production gate (vox kb eval --gate, which scores LibraryRetriever, and kb_eval_gate/kb_eval_library tests) is intact and passes; only the stale pins are red. But vox-search is an affected crate on every PR touching it, so the hosted `tests` shard goes red, and nothing in Phases 3-12 owns the fix (not deferrable under Step 9b)."
    artifacts:
      - path: "crates/vox-search/tests/kb_eval_golden_contract.rs"
        issue: "real_baseline_shape pins epoch 1; real_retriever_meets_real_baseline builds the BM25 in-memory retriever, not the Library retriever the baseline now describes"
    missing:
      - "Point the real-data ratchet tests at LibraryRetriever (or make them epoch/config-aware) and update the epoch assertion to the committed epoch; keep degraded_retriever_fails_real_baseline meaningful against the Library config"
  - truth: "vox-search passes cargo clippy --all-targets -D warnings (hosted `linux` job runs clippy on affected crates)"
    status: failed
    reason: "Verified at HEAD: clippy::cloned_ref_to_slice_refs at crates/vox-search/src/kb_eval/library.rs:185 (`&[q.clone()]`), introduced by 02-09, fails the lib-test target under -D warnings."
    artifacts:
      - path: "crates/vox-search/src/kb_eval/library.rs"
        issue: "line 185: use std::slice::from_ref(&q)"
    missing:
      - "Replace `&[q.clone()]` with `std::slice::from_ref(&q)` and re-run `cargo clippy -p vox-search --all-targets -- -D warnings`"
---

# Phase 2: Library Index and KB Tool Surface - Verification Report (re-verification)

**Phase Goal:** The curated corpus (Vox docs plus general-knowledge `docs/src/library/`) is indexed incrementally with stable chunk identity. Any MCP host reaches it through one KB tool contract.
**Verified:** 2026-10-05
**Status:** gaps_found
**Re-verification:** Yes - after gap-closure plans 02-10 and 02-11

## Summary

All five ROADMAP success criteria now hold. The prior blocker (CR-01, unsafe prune) is closed in code and by mutation-proven tests, and WR-08 / WR-02 are closed or mitigated. Two small, mechanical regressions that this phase itself introduced (02-09) remain red at HEAD: two Phase-1 ratchet tests and one clippy lint in `vox-search`. They do not falsify a success criterion, but they turn the project's own hosted CI gate red for the crate this phase owns, and no later phase picks them up, so they are held as gaps rather than carried as warnings. Both are a few lines to fix.

## Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Reindex on unchanged tree rewrites zero chunks; one-section edit rewrites one chunk; delete/rename prunes; no orphan `embeddings.source_id` - safely | VERIFIED | Re-ran: `kb_reindex_unchanged`, `kb_reindex_one_section`, `kb_reindex_insert_middle_section`, `kb_reindex_prune` PASS. CR-01 closure read in code: `corpus.rs::walk_corpus` returns `(Vec<CorpusFile>, Vec<FileError>)` with walk errors repo-relative; `mod.rs` fails with `ensure!(repo_root.join(root).is_dir())` before DB access, folds walk errors into `report.errors`, and gates the only production `prune_library_documents_not_in` call behind `prune_safe = walk_complete && (docs_seen > 0 || library_chunk_count == 0)`; a skipped prune is recorded as a `prune skipped:` error. New tests PASS: `kb_reindex_unreadable_subdir_keeps_rows`, `kb_reindex_missing_root_fails_and_keeps_rows`, `kb_reindex_empty_root_keeps_rows`, `kb_reindex_empty_root_fresh_db_ok`, `walk_corpus_reports_missing_root`, `walk_corpus_reports_unreadable_dir`. 02-10 SUMMARY records two mutants (guard -> `true`, root check -> `true`) each failing the intended tests with presence checks; I did not re-mutate, but the tests directly exercise the guarded states. |
| 2 | `#` in fence/frontmatter never chunks; every chunk has `heading_path`; oversize splits in budget; `docs/src/archive/` never indexed | VERIFIED | `chunk_fence_hash_no_split`, `chunk_oversize_split_within_budget`, `kb_archive_excluded` PASS (re-run). Walker prunes `KB_EXCLUDED_DIR_NAMES` and defers to `is_kb_corpus_path`. |
| 3 | `vox kb status` reports commit, counts, embed model, manifest, per-file errors; every doc has a card row | VERIFIED | `render_kb_status`, `kb_index_state` manifest; embed fields NULL / "none (lexical-only)" by design until Phase 5 (AP-07). |
| 4 | External MCP host: five `vox_kb_*` tools + `kb://doc`, `kb://chunk`; Library sidebar section and search | VERIFIED | `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_\|metadata_read_role\|dispatch_timeout::/)'`: 47/47 PASS, including `stdio_daemon_routing_tests::kb_resources_and_tools_via_daemon`, `kb_tool_surface`, `metadata_read_role_excludes_vox_kb_status`, `kb_reindex_gets_explicit_timeout`. WR-08 re-checked: `vox_kb_status` count in `contracts/mcp/http-read-role-governance.yaml` = 0; `KB_REINDEX_TIMEOUT` = 600 s in `EXPLICIT_TIMEOUTS`. External-host UAT approved by brbrainerd 2026-10-05 (02-09). |
| 5 | Lexical leg = probe choice; writer-less mirror gone; doc-level eval does not regress; qrels re-keyed once | VERIFIED | `tantivy_mirror_removed` PASS; no `TantivyDocsIndex` outside the guard. Baseline epoch 2: doc dev/all nDCG@10 0.4599 -> 0.5489. `kb_eval_gate`, `kb_eval_library`, `kb_eval_determinism` (8 tests) PASS. `vox kb eval --gate` (pre-push) scores `LibraryRetriever` against the epoch-2 baseline. |

**Score:** 5/5 success criteria verified. Gaps below are cross-phase regressions, not criterion failures.

## Regression Gate

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Phase-1 gate tests (synthetic baseline) | `-p vox-search -E 'binary(kb_eval_gate)\|binary(kb_eval_library)\|binary(kb_eval_determinism)'` | 8/8 passed | PASS |
| Phase-1 real-data ratchet | `-p vox-search -E 'test(/real_baseline_shape\|real_retriever_meets_real_baseline/)'` | 2/2 FAILED (3 tries) | FAIL |
| Prune / chunking / mirror / archive | 16 tests incl. 02-10 additions | 16/16 passed | PASS |
| MCP kb surface | 47 tests | 47/47 passed | PASS |
| Lint | `cargo clippy -p vox-search --all-targets -- -D warnings` | 1 error, `cloned_ref_to_slice_refs` at `kb_eval/library.rs:185` | FAIL |

Decision on the regression gate question: it IS a phase-02 gap. The phase's own plan (02-09) re-epoched the committed baseline and is the sole cause; the epoch change is sanctioned by SC5, but leaving the tests that pin that file red is not. The deferred-items note ("belongs with the eval owner") does not hold: Phase 1 is complete and no Phase 3-12 goal or success criterion covers it.

## Required Artifacts / Key Links

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-search/src/library/{mod,corpus,chunk,lexical}.rs` | VERIFIED | CR-01 defect fixed; wired from `kb_tools/library.rs`, `kb_eval/library.rs`, `kb_resources.rs` (takes `.0` of walk) |
| `crates/vox-db/src/store/ops_memory/library.rs` | VERIFIED | hash-keyed upsert/prune/readers |
| `crates/vox-orchestrator-mcp/src/{kb_tools/library,kb_resources,dispatch_timeout}.rs` | VERIFIED | handlers, resources, timeout row |
| `contracts/mcp/http-read-role-governance.yaml`, `tool-registry.canonical.yaml`, `operations/catalog.v1.yaml` | VERIFIED | consistent exclusion of `vox_kb_status` from read role |
| `crates/vox-cli/src/commands/kb.rs` | VERIFIED | status/reindex route to daemon |
| `contracts/kb/retrieval-baseline.v1.json` | VERIFIED (data) | epoch 2; but stale pins in tests (gap 1) |
| `crates/vox-search/tests/kb_eval_golden_contract.rs` | STALE | pins epoch 1 / BM25 |

## Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| IDX-01 | SATISFIED | SC1 incl. safe prune (CR-01 closed) |
| IDX-02 | SATISFIED | chunking tests |
| IDX-03 | SATISFIED | manifest + card rows (embed fields NULL until Phase 5 by design) |
| IDX-04 | SATISFIED | `kb_archive_excluded`, `walker_matches_kb_filter` |
| IDX-05 | SATISFIED | `vox kb status|reindex` via daemon |
| IDX-06 | SATISFIED | `docs/src/library/`, `Library` category/sidebar |
| IDX-07 | SATISFIED | probe -> Tantivy; mirror removed |
| RET-01 | SATISFIED | 5 tools + 2 resources; UAT approved |

All 8 IDs accounted for; none orphaned. REQUIREMENTS.md currently has IDX-01 and RET-01 checked/"Complete" and IDX-02..07 still unchecked/"Pending" - the orchestrator should reconcile once the two gaps close (all eight are satisfied in code).

## Anti-Patterns

No TBD/FIXME/XXX/TODO in `crates/vox-search/src/library/*.rs`. Open review warnings (02-REVIEW.md, not re-proven independently, none falsify a success criterion): WR-01 (non-atomic Tantivy swap), WR-02 (per-process single-flight; false lock comment), WR-03 (transaction not cancellation-safe), WR-04 (DB/Tantivy divergence undetected), WR-05 (`source_uri` hard-codes corpus), WR-06 (library chunks leak into generic `search_document_chunks`; overlaps Phase 3 RET-02), WR-07 (CLI uses shared daemon root), WR-09 (CLI deadline vs 600 s reindex). Pre-existing: `vox-search --no-default-features` fails (tavily cfg), logged in deferred-items.

## Human Verification

None outstanding (02-01 and 02-09 checkpoints recorded approved).

## Gaps Summary

Two mechanical gaps, both from 02-09, both fixable in one small plan (`/gsd-plan-phase 2 --gaps`): (1) retarget or make epoch-aware `real_baseline_shape` and `real_retriever_meets_real_baseline` so they pin the Library retriever and epoch 2; (2) replace `&[q.clone()]` with `std::slice::from_ref(&q)` at `kb_eval/library.rs:185`. After those, re-run `cargo nextest run -p vox-search -E 'test(/kb_/)'` and the scoped clippy; the phase then passes. If the maintainer prefers to accept these as debt, record an override with `accepted_by` and a reason.

---

_Verified: 2026-10-05_
_Verifier: Claude (gsd-verifier)_
