---
phase: 84-workspace-ci-gate-parity-vox-doctor-fix-release-readiness
plan: 01
subsystem: ci-gate-parity
tags: [doctor, ssot-drift, capability-index, gui-surface-registry, vox-arch-check, vox-doc-pipeline]
provides:
  - "`vox doctor` and `vox doctor --fix` (`--auto-heal`) exit 0 on healthy developer machines while reporting optional subsystems clearly"
  - "`vox ci ssot-drift`, `vox ci capability-index`, `vox ci gui-surface-coverage`, `vox ci gui-catalog-parity`, `vox ci crate-edges`, `vox-arch-check`, and `vox-doc-pipeline --lint-only` exit 0"
  - "`pnpm --dir crates/vox-gui/ui run build` and `pnpm --dir docs-astro run build` succeed with zero errors"
affects:
  - crates/vox-cli/src/cli_args.rs
  - crates/vox-cli/src/commands/diagnostics/doctor/mod.rs
  - crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts
  - crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts
  - docs/src/architecture/vox-vs-ts-empirical-benchmark-2026.md
requirements-completed: [GATE-01, GATE-02, GATE-03]
completed: 2026-10-16
---

# Phase 84 Plan 01: `vox doctor --fix` & Workspace SSOT/Architecture/Build Gate Parity Summary

## Accomplishments
- **`GATE-01` (`vox doctor` & `vox doctor --fix`)**:
  - Added `--fix` alias to `DoctorArgs::auto_heal` in `crates/vox-cli/src/cli_args.rs`.
  - Classified advisory provider/mesh/telemetry/registration checks (`Populi mesh security defaults`, `Google AI Studio Key`, `Model Scoreboard`, `Model Catalog`, `Workspace Registration`) in `OPTIONAL_CHECK_NAMES` in `crates/vox-cli/src/commands/diagnostics/doctor/mod.rs` so `vox doctor` and `vox doctor --fix` exit `0` when all build-critical checks pass.
- **`GATE-02` (SSOT, Architecture & Doc Gates)**:
  - Regenerated `crates/vox-gui/ui/src/generated/capabilityIndex.generated.ts` and `crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts` from live contracts.
  - Fixed `domain: "language"` frontmatter facet in `docs/src/architecture/vox-vs-ts-empirical-benchmark-2026.md`.
  - Verified `./target/debug/vox ci ssot-drift`, `./target/debug/vox ci capability-index`, `./target/debug/vox ci gui-surface-coverage`, `./target/debug/vox ci gui-catalog-parity`, `./target/debug/vox ci crate-edges`, `./target/debug/vox-arch-check`, and `./target/debug/vox-doc-pipeline --lint-only` all exit `0`.
- **`GATE-03` (Production Builds & Test Suites)**:
  - Verified `pnpm --dir crates/vox-gui/ui run build` (`tsc --noEmit` + `vite build`, 2,846 modules transformed) exits `0`.
  - Verified `pnpm --dir docs-astro run build` (`850 page(s) built`, Pagefind search index on `868` HTML files) exits `0`.
  - Verified `emission_ladder_test` (`19/19`), `feature_matrix_parity_test` (`6/6`), and `real_backend_harness` (`4/4`) pass.
