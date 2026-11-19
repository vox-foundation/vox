# Phase 465 Summary — Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage

**Phase:** `465` (`REPO-01..04`, `KB-01..02`)
**Milestone:** `v7.9` (`universal-kb-openwiki-v79`)
**Status:** Complete
**Date:** `2026-11-17`

---

## What Was Built

1. **Universal Hybrid Onboarding & Multi-Repo Catalog (`crates/vox-repository`, `crates/vox-project-scaffold`):**
   - Extended `RepoAccessMode` with `ReadOnlyLocal`, `RepoCapability` with `WriteFile` (`#[serde(alias = "write")]`), and repository descriptors with `read_only` and `external_cache_dir`.
   - Added `ActiveRepoContext`, `HarnessOnboardSummary`, `vox_user_home_dir`, `global_library_root[_under]` (`~/.vox/library`), `external_repo_cache_root[_under]` (`~/.vox/repos/<id>`), `save_repo_catalog`, `onboard_repository_harness[_with_home]`, `add_repository_to_catalog[_with_home]`, `remove_repository_from_catalog`, `switch_active_repository[_with_home]`, and `get_active_repository_context[_with_home]`.
   - Updated `repo_query_text`, `repo_query_file`, and `repo_query_history` in `crates/vox-repository/src/repo_catalog/query.rs` to allow both `Local` and `ReadOnlyLocal` repositories.
   - Added `scaffold_vox_project_with_options[_and_home]` and `scaffold_vox_project_under_repo_with_options` in `crates/vox-project-scaffold/src/lib.rs` to wire `--harness` onboarding.

2. **CLI & MCP Parity Without Breaking Command Registry (`crates/vox-cli`, `crates/vox-cli-research`, `crates/vox-orchestrator-mcp`):**
   - Added `--harness` on `vox init`, `--read-only` / `--external-cache-only` on `vox repo init` and `vox catalog add`, `--active` on `vox repo status`, and `--switch` / `--add` / `--remove` / `--read-only` on `vox repo catalog refresh`.
   - Extended `vox_project_init` (`harness`, `read_only`), `vox_repo_status` (`active_context`), and `vox_repo_catalog_refresh` (`action: refresh|init|add|remove|switch`) in `vox-orchestrator-mcp` with matching JSON input schemas.

3. **Strict `(repository_id, corpus_tier)` DB & Search Isolation (`crates/vox-db`, `crates/vox-search`):**
   - Added `kb_scoped_corpus_key` and fixed `VoxDb::apply_library_document` `source_uri` generation to use `format!("{}:{}", doc.corpus, doc.rel_path)` so identically named docs across different repositories/tiers never overwrite each other on `UNIQUE(source_uri)`.
   - Added `library_chunk_by_hash_in_corpus`, `library_chunks_by_hashes_in_corpus`, and `library_document_by_path_in_corpus` on `VoxDb`.
   - Added `CorpusTier`, `KbCorpusManifest` (`vox_default`, `discover_for_repo[_with_id]`, `for_tier`, `corpus_key`, `includes_rel_path`), `walk_corpus_with_manifest`, `resolve_scoped_index_dir`, `index_library_scoped`, `status_scoped`, `search_scoped_with_taxonomy`, and `search_hybrid_scoped_with_taxonomy` in `vox-search`.

## Files Modified

- `crates/vox-repository/src/{lib.rs,repo_catalog/{mod.rs,types.rs,query.rs,catalog.rs,tests.rs}}`
- `crates/vox-project-scaffold/src/lib.rs`
- `crates/vox-cli/src/{lib.rs,cli_dispatch/mod.rs,commands/{init.rs,catalog.rs}}`
- `crates/vox-cli-research/src/{repo_init.rs,repo.rs}`
- `crates/vox-orchestrator-mcp/src/{project_init_tools.rs,repo_catalog_tools.rs,dispatch_domains/vcs_jj.rs,input_schemas/vcs_and_workspace.rs}`
- `crates/vox-db/src/{lib.rs,store/{mod.rs,ops_memory/{mod.rs,library.rs}}}`
- `crates/vox-search/src/{kb_ids.rs,library/{corpus.rs,lexical.rs,mod.rs}}`

## Key Verification Highlights

- `vox-repository`: 56/56 unit + 8/8 integration tests passed.
- `vox-project-scaffold`: 13/13 unit tests passed.
- `vox-db` (`store::ops_memory::library`): 14/14 tests passed.
- `vox-search`: 281/281 non-ignored `--lib` tests passed (1 pre-existing live network test ignored) and 20/20 `kb_library_index` integration tests passed.
- `vox-cli`: `command_catalog_paths_match_baseline` (1/1) and `command_catalog` unit suite (32/32) passed.
- `vox-orchestrator-mcp`: `dispatch::registry_dispatch_tests` passed (3/3).

## Handoff to Phase 466

Ready to plan and execute Phase 466: *Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval* (`KB-03`, `KB-04`, `GRAPH-01..04`).
