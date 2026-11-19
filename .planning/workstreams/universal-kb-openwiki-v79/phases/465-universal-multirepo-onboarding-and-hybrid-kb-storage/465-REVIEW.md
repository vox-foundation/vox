# Phase 465 Code & Security Review

**Phase:** `465` — Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage
**Date:** `2026-11-17`
**Reviewer Scope:** All edits across `vox-repository`, `vox-project-scaffold`, `vox-cli`, `vox-cli-research`, `vox-orchestrator-mcp`, `vox-db`, and `vox-search`.

---

## Findings Summary

| Axis | Status | Assessment |
|---|---|---|
| **Correctness & Backward Compatibility** | Pass | Preserved `index_library` strict `docs/src` check for `KB_LIBRARY_CORPUS` callers (`kb_reindex_missing_root_fails_and_keeps_rows` passes unchanged) while routing scoped/external indexing through `index_library_scoped` + `KbCorpusManifest::discover_for_repo`. Preserved `RepoCapability::Write` backward compatibility via `#[serde(alias = "write")]` on `RepoCapability::WriteFile`. Preserved 3-arg `commands::init::run` signature for existing `play` and `new` subcommands. |
| **Security & Path Traversal Hygiene** | Pass | Repository IDs and corpus tiers pass through `sanitize_repo_id` / `sanitize_path_segment` / `sanitize_corpus_segment` (ASCII alphanumeric, `-`, `_` only) before constructing filesystem paths (`~/.vox/repos/<id>/`, `kb-lexical-v1/repos/<id>/<tier>`) or SQL corpus keys (`kb::repo::<id>::<tier>`). `KbCorpusManifest::includes_rel_path` rejects absolute paths, backslashes, colons, `#`, `.`, and `..` segments, and `walk_corpus_with_manifest` enforces `.follow_links(false)` plus symlink checks on root files. |
| **Zero-Footprint Read-Only Isolation** | Pass | When `read_only` is true, `onboard_repository_harness_with_home` and `add_repository_to_catalog_with_home` write zero files or directories inside the target repository checkout, directing `repositories.yaml` and `cache/{kb,graphify,openwiki}` solely under `<home>/.vox/repos/<id>/`. `active-repository.json` is written only under `<workspace_root>/.vox/`, keeping external repositories completely unmutated and avoiding git status pollution inside `.vox/repositories.yaml`. |
| **Cross-Repo / Cross-Tier Data Isolation** | Pass | Fixed pre-existing collision bug in `VoxDb::apply_library_document` where `source_uri` hardcoded `KB_LIBRARY_CORPUS` instead of `doc.corpus`. Now distinct `(repository_id, corpus_tier)` corpora storing identical relative paths (such as `README.md`) maintain separate `search_documents` rows and hydrate strictly through `library_chunk_by_hash_in_corpus` / `library_chunks_by_hashes_in_corpus` / `search_scoped_with_taxonomy`. |
| **Architecture & Crate Graph Compliance** | Pass | Zero new crate dependencies added to any `Cargo.toml` or `Cargo.lock` (`vox-search` uses existing `blake3` and `vox-db` dependencies cleanly without introducing a `vox-repository` edge). Existing Clap subcommand trees and MCP static dispatch registries stay 100% intact. |
| **Test & Edition Discipline** | Pass | No `std::env::set_var` calls in unit tests (compliant with Edition 2024 `unsafe_code` lint via explicit `*_with_home(..., home_dir: &Path)` test seams). All 25 modified `.rs` files formatted via `rustfmt --edition 2024`. |

## Minor Observations / Follow-ups for Later Phases

1. `vox init --harness` and `vox repo init` are now unified on `vox_repository::onboard_repository_harness`; Phase 467 (`WIKI-01..02`) will upgrade the starter `docs/wiki/README.md` `<!-- openwiki:auto:start id="repo-overview" -->` fence into the full 5-layer polyglot OpenWiki generator.
2. `vox-orchestrator-mcp`'s unfiltered `--lib` suite continues to show 5 environment-key failures in `llm_bridge::{local_health,model_route_policy}` when run offline without `OPENROUTER_API_KEY` / `GEMINI_API_KEY` / local Ollama (pre-existing environment precondition outside Phase 465 scope).
