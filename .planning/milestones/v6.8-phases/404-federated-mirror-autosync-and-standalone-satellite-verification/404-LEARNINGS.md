---
phase: 404
phase_name: "federated-mirror-autosync-and-standalone-satellite-verification"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 4
  patterns: 0
  surprises: 0
missing_artifacts:
  - "STATE.md"
---

# Phase 404 Learnings: federated-mirror-autosync-and-standalone-satellite-verification

## Decisions

### Updated .vox/repositories.yaml and crates/vox-repository/src/repo_catalo...
Updated `.vox/repositories.yaml` and `crates/vox-repository/src/repo_catalog/federation.rs` so `vox-axis` (`../vox-axis`) and `voxlang.org` (`../voxlang.org`) resolve against the real sibling repositories (matching `../vox-ops`, with `optional: true` so CI clones without sibling checkouts continue to pass cleanly).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 404-01-SUMMARY.md

---

### Added contracts/gui/ and contracts/documentation/ to vox's shared_contra...
Added `contracts/gui/` and `contracts/documentation/` to `vox`'s `shared_contracts` list so `vox ci cross-repo-parity` compares `BLAKE3` digests across `vox` and all 3 satellite repositories.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 404-01-SUMMARY.md

---

### Extended verify_cross_repo_contract_parity in crates/vox-repository/src/...
Extended `verify_cross_repo_contract_parity` in `crates/vox-repository/src/repo_catalog/federation.rs` to check key federated mirror files (`crates/vox-gui/README.md` ↔ `vox-axis/README.md`, `surfaceRegistry.generated.ts`, `capabilityIndex.generated.ts`, `docs-astro/README.md` ↔ `voxlang.org/README.md`, and `.well-known/llms.txt`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 404-01-SUMMARY.md

---

### Implemented vox_repository::sync_federated_satellites (SatelliteSyncSumm...
Implemented `vox_repository::sync_federated_satellites` (`SatelliteSyncSummary`) and added `--sync` to `vox ci cross-repo-parity` (`crates/vox-cli-ci/src/cmd_enums.rs`, `crates/vox-cli/src/commands/ci/run_body.rs`) to synchronize shared `contracts/`, `crates/vox-gui`, `docs-astro`, and `crates/vox-doc-pipeline` from `vox` into all present sibling checkouts.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 404-01-SUMMARY.md

---

### vox-ops (vox-foundation/vox-ops, private)
`vox-ops` (`vox-foundation/vox-ops`, private): `cargo test` → `7/7` passed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 404-01-SUMMARY.md

## Lessons

### cargo test -p vox-repository --test federation_tests
`cargo test -p vox-repository --test federation_tests`: `6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s`.

**Context:** Verified in phase acceptance checks.
**Source:** 404-VERIFICATION.md

---

### cargo run -p vox-cli -- ci cross-repo-parity
`cargo run -p vox-cli -- ci cross-repo-parity`: `cross-repo-parity OK (4/4 present repositories, 55 shared contract file(s) verified, skipped optional: [])`.

**Context:** Verified in phase acceptance checks.
**Source:** 404-VERIFICATION.md

---

### total: 4 passed: 4 issues: 0 pending: 0 skipped: 0
total: 4 passed: 4 issues: 0 pending: 0 skipped: 0

**Context:** Observed during user acceptance testing.
**Source:** 404-UAT.md

---

### [none]
[none]

**Context:** Observed during user acceptance testing.
**Source:** 404-UAT.md

## Patterns

## Surprises

