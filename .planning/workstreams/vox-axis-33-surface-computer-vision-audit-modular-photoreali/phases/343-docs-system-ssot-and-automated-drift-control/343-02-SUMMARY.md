# Phase 343 Plan 02 Summary: Tool Overlay Parity Guard

## Overview

Plan `343-02-PLAN.md` has been executed in full.
- Created `contracts/documentation/overlay-claims.v1.yaml` with schema version 1, overlay list (`CLAUDE.md`, `GEMINI.md`, `.github/copilot-instructions.md`, `docs/src/AGENTS.md`, `.cursor/rules/*.mdc`), and 5 normative claims anchored in `AGENTS.md`.
- Implemented `crates/vox-cli-ci/src/overlay_parity.rs` with `pub fn check(...)` and `pub fn run(root: &Path) -> Result<()>`.
- Registered `pub mod overlay_parity;` in `crates/vox-cli-ci/src/lib.rs`.
- Registered `overlay_parity` stage in `crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs` directly before `ssot_regen_parity`.
- Resolved `build-environment.mdc` per Task 2 Option A (contradiction: replaced `.cargo\bin\cargo.exe` with plain `cargo` coordinated by the build broker).
- Updated documentation in `docs/src/architecture/docs-system-ssot.md` (§4 gate row, §6 gap removed) and `docs/src/contributors/detector-coverage-ledger.md` (covered class row).

## Task 1: Tracer — overlay_parity Stage & no-hand-last-updated Claim

- **RED phase**: Added unit test module in `overlay_parity.rs` with 8 tests. Verified all 8 tests failed against stubs.
- **GREEN phase**:
  - Implemented `check` (YAML parsing, regex compilation, anchor validation against `AGENTS.md`, per-overlay line matching, negation exemptions) and `run` (bounded reading of inputs and glob expansion of `.cursor/rules/*.mdc`).
  - Wired `overlay_parity` into `ssot_drift_stages()` before `ssot_regen_parity`.
  - All 8 unit tests passed.
- **Mutation Proof (Task 1)**:
  - Appended `- New doc pages must include \`category\`, \`status\`, \`last_updated\`, and \`training_eligible\` frontmatter` to `.cursor/rules/documentation-policy.mdc`.
  - Ran `overlay_parity::tests::real_tree`.
  - Test failed with exit code 101 citing `.cursor/rules/documentation-policy.mdc:12: contradicts AGENTS.md claim no-hand-last-updated`.
  - Restored `.cursor/rules/documentation-policy.mdc` and verified file integrity.

## Task 2: Decision on build-environment.mdc Explicit Cargo Path

- **Selection**: `option-a` ("contradiction. Replace the explicit-path bullet with plain `cargo` (shim on PATH; `vox doctor` reports broker activation) and add overlay claim `no-explicit-cargo-path` forbidding `.cargo/bin/cargo` paths in overlays").
- Pre-authorized by user instruction for AFK execution.

## Task 3: Remaining Claims, Overlay Fixes, and Docs/Ledger Updates

- **RED phase**: Added 4 test cases to `overlay_parity.rs` pinning the 4 new claims. Verified all 4 failed when claims were absent from `overlay-claims.v1.yaml`.
- **GREEN phase**:
  - Added claims to `contracts/documentation/overlay-claims.v1.yaml`:
    1. `inline-vox-not-include`: anchor `"do NOT use mdBook `{{#include}}` directives for new code"`.
    2. `frontmatter-minimum-keys`: anchor `"Minimum required keys"`.
    3. `no-cargo-fmt-all`: anchor `"Never run `cargo fmt --all` on this workspace"`.
    4. `no-explicit-cargo-path`: anchor `"never call the real cargo binary by an explicit resolved path"`.
  - Fixed `.cursor/rules/build-environment.mdc` lines 26-27: replaced explicit `$env:USERPROFILE\.cargo\bin\cargo.exe` with plain `cargo` from workspace root with build broker coordination.
  - Verified `real_tree` and all 12 unit tests pass.
- **Mutation Proof (Task 3 line 7)**:
  - Appended `- use \`{{#include}}\` from \`examples/golden/\` for vox snippets` to `.cursor/rules/documentation-policy.mdc`.
  - Ran `overlay_parity::tests::real_tree`.
  - Test failed with exit code 101 citing `.cursor/rules/documentation-policy.mdc:12: contradicts AGENTS.md claim inline-vox-not-include`.
  - Restored `.cursor/rules/documentation-policy.mdc` and verified clean diff.
- **Documentation Updates**:
  - `docs/src/architecture/docs-system-ssot.md`:
    - Added §4 row for `Overlay parity (overlay_parity ssot-drift stage)`.
    - Removed §6 "Overlay contradictions" gap bullet.
  - `docs/src/contributors/detector-coverage-ledger.md`:
    - Added row for `Overlay contradicts AGENTS.md` (`overlay_parity`, blocking, pre-push fast + PR CI linux via ssot-drift).
  - Verified with `vox-doc-pipeline --lint-only` (exit 0).

## Acceptance Criteria Checklist

- `grep -c 'no-hand-last-updated' contracts/documentation/overlay-claims.v1.yaml` -> 1
- `grep -c 'pub mod overlay_parity' crates/vox-cli-ci/src/lib.rs` -> 1
- `grep -c 'overlay_parity::run' crates/vox-cli/src/commands/ci/run_body_helpers/ssot_drift.rs` -> 1
- `grep -cE 'must include.*last_updated' .cursor/rules/documentation-policy.mdc` -> 0
- `grep -cE '^  - id: (inline-vox-not-include|frontmatter-minimum-keys|no-cargo-fmt-all)$' contracts/documentation/overlay-claims.v1.yaml` -> 3
- `grep -c 'no-explicit-cargo-path' contracts/documentation/overlay-claims.v1.yaml` -> 1
- `grep -cF '.cargo\bin\cargo.exe' .cursor/rules/build-environment.mdc` -> 0
- `grep -c 'Overlay contradictions' docs/src/architecture/docs-system-ssot.md` -> 0
- `grep -c 'overlay_parity' docs/src/architecture/docs-system-ssot.md` -> 1
- `grep -c 'overlay_parity' docs/src/contributors/detector-coverage-ledger.md` -> 1
- `cargo nextest run -p vox-cli-ci overlay_parity`: 12 passed, 0 failed.
- `cargo nextest run -p vox-cli --lib ssot_drift`: 12 passed, 0 failed (`ssot_regen_parity` remains last).
- `vox-doc-pipeline --lint-only`: exit 0.
