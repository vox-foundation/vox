# Phase 466 Plan — Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval

**Phase:** `466`
**Requirements:** `KB-03`, `KB-04`, `GRAPH-01`, `GRAPH-02`, `GRAPH-03`, `GRAPH-04`

## Execution Sequence

1. **`GRAPH-01` & `GRAPH-04` (Part A) in `vox-config`, `vox-cli-ci`, `vox-cli`, `vox-repository`:**
   - Add `REPO_CI_MAPS_CACHE_DIR` & `repo_ci_maps_cache_dir` in `crates/vox-config/src/paths.rs`.
   - Update `crates/vox-cli-ci/src/{dep_cycles.rs,build_bench.rs}` and `crates/vox-cli/src/commands/graphify/mod.rs` to write CI maps to `.vox/cache/ci-maps/` (with read fallback for `crate_audit.json`).
   - Add `resolve_scoped_graphify_cache_dir`, `default_repo_graphify_corpora`, `provision_repo_graphify_corpora`, external fallback in `load_all_corpora`, and `ppr_and_community_search_graph` + tests in `crates/vox-config/src/graphify.rs`.
   - Wire `provision_repo_graphify_corpora` inside `crates/vox-repository/src/repo_catalog/catalog.rs` onboarding.

2. **`GRAPH-02` & `GRAPH-03` in `vox-graph-reader` & `vox-db`:**
   - Create `crates/vox-graph-reader/src/polyglot.rs` (`HonestyLevel`, `CrossEdgeKind`, Go/SQL/Endpoint/Markdown extractors, `materialize_cross_edges`, unit tests).
   - Export `polyglot` and add `personalized_pagerank` on `GraphifyReader` in `crates/vox-graph-reader/src/lib.rs`.
   - Wire polyglot + inline extractors in `crates/vox-graph-reader/src/ast.rs` and `augment_rebuild_with_polyglot_and_cross_edges` in `crates/vox-graph-reader/src/rebuild.rs`.
   - Add `ingest_graphify_cross_edges` + unit test in `crates/vox-db/src/store/ops_memory/knowledge.rs`.

3. **`GRAPH-04` (Part B) in `vox-search::execution`:**
   - Upgrade `SearchCorpus::GraphifyStructural` to load via `load_all_corpora` and run `ppr_and_community_search_graph`, plus 1-hop cross-edge expansion in `SearchCorpus::KnowledgeGraph`, with RRF unit tests.

4. **`KB-03` & `KB-04` (`promotion.rs` & `kb_eval`):**
   - Add `kb_finding_by_id` and `kb_finding_mark_reviewed` in `crates/vox-db/src/store/ops_memory/kb_findings.rs`.
   - Create `crates/vox-search/src/promotion.rs` (3-gate promotion + bidirectional sync + tests) and export in `crates/vox-search/src/lib.rs`.
   - Add `MultiRepoEvalReport`, `run_hermetic_multi_repo_eval`, and unit test in `crates/vox-search/src/kb_eval/mod.rs`; wire `--multi-repo` in `crates/vox-cli-research/src/kb/eval.rs`.

5. **Format, Test & Record Phase 466 Completion:**
   - Run `rustfmt --edition 2024` on all modified/created Rust files.
   - Verify `vox-config`, `vox-repository`, `vox-graph-reader`, `vox-db`, `vox-search`, `vox-cli` baseline, and zero hardcoded home-path literals.
   - Write `466-VERIFICATION.md`, `466-REVIEW.md`, `466-SUMMARY.md`, update `REQUIREMENTS.md`, `ROADMAP.md`, `STATE.md`, `state.json`, and hand back for Phase 467 planning.
