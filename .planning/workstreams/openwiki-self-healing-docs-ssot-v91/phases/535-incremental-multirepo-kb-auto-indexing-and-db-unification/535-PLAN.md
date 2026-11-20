---
phase: 535-incremental-multirepo-kb-auto-indexing-and-db-unification
plan: 01
type: execute
wave: 1
depends_on:
  - 534-honest-ground-truth-resolver-and-drift-gate
files_modified:
  - crates/vox-search/src/kb_ids.rs
  - crates/vox-search/src/library/lexical.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs
  - crates/vox-orchestrator-d/src/daemon_steps/run.rs
  - crates/vox-cli-research/src/daemon_tool.rs
  - crates/vox-cli/src/commands/ci/run_body.rs
  - crates/vox-gui/src/commands/docs_index.rs
autonomous: true
requirements:
  - KB-AUTO-01
  - KB-AUTO-02
  - KB-AUTO-03
  - KB-AUTO-04
must_haves:
  truths:
    - "KbCorpusManifest::discover_for_repo_with_id picks docs/src exclusively over docs/, falls back to docs/ + root Markdown on external repos without requiring docs/wiki/."
    - "vox_kb_reindex, vox_kb_status, vox_kb_search isolate (repository_id, corpus_tier) in Tantivy and VoxDb with 0 cross-repo leakage while preserving BEIR/legacy compatibility."
    - "vox-orchestrator-d triggers non-blocking ensure_kb_indexed_for_repo on startup unless VOX_DISABLE_AUTO_KB_REINDEX is set."
    - "vox ci wiki-sync runs --heal before --auto-apply/--apply-proposal and uses VoxDb::connect_default() with daemon IPC fallback instead of .vox/store.db."
    - "Live ./target/debug/vox kb reindex and ./target/debug/vox kb status report indexed and consistent state."
  artifacts:
    - path: "crates/vox-search/src/kb_ids.rs"
      provides: "Exclusive docs/src -> docs/wiki -> docs precedence in KbCorpusManifest"
    - path: "crates/vox-orchestrator-mcp/src/kb_tools/library.rs"
      provides: "Scoped multi-repo KB indexing/status/search and ensure_kb_indexed_for_repo/if_stale"
    - path: "crates/vox-orchestrator-d/src/daemon_steps/run.rs"
      provides: "Non-blocking background KB auto-indexing in step_server_state"
    - path: "crates/vox-cli/src/commands/ci/run_body.rs"
      provides: "Unified VoxDb connection and --heal before --auto-apply execution order"
  key_links:
    - from: "crates/vox-orchestrator-d/src/daemon_steps/run.rs"
      to: "crates/vox-orchestrator-mcp/src/kb_tools/library.rs"
      via: "ensure_kb_indexed_for_repo spawned in tokio::spawn"
    - from: "crates/vox-orchestrator-mcp/src/kb_tools/library.rs"
      to: "crates/vox-search/src/library/mod.rs"
      via: "index_library_scoped / status_scoped / search_scoped_with_taxonomy"
---

# Phase 535 Plan 01: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon VoxDb

<objective>
Upgrade KB corpus discovery, scoped Tantivy/VoxDb partitioning, background daemon auto-indexing, and CLI proposal persistence so Vox (`docs/src/`) and external repositories (`docs/` or root Markdown) stay automatically indexed and query-isolated without `.vox/store.db` split-brain.
</objective>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: TDD Red — exclusive docs root precedence in vox-search::kb_ids</name>
  <files>crates/vox-search/src/kb_ids.rs</files>
  <action>
    Add unit test `discover_for_repo_exclusive_precedence_and_external_docs` verifying exclusive `docs/src` precedence over `docs/`, external `docs/` fallback, and root Markdown fallback, and run `timeout 900s cargo test -p vox-search --lib kb_ids` to confirm RED failure.
  </action>
  <verify>New unit test fails on current multi-push loop.</verify>
</task>

<task type="auto">
  <name>Task 2: Implement exclusive precedence and scoped index helper in vox-search</name>
  <files>crates/vox-search/src/kb_ids.rs, crates/vox-search/src/library/lexical.rs</files>
  <action>
    Update `KbCorpusManifest::discover_for_repo_with_id` and `for_tier` for exclusive `docs/src` -> `docs/wiki` -> `docs` precedence, ensure `allows_rel_path` permits `"docs"` root files when `root_dirs` includes `"docs"`, and add `default_scoped_index_dir` in `lexical.rs`.
  </action>
  <verify>`timeout 900s cargo test -p vox-search --lib kb_ids` passes.</verify>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Upgrade vox-orchestrator-mcp kb_tools + repo_catalog_tools for multi-repo isolation and auto-reindex</name>
  <files>crates/vox-orchestrator-mcp/src/kb_tools/library.rs, crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs</files>
  <action>
    Implement `committed_clean_docs_for_manifest`, `reindex_scoped_at`, `ensure_kb_indexed_for_repo`, `ensure_kb_indexed_if_stale`, and scoped routing across `kb_reindex`, `kb_status`, `kb_search`, `kb_get`, `kb_cite` (preserving legacy `KB_LIBRARY_CORPUS` fallback and `AP-15` zero-fs rule above `#[cfg(test)]`). Add multi-repo isolation test `multirepo_kb_reindex_status_and_search_isolate_external_repos` and wire `ensure_kb_indexed_if_stale` into `repo_catalog_tools.rs`.
  </action>
  <verify>`timeout 900s cargo test -p vox-orchestrator-mcp --lib kb_tools` passes.</verify>
</task>

<task type="auto">
  <name>Task 4: Wire daemon background auto-reindex and unify vox ci wiki-sync onto canonical VoxDb</name>
  <files>crates/vox-orchestrator-d/src/daemon_steps/run.rs, crates/vox-cli/src/commands/ci/run_body.rs</files>
  <action>
    Spawn non-blocking `ensure_kb_indexed_for_repo` inside `step_server_state` (unless `VOX_DISABLE_AUTO_KB_REINDEX` is set), reorder `CiCmd::WikiSync` so `if heal` runs before `--auto-apply`/`--apply-proposal`, replace `.vox/store.db` with `VoxDb::connect_default().await` + daemon IPC fallback. Format all touched files, run scoped tests/builds, and run live `vox kb reindex` + `vox kb status`.
  </action>
  <verify>Scoped `vox-search`, `vox-orchestrator-mcp`, `vox-orchestrator-d` tests and `vox-cli` build succeed; `vox kb status` reports `indexed: true` and `consistent: true`.</verify>
</task>

</tasks>
