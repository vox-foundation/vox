---
phase: 535-incremental-multirepo-kb-auto-indexing-and-db-unification
workstream: openwiki-self-healing-docs-ssot-v91
status: passed
verified_at: "2026-11-19T11:21:36Z"
requirements:
  - KB-AUTO-01
  - KB-AUTO-02
  - KB-AUTO-03
  - KB-AUTO-04
---

# Phase 535 Verification Report: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon `VoxDb`

## Verdict: PASSED

All 4 Phase 535 requirements (`KB-AUTO-01`..`KB-AUTO-04`) verified with observed unit/crate test outputs and live `vox kb reindex --json` + `vox kb status --json` checks after final edits.

## Verified Checks & Observed Output

| Check | Command | Outcome | Observed Result |
|---|---|---|---|
| `vox-search` Library Tests | `timeout 900s cargo test -p vox-search --lib` | PASSED (`exit 0`) | `286 passed; 0 failed; 1 ignored` (`deep_lane_probe_searxng_vs_tavily_subquery` owner-ignored live probe; includes `12` `kb_ids` tests and `discover_for_repo_exclusive_precedence_and_external_docs` after Red-to-Green fix) |
| MCP KB Tools Tests | `timeout 900s cargo test -p vox-orchestrator-mcp --lib kb_tools` | PASSED (`exit 0`) | `27 passed; 0 failed` (`1042 filtered out`; includes AP-15 `kb_get_never_reads_files` and `multirepo_kb_reindex_status_and_search_isolate_external_repos` verifying `repo_a`, `repo_b`, and `GlobalLibrary` isolation) |
| Daemon Lib Tests | `timeout 900s cargo test -p vox-orchestrator-d --lib` | PASSED (`exit 0`) | `37 passed; 0 failed` (re-run after final `daemon_steps/run.rs` edit) |
| GUI Docs Index Tests | `timeout 900s cargo test -p vox-gui docs_index` | PASSED (`exit 0`) | `9 passed; 0 failed` (`vox_gui` bin) + `9 passed; 0 failed` (`search_probe_test.rs`) after final `docs_index.rs` edit |
| CLI Binary Build | `timeout 900s cargo build -p vox-cli` | PASSED (`exit 0`) | Compiled cleanly (`Finished dev profile`) |
| Live KB Reindex | `timeout 900s ./target/debug/vox kb reindex --json` | PASSED (`exit 0`) | `docs_seen: 912`, `docs_changed: 912`, `docs_untrusted: 15`, `chunks_inserted: 17691`, `cards: 912`, `lexical_backend: "tantivy-0.22"`, `lexical_docs: 17691`, `corpus_commit: "c2edb14aee6bba41c4bee98005be01cf1f968290"`, `errors: []`, `duration_ms: 25585` |
| Live KB Status | `timeout 900s ./target/debug/vox kb status --json` | PASSED (`exit 0`) | `"indexed": true`, `"consistent": true`, `"doc_count": 912`, `"chunk_count": 17691`, `"db_chunk_count": 17691`, `"lexical_docs": 17691`, `"trust_tier": "trusted"`, `"errors": []` |

## Requirement Traceability

- **KB-AUTO-01:** `KbCorpusManifest::discover_for_repo_with_id` and `for_tier` use exclusive precedence (`docs/src/` -> `docs/wiki/` -> `docs/` -> root docs) without double-walking `docs/` or requiring `docs/wiki/`. `crates/vox-search/src/library/lexical.rs` adds `default_scoped_index_dir` keyed by `(repository_id, tier)` and `crates/vox-orchestrator-mcp/src/kb_tools/library.rs` routes `kb_reindex`, `kb_status`, `kb_search`, `kb_get`, `kb_cite` via `resolve_library_scope` with legacy `KB_LIBRARY_CORPUS` compatibility preserved.
- **KB-AUTO-02:** `ensure_kb_indexed_for_repo` / `ensure_kb_indexed_if_stale` performs incremental `HEAD != corpus_commit` and first-index bootstrapping in `kb_search`, `repo_catalog_tools` (`switch` and `wiki_heal_apply`), `vox-orchestrator-d` startup (`step_server_state`, behind `VOX_DISABLE_AUTO_KB_REINDEX`), and `vox-gui::commands::docs_index::write_doc_markdown` (cache invalidation + IPC `vox_kb_reindex` call).
- **KB-AUTO-03:** `crates/vox-cli/src/commands/ci/run_body.rs` executes `if heal` prior to `--apply-proposal`/`--reject-proposal`/`--auto-apply` (supporting `--heal --auto-apply`) and replaces `.vox/store.db` with `VoxDb::connect_default().await` + daemon IPC `vox_repo_catalog_refresh` fallback via `vox_cli_research::daemon_tool`.
- **KB-AUTO-04:** External `docs/`, `docs/src/`, and `~/.vox/library/` (`CorpusTier::GlobalLibrary` via `ensure_global_library_indexed`) index into isolated `(repository_id, tier)` shards, proven by `multirepo_kb_reindex_status_and_search_isolate_external_repos` with `0` cross-repo and `0` global-into-local leakage.
