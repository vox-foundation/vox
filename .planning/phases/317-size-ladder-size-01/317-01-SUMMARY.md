---
phase: 317-size-ladder-size-01
plan: 01
subsystem: training
tags: [qwen3, qlora, mens, gpu-specs, floor_measured]

requires:
  - phase: 316-before-after-comparison
    provides: resolve_base_pin size-name path used by eval-reasoning-compare
provides:
  - Unmeasured Qwen3-1.7B and Qwen3-4B QLoRA pins on train_bases.qwen3_code only
  - resolve_qwen3_size for the six dense sizes and the Qwen3- / Qwen/Qwen3- spellings
  - pick_base skips floor_measured false and leaves measured floors unchanged
affects: [318-live-vram-selector, 320-train-the-sizes]

actuals:
  tokens: 7455
  tasks: 2
  commits: 2

commits: 2
plan_head_before: 931ff26e2af1a7cbc7de2b290a3b296930fe5533
plan_head_after: 3592ec1e69305f2d6a0f92b14cba7a3a0c54387d

tech-stack:
  added: []
  patterns:
    - "floor_measured serde-defaults true; missing floor_mb is None"
    - "Size spellings resolve only through resolve_qwen3_size on qwen3_code QLoRA rows"

key-files:
  created: []
  modified:
    - mens/config/gpu-specs.yaml
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-populi/tests/training_presets_yaml_contract.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs

key-decisions:
  - "1.7B and 4B are unmeasured qwen3_code QLoRA rows with no floor_mb; pick_base cannot select them"
  - "floor_measured defaults true so existing measured floors stay eligible"
  - "Capability tags such as qwen3_16g are not size names and do not enter resolve_qwen3_size"

patterns-established:
  - "A Qwen3 size name maps to exactly one licensed qwen3_code QLoRA pin"
  - "An unmeasured rung records floor_measured false and omits floor_mb"

requirements-completed: [SIZE-01]

coverage:
  - id: D1
    description: "1.7B and 4B are pinned on train_bases.qwen3_code only, apache-2.0, attribution_required false, floor_measured false, and no floor_mb"
    requirement: SIZE-01
    verification:
      - kind: unit
        ref: "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs#resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged"
        status: pass
      - kind: unit
        ref: "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs#unmeasured_pins_are_only_on_qwen3_code_and_measured_floors_stay"
        status: pass
      - kind: integration
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs#size_name_1_7b_records_the_pinned_unmeasured_base"
        status: pass
      - kind: integration
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs#size_name_4b_records_the_pinned_unmeasured_base"
        status: pass
    human_judgment: false
  - id: D2
    description: "0.6B, 1.7B, 4B, 8B, 14B, and 32B, plus Qwen3-size and Qwen/Qwen3-size spellings, resolve to the locked qwen3_code QLoRA pin"
    requirement: SIZE-01
    verification:
      - kind: unit
        ref: "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs#resolve_qwen3_size_covers_the_dense_ladder"
        status: pass
      - kind: unit
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs#base_resolution_size_name_uses_train_bases"
        status: pass
    human_judgment: false
  - id: D3
    description: "pick_base still fails closed below 2000 MB, stays on 8B at 16384 MB, and ignores unmeasured rows; measured floors stay"
    requirement: SIZE-01
    verification:
      - kind: unit
        ref: "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs#resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged"
        status: pass
      - kind: integration
        ref: "crates/vox-populi/tests/training_presets_yaml_contract.rs#gpu_specs_qwen3_train_bases_floor_mb_pins_against_real_weight_size"
        status: pass
      - kind: integration
        ref: "crates/vox-populi/tests/training_presets_yaml_contract.rs#gpu_specs_agentic_default_16g_floor_matches_training_presets_default"
        status: pass
    human_judgment: false
  - id: D4
    description: "Unknown size spellings error. Capability tags stay unpinned. A non-40-hex revision or a missing license does not resolve"
    requirement: SIZE-01
    verification:
      - kind: unit
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs#resolve_qwen3_size_rejects_unknown_spellings"
        status: pass
      - kind: unit
        ref: "crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs#resolve_base_pin_leaves_capability_tag_unpinned"
        status: pass
      - kind: unit
        ref: "crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs#resolve_qwen3_size_rejects_bad_revision_and_missing_license"
        status: pass
    human_judgment: false

duration: 41min
completed: 2026-10-28
status: complete
---

# Phase 317 Plan 01: Size Ladder Summary

Qwen3 dense sizes 0.6B through 32B resolve to pinned, licensed `qwen3_code` QLoRA rungs. 1.7B and 4B are unmeasured, so `pick_base` cannot select them, and the measured 8B / 14B / 32B floors are unchanged.

## Performance

- **Duration:** 41 min
- **Started:** 2026-10-29T03:10:00Z
- **Completed:** 2026-10-29T03:51:17Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- Added `Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` and `Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c` under `train_bases.qwen3_code` only, apache-2.0, `attribution_required: false`, `floor_measured: false`, with no `floor_mb`.
- `floor_measured` serde-defaults to true. `pick_base` skips `floor_measured: false`. Existing measured floors and the 14B/32B LoRA rows stay. The new rows are not on `small_code_default` or `strong_code_default`.
- `resolve_qwen3_size` accepts `0.6B`..`32B` and the exact `Qwen3-<size>` / `Qwen/Qwen3-<size>` spellings. Unknown sizes error. Phase 316's size-name path calls it. `qwen3_16g` stays on the memory picker.

## Task Commits

Each task was committed atomically:

1. **Task 1: Pin unmeasured 1.7B and 4B and name them through the compare entry** - `2d6aea459` (feat)
2. **Task 2: Cover every size, unknown spellings, and other tags** - `3592ec1e6` (test)

## Files Created/Modified

- `mens/config/gpu-specs.yaml` — 1.7B and 4B QLoRA rows on `qwen3_code` only
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` — optional `floor_mb`, `floor_measured`, `resolve_qwen3_size`, `pick_base` skip
- `crates/vox-populi/tests/training_presets_yaml_contract.rs` — floor contract skips unmeasured rows
- `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` — size-name path calls `resolve_qwen3_size`
- `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` — 1.7B and 4B absence records name the locked pins

## Decisions Made

- 1.7B (`70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`) and 4B (`1cfa9a7208912126459214e8b04321603b3df60c`) are unmeasured `qwen3_code` QLoRA rows with no `floor_mb`.
- `floor_measured` defaults true so measured rows stay eligible. `pick_base` requires both `floor_measured` and a `floor_mb` that fits.
- Size names go through `resolve_qwen3_size`. Capability tags are not passed to it.
- A revision must be 40 lowercase hex characters, and license is required.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Lib and contract tests need feature flags or the harness is empty**
- **Found during:** Task 1 verification
- **Issue:** `cargo test -p vox-populi --lib` without features runs 0 tests because `spoke_base_resolver` is `#[cfg(feature = "mens")]`. The contract binary is `#![cfg(feature = "mens-train")]`, so `--features mens` alone also runs 0 tests. Both commands exit 0.
- **Fix:** Ran the resolver tests with `--features mens` and the contract tests with `--features mens-train`.
- **Files modified:** none (command only)
- **Verification:** 27 `spoke_base_resolver` tests, then the two named contract tests, all passed
- **Committed in:** n/a (verification deviation)

**2. [Rule 2 - Missing critical check] Reject a non-40-hex revision and a missing license**
- **Found during:** Task 1
- **Issue:** T-317-01 and T-317-03 require a 40-character lowercase hex revision and an explicit license. The plan's happy-path tests did not cover a bad revision or a missing license.
- **Fix:** `resolve_qwen3_size_rejects_bad_revision_and_missing_license` covers `not-a-sha`, uppercase hex, and `license: None`.
- **Files modified:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`
- **Verification:** that test passed under `--features mens`
- **Committed in:** `2d6aea459`

**3. [Rule 1 - Bug] Same-file TDD landed tests and the floor type change in one commit**
- **Found during:** Task 1
- **Issue:** The plan is `type: execute`. The new tests and the `floor_mb: Option<u32>` change live in the same Rust files. Interactive hunk staging is not available, so a RED commit and a GREEN commit could not be split without staging unrelated dirty work.
- **Fix:** Task 1 is one `feat` commit containing the pins, the resolver, and the failing-then-passing tests. Task 2 is a separate `test` commit.
- **Files modified:** the five plan files
- **Verification:** the tests listed below passed after the implementation
- **Committed in:** `2d6aea459`

---

**Total deviations:** 3 auto-fixed (1 Rule 1, 1 Rule 2, 1 Rule 3)
**Impact on plan:** The pins, skip rule, and size resolver match the locked decisions. The feature flags are required for a real test run. No floors were invented and no weights were downloaded.

## Issues Encountered

Edits to `gpu-specs.yaml` and `spoke_base_resolver.rs` were briefly overwritten by a compressed file view. Both files were restored from git and the intended edits were reapplied. The committed yaml diff is the two new rows only. A later whole-file parse error was that corrupted yaml, not `Option<u32>` deserialization.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phases 318 and 320 can name every dense size. 1.7B and 4B still have no measured `floor_mb`, so a memory picker must not treat them as fitting. This plan did not train, download weights, or call Hugging Face.

## Tests

Passed, offline, no Hugging Face:

- `cargo test -p vox-populi --features mens --lib spoke_base_resolver` — 27 passed (task 1, before the task 2 tests existed)
- `cargo test -p vox-populi --features mens --lib resolve_qwen3_size` — 3 passed (`resolve_qwen3_size_1_7b_pin_and_16g_pick_unchanged`, `resolve_qwen3_size_rejects_bad_revision_and_missing_license`, `resolve_qwen3_size_covers_the_dense_ladder`)
- `cargo test -p vox-populi --features mens --lib unmeasured_pins` — 1 passed (`unmeasured_pins_are_only_on_qwen3_code_and_measured_floors_stay`)
- `cargo test -p vox-populi --features mens-train --test training_presets_yaml_contract -- gpu_specs_qwen3_train_bases_floor_mb_pins_against_real_weight_size gpu_specs_agentic_default_16g_floor_matches_training_presets_default` — 2 passed
- `cargo test -p vox-ml-cli --features gpu --lib -- base_resolution_ size_name_ resolve_qwen3_size_rejects resolve_base_pin_leaves resolve_qwen3_size_errors` — 10 passed, including `size_name_1_7b_records_the_pinned_unmeasured_base`, `size_name_4b_records_the_pinned_unmeasured_base`, `resolve_qwen3_size_rejects_unknown_spellings`, `resolve_base_pin_leaves_capability_tag_unpinned`, and `resolve_qwen3_size_errors_when_qwen3_code_has_two_qlora_pins`
- `rustfmt --edition 2024 --check` on the four Rust files — passed

## Known Stubs

None. Unmeasured rows omit `floor_mb` on purpose. They are not placeholders for a guessed floor.

## Self-Check: PASSED

- FOUND: mens/config/gpu-specs.yaml
- FOUND: crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
- FOUND: crates/vox-populi/tests/training_presets_yaml_contract.rs
- FOUND: crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
- FOUND: crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
- FOUND: 2d6aea459
- FOUND: 3592ec1e6

## TDD Gate Compliance

Plan type is `execute`, not `tdd`. `tdd_mode` is false. Task 1's tests and implementation share the same files and landed in `2d6aea459`. Task 2's coverage tests landed in `3592ec1e6` after the tracer slice was already green (27 resolver tests, 2 contract tests, 32 `reasoning_compare` tests).
