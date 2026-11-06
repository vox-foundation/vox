---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 02
subsystem: runtime
tags: [numpy, std.array, python-bridge, array-plan]

requires:
  - phase: 326-01
    provides: seven z-score rows and the generated python_array_ops table
  - phase: 326-06
    provides: HANDLERS for those seven names and materialize_to_list
provides:
  - twelve D-01 yaml rows ending in item and shape
  - sort as an ArrayPlan that records nulls and does not call the worker
  - item and shape as one materialize_to_list call each, gated on HirCapability::Python
affects: [326-04, 326-07, 326-08]

actuals:
  tokens: 9352
  tasks: 2
  commits: 2
commits: 2
plan_head_before: 40bbbdb93b77e94d089ebf893731566c657f0bdd
plan_head_after: 97d5ff2a58af72dca918c711be991b85820026c0

tech-stack:
  added: []
  patterns:
    - sort stores nulls on the plan step; the worker partitions NaNs and does not pass nulls to np.sort
    - item and shape append one op and call materialize_to_list; the wire method stays execute_plan
    - add and mul are binary operators on ArrayPlan, with the same 0-d broadcast as sub and div

key-files:
  created:
    - crates/vox-compiler/tests/python_bridge_parity_test.rs
    - crates/vox-compiler/tests/python_bridge_parity.rs
  modified:
    - contracts/python/ecosystem-support.yaml
    - contracts/python/plan-ir.v1.schema.json
    - crates/vox-python-bridge/worker/vox_bridge/array_ops.py
    - crates/vox-compiler/src/typeck/builtins.rs
    - crates/vox-compiler/src/eval/array_plan.rs
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/src/eval/expr.rs
    - crates/vox-compiler/src/typeck/checker/expr.rs
    - crates/vox-compiler/src/typeck/checker/expr_ops.rs

key-decisions:
  - "item's generated signature is Result[float], the f64 host scalar, matching to_list's float mapping."
  - "add and mul are excluded from Ty::Array method lookup. Their signatures still come from python_array_ops."
  - "The cargo target python_bridge_parity is a one-line path module over python_bridge_parity_test.rs so the plan command works without a Cargo.toml edit."

patterns-established:
  - "Plan builders for add, mul, and sort copy plan_worker_calls and never increment it."
  - "Materializers item and shape require uses python. sort does not."

requirements-completed: [ARR-01]

coverage:
  - id: D1
    description: "The yaml ops list is the twelve D-01 names in order, and each row joins a generated signature, a column-0 def, a HANDLERS key, and a non-null expected object."
    requirement: ARR-01
    verification:
      - kind: integration
        ref: "crates/vox-compiler/tests/python_bridge_parity_test.rs#on_disk_expected_matches_behavior_cases"
        status: pass
      - kind: unit
        ref: "crates/vox-compiler/src/eval/array_plan.rs#array_plan_zscore_is_pure"
        status: pass
    human_judgment: false
  - id: D2
    description: "sort of an ArrayPlan returns another ArrayPlan, stores nulls=first or nulls=last, and records zero worker calls."
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/eval/array_plan.rs#sort_is_a_plan"
        status: pass
    human_judgment: false
  - id: D3
    description: "item and shape each call materialize_to_list once. The denial is py.to_list. No Bridge::call method is named item or shape."
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/eval/builtins.rs#item_shape_call_execute_plan"
        status: pass
    human_judgment: false
  - id: D4
    description: "item and shape without uses python are diagnostics. With the effect they typecheck. sort does not require the effect."
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/typeck/checker/expr.rs#item_shape_require_python"
        status: pass
    human_judgment: false
  - id: D5
    description: "On-disk expected objects match the six behavior cases for add, mul, sort nulls=first, sort nulls=last, item, and shape."
    requirement: ARR-01
    verification:
      - kind: integration
        ref: "cargo test -p vox-compiler --test python_bridge_parity on_disk_expected_matches_behavior_cases"
        status: pass
    human_judgment: false

duration: 16min
completed: 2026-11-03
status: complete
---

# Phase 326 Plan 02: Make the other five ops callable Summary

**add, mul, and sort build ArrayPlan values, and item and shape each call `materialize_to_list` once under the python capability.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-11-03T23:09:42Z
- **Completed:** 2026-11-03T23:25:05Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- The yaml ops list is `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. The plan-ir name enum is those twelve names.
- `sort` appends a plan step with `nulls` of `first` or `last`. The worker partitions NaNs, sorts the finite values with `np.sort`, and concatenates. `nulls=first` puts NaNs first.
- `item` and `shape` append their op and call `materialize_to_list`. They require `HirCapability::Python`. `sort` does not. `add` and `mul` broadcast a rank-0 operand the same way `sub` and `div` do.
- `on_disk_expected_matches_behavior_cases` locks add `[2.0, 4.0]`, mul `[8.0, 15.0]`, sort nulls=first `[NaN, 1.0, 3.0]`, sort nulls=last `[1.0, 3.0, NaN]`, item host float `2.0`, and shape `[2]`.

## Task Commits

1. **Task 1: Rows, handlers, and method lookup for the five names** - `46e96526b` (feat)
2. **Task 2: sort is a plan, item and shape call execute_plan** - `97d5ff2a5` (feat)

## Files Created/Modified

- `contracts/python/ecosystem-support.yaml` - twelve stats-slice rows and the five expected objects
- `contracts/python/plan-ir.v1.schema.json` - legal node names are the twelve D-01 ops
- `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` - column-0 `add`, `mul`, `sort`, `item`, and `shape`
- `crates/vox-compiler/src/typeck/builtins.rs` - `Ty::Array` methods gain `sort`, `item`, and `shape`; `add` and `mul` stay off the method map
- `crates/vox-compiler/src/eval/array_plan.rs` - `add`, `mul`, and `sort` plan builders
- `crates/vox-compiler/src/eval/builtins.rs` - `materialize_item` and `materialize_shape`
- `crates/vox-compiler/src/eval/expr.rs` - binary `+`/`*` and the item/shape materializer dispatch
- `crates/vox-compiler/src/typeck/checker/expr.rs` - python capability for item and shape
- `crates/vox-compiler/src/typeck/checker/expr_ops.rs` - array `+` and `*` use the same dtype rule as sub and div
- `crates/vox-compiler/tests/python_bridge_parity_test.rs` - on-disk expected objects
- `crates/vox-compiler/tests/python_bridge_parity.rs` - cargo target name `python_bridge_parity`

## Decisions Made

- `item()` is `Result[float]` in the generated table. That is the f64 host scalar, the same float mapping `to_list` uses for this slice.
- `add` and `mul` are not methods. Lookup still reads signatures from `python_array_ops`.
- A one-line `#[path]` test crate supplies the cargo target `python_bridge_parity`. `Cargo.toml` was not edited.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `matches!` on `&String` does not accept `&str` patterns**
- **Found during:** Task 2
- **Issue:** The first `vox-compiler` compile failed with E0308 in `eval/expr.rs` and `typeck/checker/expr.rs`.
- **Fix:** Compare `method.as_str()`.
- **Files modified:** `crates/vox-compiler/src/eval/expr.rs`, `crates/vox-compiler/src/typeck/checker/expr.rs`
- **Verification:** `item_shape_require_python` and the lib tests passed
- **Committed in:** `97d5ff2a5`

**2. [Rule 1 - Bug] `array_plan_zscore_is_pure` still banned the five new names**
- **Found during:** Task 1
- **Issue:** The existing test required exactly the seven z-score names and rejected `add`, `mul`, `sort`, `item`, and `shape`.
- **Fix:** Expect the twelve D-01 names, treat `item` and `shape` as materializers, and require `sort`'s `required_named` to be `nulls`.
- **Files modified:** `crates/vox-compiler/src/eval/array_plan.rs`
- **Verification:** `array_plan_zscore_is_pure` passed (1 passed)
- **Committed in:** `46e96526b`

**3. [Rule 2 - Missing critical] add, mul, item, and shape were not reachable from eval and typeck**
- **Found during:** Task 2
- **Issue:** `sub` and `div` are binary arms in `eval/expr.rs` and `expr_ops.rs`. `to_list` is dispatched in `eval/expr.rs` because `materialize_to_list` needs the interpreter. Those files were not in the plan's file list.
- **Fix:** Add the same arms for `+` and `*`, and dispatch `item` and `shape` through `materialize_item` and `materialize_shape`.
- **Files modified:** `crates/vox-compiler/src/eval/expr.rs`, `crates/vox-compiler/src/typeck/checker/expr_ops.rs`
- **Verification:** `add_mul_are_plans` and `item_shape_require_python` passed
- **Committed in:** `97d5ff2a5`

**4. [Rule 3 - Blocking] cargo target name did not match the plan command**
- **Found during:** Task 2
- **Issue:** `tests/python_bridge_parity_test.rs` registers as `--test python_bridge_parity_test`. The plan command is `--test python_bridge_parity`. Editing `Cargo.toml` was forbidden.
- **Fix:** Add `tests/python_bridge_parity.rs` with `#[path = "python_bridge_parity_test.rs"]`.
- **Files modified:** `crates/vox-compiler/tests/python_bridge_parity.rs`
- **Verification:** `cargo test -p vox-compiler --test python_bridge_parity on_disk_expected_matches_behavior_cases` exited 0 with 1 passed and 0 filtered out
- **Committed in:** `97d5ff2a5`

**5. [Rule 2 - Missing critical] `add` and `mul` needed a same-file test**
- **Found during:** Task 1
- **Issue:** The new `pub fn` plan builders need a test in `array_plan.rs`. A per-op `@test` is still absent.
- **Fix:** `add_mul_are_plans` checks both ops return an `ArrayPlan`, record zero worker calls, and broadcast a rank-0 addend.
- **Files modified:** `crates/vox-compiler/src/eval/array_plan.rs`
- **Verification:** `cargo test -p vox-compiler --lib add_mul_are_plans` exited 0 with 1 passed
- **Committed in:** `46e96526b`

---

**Total deviations:** 5 auto-fixed (2 Rule 1, 2 Rule 2, 1 Rule 3)
**Impact on plan:** The five ops are callable as specified. No new HirType, keyword, crate, dependency, or `@test`.

## TDD Gate Compliance

Plan `type` is `execute`. Both tasks set `tdd="true"`, and Task 2 says to write the four tests first. There is no `test(326-02):` commit. `cargo test` output is not a supported RED-evidence report, so a RED commit was not recorded. The four named tests were run after the implementation and passed.

## Authentication Gates

None.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required. This plan installed nothing.

## Next Phase Readiness

Ready for 326-04. `nulls=` diagnostics stay in 326-07. The synthetic incomplete row and `vox ci python-bridge-parity` stay in 326-08.

STATE.md, ROADMAP.md, and REQUIREMENTS.md were left unstaged. They were already dirty, and this run was told not to commit them.

## Self-Check: PASSED

- FOUND: contracts/python/ecosystem-support.yaml
- FOUND: contracts/python/plan-ir.v1.schema.json
- FOUND: crates/vox-python-bridge/worker/vox_bridge/array_ops.py
- FOUND: crates/vox-compiler/src/eval/array_plan.rs
- FOUND: crates/vox-compiler/tests/python_bridge_parity_test.rs
- FOUND: crates/vox-compiler/tests/python_bridge_parity.rs
- FOUND: 46e96526b
- FOUND: 97d5ff2a5

## Known Stubs

None.
