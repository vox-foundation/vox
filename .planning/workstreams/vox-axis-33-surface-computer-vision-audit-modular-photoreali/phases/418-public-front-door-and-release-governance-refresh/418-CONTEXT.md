---
phase: 418
title: "Public Front-Door & Release Governance Refresh"
status: ready
requirements:
  - HYGIENE-DOCS-01
  - HYGIENE-DOCS-02
  - HYGIENE-DOCS-03
  - HYGIENE-DOCS-04
---

# Phase 418 Context: Public Front-Door & Release Governance Refresh (`HYGIENE-DOCS-01..04`)

## Architectural Ground Truth (`0.6.0` / `v7.1`)
1. **`RELEASE_NOTES.md` (`HYGIENE-DOCS-01`):** Currently frozen at Feb 2026 (`2026-02-16` and `2026-02-22`) referencing deleted pre-consolidation crates (`vox-parser`, `vox-typeck`, `vox-codegen-rust`, `vox-codegen-ts`, `vox-hir`, `vox-integration-tests`). Must be refreshed to `0.6.0` (`v6.7–v7.1` Public Readiness Release) matching `README.md` truth:
   - Unified compiler & codegen pipeline (`vox-compiler` + `vox-codegen`, Rust Axum/Turso backend + React/TanStack Start islands).
   - Capability-sandboxed interpreter (`vox run --interp --caps`) and `.vox` project automation.
   - Local-first `vox-db` (Turso/libSQL schema baseline v97+) + Tantivy 0.22 + RRF `vox kb` Library index & SKOS-lite facets.
   - Multi-agent `vox-orchestrator` + `vox-orchestrator-mcp` + Vox Axis workbench (32 registered surfaces, Sunlit Imperial Carrara White & Gold + Dark Slate dual theme).
   - `vox mens` QLoRA fine-tuning, evaluation, and serving (`mens-candle-metal`, `mens-candle-cuda`, `mens-candle-cpu`).
   - 33 standalone `crates.io` packages in `crates/_public.toml` + zero-red gate convergence (`v7.1`).
2. **`CHANGELOG.md` & `CONTRIBUTING.md` (`HYGIENE-DOCS-02`):**
   - `CHANGELOG.md` line 6 carries a forbidden manual `last_updated: "2026-05-26"` frontmatter key (violates `AGENTS.md §Authored Markdown Frontmatter`); `[Unreleased]` must also record the `v6.7–v7.1` milestones (keystone compile firewalls, 33-crate `crates.io` Wave 2/3, hybrid 4-repo federation + `vox-ops`, Sunlit Roman GUI & `voxlang.org` portal, hardcoded abs-dir guard, zero-red CI/pre-push & `toestub`/`vox-drift-check` signal restoration, feature-independent test hardening).
   - `CONTRIBUTING.md` line 17 instructs contributors to run `cargo check --workspace` (slow and discouraged vs scoped `cargo check -p <crate>`); its companion binaries table should include `vox-doc-pipeline`, `toestub` (`vox-code-audit`), `vox-drift-check`, and `vox-arch-check`, and reference `.vox` automation (`vox run`).
3. **Doc Lint & Standalone Crate Verification (`HYGIENE-DOCS-03`):**
   - Verify `vox-doc-pipeline --lint-only` exits `0` across modified root docs and full `docs/src/`.
   - Verify `crates/_public.toml` (`33` crates) distribution parity offline (`voxup` distribution parity test + manifest validation).
4. **End-to-End Readiness Scorecard (`HYGIENE-DOCS-04`):**
   - Run all local & CI gates (`vox ci pre-push`, `vox-arch-check`, `vox ci fan-in-budget`, `vox ci secret-env-guard`, `vox ci crate-edges`, `vox ci ssot-drift`, `vox ci cross-repo-parity`, `vox ci ignored-test-age --mode enforce`, `vox-drift-check . --severity warning`, `toestub crates --min-severity error`) and emit `contracts/reports/v71-public-readiness-scorecard.v1.json`.
