# Phase 465 Context — Universal Multi-Repo Onboarding & Hybrid KB Storage

**Milestone:** `v7.9` — Universal KB, Graphify, Self-Healing OpenWiki & Multi-Repo GUI
**Workstream:** `universal-kb-openwiki-v79`
**Requirements:** `REPO-01`, `REPO-02`, `REPO-03`, `REPO-04`, `KB-01`, `KB-02`
**Status:** Executing

## Locked Decisions

1. **Two-Tier Hybrid Storage Topology (`KB-01`, `REPO-02`, `REPO-04`):**
   - **Committed repo-local harness & human-readable tier:** `VOX.md`, `AGENTS.md`, `.voxignore`, `.well-known/llms.txt`, `docs/wiki/`, and `.vox/repositories.yaml`.
   - **Gitignored repo-local machine cache:** `.vox/cache/{kb,graphify,openwiki}/` plus `.vox/active-repository.json`.
   - **External zero-footprint read-only mode (`--read-only` / `--external-cache-only`):** Never writes files inside the target external repository; redirects cache and catalog artifacts to `~/.vox/repos/<repository_id>/` via `vox_user_home_dir()`.
   - **Global cross-repo personal library (`KB-01`):** Anchored at `~/.vox/library/`.

2. **Command Catalog Stability Constraint (`REPO-01`, `REPO-03`):**
   - Preserve the exact existing Clap command path tree verified by `every_compiled_clap_path_is_in_command_registry` and `command_catalog_paths_baseline`: `vox init`, `vox repo {init, status, catalog {list, refresh}, query}`, `vox catalog {list, add, remove, focus}`.
   - Expose all Phase 465 capabilities via flags and sub-options on these existing commands and existing MCP tools (`vox_project_init`, `vox_repo_status`, `vox_repo_catalog_list`, `vox_repo_catalog_refresh`).

3. **Strict `(repository_id, corpus_tier)` Retrieval Isolation (`KB-02`):**
   - Fix `apply_library_document` in `vox-db` so `source_uri` uses `format!("{}:{}", doc.corpus, doc.rel_path)` instead of hardcoding `KB_LIBRARY_CORPUS`.
   - Scope KB corpus keys cleanly via `kb_scoped_corpus_key(repository_id, corpus_tier)` (`kb::repo::<id>::<tier>`) and expose `*_in_corpus` query methods on `VoxDb`.
   - Introduce `CorpusTier` (`DocsSrc`, `Wiki`, `RootDocs`, `ExternalCustom`, `GlobalLibrary`) and `KbCorpusManifest::discover_for_repo(repo_root)` in `vox-search` so external repositories without `docs/src` discover `docs/wiki`, `docs`, `README.md`, `VOX.md`, `AGENTS.md`, `ARCHITECTURE.md` automatically.
   - Keep `index_library` backward-compatible by delegating to `KbCorpusManifest::discover_for_repo(repo_root)`.

4. **Safe Editions & Tests:**
   - Avoid `std::env::set_var` in unit tests under Edition 2024 `unsafe_code` lint by providing `*_with_home(..., home_dir: &Path)` helpers accepting `TempDir` paths.
   - Never hardcode user-home absolute paths in docs, code, or planning artifacts.
