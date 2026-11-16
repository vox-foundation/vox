# Phase 415: 4-Repo Federation Truth Reconciliation - Context

**Gathered:** 2026-11-13
**Status:** Ready for planning
**Mode:** Autonomous

<domain>
## Phase Boundary

Reconcile `.vox/repositories.yaml`, `crates/vox-repository/src/repo_catalog/federation.rs`, `crates/vox-repository/tests/federation_tests.rs`, and `docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md` so the hybrid 4-repository federation topology (`vox` `.` + `vox-ops` `../vox-ops` optional private sibling + `vox-axis` `crates/vox-gui` & `voxlang.org` `docs-astro` canonical in-monorepo roots with automatic external sibling promotion when `../vox-axis` / `../voxlang.org` exist) is documented accurately, free of raw `.vox/` path drift literals, and covered by unit/integration tests.

</domain>

<decisions>
## Implementation Decisions

### Hybrid Federation Resolution Contract (`HYGIENE-FED-01`)
- `.vox/repositories.yaml` declares the canonical roots `vox` (`.`), `vox-ops` (`../vox-ops`, `optional: true`), `vox-axis` (`crates/vox-gui`), and `voxlang.org` (`docs-astro`).
- Document this hybrid topology explicitly in `.vox/repositories.yaml` comments and `federation.rs` rustdoc: in-monorepo directories (`crates/vox-gui`, `docs-astro`) are always-present canonical sources so a single `vox` clone satisfies `4/4` surfaces (or `3/4` when private `../vox-ops` is absent), while `resolve_external_satellite_root` automatically promotes external sibling checkouts (`../vox-axis`, `../voxlang.org`) into active mirror sync targets whenever checked out alongside `vox`.
- Route `.vox/repositories.yaml` path lookups in `federation.rs` and `federation_tests.rs` through `repo_catalog_manifest_path` so `vox-drift-check` reports zero `[drift/vox-path-literal]` findings in `vox-repository`.

### Test Coverage & SSOT Documentation (`HYGIENE-FED-02` & `HYGIENE-FED-03`)
- Add `test_hybrid_in_monorepo_surfaces_without_external_siblings` in `crates/vox-repository/tests/federation_tests.rs` proving a standalone `vox` checkout + optional `../vox-ops` sibling (with no `../vox-axis` or `../voxlang.org` sibling directories) passes both `verify_cross_repo_contract_parity` and `sync_federated_satellites` cleanly.
- Update `docs/src/architecture/monorepo-to-multirepo-flywheel-ssot-2026.md` Section 1 table and summary so reader-facing architecture documentation accurately distinguishes canonical in-monorepo roots (`crates/vox-gui`, `docs-astro`) from optional external sibling mirrors (`../vox-axis`, `../voxlang.org`) and the physical private sibling repo (`../vox-ops`), then lint with `vox-doc-pipeline --lint-only`.

</decisions>
