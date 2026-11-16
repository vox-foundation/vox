---
phase: 203-clone-t3-ecosystem-repos-into-dev-build-deterministic-surfac
verified: 2026-10-24T17:27:10-07:00
status: passed
score: 4/4 must-haves verified
---

# Phase 203 Verification: Clone T3 Ecosystem Repos into `~/dev` & Build Deterministic Surface/AST Inventory Graph

- [x] **T3-CLONE-GRAPH-01**: Cloned `pingdotgg/t3code`, `vibheksoni/t3router`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, and `helioschat/app` into `~/dev/` and recorded commit SHAs.
- [x] **T3-CLONE-GRAPH-02**: Verified `vox check scripts/t3-surface-inventory.vox` (`0` warnings) and `vox run --interp scripts/t3-surface-inventory.vox` (`verified 5/5 T3 ecosystem repository graphs`).
- [x] **T3-CLONE-GRAPH-03**: Published `docs/src/architecture/t3-ecosystem-surface-map-research-2026.md`.
- [x] **T3-CLONE-GRAPH-04**: Verified `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/t3-ecosystem-surface-map-research-2026.md` (`no hard errors`).
