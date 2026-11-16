---
phase: 395-vox-axis-voxlang-org-and-plugin-workspace-pruning
plan: 01
subsystem: workspace-graph
tags: [vox-axis, voxlang.org, default-members, cargo-pruning, arch-check]
completed: 2026-11-10
requirements:
  - WORKSPACE-PRUNE-01
  - WORKSPACE-PRUNE-02
  - WORKSPACE-PRUNE-03
  - WORKSPACE-PRUNE-04
---

# Phase 395 Plan 01 Summary: `vox-axis` & `voxlang.org` Separation & Default Cargo Graph Pruning

## Accomplishments

1. **Standalone Sibling Entrypoints (`WORKSPACE-PRUNE-01`):**
   - Initialized `/Users/brbrainerd/dev/vox-axis` (`gui-surface`, `contracts/{gui,telemetry}/`, `AGENTS.md`, `README.md`, `scripts/dev-ui.sh`) and `/Users/brbrainerd/dev/voxlang.org` (`docs-site`, `contracts/documentation/`, `AGENTS.md`, `README.md`, `scripts/dev-docs.sh`).
2. **Default Cargo Graph Decoupling (`WORKSPACE-PRUNE-02`):**
   - Preserved `default-members = ["crates/vox-cli"]` in root `Cargo.toml` so default checks do not resolve `tauri` or heavy peripheral surface crates unless targeted explicitly.
3. **Cross-Repo Parity & Architecture Verification (`WORKSPACE-PRUNE-03..04`):**
   - Verified `vox ci cross-repo-parity` (`55` shared contracts across all 4 repositories, `0` drifts).
   - Verified `vox ci standalone-sovereignty` (`standalone-sovereignty OK`) and `cargo run -p vox-arch-check` (`0` violations).
