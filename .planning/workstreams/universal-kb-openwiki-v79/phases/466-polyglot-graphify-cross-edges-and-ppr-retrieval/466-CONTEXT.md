# Phase 466 Context — Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval

**Milestone:** `v7.9` — Universal KB, Graphify, Self-Healing OpenWiki & Multi-Repo GUI
**Workstream:** `universal-kb-openwiki-v79`
**Requirements:** `KB-03`, `KB-04`, `GRAPH-01`, `GRAPH-02`, `GRAPH-03`, `GRAPH-04`
**Status:** Executing

## Locked Decisions

1. **CI Maps vs Graphify Shards (`GRAPH-01`):**
   - Relocate all non-Graphify CI maps (`DEP_CYCLES.md`, `build-bench/`, `rebuild_causes.json`, `rebuild_fingerprint.log`, `crate_audit.json`, `edge_weights.json`) out of `graphify-out/` into `.vox/cache/ci-maps/` via `vox_config::paths::repo_ci_maps_cache_dir`.
   - Provision isolated per-repo Graphify corpus manifests (`repo-code-graph`, `doc-wiki-graph`, `gui-surface-graph`) under `<cache_base>/graphify/<repo_id>/<corpus_id>/` during `onboard_repository_harness[_with_home]`.
   - When `contracts/retrieval/vox-graph-corpora.v1.yaml` is absent on an external repository, `load_all_corpora` falls back seamlessly to `load_registered_corpora`.

2. **Polyglot Extraction & Code ↔ Doc ↔ Decision Cross-Edges (`GRAPH-02`, `GRAPH-03`):**
   - Isolate new polyglot & cross-edge logic in `crates/vox-graph-reader/src/polyglot.rs` (<500 lines) to honor the `vox-graph-reader` module cap (`rebuild.rs` stays under 500 lines).
   - Extract Go packages/structs/interfaces/methods/functions, SQL `CREATE TABLE` / `REFERENCES` schemas, HTTP/API routes (`/path`), and Markdown / ADR citations (`#Lstart-Lend`, qualified `Type::sym`, bare identifiers).
   - Tag cross-edges (`documented_in`, `cites_symbol`, `governed_by_adr`, `touched_by_decision`) with honesty levels `EXTRACTED` (`1.0`), `INFERRED` (`0.7`), and `AMBIGUOUS` (`0.35`), persisting into both Graphify JSON exports and `VoxDb` `knowledge_nodes`/`knowledge_edges`.

3. **HippoRAG PPR & Leiden Community Summaries (`GRAPH-04`):**
   - Expose deterministic Personalized PageRank (`ppr_and_community_search_graph` in `vox-config::graphify` and `GraphifyReader::personalized_pagerank` in `vox-graph-reader`) plus Leiden community summary aggregation weighted by edge honesty.
   - Integrate PPR + Leiden community summary retrieval into `vox-search::execute_search_plan` (`SearchCorpus::GraphifyStructural` + 1-hop `KnowledgeGraph` seed expansion) and fuse via RRF.

4. **Bi-Directional Promotion & Multi-Repo Eval (`KB-03`, `KB-04`):**
   - Implement `promote_finding_to_kb`, `promote_mempalace_learning_to_kb`, and `sync_promoted_note_bidirectional` in `crates/vox-search/src/promotion.rs`, gated strictly by `SanitizedText`, `promotion_eligibility`, and `verify_citations_mechanically`.
   - Implement hermetic `run_hermetic_multi_repo_eval` in `crates/vox-search/src/kb_eval/mod.rs` and expose `--multi-repo` flag on `vox kb eval`.
