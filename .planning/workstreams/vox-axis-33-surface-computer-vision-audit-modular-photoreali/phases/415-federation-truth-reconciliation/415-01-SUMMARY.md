---
phase: 415-federation-truth-reconciliation
plan: 01
subsystem: federation
tags: [federation, repositories-yaml, cross-repo-parity, architecture-docs]
provides:
  - Reconciled hybrid 4-repository federation catalog, code, tests, and SSOT documentation
affects:
  - .vox/repositories.yaml
  - crates/vox-repository/src/repo_catalog/federation.rs
  - crates/vox-repository/tests/federation_tests.rs
  - docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md
requirements-completed:
  - HYGIENE-FED-01
  - HYGIENE-FED-02
  - HYGIENE-FED-03
completed: 2026-11-13
---

# Phase 415 Plan 01: 4-Repo Federation Truth Reconciliation Summary

Reconciled `.vox/repositories.yaml`, `federation.rs`, `federation_tests.rs`, and `monorepo-to-multirepo-flywheel-ssot-2026.md` around Vox's hybrid 4-repository federation architecture.

## Key Changes

1. **Hybrid Topology Contract & Drift-Free Manifest Lookup (`HYGIENE-FED-01`):**
   - Documented the hybrid resolution rules in `.vox/repositories.yaml` and `resolve_external_satellite_root` (`crates/vox-repository/src/repo_catalog/federation.rs`): `crates/vox-gui` (`vox-axis`) and `docs-astro` (`voxlang.org`) serve as canonical in-monorepo roots, `../vox-ops` is the optional private sibling repository, and `../vox-axis` / `../voxlang.org` automatically promote to external mirror sync targets when checked out on disk.
   - Replaced raw `".vox/repositories.yaml"` path literals in `federation.rs` and `federation_tests.rs` with `repo_catalog_manifest_path(...)`, eliminating all 3 `vox-drift-check` warnings in `vox-repository`.
2. **Integration Test & Parity/Graph Gate Verification (`HYGIENE-FED-02`):**
   - Added `test_hybrid_in_monorepo_surfaces_without_external_siblings` in `crates/vox-repository/tests/federation_tests.rs` (`8/8` passing) and verified `vox ci cross-repo-parity` (`4/4` repositories, `56` shared contracts) and `vox ci federated-symbol-graph` (`404` nodes, `8` cross-repo edges).
3. **Architecture SSOT Documentation Refresh (`HYGIENE-FED-03`):**
   - Updated Section 1 and `kb:summary` of `docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md` to distinguish canonical roots from optional sibling checkouts and verified with `vox-doc-pipeline --lint-only`.
