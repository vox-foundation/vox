---
phase: 02-library-index-and-kb-tool-surface
verified: 2026-10-05T00:00:00Z
status: gaps_found
score: 4/5 must-haves verified
covered_files:
  - .planning/workstreams/kb/REQUIREMENTS.md
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
  - contracts/kb/retrieval-baseline.v1.json
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-orchestrator-mcp/src/kb_resources.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-search/src/library/chunk.rs
  - crates/vox-search/src/library/corpus.rs
  - crates/vox-search/src/library/lexical.rs
  - crates/vox-search/src/library/mod.rs
covered_digest: "v1:sha256:1c78bc38f38b133d2b57ad151b0b948a476e1fcdca1a9cef68e2562ad96d68a6"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "SC1: incremental reindex deletes/renames prune their rows (and only those) - safely, without silently destroying the Library"
    status: partial
    reason: "CR-01 (confirmed in code): walk_corpus uses walker.flatten() (drops every walkdir error; a missing docs/src yields an empty Vec with no error) and index_library feeds that list straight into prune_library_documents_not_in, which deletes every stored doc not in `walked`, then prune_orphan_kb_chunk_embeddings and a Tantivy rebuild run. A transient subdir error or a daemon whose repository.root lacks docs/src prunes the whole Library while the report shows errors: [] and success. The in-code comment only covers per-file read errors. No test covers a missing/unreadable root."
    artifacts:
      - path: "crates/vox-search/src/library/corpus.rs"
        issue: "line 34 `for entry in walker.flatten()` swallows walk errors; walk_corpus returns no error channel"
      - path: "crates/vox-search/src/library/mod.rs"
        issue: "lines ~79-141: prune + vector cleanup + lexical rebuild run on an unverified-complete walk"
    missing:
      - "Make walk_corpus fallible (collect walk errors); skip prune/orphan-cleanup/rebuild and record the error in report.errors when any walk error occurred or docs_seen == 0 with prior docs > 0"
      - "Fail the call when <repo_root>/docs/src does not exist"
      - "Regression test: empty/missing corpus root and an unreadable subdirectory leave stored rows intact (mutation-prove by removing the guard)"
---

# Phase 2: Library Index and KB Tool Surface - Verification Report

**Phase Goal:** The curated corpus (Vox docs plus general-knowledge `docs/src/library/`) is indexed incrementally with stable chunk identity. Any MCP host reaches it through one KB tool contract.
**Verified:** 2026-10-05
**Status:** gaps_found
**Re-verification:** No - initial verification

## Goal Achievement

Every literal success criterion has code and test evidence except that the "prune" half of SC1 is unsafe: a single, narrow, cheap-to-fix defect (CR-01) lets an ordinary reindex delete the entire indexed Library while reporting success. The remaining findings in 02-REVIEW.md are warnings that do not falsify a success criterion.

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `vox kb reindex` on unchanged tree rewrites zero chunks; one-section edit rewrites one chunk; delete/rename prunes; no orphan `embeddings.source_id` | PARTIAL (FAILED on safe prune) | Tests `kb_reindex_unchanged`, `kb_reindex_one_section`, `kb_reindex_insert_middle_section`, `kb_reindex_prune`, `kb_no_orphan_embeddings`, `kb_kept_hash_keeps_vector` exist in `crates/vox-search/tests/kb_library_index.rs`; I re-ran unchanged/one_section/insert_middle/prune: PASS. File-hash skip + chunk-hash diff in `library/mod.rs` and `ops_memory/library.rs::apply_library_document`. BUT CR-01: prune trusts an infallible, possibly empty/partial walk (see gap). |
| 2 | `#` in fence / frontmatter never creates/pollutes chunks; every chunk has `heading_path`; oversize splits within embedder budget; `docs/src/archive/` never indexed | VERIFIED | `kb_library_chunking.rs`: `chunk_fence_hash_no_split`, `chunk_frontmatter_never_in_body`, `chunk_heading_path_h3_to_h6`, `chunk_oversize_split_within_budget`, `kb_archive_excluded`, `walker_matches_kb_filter` exist; I re-ran fence/oversize/archive: PASS. `corpus.rs` walker prunes `KB_EXCLUDED_DIR_NAMES` and defers to `is_kb_corpus_path`. |
| 3 | `vox kb status` reports commit, doc/chunk counts, embed model, manifest, per-file errors; every doc has a summary-card row | VERIFIED (with note) | `render_kb_status` in `vox-cli/src/commands/kb.rs` prints commit, docs, chunks, cards, chunker version, lexical backend, embed model, errors, consistency; `kb_manifest_and_cards`, `kb_status_reports_errors` tests; `kb_index_state` table. Note: embed model reports `none (lexical-only)` and embed id/revision/dim are NULL - deliberate (AP-07; embedder is Phase 5). |
| 4 | From an external MCP host: `vox_kb_search/get/cite/status/reindex` + `kb://doc/{path}`, `kb://chunk/{hash}`; a page added under `docs/src/library/` appears in `Library` sidebar and search | VERIFIED | Five tools in `contracts/operations/catalog.v1.yaml` and `kb_tools/library.rs` (handler tests incl. single-flight, bad-input rejection, never-reads-files); resources in `kb_resources.rs`, wired in `server.rs` `read_resource` via daemon; `stdio_daemon_routing_tests.rs` covers tools+`kb://` through the real daemon path. Sidebar: `Library` in `lint.rs` VALID_CATEGORIES + sidebar contract JSON + parity test; `docs/src/library/index.md` exists. External-host UAT (Claude Code, probe page found then pruned) recorded in 02-09 SUMMARY; human approval by brbrainerd 2026-10-05 per orchestrator. |
| 5 | Lexical leg = recorded probe's choice; writer-less `TantivyDocsIndex` mirror gone; doc-level eval does not regress vs Phase 1 baseline (qrels re-keyed once, delta recorded) | VERIFIED | Probe `turso_fts_probe` (Phase 1, re-run in 02-04: 5/5) found no FTS5 -> Tantivy chosen; documented in where-things-live row. `grep TantivyDocsIndex` over crates/contracts: only the guard test `tantivy_mirror_removed.rs` (PASS). `contracts/kb/retrieval-baseline.v1.json` epoch 2: doc dev/all nDCG@10 0.4599 -> 0.5489, all dev/all metrics above epoch 1; epoch_note records 2 deepened qrels (q046, x041) and boost retune (dev split only); human-approved. |

**Score:** 4/5 truths verified.

### Required Artifacts / Key Links

| Artifact | Status | Details |
|----------|--------|---------|
| `crates/vox-search/src/library/{mod,corpus,chunk,lexical}.rs` | VERIFIED (CR-01 defect in mod/corpus) | Substantive, wired from `kb_tools/library.rs` and `kb_eval/library.rs` |
| `crates/vox-db/src/store/ops_memory/library.rs` + schema 95 | VERIFIED | hash-keyed upsert/prune/readers; `kb_index_state` |
| `crates/vox-orchestrator-mcp/src/kb_tools/library.rs`, `kb_resources.rs` | VERIFIED | handlers, catalog, registry, HTTP read-role entry |
| `crates/vox-cli/src/commands/kb.rs` | VERIFIED | `status`/`reindex` route to daemon tools |
| `docs/src/library/index.md`, `Library` category | VERIFIED | lint vocab + sidebar parity test |
| `contracts/kb/retrieval-baseline.v1.json` | VERIFIED | epoch 2, gate passes per 02-09 |

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| IDX-01 | PARTIAL | Incremental/hash-skip/orphan cleanup verified; prune is unsafe on an incomplete walk (CR-01) |
| IDX-02 | SATISFIED | chunking tests (SC2) |
| IDX-03 | SATISFIED (embed fields NULL until Phase 5, by design) | `kb_index_state` manifest + card rows |
| IDX-04 | SATISFIED | `kb_archive_excluded`, `walker_matches_kb_filter` |
| IDX-05 | SATISFIED | CLI `vox kb status|reindex` via daemon |
| IDX-06 | SATISFIED | `docs/src/library/`, Library category/sidebar |
| IDX-07 | SATISFIED | probe -> Tantivy; mirror removed |
| RET-01 | SATISFIED | 5 tools + 2 resources; UAT approved |

All 8 phase IDs are accounted for; none are ORPHANED (REQUIREMENTS.md maps exactly IDX-01..07 and RET-01 to Phase 2). REQUIREMENTS.md still shows them "Pending" - the orchestrator should flip them on completion (IDX-01 once CR-01 closes).

### Anti-Patterns / Review Findings

No TODO/FIXME/XXX/TBD in the phase's new files. Stub scan clean. From 02-REVIEW.md (code verified for CR-01; others taken from the review, not independently re-proven):

| Finding | Severity here | Impact |
|---------|---------------|--------|
| CR-01 empty/partial walk prunes whole Library | BLOCKER (gap above) | Silent destruction of indexed data + vectors, success reported; reachable via shared daemon with wrong `repository.root` (WR-07) |
| WR-01 non-atomic Tantivy dir swap / deletes any dir at index path | WARNING | Search outage window; rename failure after delete loses live index |
| WR-02 single-flight per-process only; no timeout entry for `vox_kb_reindex`; dropped future leaves blocking rebuild running | WARNING | Concurrent `.tmp` clobber from second process |
| WR-03 per-doc transaction not cancellation-safe on shared connection | WARNING | A 120s dispatch timeout can leave the daemon connection in an open transaction |
| WR-04 DB/Tantivy can diverge after failed rebuild; `status` count-only consistency | WARNING | New one-for-one edit unsearchable, status says consistent |
| WR-05 `source_uri` hard-codes library corpus | WARNING (latent; single corpus today) | Re-labels docs if a 2nd corpus ever reuses a rel_path |
| WR-06 library chunks leak into generic `search_document_chunks` readers | WARNING | Library rows mixed into RAG/Socrates evidence with web docs; overlaps Phase 3 single-retrieval-path work (RET-02) - not formally deferred |
| WR-07 CLI acts on shared daemon's root, not caller's | WARNING | Amplifies CR-01 |
| WR-08 `vox_kb_status` leaks abs paths to HTTP read role | WARNING | Information disclosure |
| IN-01..06 | INFO | see 02-REVIEW.md |

Pre-existing, out of scope: `vox-search --no-default-features` fails to compile (tavily cfg; `deferred-items.md`), not introduced by this phase.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| reindex incremental/prune, chunking, archive, mirror gone | `cargo nextest run -p vox-search -E 'test(/tantivy_mirror/) \| test(/kb_reindex/) \| test(/kb_archive/) \| test(/chunk_fence/) \| test(/chunk_oversize/)'` | 8/8 passed | PASS |
| Broad regression (orchestrator-run) | `-p vox-search -E 'test(/kb_/)'` | 75/75 (per orchestrator) | PASS |
| Empty/missing root keeps stored rows | none exists | no test | FAIL (confirms gap: unprotected) |

### Human Verification

None outstanding. The two human checkpoints (02-01 schema/placement decision; 02-09 external-host UAT, epoch-2 baseline, boost retune) are recorded as approved by brbrainerd 2026-10-05.

### Gaps Summary

One gap, one root cause (CR-01): the prune step treats an infallible walk as authoritative. It is a small fix (return walk errors from `walk_corpus`, skip prune/cleanup/rebuild on error or on an empty walk with existing rows, assert `docs/src` exists, add a mutation-proven regression test). Index rows are derived data, so the loss is recoverable by a later successful reindex, but it is silent and the report says success, which is why it is held as a blocker against SC1/IDX-01 rather than a warning. Recommend closing CR-01 via `/gsd-plan-phase 2 --gaps` and, in the same pass, cheaply taking WR-02 (explicit reindex timeout) and WR-08 (drop abs paths for read role); WR-03/04/06/07 can be scheduled (WR-06 naturally with Phase 3). If the maintainer judges CR-01 acceptable-for-now, record an override with a reason and accepted_by.

---

_Verified: 2026-10-05_
_Verifier: Claude (gsd-verifier)_
