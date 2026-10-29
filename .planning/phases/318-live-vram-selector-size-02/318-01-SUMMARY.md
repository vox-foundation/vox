---
phase: 318-live-vram-selector-size-02
plan: 01
subsystem: training
tags: [qwen3, qlora, vram, vox-populi]

requires: []
provides:
  - Measured qwen3_code QLoRA pick from an injected free-memory reading
  - Explicit Qwen3 size refusal when unmeasured or over floor
  - One free-MiB reading from CUDA memory.free or Metal memory_free_mb
affects: [318-02, 318-03]

actuals:
  tokens: 3984
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Largest measured QLoRA floor goes through pick_base"
    - "Explicit sizes are refused, never substituted"

key-files:
  created: []
  modified:
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs

key-decisions:
  - "1.7B and 4B stay ineligible because floor_measured is false"
  - "CUDA free wins when present; Metal memory_free_mb is the fallback"
  - "Unit tests inject readings and do not call read_accelerator_free_mb"

patterns-established:
  - "select_qwen3_code_qlora_for_free_mb is the only max-by-floor loop"

requirements-completed: [SIZE-02]

coverage:
  - id: D1
    description: Largest measured qwen3_code QLoRA floor that fits an injected reading, with a pre-load step-down that names both sizes
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-populi --lib spoke_base_resolver --features mens
        status: pass
    human_judgment: false
  - id: D2
    description: Explicit Qwen3 spellings fail when unmeasured or over floor and are not replaced
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: explicit_qwen3_size_refuses_unmeasured_and_over_floor
        status: pass
    human_judgment: false

duration: 30min
completed: 2026-10-29
status: complete
plan_head_before: 07f504dfbcce790cf2ea44325583308aeecb3fb3
plan_head_after: ba5e86db075b4504221218752625d764564f035f
---

# Phase 318 Plan 01: Measured QLoRA pick from free memory Summary

**The largest measured qwen3_code QLoRA floor that fits an injected free-memory reading wins, and an explicit size that does not fit is refused.**

## Accomplishments

- `select_qwen3_code_qlora_for_free_mb` filters `qwen3_code` rows whose methods contain `qlora` and calls `pick_base`.
- `confirm_before_load` keeps an automatic pick whose floor still fits and otherwise steps down through that same picker.
- `explicit_qwen3_size_for_free_mb` errors for 1.7B, 4B, and any floor above the reading.
- `free_mb_from_readings` uses CUDA `free_gb` when present, otherwise Metal `memory_free_mb`.

## Tests

- Unfeatured `cargo test -p vox-populi --lib spoke_base_resolver` ran 0 tests (25 filtered, exit 0). Not a pass.
- `cargo test -p vox-populi --lib spoke_base_resolver --features mens -- --test-threads=8`: 36 passed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Feature gate**
- **Found during:** Task 1
- **Issue:** `spoke_base_resolver` compiles only under `vox-populi` feature `mens`. The unfeatured filter exits 0 with 0 tests.
- **Fix:** Reran with `--features mens`.
- **Files modified:** none
- **Commit:** n/a

**2. [process] Tests and implementation share each task commit**
- Plan type is `execute` and `tdd_mode` is false. A separate red compile of `--features mens` is about ten minutes, so each task commit contains the test and the implementation. Tests were written before the green run.

## Self-Check: PASSED

- Commits `5077c9ea5`, `59e037472`, and `ba5e86db0` are ancestors of HEAD.
- `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` contains the selector.
