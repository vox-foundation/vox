---
phase: 82-operational-gui-surfaces-cli-side-effect-dedicated-floor-ratchet
plan: 02
subsystem: contracts
tags: [operations-catalog, side-effect-ratchet, gui-surface-registry, dedicated-floor]
requires: []
provides:
  - Handler-evidenced side_effect_class for 215 CLI operations in ci.* and db.*
  - Tightened side-effect-ratchet.v1.json (max_unresolved: 649 <= 756)
  - Promoted audit, db, doctor, and secrets in surface-registry.v1.yaml (dedicated-floor-ratchet.v1.json: 16 >= 16)
affects: [vox-cli, vox-gui, contracts]
tech-stack:
  added: []
  patterns: [handler-evidenced-side-effects, dedicated-surface-ratchet]
key-files:
  created: []
  modified:
    - contracts/operations/catalog.v1.yaml
    - contracts/capabilities/side-effect-ratchet.v1.json
    - contracts/capabilities/capability-index.v1.json
    - contracts/gui/surface-registry.v1.yaml
    - contracts/gui/dedicated-floor-ratchet.v1.json
    - contracts/reports/gui-surface-coverage.v1.json
    - contracts/reports/gui-surface-registry.v1.json
key-decisions:
  - "Classified 215 CLI operations across vox ci * and vox db * by inspecting crates/vox-cli/src/commands/ci/run_body.rs and crates/vox-cli/src/commands/db_cli/mod.rs, ratcheting max_unresolved to 649."
  - "Bound audit -> harness-health, db -> vox-search, doctor -> explorer, and secrets -> approvals in contracts/gui/surface-registry.v1.yaml, ratcheting dedicated-floor-ratchet.v1.json from 12 to 16."
patterns-established:
  - "1-to-1 unique view_key <-> cli_group binding in contracts/gui/surface-registry.v1.yaml."
requirements-completed: [OPS-04, OPS-05]
duration: 20min
completed: 2026-10-16
---

# Phase 82 Plan 02: CLI Side-Effect Classification & GUI Dedicated-Floor Ratchet (`OPS-04..05`) Summary

Classified 215 CLI operations (`ci.*` and `db.*`) in `contracts/operations/catalog.v1.yaml`, ratcheting `contracts/capabilities/side-effect-ratchet.v1.json` `max_unresolved` to `649` (`<= 756`), and promoted `audit`, `db`, `doctor`, and `secrets` in `contracts/gui/surface-registry.v1.yaml`, ratcheting `contracts/gui/dedicated-floor-ratchet.v1.json` to `16` (`>= 16`).
