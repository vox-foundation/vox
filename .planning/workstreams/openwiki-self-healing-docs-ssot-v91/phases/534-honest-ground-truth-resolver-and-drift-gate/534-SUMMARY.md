---
phase: 534-honest-ground-truth-resolver-and-drift-gate
workstream: openwiki-self-healing-docs-ssot-v91
status: completed
completed_at: "2026-11-19T09:54:16Z"
---

# Phase 534 Summary: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate

## What Changed

- **`crates/vox-doc-pipeline/Cargo.toml`:** Added `vox-graph-reader = { workspace = true }` (`layers.toml` L3 -> L1 compliant).
- **`crates/vox-doc-pipeline/src/claims/extractor.rs`:** Added `is_non_prescriptive_document` to downgrade `/archive/` paths, `category: archive`, and frontmatter `status: legacy|deprecated|archived|research|roadmap|superseded` mentions to `MentionRole::Mentions`.
- **`crates/vox-doc-pipeline/src/claims/resolver.rs`:** Replaced the 58 hardcoded crate aliases with real `crates/*/Cargo.toml` (`[package]` + `[[bin]]`), `catalog.toml`, `.vox/repositories.yaml`, `contracts/retrieval/vox-graph-corpora.v1.yaml`, and `contracts/mcp/tool-registry.canonical.yaml` loading plus `RETIRED_CRATE_REPLACEMENTS`; removed `FilePath` prefix bypasses; filtered `command-registry.yaml` to `vox-cli` active/retired commands plus clap aliases and `cli_parent_groups`; implemented `CrateSymbolIndex` with `vox_graph_reader::ast::extract_ast_in_module` and `pub use` resolution.
- **`crates/vox-doc-pipeline/src/claims/{gate,findings,ledger,mod}.rs` & `pipeline/mod.rs`:** Added `CorpusRatchetFile`, `is_active_corpus_page`, and `evaluate_corpus_ratchet`; replaced hardcoded `2026-10-13T00:00:00Z` timestamp with `Utc::now().to_rfc3339()`; broadened ledger broken-mention tracking to active `status: "current"` `docs/src/` pages and root `README.md`/`AGENTS.md` scanning.
- **`crates/vox-cli-research/src/audit.rs`:** Routed `--write-findings` to `contracts/reports/docs-freshness/{findings,metrics}.v1.json`, added `--seed-corpus-ratchet`, and updated `--check-drift` and `--tighten-baseline` to enforce `corpus-ratchet.v1.json` alongside `baseline.v1.json`.
- **Docs & Freshness Contracts:** Updated `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` (`TRUTH-04`), repaired broken claims in `docs/src/contributors/agent-instruction-architecture.md`, `docs/src/reference/mcp-tool-registry-contract.md`, and `docs/src/reference/cli.md`, attested all 15 previously-drifting `llms.txt` targets, and refreshed `contracts/reports/docs-freshness/{ledger,findings,metrics,corpus-ratchet}.v1.json`.

## Verification Highlights

- `cargo test -p vox-doc-pipeline --lib claims`: `36 passed; 0 failed` (re-run after final edit).
- `cargo test -p vox-doc-pipeline --test openwiki_e2e_integration`: `5 passed; 0 failed`.
- `./target/debug/vox audit docs --check-drift`: PASSED (`970` ratcheted corpus deficits, `0` reader-facing baseline deficits).
- `./target/debug/vox ci docs-reality-audit verify`: OK (`10 claims, 0 findings`).
- `./target/debug/vox ci retired-symbol-check`: OK.
- `./target/debug/vox-doc-pipeline --lint-only` on all 4 edited `docs/src/` pages: exited `0`.
