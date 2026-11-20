---
phase: 535-incremental-multirepo-kb-auto-indexing-and-db-unification
workstream: openwiki-self-healing-docs-ssot-v91
milestone: v9.1
status: executing
---

# Phase 535 Context: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon VoxDb

## Goal

Eliminate empty/stale KB indexes and `.vox/store.db` split-brain across repositories by:
- Switching KB discovery (`KbCorpusManifest::discover_for_repo_with_id`) to exclusive precedence (`docs/src/` -> `docs/wiki/` -> `docs/` + root Markdown), never double-walking `docs/` on Vox and never requiring `docs/wiki/` on external repositories.
- Wiring scoped `(repository_id, corpus_tier)` Tantivy + `VoxDb` indexing (`index_library_scoped`, `status_scoped`, `search_scoped_with_taxonomy`, `default_scoped_index_dir`) into `vox-orchestrator-mcp` `kb_tools` (`kb_reindex`, `kb_status`, `kb_search`, `kb_get`, `kb_cite`), with seamless backward compatibility for `KB_LIBRARY_CORPUS` legacy tests.
- Running non-blocking background `ensure_kb_indexed_for_repo` on `vox-orchestrator-d` startup (`step_server_state`, respecting `VOX_DISABLE_AUTO_KB_REINDEX`) and after repo switch / heal application.
- Removing the `.vox/store.db` split-brain in `crates/vox-cli/src/commands/ci/run_body.rs` (`CiCmd::WikiSync`), ordering `--heal` before `--apply-proposal` / `--reject-proposal` / `--auto-apply`, and routing DB reads/writes through `VoxDb::connect_default()` with daemon IPC fallback.

## Requirements Covered

- **KB-AUTO-01:** Multi-repo `(repository_id, corpus_tier)` scoped Tantivy + `VoxDb` indexing via `KbCorpusManifest::discover_for_repo_with_id`.
- **KB-AUTO-02:** Non-blocking daemon startup and post-heal/switch incremental `blake3` + `HEAD` auto-reindexing.
- **KB-AUTO-03:** `vox ci wiki-sync` unified canonical `VoxDb` + daemon IPC proxy and `--heal --auto-apply` ordering.
- **KB-AUTO-04:** Strict per-repo isolation (`0` cross-repo leakage in `repo_local` queries across `docs/src/` and `docs/` + root Markdown repos).

## Critical Invariant

`crates/vox-orchestrator-mcp/src/kb_tools/library.rs` test `kb_get_never_reads_files` (`AP-15`) scans everything above `#[cfg(test)]` for `std::fs`, `tokio::fs`, `File::open`, and `read_to_string`. Keep all filesystem I/O inside `vox-search`.
