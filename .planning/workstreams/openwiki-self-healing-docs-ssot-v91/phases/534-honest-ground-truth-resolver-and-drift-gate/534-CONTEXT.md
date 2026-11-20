---
phase: 534-honest-ground-truth-resolver-and-drift-gate
workstream: openwiki-self-healing-docs-ssot-v91
milestone: v9.1
status: executing
---

# Phase 534 Context: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate

## Goal

Eliminate false-green blind spots in `vox-doc-pipeline` claims extraction/resolution/gating and `vox audit docs` so that:
- Retired crates/commands are accurately classified as `Retired` with canonical replacements (without breaking `vox ci retired-symbol-check` on uncleaned corpus files prior to Phase 539).
- Deleted/missing workspace file paths, multi-token CLI subcommands, and Rust module/symbol paths are verified against real workspace files, CLI catalogs, and `vox-graph-reader` AST exports rather than prefix-bypass heuristics or hardcoded alias lists.
- Non-reader-facing `status: "current"` documentation (`architecture/`, `adr/`, `contributors/`, `ci/`, `api/`, root guides) is tracked under a shrink-only corpus ratchet (`contracts/reports/docs-freshness/corpus-ratchet.v1.json`) while keeping `baseline.v1.json` at zero deficits for reader-facing pages and `llms.txt` targets.
- `vox audit docs --write-findings` outputs live RFC3339 timestamps and writes directly to `contracts/reports/docs-freshness/{findings,metrics}.v1.json` (preserving `contracts/reports/docs-reality-audit/` for `vox ci docs-reality-audit verify`).
- `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` reflects `docs/src/` (`voxlang.org`) as Vox's canonical OpenWiki documentation database and documents the honest ground-truth resolver and full-corpus ratchet.

## Requirements Covered

- **TRUTH-01:** Honest crate, bundle, repository, and retired-symbol ground truth (`Cargo.toml`, `catalog.toml`, `.vox/repositories.yaml`, `RETIRED_CRATE_REPLACEMENTS`).
- **TRUTH-02:** Real verification for `FilePath`, multi-token `CliCommand`, and `RustSymbol` (using `vox-graph-reader` AST/crate source inspection) plus frontmatter `status`/archive role downgrade in `extractor.rs`.
- **TRUTH-03:** Full active-corpus shrink-only ratchet (`corpus-ratchet.v1.json`), live `generated_at` timestamps, unified `docs-freshness` output path for `--write-findings`, and honest `ledger.v1.json` staleness/deficit accounting.
- **TRUTH-04:** Updated `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` SSOT and refreshed committed `contracts/reports/docs-freshness/{ledger,findings,metrics,corpus-ratchet}.v1.json`.

## Locked Constraints

- Zero separate `docs/wiki/` directory on Vox; `docs/src/` (`voxlang.org` Astro corpus) is canonical.
- Preserve `baseline.v1.json` as `[]` for `is_reader_facing_page` and keep `llms.txt` targets `verified` in `ledger.v1.json` (`docs-astro/tests/build-output/llms.spec.ts`).
- Preserve `contracts/reports/docs-reality-audit/{inventory,findings,metrics}.v1.json` so `vox ci docs-reality-audit verify` stays green.
- Only migrate retired symbols to `contracts/documentation/retired-symbols.v1.yaml` when active `docs/` pages pass `vox ci retired-symbol-check` (deferring uncleaned legacy crate names until Phase 539 cleanup while mapping them as `Retired` inside `resolver.rs`).
- Strictly honor `layers.toml` (`vox-doc-pipeline` L3 -> `vox-graph-reader` L1) and introduce zero new external crate dependencies.
