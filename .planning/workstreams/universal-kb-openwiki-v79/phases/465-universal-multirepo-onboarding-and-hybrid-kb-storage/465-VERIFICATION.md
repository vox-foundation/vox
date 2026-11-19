# Phase 465 Verification — Universal Multi-Repo Onboarding & Hybrid KB Storage

**Phase:** `465`
**Workstream:** `universal-kb-openwiki-v79`
**Requirements Covered:** `REPO-01`, `REPO-02`, `REPO-03`, `REPO-04`, `KB-01`, `KB-02`
**Date:** `2026-11-17`
**Status:** Passed

---

## Automated Verification Runs (Observed After Final Edits)

| Crate / Target | Command | Scope & Result |
|---|---|---|
| `vox-repository` | `timeout 900s cargo test -p vox-repository` | **Passed**: 56 unit tests passed (0 failed, 0 skipped), including 4 new tests (`onboard_repository_harness_creates_expected_layout_and_is_idempotent`, `onboard_repository_harness_read_only_leaves_external_repo_untouched`, `catalog_add_switch_and_remove_persist_active_context`, `text_query_groups_hits_by_repository`); plus 8/8 `federation_tests` integration tests passed |
| `vox-project-scaffold` | `timeout 900s cargo test -p vox-project-scaffold` | **Passed**: 13 unit tests passed (0 failed, 0 skipped), including `scaffold_with_harness_creates_vox_and_wiki_layout` |
| `vox-db` | `timeout 900s cargo test -p vox-db --lib store::ops_memory::library` | **Passed**: 14 library store tests passed (0 failed; 301 unrelated lib tests filtered out), including `scoped_corpora_isolate_same_rel_path_and_chunks` |
| `vox-search` (unit) | `timeout 900s cargo test -p vox-search --lib` | **Passed**: 281 unit tests passed (0 failed, 1 pre-existing `#[ignore]` live network probe `deep_lane_probe_searxng_vs_tavily_subquery`), including `index_external_repo_without_docs_src_discovers_wiki_and_root_docs` and `scoped_corpora_prevent_cross_repo_and_cross_tier_leakage` |
| `vox-search` (`library::tests`) | `timeout 900s cargo test -p vox-search --lib library::tests` | **Passed**: 6 tests passed (0 failed, 0 skipped; 276 filtered out) |
| `vox-search` (integration) | `timeout 900s cargo test -p vox-search --test kb_library_index` | **Passed**: 20 integration tests passed (0 failed, 0 skipped), including `kb_reindex_missing_root_fails_and_keeps_rows` and `kb_library_tier_follows_the_commit_gate_per_file` |
| `vox-cli` (baseline) | `timeout 900s cargo test -p vox-cli --test command_catalog_paths_baseline` | **Passed**: 1 test passed (`command_catalog_paths_match_baseline`) |
| `vox-cli` (`command_catalog`) | `timeout 900s cargo test -p vox-cli --lib command_catalog` | **Passed**: 32 tests passed (0 failed; 602 filtered out), including `every_compiled_clap_path_is_in_command_registry` |
| `vox-orchestrator-mcp` (unfiltered `--lib`) | `timeout 900s cargo test -p vox-orchestrator-mcp --lib` | **Partial (Env-gated pre-existing)**: 1,060 passed (including `kb_tools::library::tests::*` and `dispatch::registry_dispatch_tests::*`), 5 failed in `llm_bridge::{local_health,model_route_policy}` because `OPENROUTER_API_KEY` / `GEMINI_API_KEY` / live Ollama are unset in this offline test environment |
| `vox-orchestrator-mcp` (`project_init`) | `timeout 900s cargo test -p vox-orchestrator-mcp --lib project_init` | Compiles cleanly with 0 warnings on modified targets (`0` tests matched `project_init` substring filter) |
| `vox-orchestrator-mcp` (`dispatch::registry_dispatch_tests`) | `timeout 900s cargo test -p vox-orchestrator-mcp --lib dispatch::registry_dispatch_tests` | **Passed**: 3/3 passed (`tool_registry_names_are_unique`, `yaml_registry_tools_have_dispatch_match_arms`, `every_registry_tool_has_static_dispatch`) |

---

## Requirement Traceability Matrix

| Req ID | Requirement Summary | Verification Evidence |
|---|---|---|
| `REPO-01` | Repo onboarding (`vox init --harness`, `vox repo init`, `vox catalog`, `vox_project_init`, `vox_repo_*`) generates `.vox/repositories.yaml`, `VOX.md`, `AGENTS.md`, `.voxignore`, `.well-known/llms.txt`, `docs/wiki/`, and `.vox/cache/{kb,graphify,openwiki}/` | `onboard_repository_harness_creates_expected_layout_and_is_idempotent`, `scaffold_with_harness_creates_vox_and_wiki_layout`, `command_catalog_paths_match_baseline`, `every_compiled_clap_path_is_in_command_registry`, `every_registry_tool_has_static_dispatch` |
| `REPO-02` | Active repository context persisted (`active-repository.json` + MCP `vox_repo_status` / `vox_repo_catalog_refresh` action `switch`) | `catalog_add_switch_and_remove_persist_active_context` (`switch_active_repository_with_home`, `get_active_repository_context_with_home`) |
| `REPO-03` | Two-Tier Hybrid repo-local docs + gitignored `.vox/cache/` + global `~/.vox/library/` directory layout | `onboard_repository_harness_creates_expected_layout_and_is_idempotent` checks `.vox/.gitignore`, `.vox/cache/{kb,graphify,openwiki}`, and `<home>/.vox/library` |
| `REPO-04` | Zero-footprint external `--read-only` / `RepoAccessMode::ReadOnlyLocal` indexing under `~/.vox/repos/<id>/` with 0 writes to target checkout | `onboard_repository_harness_read_only_leaves_external_repo_untouched`, `catalog_add_switch_and_remove_persist_active_context` |
| `KB-01` | Dynamic `KbCorpusManifest` discovers `docs/src/`, `docs/wiki/`, `docs/`, `VOX.md`, `AGENTS.md`, `README.md`, `ARCHITECTURE.md`, `CLAUDE.md` while keeping Vox `docs/src` golden behavior intact | `index_external_repo_without_docs_src_discovers_wiki_and_root_docs`, 20/20 `kb_library_index` integration tests |
| `KB-02` | Strict `(repository_id, corpus_tier)` DB (`search_documents` unique `source_uri` prefixing `corpus:rel_path`, `*_in_corpus` hydration) & Tantivy (`resolve_scoped_index_dir`) isolation | `scoped_corpora_isolate_same_rel_path_and_chunks` (`vox-db`), `scoped_corpora_prevent_cross_repo_and_cross_tier_leakage` (`vox-search`) |
