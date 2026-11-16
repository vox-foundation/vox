---
phase: 404-federated-mirror-autosync-and-standalone-satellite-verification
plan: 01
subsystem: repo-federation
tags: [federation, cross-repo-parity, vox-axis, voxlang.org, vox-ops]
completed: 2026-11-12
coverage:
  - id: D-404-01
    description: "Sibling root resolution for ../vox-ops, ../vox-axis, ../voxlang.org and shared_contracts (FEDERATION-SYNC-01)"
    human_judgment: false
    verification:
      - kind: integration
        status: pass
        ref: "cargo test -p vox-repository --test federation_tests (5/5 passed)"
  - id: D-404-02
    description: "Federated mirror parity verification for BLAKE3 contracts and mirror files across all 4 repositories (FEDERATION-SYNC-02)"
    human_judgment: false
    verification:
      - kind: integration
        status: pass
        ref: "cargo run -p vox-cli -- ci cross-repo-parity (4/4 present repositories, 55 shared contract files verified, skipped optional: [])"
  - id: D-404-03
    description: "One-command satellite auto-sync via vox ci cross-repo-parity --sync and sync_federated_satellites (FEDERATION-SYNC-03)"
    human_judgment: false
    verification:
      - kind: integration
        status: pass
        ref: "cargo test -p vox-repository --test federation_tests::test_verify_cross_repo_contract_parity_real_workspace + vox ci cross-repo-parity"
  - id: D-404-04
    description: "Standalone build and test verification across vox-ops, vox-axis, voxlang.org, and vox (FEDERATION-SYNC-04)"
    human_judgment: false
    verification:
      - kind: integration
        status: pass
        ref: "cargo test -p vox-repository --test federation_tests (5/5 passed) + vox ci cross-repo-parity (0 drifts)"
---

# Phase 404 Plan 01 Summary: Federated Mirror Auto-Sync (`vox ci cross-repo-parity --sync`), Sibling Root Resolution & Standalone Satellite Verification

## Accomplishments

1. **Sibling Root Resolution in `.vox/repositories.yaml` & `default_federation_catalog()` (`FEDERATION-SYNC-01`):**
   - Updated `.vox/repositories.yaml` and `crates/vox-repository/src/repo_catalog/federation.rs` so `vox-axis` (`../vox-axis`) and `voxlang.org` (`../voxlang.org`) resolve against the real sibling repositories (matching `../vox-ops`, with `optional: true` so CI clones without sibling checkouts continue to pass cleanly).
   - Added `contracts/gui/` and `contracts/documentation/` to `vox`'s `shared_contracts` list so `vox ci cross-repo-parity` compares `BLAKE3` digests across `vox` and all 3 satellite repositories.
2. **Federated Mirror Parity Verification (`FEDERATION-SYNC-02`):**
   - Extended `verify_cross_repo_contract_parity` in `crates/vox-repository/src/repo_catalog/federation.rs` to check key federated mirror files (`crates/vox-gui/README.md` ↔ `vox-axis/README.md`, `surfaceRegistry.generated.ts`, `capabilityIndex.generated.ts`, `docs-astro/README.md` ↔ `voxlang.org/README.md`, and `.well-known/llms.txt`).
3. **One-Command Satellite Auto-Sync (`vox ci cross-repo-parity --sync`, `FEDERATION-SYNC-03`):**
   - Implemented `vox_repository::sync_federated_satellites` (`SatelliteSyncSummary`) and added `--sync` to `vox ci cross-repo-parity` (`crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-cli/src/commands/ci/run_body.rs`) to synchronize shared `contracts/`, `crates/vox-gui`, `docs-astro`, and `crates/vox-doc-pipeline` from `vox` into all present sibling checkouts.
4. **Standalone Build & Test Verification Across All 4 Repositories (`FEDERATION-SYNC-04`):**
   - `vox-ops` (`vox-foundation/vox-ops`, private): `cargo test` → `7/7` passed.
   - `vox-axis` (`vox-foundation/vox-axis`): `./scripts/build.sh` (`2,874 modules transformed`) and `./scripts/test.sh` (`360/360` Vitest suites passed with `VOX_ROOT` fallback).
   - `voxlang.org` (`vox-foundation/voxlang.org`): `./scripts/build.sh` (`919` pages built, `All internal links are valid`) and `./scripts/test.sh` (`vox-doc-pipeline lint complete — no hard errors`).
   - `vox` (`vox-foundation/vox`): `cargo test -p vox-repository` (`58/58` passed) and `vox ci cross-repo-parity --sync` (`0` drifts).
