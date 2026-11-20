---
phase: 536-uncapped-polyglot-openwiki-and-tier1-ast-heal
workstream: openwiki-self-healing-docs-ssot-v91
status: partial
verified_at: "2026-11-19T12:04:01Z"
requirements:
  - WIKI-UNIFY-01
  - WIKI-UNIFY-02
  - WIKI-UNIFY-04
open_requirements:
  - WIKI-UNIFY-03
---

# Phase 536 Verification Report (PARTIAL)

## Final re-verification (2026-11-19T12:43Z, after all edits)

| Check | Result |
|---|---|
| `cargo test -p vox-doc-pipeline --lib openwiki_polyglot` | `6 passed; 0 failed` (includes `sql_foreign_keys_are_annotated_via_graph_reader`) |
| `cargo test -p vox-orchestrator-mcp --lib repo_catalog_tools` | `3 passed; 0 failed` |
| `cargo test -p vox-orchestrator-mcp --lib kb_tools` | `27 passed; 0 failed` |
| `cargo build -p vox-cli` | exit 0 |
| Live `vox ci wiki-sync --polyglot --diff --json` on Vox (after excluding `patches`/`vendor`/`third_party`) | 295 pages, 5,566,020 bytes total, largest 386,714 bytes (`interfaces-and-apis/crates_vox-orchestrator.md`) |

Verdict update: `WIKI-UNIFY-03` is PARTIAL — SQL foreign-key relations now come from `vox_graph_reader::polyglot::extract_sql_in_module`; Rust/TS/Python/Go symbol spans still come from the existing in-crate extractors because `vox-graph-reader` nodes carry no line spans.

Still not done: `--init` on Vox (pages not written, not linted, drift gate not run on them); `vox-arch-check` not re-run after the split; new tests were written after the implementation.

## Original (earlier) verification report

## Observed checks

| Check | Command | Result |
|---|---|---|
| OpenWiki lib tests | `timeout 900s cargo test -p vox-doc-pipeline --lib openwiki` | `16 passed; 0 failed` |
| Full doc-pipeline suite | `timeout 900s cargo test -p vox-doc-pipeline` | lib `150 passed; 0 failed`; `docs_production_audit_e2e` 4 passed; `fence_issues_public` 1; `frontmatter_governance_parity` 1; `openwiki_e2e_integration` 5 passed (re-run after the final edit) |
| MCP wiki tests | `timeout 900s cargo test -p vox-orchestrator-mcp --lib repo_catalog_tools` | `3 passed; 0 failed` (run before the sub-page split and the `patches/vendor/third_party` walk exclusion; NOT re-run after) |
| Polyglot flywheel | `timeout 900s cargo test -p vox-cli --test v79_polyglot_flywheel_proof` | `1 passed; 0 failed` (run after the split) |
| Layers | `timeout 900s cargo run -p vox-arch-check` | exit 0 (LoC-delta and stale evidence-ledger warnings only; run before the split) |
| Live read-only diff on Vox | `./target/debug/vox ci wiki-sync --polyglot --diff` | resolves to `docs/src/openwiki`; `ls docs/wiki` fails (not created) |
| Live diff after split | `... --diff --json` (before the `patches/vendor/third_party` exclusion) | 297 pages, ~6.16 MB total, largest ~456 KB (`interfaces-and-apis/patches.md`); unsplit Layer 2 had been 3,792,170 bytes |
| Cargo.lock | `git diff --numstat -- Cargo.lock` | `3 0` (dependency-list lines only) |

## Not verified / not done
- `vox ci wiki-sync --init` was NOT run on Vox; generated pages were not written, linted or fed to `vox audit docs --check-drift`.
- Live diff was not re-run after excluding `patches`, `vendor`, `third_party`.
- `vox-orchestrator-mcp` kb_tools tests and `vox-arch-check` were not re-run after the split.
- New tests were written after the implementation (not observed failing first).
- `WIKI-UNIFY-03`: the planned supplementary `vox-graph-reader` SQL/Go extraction was not implemented.
- `WIKI-UNIFY-02`: grouping is by module/top-level directory, not Leiden clustering. Several per-module pages remain in the hundreds of KB (for example `crates/vox-orchestrator`).
- Scaffold/harness text that creates `docs/wiki/` for new external repos (`vox-project-scaffold`, `vox-repository`, `vox init`) is unchanged; deferred to Phase 540.
- GUI `OpenWikiHealStudio.tsx` still builds `docs/wiki/${pg.rel_path}`; the MCP status now supplies `doc_path` per page for a later GUI change (`crates/vox-gui/` was not touched).
- Sub-pages are not pruned if a module disappears.
