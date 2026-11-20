---
phase: 535-incremental-multirepo-kb-auto-indexing-and-db-unification
workstream: openwiki-self-healing-docs-ssot-v91
status: completed
completed_at: "2026-11-19T11:21:36Z"
---

# Phase 535 Summary: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon `VoxDb`

## What Changed

- **`crates/vox-search/src/kb_ids.rs` & `crates/vox-search/src/library/lexical.rs`:** Switched `KbCorpusManifest::discover_for_repo_with_id` and `for_tier(RootDocs)` to exclusive precedence (`docs/src/` -> `docs/wiki/` -> `docs/` -> root Markdown candidates) so external repos without `docs/src/` index `docs/` + root docs without double-walking `docs/` or requiring `docs/wiki/`; added `default_scoped_index_dir` partitioned by `(repository_id, tier)`.
- **`crates/vox-orchestrator-mcp/src/kb_tools/library.rs`:** Added `resolve_library_scope`, `ensure_kb_indexed_for_repo`, `ensure_kb_indexed_if_stale`, `ensure_global_library_indexed`, `committed_clean_docs_for_roots`/`_for_manifest`, `reindex_scoped_at`, and `validate_doc_path_for_manifest`; switched `kb_reindex`, `kb_status`, `kb_search`, `kb_get`, and `kb_cite` to scoped Tantivy/`VoxDb` queries with legacy `KB_LIBRARY_CORPUS` fallback; added `multirepo_kb_reindex_status_and_search_isolate_external_repos`.
- **`crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs`:** Wired non-blocking `ensure_kb_indexed_for_repo` on repository `switch` and `wiki_heal_apply`.
- **`crates/vox-orchestrator-d/src/daemon_steps/run.rs`:** Spawned non-blocking startup `ensure_kb_indexed_for_repo` and `ensure_global_library_indexed` (`~/.vox/library`) inside `step_server_state` unless `VOX_DISABLE_AUTO_KB_REINDEX` is set.
- **`crates/vox-cli-research/src/daemon_tool.rs` & `crates/vox-cli/src/commands/ci/run_body.rs`:** Exposed `daemon_client`/`tool_call`, reordered `CiCmd::WikiSync` so `if heal` runs before `--auto-apply`/`--apply-proposal`, and replaced `.vox/store.db` split-brain reads/writes with `VoxDb::connect_default()` + daemon IPC `vox_repo_catalog_refresh` fallback.
- **`crates/vox-gui/src/commands/docs_index.rs`:** Updated `vox_docs_index` cache (`RwLock<Option<Vec<DocEntry>>>`) + exclusive docs root precedence and wired `write_doc_markdown` to clear `DOCS_CACHE` and trigger asynchronous `vox_kb_reindex` over `BackendCalls`.

## Verification Highlights

- `cargo test -p vox-search --lib`: `286 passed; 0 failed; 1 ignored` (owner-ignored live web probe).
- `cargo test -p vox-orchestrator-mcp --lib kb_tools`: `27 passed; 0 failed` (1042 filtered out; includes `kb_get_never_reads_files` and `multirepo_kb_reindex_status_and_search_isolate_external_repos`).
- `cargo test -p vox-orchestrator-d --lib`: `37 passed; 0 failed`.
- `cargo test -p vox-gui docs_index`: `9 passed; 0 failed` (`bin` + `search_probe_test.rs`).
- `./target/debug/vox kb reindex --json`: `912 docs_seen`, `17691 chunks_inserted`, `912 cards`, `errors: []`.
- `./target/debug/vox kb status --json`: `"indexed": true`, `"consistent": true`, `db_chunk_count: 17691 == lexical_docs: 17691`.
