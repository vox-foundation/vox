# Phase 465 Plan — Universal Multi-Repo Onboarding & Hybrid KB Storage

**Phase:** `465`
**Requirements:** `REPO-01`, `REPO-02`, `REPO-03`, `REPO-04`, `KB-01`, `KB-02`

## Execution Steps

1. **`vox-repository` Catalog & Onboarding Primitives (`REPO-01`, `REPO-02`, `REPO-03`, `REPO-04`):**
   - `crates/vox-repository/src/repo_catalog/types.rs`: Add `RepoAccessMode::ReadOnlyLocal`, `RepoCapability::WriteFile` (with `#[serde(alias = "write")]`), `read_only` & `external_cache_dir` on `RepositoryDescriptor` / `ResolvedRepositoryDescriptor`, plus `ActiveRepoContext` and `HarnessOnboardSummary`.
   - `crates/vox-repository/src/repo_catalog/query.rs`: Permit both `RepoAccessMode::Local` and `RepoAccessMode::ReadOnlyLocal` in cross-repo read/search/history queries.
   - `crates/vox-repository/src/repo_catalog/catalog.rs`, `mod.rs`, `crates/vox-repository/src/lib.rs`, `tests.rs`: Implement `vox_user_home_dir`, `global_library_root[_under]`, `external_repo_cache_root[_under]`, `save_repo_catalog`, `onboard_repository_harness[_with_home]`, `add_repository_to_catalog[_with_home]`, `remove_repository_from_catalog`, `switch_active_repository`, and `get_active_repository_context`, with full unit tests using `TempDir`.

2. **`vox-project-scaffold` Harness Integration (`REPO-01`):**
   - `crates/vox-project-scaffold/src/lib.rs`: Add `scaffold_vox_project_with_options` and `scaffold_vox_project_under_repo_with_options` supporting `with_harness: bool` (calling `vox_repository::onboard_repository_harness`), plus unit test `scaffold_with_harness_creates_vox_and_wiki_layout`.

3. **CLI & MCP Wiring (`REPO-01`, `REPO-02`, `REPO-03`):**
   - `crates/vox-cli/src/{lib.rs,cli_dispatch/mod.rs,commands/init.rs,commands/catalog.rs}` and `crates/vox-cli-research/src/{repo_init.rs,repo.rs}`: Expose `--harness`, `--read-only`, `--external-cache-only`, `--switch`, `--add`, `--remove`, `--sync` on existing Clap subcommand paths without changing command path names.
   - `crates/vox-orchestrator-mcp/src/{project_init_tools.rs,repo_catalog_tools.rs,dispatch_domains/vcs_jj.rs,input_schemas/vcs_and_workspace.rs}`: Expose `harness`/`read_only` on `vox_project_init`, `active_context` on `vox_repo_status`, and `action` (`refresh|init|add|remove|switch`) on `vox_repo_catalog_refresh`.

4. **`vox-db` Corpus Scoping (`KB-02`):**
   - `crates/vox-db/src/store/ops_memory/library.rs`, `crates/vox-db/src/store/mod.rs`, `crates/vox-db/src/lib.rs`: Fix `source_uri` to `format!("{}:{}", doc.corpus, doc.rel_path)`, add `kb_scoped_corpus_key`, add `*_in_corpus` store query helpers, and add unit tests verifying cross-corpus isolation.

5. **`vox-search` Adaptive Corpus Manifest & Scoped Indexing (`KB-01`, `KB-02`):**
   - `crates/vox-search/src/kb_ids.rs` and `crates/vox-search/src/library/{corpus.rs,lexical.rs,mod.rs}`: Add `CorpusTier`, `KbCorpusManifest`, `walk_corpus_with_manifest`, `resolve_scoped_index_dir`, `index_library_scoped`, `status_scoped`, `search_scoped_with_taxonomy`, delegate `index_library` via `KbCorpusManifest::discover_for_repo(repo_root)`, and add unit tests for external non-`docs/src` repos and strict `(repository_id, corpus_tier)` isolation.

6. **Verification & Handoff:**
   - Format all modified `.rs` files with `rustfmt --edition 2024`.
   - Run crate tests for `vox-repository`, `vox-project-scaffold`, `vox-db`, `vox-search`, `vox-cli` baseline, and `vox-orchestrator-mcp`.
   - Record `465-VERIFICATION.md`, `465-REVIEW.md`, `465-SUMMARY.md`, update `.planning/workstreams/universal-kb-openwiki-v79/{REQUIREMENTS.md,ROADMAP.md,STATE.md,state.json}`, and call `enter_plan_mode` for Phase 466.
