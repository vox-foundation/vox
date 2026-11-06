---
phase: 190-zero-deficit-freshness-baseline-ratchet-duplicate-cluster-consolidation
plan: 01
subsystem: docs-freshness
tags: [freshness-gate, baseline-ratchet, consolidation, redirects]
status: completed
---

# Phase 190 Summary: Zero-Deficit Freshness Baseline Ratchet & Duplicate Cluster Consolidation

## Requirements Completed (`OPENWIKI-FRESH-01..04`)
- **`OPENWIKI-FRESH-01`**: Excluded machine-generated (`*.generated.md`) and changelog (`/changelog.md`) pages from reader-facing drift deficits in `crates/vox-doc-pipeline/src/claims/gate.rs` (matching `ledger.rs`), burning `contracts/reports/docs-freshness/baseline.v1.json` down from `7` deficits to `0` (`[]`).
- **`OPENWIKI-FRESH-02`**: Wired `--consolidate-duplicates` in `vox audit docs` (`crates/vox-cli/src/commands/audit.rs`) to `plan_duplicate_cluster_consolidation` and `render_redirects_block`, adding the 301 rule in `docs-astro/public/_redirects`.
- **`OPENWIKI-FRESH-03`**: Ensured `generate_subsystem_hubs` and `crate_api_docs` emit `code_anchors` and valid frontmatter for freshness ledger tracking.
- **`OPENWIKI-FRESH-04`**: Verified `evaluate_drift_gate` passes with `0` active grandfathered deficits and `0` expired suppressions.

## Verification
- `cargo test -p vox-doc-pipeline --test openwiki_e2e_integration` (`phase_190_zero_deficit_baseline_and_duplicate_consolidation ... ok`).
