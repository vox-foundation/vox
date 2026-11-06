---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 01
subsystem: compiler
tags: [numpy, std.array, array-plan, golden, interp]

requires:
  - phase: 325-bridge-runtime-environment-br-01-04-gov-01
    provides: vox-python-bridge crate edge, Bridge::call, and HirCapability::Python
provides:
  - Pure ArrayPlan z-score for [1.0, 3.0] with zero worker calls
  - Seven-row contracts/python/ecosystem-support.yaml and generated python_array_ops table
  - examples/golden/py_array_stats.vox listed in KNOWN_INTERP_GAPS
affects: [326-02, 326-05, 326-06]

actuals:
  tokens: 5694
  tasks: 2
  commits: 2
commits: 2
plan_head_before: fbe41d5eec2748a7936cad75bdc06faceabe820c
plan_head_after: 71d553666e3a217d2369c757bce1c7b4ac0cdabf

tech-stack:
  added: []
  patterns:
    - "std.array ops are a generated signature table, separate from GENERATED_RUST_INTEROP_POLICY"
    - "ArrayPlan records op names and a worker-call count that plan builders never increment"

key-files:
  created:
    - contracts/python/ecosystem-support.yaml
    - crates/vox-compiler/src/python_array_support.rs
    - crates/vox-compiler/src/eval/array_plan.rs
    - examples/golden/py_array_stats.vox
  modified:
    - crates/vox-compiler/build.rs
    - crates/vox-compiler/src/lib.rs
    - crates/vox-compiler/src/eval/mod.rs
    - crates/vox-compiler/src/eval/value.rs
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/tests/golden_interp_test.rs

key-decisions:
  - "mean and std return rank-0 plans and copy a worker-call counter that stays at zero"
  - "build.rs panics unless every row has a signature, a handler equal to its name, a known kind, and a non-null expected object"
  - "py_array_stats.vox is an in-process interp gap until plan 06 runs it through the worker"

patterns-established:
  - "Pattern: python array allowlist is contracts/python/ecosystem-support.yaml, emitted to OUT_DIR/python_array_ops.rs"
  - "Pattern: VoxValue::ArrayPlan wraps the pure plan; materializers are not called in this plan"

requirements-completed: [ARR-01]

coverage:
  - id: D1
    description: "Pure z-score plan for from_list of [1.0, 3.0], astype f64, subtract mean, divide std with correction 0, with zero worker calls and seven yaml rows that each have expected"
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/eval/array_plan.rs#array_plan_zscore_is_pure"
        status: pass
    human_judgment: false
  - id: D2
    description: "Golden py_array_stats.vox asserts [-1.0, 1.0] with one to_list, and KNOWN_INTERP_GAPS lists it so the in-process sweep does not execute it"
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/tests/golden_interp_test.rs#py_array_stats_is_an_interp_gap"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-11-03
status: complete
---

# Phase 326 Plan 01: Z-score plan and golden source Summary

**Pure z-score `ArrayPlan` for `[1.0, 3.0]` records `from_list`, `astype`, `sub`, `div`, `mean`, and `std` with zero worker calls, and the golden that asserts `[-1.0, 1.0]` is an interp-sweep gap.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-03T13:36:18Z
- **Completed:** 2026-11-03T13:51:03Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- `ArrayPlan::zscore` builds from_list of `[1.0, 3.0]`, astype f64, subtract mean, divide std with correction 0. Mean and std stay rank 0. The plan's worker-call count stays 0.
- `contracts/python/ecosystem-support.yaml` lists exactly `from_list`, `astype`, `sub`, `div`, `mean`, `std`, `to_list`. Each row has a signature, a handler equal to its name, and a non-null expected object. `std` requires named `correction`. Empty mean and empty std are NaN. `to_list` records host `list_f64` for `[1.0, 3.0]` and the z-score list `[-1.0, 1.0]`.
- `build.rs` writes `OUT_DIR/python_array_ops.rs` and panics when signature, handler, or expected is missing, or the kind is unknown. The rows are not folded into `GENERATED_RUST_INTEROP_POLICY`.
- `examples/golden/py_array_stats.vox` calls `to_list` once and asserts `[-1.0, 1.0]`. `KNOWN_INTERP_GAPS` contains `("py_array_stats.vox", "needs the uv python worker")`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Pure z-score plan, zero worker calls** - `64a990a0f` (feat)
2. **Task 2: Golden source and the interp-sweep gap** - `71d553666` (feat)

## Files Created/Modified

- `contracts/python/ecosystem-support.yaml` - seven z-score rows with expected objects
- `crates/vox-compiler/build.rs` - emits `python_array_ops.rs` and refuses an incomplete row
- `crates/vox-compiler/src/python_array_support.rs` - `python_array_ops()` over the generated table
- `crates/vox-compiler/src/eval/array_plan.rs` - `ArrayPlan` and `array_plan_zscore_is_pure`
- `crates/vox-compiler/src/eval/value.rs` - `VoxValue::ArrayPlan`
- `crates/vox-compiler/src/eval/mod.rs` - declares `array_plan`
- `crates/vox-compiler/src/eval/builtins.rs` - type-name and display arms for `ArrayPlan`
- `crates/vox-compiler/src/lib.rs` - declares `python_array_support`
- `examples/golden/py_array_stats.vox` - z-score golden with one `to_list`
- `crates/vox-compiler/tests/golden_interp_test.rs` - gap entry and `py_array_stats_is_an_interp_gap`

## Decisions Made

- Plan construction copies `plan_worker_calls()` and never increments it. Mean and std therefore record zero worker calls.
- A row counts only when `expected` is a YAML mapping. `build.rs` panics on null, on a missing field, and on an unknown kind.
- The golden source is not an input to expected values. The unit test reads the yaml and the generated table.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Exhaustive `VoxValue` matches needed `ArrayPlan` arms**
- **Found during:** Task 1 (Pure z-score plan, zero worker calls)
- **Issue:** `vox_value_type_name` and `vox_value_display` match every variant. Adding `VoxValue::ArrayPlan` would not compile without arms.
- **Fix:** Added `ArrayPlan` to both matches. Display prints the plan with `Debug`.
- **Files modified:** `crates/vox-compiler/src/eval/builtins.rs`
- **Verification:** `cargo test -p vox-compiler --lib array_plan_zscore_is_pure` passed (1 passed).
- **Committed in:** `64a990a0f` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Required for the new value variant to compile. No new ops, no worker call, no crate edge.

## TDD Gate Compliance

Plan type is `execute`. Both tasks set `tdd="true"`. RED was observed in this session, then each task was committed once after green:

- Task 1: the first `array_plan_zscore_is_pure` run failed because serde_yaml re-serializes flow lists as block lists, so `expected_yaml.contains("[1.0, 3.0]")` was false. The test now parses the generated yaml and still checks the source file for `identity: [1.0, 3.0]` and `zscore: [-1.0, 1.0]`.
- Task 2: `py_array_stats_is_an_interp_gap` failed with `py_array_stats.vox must be an interp gap` before the slice entry existed.

There is no separate `test(326-01)` commit before the `feat(326-01)` commits. The dispatch asked for one commit per task.

## Issues Encountered

None remaining. Both scoped tests passed on the committed tree's sources.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 02 can add `add`, `mul`, `sort`, `item`, and `shape` on this table and plan type.
- Plan 05 typechecks `examples/golden/py_array_stats.vox`. Plan 06 is the `vox run`. This plan does not call `Bridge::call` and does not install numpy.
- `KNOWN_INTERP_GAPS` must keep `py_array_stats.vox` until that run exists.

## Verification

- `cargo test -p vox-compiler --lib array_plan_zscore_is_pure` — ok, 1 passed, 0 failed.
- `cargo test -p vox-compiler --test golden_interp_test py_array_stats_is_an_interp_gap` — ok, 1 passed, 0 failed. An earlier run failed before the gap entry was added.

## Self-Check: PASSED

- FOUND: contracts/python/ecosystem-support.yaml
- FOUND: crates/vox-compiler/src/python_array_support.rs
- FOUND: crates/vox-compiler/src/eval/array_plan.rs
- FOUND: examples/golden/py_array_stats.vox
- FOUND: 64a990a0f
- FOUND: 71d553666

---
*Phase: 326-std-array-on-numpy-arr-01-04-k-01*
*Completed: 2026-11-03*
