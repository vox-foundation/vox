# Phase 84: Workspace CI Gate Parity, `vox doctor --fix` & Release Readiness - Context

**Gathered:** 2026-10-16
**Status:** Ready for execution & verification

<domain>
## Phase Boundary

Deliver Milestone v1.6 Phase 84 (`GATE-01..04`):
- **GATE-01**: `vox doctor` and `vox doctor --fix` inspect and auto-remediate daemon reachability, `VoxDb` schema baseline `98`, Clavis keyring/vault health, `~/.vox/cache/model-catalog.v1.json`, and shell integration.
- **GATE-02**: `vox ci ssot-drift`, `vox ci gui-coverage-gate`, `vox ci capability-index`, `vox ci feature-matrix-gate`, `vox ci crate-edges`, `vox-arch-check`, and `vox-doc-pipeline --lint-only` all exit `0` with zero drift.
- **GATE-03**: `pnpm --dir crates/vox-gui/ui run build`, `pnpm --dir docs-astro run build`, and Rust compilation/tests across core crates complete with zero errors.
- **GATE-04**: Milestone `/gsd-code-review --fix` and `audit-open` pass with zero Critical/Warning issues and zero open deferred items.
</domain>

<decisions>
## Implementation Decisions

### Self-Heal & Doctor Verification (`GATE-01`)
- `crates/vox-cli/src/commands/doctor.rs` implements `--fix` remediation for daemon reachability, `VoxDb` schema baseline, Clavis keyring/vault state, model catalog cache, and shell hooks.

### Workspace CI, SSOT & Build Gate Parity (`GATE-02..04`)
- Run all SSOT, GUI coverage, capability index, crate-edge, architecture, and doc-pipeline gates to verify zero drift.
- Verify frontend (`crates/vox-gui/ui`) and docs (`docs-astro`) production builds alongside Rust unit and integration test suites.
- Complete Milestone `v1.6` code review and audit-open checks with zero open items.
</decisions>
