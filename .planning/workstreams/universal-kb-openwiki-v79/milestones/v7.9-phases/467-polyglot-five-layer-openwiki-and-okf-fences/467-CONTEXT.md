# Phase 467 Context: Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions

**Workstream:** `universal-kb-openwiki-v79`
**Phase:** `467`
**Requirements:** `WIKI-01`, `WIKI-02`
**Depends on:** Phases 465 (`REPO-01..04`, `KB-01..02`) and 466 (`KB-03..04`, `GRAPH-01..04`)

## Architectural Decisions (Locked)
1. **Polyglot 5-Layer OpenWiki (`WIKI-01`, `WIKI-02`):**
   - `vox-doc-pipeline::pipeline::openwiki` previously required `docs/src/architecture/layers.toml` and Rust-only crate trees.
   - Phase 467 generalizes OpenWiki with `openwiki_polyglot.rs` so any repository (Cargo/Rust with or without `layers.toml`, TypeScript/Node, Python, Go, `.vox`, SQL, or mixed polyglot) synthesizes OKF-compliant pages under `docs/wiki/`:
     1. `docs/wiki/README.md` — Repository OpenWiki Hub Index (`domain: documentation`, `topic: doc-pipeline`)
     2. `docs/wiki/architecture-topology.md` — Layer 1: Architecture & Module/Community Topology (`domain: knowledge`, `topic: search-retrieval`)
     3. `docs/wiki/interfaces-and-apis.md` — Layer 2: CLI / HTTP / MCP / API Interfaces (`domain: platform`, `topic: cli-surface`)
     4. `docs/wiki/data-models-and-schemas.md` — Layer 3: Data Models & SQL / VoxDb Schemas (`domain: data`, `topic: database`)
     5. `docs/wiki/gui-surfaces-and-flows.md` — Layer 4: GUI / Frontend Surfaces & Flows (`domain: gui`, `topic: desktop-app`)
     6. `docs/wiki/workflows-and-runbooks.md` — Layer 5: Workflows, Actors & Runbooks (`domain: runtime`, `topic: durable-workflows`)
   - Every diagram uses validated ```` ```vox-diagram ```` (`vox_diagram::check`), emitting `0` legacy Mermaid blocks.
2. **Non-Destructive OKF Deterministic Fences (`WIKI-02`):**
   - Machine-generated sections live inside `<!-- openwiki:auto:start id="<region_id>" -->` ... `<!-- openwiki:auto:end id="<region_id>" -->` (also accepting closing `<!-- openwiki:auto:end -->`).
   - Re-running OpenWiki build merges updated deterministic bodies in-place inside fences while preserving 100% of surrounding human prose, custom headings, and engineering notes.
3. **External Repo `--lint-only` Support & Vox Guardrail Preservation (`WIKI-02`):**
   - On external repositories (`!root.join("docs/src").exists()`), `vox-doc-pipeline --lint-only` discovers `docs/wiki/` (or `docs/`), validates `docs/wiki/**/*.md` against OKF facets via `kb-taxonomy.v1.yaml` (embedded fallback when `contracts/documentation/` does not exist in external repos), checks `vox-diagram` fences, and enforces `0` legacy Mermaid blocks.
   - On Vox (`root.join("docs/src").exists()`), `FacetCtx::load(root)` stays strictly fail-closed on `contracts/documentation/` and `generate_subsystem_hubs` stays 100% backward-compatible.
4. **CLI & MCP Parity (`WIKI-01`):**
   - Preserve exact Clap subcommand paths in `command_catalog_paths_baseline.txt` (`ci/wiki-sync`, `repo`, etc.) and tool names in `tool-registry.canonical.yaml`.
   - Expose polyglot OpenWiki `{init, build, status, diff}` via `vox ci wiki-sync` flags (`--polyglot`, `--init`, `--status`, `--diff`, `--repo-root`, `--read-only`, `--json`), `vox repo --wiki <ACTION> [--write]`, and `vox_repo_catalog_refresh` actions (`wiki_init`, `wiki_build`, `wiki_status`, `wiki_diff`) + `vox_repo_status` `openwiki` summary.
