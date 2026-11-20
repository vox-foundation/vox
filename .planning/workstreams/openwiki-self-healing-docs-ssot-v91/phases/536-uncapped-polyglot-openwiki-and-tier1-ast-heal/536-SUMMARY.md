---
phase: 536-uncapped-polyglot-openwiki-and-tier1-ast-heal
workstream: openwiki-self-healing-docs-ssot-v91
status: partial
---

# Phase 536 Summary (PARTIAL)

## What changed
- `crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs`: `openwiki_pages_rel`/`openwiki_pages_dir` (precedence: existing `docs/wiki` → `docs/src/openwiki` → `docs/openwiki`); removed `MAX_SCAN_FILES`, `MAX_ENTRIES_PER_LAYER`, the `layers.toml` `take(40)` and the table `take()` limits; `ignore::WalkBuilder` walk (skips `.git`, `.vox`, `target`, `node_modules`, `dist`, `build`, `.venv`, `__pycache__`, `.next`, `coverage`, `vendor`, `third_party`, `patches`); tables grouped per module; layers above 400 rows become a rollup index plus per-module sub-pages (`render_symbol_section`); page-depth-aware source anchors (`render_polyglot_openwiki_pages_at`); `total_pages` now counts the real specs.
- `openwiki_heal.rs`: blast radius and Tier-1 heal use the discovered pages root (and `docs/src`, `docs`, `docs/adr`) instead of hardcoded `docs/wiki`.
- `openwiki.rs`: re-exports the new helpers.
- `crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs`: the 1-node placeholder generator is gone; `wiki_init|build|status|diff|sync|heal_tier1` call the shared `vox-doc-pipeline` engine and keep the status keys the Axis studio reads (adds `doc_path`, `has_changes`, `report`). Adds the `vox-doc-pipeline` dependency (also `ignore` for the pipeline); `Cargo.lock` gained 3 dependency lines.
- `crates/vox-cli-ci/src/cmd_enums.rs`: `--polyglot` / `--init` help text no longer says `docs/wiki/`.
- Tests: new `pages_dir_precedence_docs_wiki_then_docs_src_then_docs`, `uncapped_snapshot_covers_every_symbol_in_large_synthetic_repo`, `docs_src_repo_builds_under_docs_src_openwiki_and_never_creates_docs_wiki`, `mcp_wiki_build_on_docs_src_repo_targets_docs_src_and_creates_no_docs_wiki`; existing fixtures updated to derive the pages root.

## Requirement status
- `WIKI-UNIFY-01`: met (MCP delegates to the shared engine; verified by the MCP tests; no `docs/wiki` on Vox in the live diff).
- `WIKI-UNIFY-02`: met with deviation (caps removed; module/directory rollups instead of Leiden; some pages still large).
- `WIKI-UNIFY-03`: NOT met (no supplementary `vox-graph-reader` extraction; Rust spans still come from the existing extractors).
- `WIKI-UNIFY-04`: met for Tier-1 heal following the discovered root; verified by the doc-pipeline Tier-1 test and the flywheel test.

See `536-VERIFICATION.md` for exact outputs and what was not re-run.
