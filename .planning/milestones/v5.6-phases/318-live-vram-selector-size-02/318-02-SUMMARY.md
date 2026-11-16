---
phase: 318-live-vram-selector-size-02
plan: 02
subsystem: training
tags: [qwen3, qlora, vram, spokes, preload]

requires:
  - phase: 318-01
    provides: select_qwen3_code_qlora_for_free_mb, confirm_before_load, read_accelerator_free_mb
provides:
  - One measured QLoRA size and preset for every spoke when no size was named
  - Fail-closed local spoke resolution when the accelerator reading is missing
  - Pre-load re-read that may step an automatic pick down and refuses an explicit pin
affects: [318-03]

actuals:
  tokens: 7579
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Unnamed size uses select_qwen3_code_qlora_for_free_mb"
    - "Named capability tags still pass None into resolve_base_model"
    - "apply_preload_free_mb does not read the accelerator"

key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/mens/training_selection.rs
    - crates/vox-ml-cli/src/commands/mens/pipeline.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
    - crates/vox-ml-cli/Cargo.toml

key-decisions:
  - "A missing free-memory reading is an error for an unnamed size and for an explicit Qwen3 spelling"
  - "small_code_default and strong_code_default stay on pick_base"
  - "Dogfood passes automatic true because its model is None"
  - "Live train_arm probes stay behind the empty hw-probe-live-test feature"

patterns-established:
  - "automatic_qwen3 is the last argument of schola run_train"

requirements-completed: [SIZE-02]

coverage:
  - id: D1
    description: Every live spoke at 25000 MB trains the 14B QLoRA pin on qwen3_24g, and a missing reading is an error
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-ml-cli --lib training_selection
        status: pass
    human_judgment: false
  - id: D2
    description: Local spoke resolution calls read_accelerator_free_mb and does not invent 16384
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-ml-cli --lib resolve_local_spoke_base_does_not_invent_a_vram_reading --features gpu
        status: pass
    human_judgment: false
  - id: D3
    description: Automatic 32B at 25000 MB loads as 14B; explicit 32B at 25000 MB errors
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-ml-cli --lib apply_preload_free_mb --features gpu
        status: pass
    human_judgment: false

duration: 40min
completed: 2026-10-29
status: complete
plan_head_before: ba5e86db075b4504221218752625d764564f035f
plan_head_after: 839bffed013bb3634203b60691ebe7f37c55023c
---

# Phase 318 Plan 02: One size for every spoke Summary

**An unnamed size picks one measured qwen3_code QLoRA pin for every spoke, and schola re-reads free memory before it loads.**

## Accomplishments

- `resolve_training_selection` uses the live selector when no model was named and a reading is present.
- `resolve_local_spoke_base` calls `read_accelerator_free_mb` and no longer substitutes `get_system_vram_gb` or `16_384`.
- The train stage passes that reading only when the CLI model is absent or a `resolve_qwen3_size` spelling.
- `apply_preload_free_mb` steps an automatic pick down and refuses an explicit measured pin.

## Tests

- `cargo test -p vox-ml-cli --lib training_selection -- --test-threads=8`: 6 passed.
- Unfeatured `resolve_local_spoke_base_does_not_invent_a_vram_reading`: 0 tests. `train_arm` is `cfg(feature = "gpu")`.
- With `--features gpu`: that source-slice test passed (1).
- Unfeatured `apply_preload_free_mb`: 0 tests. Schola train is compiled with `gpu`.
- With `--features gpu`: 4 `apply_preload_free_mb*` tests passed. `run_train_checks_preload_before_select_mens_backend` passed under the same feature (its name does not match the `apply_preload_free_mb` filter).
- `rustfmt --edition 2024 --check` on the five Rust files: exit 0.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] gpu feature**
- **Found during:** Task 2
- **Issue:** `train_arm` and schola `run_train` tests are not in the default lib test binary.
- **Fix:** Reran the plan filters with `--features gpu`.
- **Files modified:** none
- **Commit:** n/a

**2. [Rule 1 - Bug] unexpected cfg**
- **Found during:** Task 2
- **Issue:** `#[cfg(feature = "hw-probe-live-test")]` is a compile error on `vox-ml-cli` because the feature was not declared. The two existing tests call `resolve_local_spoke_base`, which now probes.
- **Fix:** Declared an empty `hw-probe-live-test` feature and left those tests behind it. Default unit tests do not spawn nvidia-smi or create a Metal device.
- **Files modified:** `crates/vox-ml-cli/Cargo.toml`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`
- **Commit:** 0e4a5517a

**3. [process] Tests and implementation share each task commit**
- Same as plan 01. No separate red commit.

**4. [Rule 2] Non-Qwen ids do not require a live reading**
- `apply_preload_free_mb` returns `org/Manual` unchanged. `run_train` calls `read_accelerator_free_mb` only for an automatic pick, a Qwen3 size spelling, or an id whose repo contains `Qwen3-`. Other ids pass `free_mb` 0 into the helper, which returns them unchanged before using the reading.

## Self-Check: PASSED

- Commits `26b058797`, `0e4a5517a`, and `839bffed0` are ancestors of HEAD.
