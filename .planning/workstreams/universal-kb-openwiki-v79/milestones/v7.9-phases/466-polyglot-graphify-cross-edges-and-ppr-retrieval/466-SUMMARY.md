# Phase 466 Summary — Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval

**Phase:** `466` (`KB-03`, `KB-04`, `GRAPH-01`, `GRAPH-02`, `GRAPH-03`, `GRAPH-04`)
**Milestone:** `v7.9` (`universal-kb-openwiki-v79`)
**Status:** Complete
**Date:** `2026-11-17`

---

## What Was Built

1. **CI-Map Relocation & Scoped Per-Repo Graphify Corpora (`GRAPH-01`):**
   - Added `REPO_CI_MAPS_CACHE_DIR` (`.vox/cache/ci-maps`) and `repo_ci_maps_cache_dir` in `crates/vox-config/src/paths.rs`, migrating `dep_cycles.rs`, `build_bench.rs`, and `commands/graphify/mod.rs` away from `graphify-out/`.
   - Added `resolve_scoped_graphify_cache_dir`, `default_repo_graphify_corpora`, `provision_repo_graphify_corpora`, and `load_all_corpora` registered-overlay fallback in `crates/vox-config/src/graphify.rs`, wired into `onboard_repository_harness_with_home` in `crates/vox-repository/src/repo_catalog/catalog.rs`.

2. **Polyglot Extraction & Code ↔ Doc ↔ Decision Cross-Edges (`GRAPH-02`, `GRAPH-03`):**
   - Created `crates/vox-graph-reader/src/polyglot.rs` and `polyglot_edges.rs` (`HonestyLevel::{Extracted, Inferred, Ambiguous}`, `CrossEdgeKind::{DocumentedIn, CitesSymbol, GovernedByAdr, TouchedByDecision}`, Go package/struct/interface/method/fn extraction, SQL `CREATE TABLE`/`REFERENCES` DDL extraction, HTTP/CLI endpoint extraction, Markdown/ADR citation extraction, and `materialize_cross_edges`), wired cleanly through `ast.rs` and `rebuild.rs` while keeping all `vox-graph-reader` modules `< 500` lines.
   - Added `GraphifyReader::personalized_pagerank` in `crates/vox-graph-reader/src/lib.rs` and `VoxDb::ingest_graphify_cross_edges` in `crates/vox-db/src/store/ops_memory/knowledge.rs`.

3. **HippoRAG PPR & Leiden Community Retrieval (`GRAPH-04`):**
   - Implemented `ppr_and_community_search_graph` (`PprGraphHit`, `CommunitySummaryHit`) in `crates/vox-config/src/graphify.rs` and wired it into `SearchCorpus::GraphifyStructural` + 1-hop cross-edge seed expansion in `SearchCorpus::KnowledgeGraph` inside `crates/vox-search/src/execution.rs`.

4. **Bi-Directional KB Promotion & Hermetic Multi-Repo Eval (`KB-03`, `KB-04`):**
   - Added `kb_finding_by_id` and `kb_finding_mark_reviewed` in `crates/vox-db/src/store/ops_memory/kb_findings.rs`.
   - Created `crates/vox-search/src/promotion.rs` (`verify_citations_mechanically`, `promote_finding_to_kb`, `promote_mempalace_learning_to_kb`, `sync_promoted_note_bidirectional`) gated by `SanitizedText`, fail-closed `promotion_eligibility`, and disk citation verification.
   - Added `MultiRepoEvalReport`, `run_hermetic_multi_repo_eval`, and `--multi-repo` on `vox kb eval` (`crates/vox-search/src/kb_eval/mod.rs`, `crates/vox-cli-research/src/kb/{eval.rs,mod.rs}`).

## Files Modified / Created

- `crates/vox-config/src/{paths.rs,graphify.rs}`
- `crates/vox-cli-ci/src/{dep_cycles.rs,build_bench.rs}`
- `crates/vox-cli/src/commands/graphify/mod.rs`
- `crates/vox-repository/src/repo_catalog/catalog.rs`
- `crates/vox-graph-reader/src/{lib.rs,ast.rs,rebuild.rs,polyglot.rs,polyglot_edges.rs}`
- `crates/vox-db/src/store/ops_memory/{knowledge.rs,kb_findings.rs}`
- `crates/vox-search/src/{lib.rs,execution.rs,promotion.rs,kb_eval/mod.rs}`
- `crates/vox-cli-research/src/kb/{mod.rs,eval.rs}`

## Key Verification Highlights

- `vox-config --lib graphify`: 26/26 passed.
- `vox-repository`: 56/56 unit + 8/8 integration tests passed.
- `vox-graph-reader`: 95/95 unit + all integration suites passed with 0 compiler warnings (`1` pre-existing ignored history test); all modified files `< 500` lines.
- `vox-db --lib store::ops_memory`: 24/24 passed.
- `vox-search --lib`: 284 passed, 0 failed, 1 pre-existing ignored.
- `vox-cli --test command_catalog_paths_baseline`: 1/1 passed.

## Handoff to Phase 467

Ready to plan and execute Phase 467: *Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions* (`WIKI-01..02`).
