---
phase: 395
phase_name: "vox-axis-voxlang-org-and-plugin-workspace-pruning"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 395 Learnings: vox-axis-voxlang-org-and-plugin-workspace-pruning

## Decisions

### Initialized /Users/brbrainerd/dev/vox-axis (gui-surface, contracts/{gui,...
Initialized `/Users/brbrainerd/dev/vox-axis` (`gui-surface`, `contracts/{gui,telemetry}/`, `AGENTS.md`, `README.md`, `scripts/dev-ui.sh`) and `/Users/brbrainerd/dev/voxlang.org` (`docs-site`, `contracts/documentation/`, `AGENTS.md`, `README.md`, `scripts/dev-docs.sh`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 395-01-SUMMARY.md

---

### Preserved default-members = ["crates/vox-cli"] in root Cargo.toml so def...
Preserved `default-members = ["crates/vox-cli"]` in root `Cargo.toml` so default checks do not resolve `tauri` or heavy peripheral surface crates unless targeted explicitly.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 395-01-SUMMARY.md

---

### Verified vox ci cross-repo-parity (55 shared contracts across all 4 repo...
Verified `vox ci cross-repo-parity` (`55` shared contracts across all 4 repositories, `0` drifts).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 395-01-SUMMARY.md

---

### Verified vox ci standalone-sovereignty (standalone-sovereignty OK) and c...
Verified `vox ci standalone-sovereignty` (`standalone-sovereignty OK`) and `cargo run -p vox-arch-check` (`0` violations).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 395-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

