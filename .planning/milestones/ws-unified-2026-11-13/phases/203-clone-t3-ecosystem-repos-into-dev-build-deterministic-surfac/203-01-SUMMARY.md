---
phase: 203-clone-t3-ecosystem-repos-into-dev-build-deterministic-surfac
plan: 01
subsystem: architecture
tags:
  - t3code
  - t3router
  - pi-t3chat
  - thom-chat
  - helioschat
  - surface-inventory
dependency_graph:
  requires: []
  provides:
    - contracts/reports/t3-ecosystem-surface-graph.v1.json
    - scripts/t3-surface-inventory.vox
    - docs/src/architecture/t3-ecosystem-surface-map-research-2026.md
  affects:
    - docs/src/architecture/
tech_stack:
  added: []
  patterns:
    - Token-bounded surface inventory graph (.vox script + JSON report + Starlight KB doc)
key_files:
  created:
    - scripts/t3-surface-inventory.vox
    - contracts/reports/t3-ecosystem-surface-graph.v1.json
    - docs/src/architecture/t3-ecosystem-surface-map-research-2026.md
  modified: []
decisions:
  - Cloned all 5 T3 ecosystem repositories into ~/dev/{t3code,t3router,pi-t3chat,thom-chat,helioschat} with pinned commit SHAs.
metrics:
  completed_date: "2026-10-24"
---

# Phase 203 Plan 01: Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph Summary

Cloned all 5 open-source T3 ecosystem repositories (`pingdotgg/t3code` at `3eb83666`, `vibheksoni/t3router` at `5314bbbf`, `vibheksoni/pi-t3chat` at `b3987324`, `TGlide/thom-chat` at `062193d7`, and `helioschat/app` at `08e2c3fc`) into `~/dev`, built and verified `scripts/t3-surface-inventory.vox` (`0` compiler warnings), emitted `contracts/reports/t3-ecosystem-surface-graph.v1.json`, and published `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md` (`0` `vox-doc-pipeline` lint errors).
