# Phase 466 Code & Security Review

**Phase:** `466` — Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval
**Date:** `2026-11-17`
**Reviewer Scope:** Edits across `vox-config`, `vox-cli-ci`, `vox-cli`, `vox-repository`, `vox-graph-reader`, `vox-db`, `vox-search`, and `vox-cli-research`.

---

## Findings Summary

| Axis | Status | Assessment |
|---|---|---|
| **Namespace & Cache Isolation (`GRAPH-01`)** | Pass | Relocating `DEP_CYCLES.md`, `build-bench`, `rebuild_causes.json`, `rebuild_fingerprint.log`, `crate_audit.json`, and `edge_weights.json` into `.vox/cache/ci-maps/` eliminates `graphify-out/` collisions while preserving a read fallback for legacy `crate_audit.json`. Per-repo Graphify manifests sanitize `repository_id` and `corpus_id` and provision under `<cache_base>/graphify/<repo_id>/<corpus_id>/`. External repos without `contracts/retrieval/vox-graph-corpora.v1.yaml` resolve seamlessly via `load_all_corpora` overlay fallback without weakening `load_graphify_corpora` strictness. |
| **Module Size & Dependency Discipline** | Pass | Split polyglot extractors (`polyglot.rs`: 363 lines) and cross-edge materialization (`polyglot_edges.rs`: 394 lines) cleanly so every file in `vox-graph-reader` remains strictly `< 500` lines (`rebuild.rs` at 483 lines). Zero new crates added to any `Cargo.toml` or `Cargo.lock` (catching and replacing `tempfile::tempdir()` in `run_hermetic_multi_repo_eval` with a zero-dep RAII `EvalTempDir`). |
| **Determinism & Edge Honesty (`GRAPH-02`, `GRAPH-03`)** | Pass | Go, SQL, Endpoint, Markdown/ADR extractors and `materialize_cross_edges` use deterministic sorting/deduplication (`BTreeSet` / sorted paths) and explicit honesty tagging (`EXTRACTED` = 1.0, `INFERRED` = 0.7, `AMBIGUOUS` = 0.35). Ambiguous symbol names never claim `EXTRACTED` certainty. |
| **Promotion Safety Gates (`KB-03`)** | Pass | `promote_finding_to_kb`, `promote_mempalace_learning_to_kb`, and `sync_promoted_note_bidirectional` fail closed on `PromotionEligibility::Ineligible` (e.g., `Retracted` / `Unavailable`), sanitize all fields via `SanitizedText`, sanitize destination slugs to prevent path traversal, and mechanically verify repo-relative Markdown `#Lstart-Lend` citations against live files on disk. |
| **Search & Multi-Repo Retrieval (`GRAPH-04`, `KB-04`)** | Pass | `ppr_and_community_search_graph` performs weighted Personalized PageRank (12 power iterations, dangling mass preserved) plus Leiden community rollups, and `execute_search_plan` fuses both PPR node hits and community summaries alongside 1-hop `KnowledgeGraph` cross-edge expansion. `run_hermetic_multi_repo_eval` verifies `Recall@5`, `MRR@10`, multi-hop ADR grounding, and strictly `0` cross-repo leakage. |
