---
phase: 343-docs-system-ssot-and-automated-drift-control
plan: 09
subsystem: ci-ssot-parity
tags: [docs-system-ssot, meta-gate, ssot-drift, parity]
requires:
  - phase: 343-01
    provides: ssot_drift stage framework
  - phase: 343-08
    provides: finalized docs-drift-refresh workflow and retired link_checker
provides:
  - `crates/vox-cli/src/commands/ci/docs_system_parity.rs` meta-gate checking all 20 §4 gates and §6 open/closed gaps against live code and workflows
  - `docs_system_parity` stage in `ssot_drift_stages()` (`run_body_helpers/ssot_drift.rs`) and `pre_push::tier_labels`
  - Annotated `docs/src/architecture/docs-system-ssot.md` (`<!-- gate: <id> -->` markers, normalized `Where it runs`, `### Open gaps` vs `### Closed in Phase 343`)
affects: [ssot-drift]
completed: 2026-11-13
---

# Phase 343 Plan 09: `docs_system_parity` Meta-Gate Summary

Wired `docs_system_parity` into `vox ci ssot-drift` so `docs/src/architecture/docs-system-ssot.md` cannot drift silently from `pre_push.rs`, `ssot_drift.rs`, `lefthook.yml`, or `.github/workflows/*.yml`.

## Changes Made
- **Parity verifier (`crates/vox-cli/src/commands/ci/docs_system_parity.rs`, `crates/vox-cli/src/commands/ci/mod.rs`):** Implemented `Loc`, `GateRow`, `GapBullet`, `Probe`, `GATE_PROBES` (20 gates), and `CLOSED_GAPS` (6 gaps) with parsers and parity checks, plus `pub(crate) fn tier_labels` in `crates/vox-cli/src/commands/ci/pre_push.rs`.
- **Pipeline wiring (`crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs`):** Added `docs_system_parity` stage before `ssot_regen_parity` and asserted it in `real_tree_ssot_drift_stages_contract`.
- **SSOT & Ledger (`docs/src/architecture/docs-system-ssot.md`, `docs/src/contributors/detector-coverage-ledger.md`):** Added `<!-- gate: <id> -->` comments across all 20 §4 rows, split §6 into `### Open gaps` (`prose-behavior-drift`) and `### Closed in Phase 343` (6 closed gaps), and added `docs-drift-refresh` / `docs_system_parity` rows to `detector-coverage-ledger.md`.

## Verification & Mutation Proofs
- `timeout 900s cargo test -p vox-cli --lib docs_system_parity` -> `4 passed; 0 failed`.
- `timeout 900s cargo test -p vox-cli --lib real_tree_ssot_drift_stages_contract` -> `1 passed; 0 failed`.
- Mutation proof 1: replacing `docs-drift-refresh.yml` with `nonexistent-wf.yml` in §4 caused `real_tree_docs_system_ssot_parity` to fail (`0 passed; 1 failed`), then passed after restore.
- Mutation proof 2: inserting `- <!-- gap: dormant-freshness-ledger --> Still manual.` under `### Open gaps` caused `real_tree_docs_system_ssot_parity` to fail (`0 passed; 1 failed`), then passed (`1 passed; 0 failed`) after restore.
- `vox-doc-pipeline --lint-only` passed on both docs with zero hard errors.
