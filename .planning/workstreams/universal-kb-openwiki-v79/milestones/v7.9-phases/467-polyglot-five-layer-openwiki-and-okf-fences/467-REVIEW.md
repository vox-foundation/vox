# Phase 467 Review — Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions

**Requirements Reviewed:** `WIKI-01`, `WIKI-02`
**Outcome:** Approved — zero warnings, zero new dependencies, zero Clap or MCP registry drift.

## Findings & Architecture Alignment
1. **Repo-Agnostic 5-Layer Polyglot Synthesis (`openwiki_polyglot.rs`):**
   - Cleanly decoupled polyglot OpenWiki generation from Vox-specific `docs/src/architecture/layers.toml` while preserving `generate_subsystem_hubs` unchanged on Vox.
   - Extracts modules/manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `go.mod`, `Vox.toml`, optional `layers.toml`), public API/CLI/HTTP routes, SQL DDL + typed data models, TSX/JSX/Vox GUI surfaces, and `.github/workflows` / `scripts/*.vox` / actor workflows into 6 OKF pages under `docs/wiki/`.
   - Uses existing `vox_doc_pipeline::claims::hasher::hash_normalized_code` rather than importing raw hashing crates (`AGENTS.md §Cryptography Policy`), and emits validated `vox-diagram` fences (`vox_diagram::check`) with zero legacy Mermaid blocks.
2. **Non-Destructive Fence Preservation (`merge_openwiki_auto_regions`):**
   - Replaces interior content of `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->` regions in-place while leaving surrounding YAML frontmatter, `<!-- kb:summary -->`, and human prose intact; appends any newly introduced deterministic sections rather than clobbering manual sections.
3. **Strict Fail-Closed Guardrails Preserved on Vox (`facets.rs`, `mermaid_gate.rs`, `pipeline/mod.rs`):**
   - `FacetCtx::load_or_embedded` keeps `docs/src/` strictly fail-closed on local `contracts/documentation/` (`missing_contract_is_one_hard_error` remains passing) while enabling `vox-doc-pipeline --lint-only` on external repositories (`docs/wiki/`).
   - `mermaid_gate::docs(root)` scans `docs/wiki/` when present so zero legacy Mermaid blocks are permitted in external or onboarded wikis.
4. **CLI & MCP Parity:**
   - Added `--polyglot`, `--init`, `--status`, `--diff`, `--repo-root`, `--read-only`, `--json` flags on `vox ci wiki-sync` and `--wiki {init,build,status,diff}` + `--write` on `vox repo`, keeping `command_catalog_paths_baseline.txt` and `contracts/cli/command-registry.yaml` unchanged.
   - Extended `vox_repo_catalog_refresh` (`wiki_init`, `wiki_build`, `wiki_status`, `wiki_diff`) and `vox_repo_status` (`openwiki` summary) in `vox-orchestrator-mcp` using existing workspace dependencies.
