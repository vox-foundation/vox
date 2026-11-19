# Phase 467 Plan: Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions

**Phase:** `467`
**Requirements:** `WIKI-01`, `WIKI-02`

## Execution Steps

1. **Step 0 (Hygiene):**
   - Remove the literal home-path mention in `.planning/workstreams/universal-kb-openwiki-v79/phases/466-polyglot-graphify-cross-edges-and-ppr-retrieval/466-PLAN.md` so path hygiene scans stay at 0 hits.

2. **Step 2 (`WIKI-01`, `WIKI-02` in `crates/vox-doc-pipeline/src/pipeline/`):**
   - Create `openwiki_polyglot.rs` implementing:
     - OKF fence parser and non-destructive merger (`extract_openwiki_auto_regions`, `merge_openwiki_auto_regions`) supporting both `<!-- openwiki:auto:end -->` and `<!-- openwiki:auto:end id="..." -->`.
     - Polyglot AST/regex/file extractor (`extract_polyglot_wiki_snapshot`) covering Rust, TypeScript/JS, Python, Go, Vox, SQL/DDL, GUI surfaces, and CI/workflow definitions with 1-based `#Lstart-Lend` source anchors.
     - Validated `vox-diagram` renderers for the hub overview and all 5 system layers (`0` legacy Mermaid).
     - `openwiki_init`, `openwiki_build`, `openwiki_status`, and `openwiki_diff` writing `docs/wiki/{README,architecture-topology,interfaces-and-apis,data-models-and-schemas,gui-surfaces-and-flows,workflows-and-runbooks}.md` plus cache manifest `.vox/cache/openwiki/manifest.v1.json` (respecting `output_override` for read-only mode).
   - Export `openwiki_polyglot` in `pipeline/mod.rs` and re-export core helpers in `pipeline/openwiki.rs`.
   - Extend `pipeline/{mod.rs,facets.rs,lint.rs,mermaid_gate.rs}` so `vox-doc-pipeline --lint-only` supports `docs/wiki/**/*.md` on both Vox and external repositories without weakening fail-closed Vox contract checks.

3. **Step 3 (`WIKI-01` CLI, Onboarding & MCP Wiring):**
   - Upgrade `render_wiki_readme` in `crates/vox-repository/src/repo_catalog/catalog.rs` to include valid OKF YAML frontmatter and `<!-- kb:summary -->`.
   - Extend `CiCmd::WikiSync` (`crates/vox-cli-ci/src/cmd_enums.rs`) and dispatch (`crates/vox-cli/src/commands/ci/run_body.rs`) with `--polyglot`, `--init`, `--status`, `--diff`, `--repo-root`, `--read-only`, `--json`.
   - Add `--wiki <ACTION>` (`init | build | status | diff`) + `--write` to `vox repo` (`crates/vox-cli/src/lib.rs`, `crates/vox-cli/src/cli_dispatch/mod.rs`, `crates/vox-cli-research/src/repo.rs`).
   - Add `wiki_init | wiki_build | wiki_status | wiki_diff` actions in `vox_repo_catalog_refresh` and `openwiki` summary in `vox_repo_status` (`crates/vox-orchestrator-mcp/src/{repo_catalog_tools.rs,input_schemas/vcs_and_workspace.rs}`).

4. **Step 4 (Verification & Artifacts):**
   - Run `rustfmt --edition 2024`, `vox-doc-pipeline`, `vox-repository`, `vox-cli --test command_catalog_paths_baseline`, and `vox-orchestrator-mcp --lib dispatch::registry_dispatch_tests`.
   - Record `467-VERIFICATION.md`, `467-REVIEW.md`, `467-SUMMARY.md`, and update `v7.9` state/roadmap/requirements to Phase `468`.
