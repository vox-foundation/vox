---
phase: 466
phase_name: "polyglot-graphify-cross-edges-and-ppr-retrieval"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 466 Learnings: polyglot-graphify-cross-edges-and-ppr-retrieval

## Decisions

### Added REPO_CI_MAPS_CACHE_DIR (.vox/cache/ci-maps) and repo_ci_maps_cache...
Added `REPO_CI_MAPS_CACHE_DIR` (`.vox/cache/ci-maps`) and `repo_ci_maps_cache_dir` in `crates/vox-config/src/paths.rs`, migrating `dep_cycles.rs`, `build_bench.rs`, and `commands/graphify/mod.rs` away from `graphify-out/`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 466-SUMMARY.md

---

### Added resolve_scoped_graphify_cache_dir, default_repo_graphify_corpora, ...
Added `resolve_scoped_graphify_cache_dir`, `default_repo_graphify_corpora`, `provision_repo_graphify_corpora`, and `load_all_corpora` registered-overlay fallback in `crates/vox-config/src/graphify.rs`, wired into `onboard_repository_harness_with_home` in `crates/vox-repository/src/repo_catalog/catalog.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 466-SUMMARY.md

---

### Created crates/vox-graph-reader/src/polyglot.rs and polyglot_edges.rs (H...
Created `crates/vox-graph-reader/src/polyglot.rs` and `polyglot_edges.rs` (`HonestyLevel::{Extracted, Inferred, Ambiguous}`, `CrossEdgeKind::{DocumentedIn, CitesSymbol, GovernedByAdr, TouchedByDecision}`, Go package/struct/interface/method/fn extraction, SQL `CREATE TABLE`/`REFERENCES` DDL extraction, HTTP/CLI endpoint extraction, Markdown/ADR citation extraction, and `materialize_cross_edges`), wired cleanly through `ast.rs` and `rebuild.rs` while keeping all `vox-graph-reader` modules `< 500` lines.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 466-SUMMARY.md

---

### Added GraphifyReader::personalized_pagerank in crates/vox-graph-reader/s...
Added `GraphifyReader::personalized_pagerank` in `crates/vox-graph-reader/src/lib.rs` and `VoxDb::ingest_graphify_cross_edges` in `crates/vox-db/src/store/ops_memory/knowledge.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 466-SUMMARY.md

---

### Implemented ppr_and_community_search_graph (PprGraphHit, CommunitySummar...
Implemented `ppr_and_community_search_graph` (`PprGraphHit`, `CommunitySummaryHit`) in `crates/vox-config/src/graphify.rs` and wired it into `SearchCorpus::GraphifyStructural` + 1-hop cross-edge seed expansion in `SearchCorpus::KnowledgeGraph` inside `crates/vox-search/src/execution.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 466-SUMMARY.md

## Lessons

## Patterns

## Surprises

