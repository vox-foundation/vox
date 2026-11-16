---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 07
subsystem: compiler
tags: [numpy, std.array, typeck, dtype, nulls]

requires:
  - phase: 326-std-array-on-numpy-arr-01-04-k-01
    provides: Array arm for same-dtype arithmetic, named std correction, and sort on Ty::Array
provides:
  - Compile errors that name astype for mixed array add, sub, mul, and div
  - sort accepts only nulls=first or nulls=last
  - from_list maps list[int] to Array of i64 and list[bool] to Array of bool
  - to_list on those dtypes returns the host list, and i32/f32 exist only via astype
affects: [326-04, 326-08]

actuals:
  tokens: 5368
  tasks: 2
  commits: 4
commits: 4
plan_head_before: 22493deac4c247f4c9952814561f670f8ed03c7e
plan_head_after: 651a6267ce7fe072cbda6e0ee00c30674a801648

tech-stack:
  added: []
  patterns:
    - "Array add, sub, mul, and div share array_same_dtype_op and do not fall through to host promotion"
    - "sort checks HirArg.name before any positional zip; nulls is first or last and is not defaulted"
    - "Host list int and bool map to plan dtypes i64 and bool; i32 and f32 are astype tokens"

key-files:
  created:
    - crates/vox-compiler/tests/array_dtype_and_required_names.rs
  modified:
    - crates/vox-compiler/src/typeck/checker/expr_ops.rs
    - crates/vox-compiler/src/typeck/checker/expr.rs
    - crates/vox-compiler/src/typeck/builtins.rs

key-decisions:
  - "Add and Mul already shared the same-dtype Array arm; this plan locks them in array_dtype_and_required_names and adds the nulls check"
  - "from_list of list[int] is Array of i64, list[bool] is Array of bool, and list[float] stays Array of f64"
  - "mean and std typecheck only on Array of f32 or f64 and the diagnostic names astype"
  - "astype accepts i32, i64, f32, f64, and bool so each plan dtype can be spelled; from_list still never yields i32 or f32"

patterns-established:
  - "Pattern: required array names are checked on HirArg.name before check_arguments zips positionally"
  - "Pattern: host-list dtype mapping lives in typeck, not in a new yaml row"

requirements-completed: [ARR-02]

coverage:
  - id: D1
    description: "Mixed Array i64/f64 arithmetic under add, sub, mul, and div, plus i32 versus i64, is a diagnostic that names astype. Same-dtype add and mul, including a 0-d mean, stay Array of f64. std() and positional std(0) diagnose; std(correction=0) typechecks."
    requirement: ARR-02
    verification:
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#mixed_array_arithmetic_names_astype"
        status: pass
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#std_requires_named_correction"
        status: pass
    human_judgment: false
  - id: D2
    description: "sort() and positional sort(\"first\") are diagnostics that name nulls. nulls=first and nulls=last typecheck on Array of f64. Any other string is a diagnostic."
    requirement: ARR-02
    verification:
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#sort_requires_nulls_first_or_last"
        status: pass
    human_judgment: false
  - id: D3
    description: "from_list of list[int] is Array of i64 and to_list returns Result of list of int. from_list of list[bool] is Array of bool and to_list returns Result of list of bool. mean and std on Array of i64 diagnose. They typecheck on Array of f32 and f64. astype(i32) and astype(f32) produce those dtypes; from_list does not."
    requirement: ARR-02
    verification:
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#from_list_int_and_bool_map_to_host_lists"
        status: pass
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#mean_and_std_only_on_f32_and_f64"
        status: pass
      - kind: integration
        ref: "crates/vox-compiler/tests/array_dtype_and_required_names.rs#i32_and_f32_exist_only_after_astype"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-11-03
status: complete
---

# Phase 326 Plan 07: Dtype lattice and required names Summary

**Mixed array arithmetic names `astype`, `sort` accepts only `nulls=first|last`, and `from_list` maps host int and bool to Array of i64 and bool.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-03T23:31:07Z
- **Completed:** 2026-11-03T23:45:31Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `array_dtype_and_required_names` covers add, sub, mul, and div of Array of i64 with Array of f64, and add of Array of i32 with Array of i64. Each diagnostic names `astype`. Same-dtype add and mul, including `a + a.mean()`, stay Array of f64. Host scalar promotion is not asserted.
- `a.std()` and `a.std(0)` are diagnostics. `a.std(correction=0)` typechecks on Array of f64.
- `a.sort()` and `a.sort("first")` are diagnostics that name `nulls`. `nulls="first"` and `nulls="last"` typecheck. `nulls="middle"` is a diagnostic. `nulls` is not defaulted.
- `from_list` of `list[int]` is Array of i64 and of `list[bool]` is Array of bool. `to_list` on those dtypes returns Result of list of int and Result of list of bool. `mean` and `std` diagnose on Array of i64 and typecheck on Array of f32 and f64. `astype(i32)` and `astype(f32)` produce those dtypes. `from_list` does not.

## Task Commits

Each task was committed atomically:

1. **Task 1 RED: Diagnostics for mixed dtypes and nulls=** - `f10d75fdf` (test)
2. **Task 1 GREEN: reject sort unless nulls is first or last** - `49271be8b` (feat)
3. **Task 2 RED: Integer, bool, and astype-only dtypes** - `de2d08349` (test)
4. **Task 2 GREEN: map host lists and astype-only dtypes** - `651a6267c` (feat)

**Plan metadata:** this summary commit.

## Files Created/Modified

- `crates/vox-compiler/tests/array_dtype_and_required_names.rs` - Integration coverage for mixed dtypes, `std`, `sort`, host lists, and astype-only dtypes
- `crates/vox-compiler/src/typeck/checker/expr_ops.rs` - `nulls` check, `from_list` host mapping, `astype` tokens, float-only `mean`/`std`, `to_list` host result
- `crates/vox-compiler/src/typeck/checker/expr.rs` - Calls those checks without skipping inferred-type recording
- `crates/vox-compiler/src/typeck/builtins.rs` - Registers plan dtype tokens `f64`, `f32`, `i64`, and `i32`

## Decisions Made

- Add and Mul already used `array_same_dtype_op`. The new test locks that behavior, including a 0-d `mean` operand. This plan's new arithmetic work is the i32/i64 and four-op diagnostic coverage, not a second arm.
- `sort` checks `HirArg.name` before `check_arguments` can zip a positional string onto `str`.
- Host `list[int]` becomes Array of i64, host `list[bool]` becomes Array of bool, host `list[float]` stays Array of f64. Signatures stay on the generated table.
- `mean` and `std` are legal only when the inner dtype is `f32` or `f64`. The diagnostic names `astype`.
- `astype` accepts `i32`, `i64`, `f32`, `f64`, and `bool`. `i32` and `f32` are not results of `from_list`.

## Deviations from Plan

None - plan executed exactly as written.

The sort call site is in `expr.rs`, which the plan's `files_modified` already lists. Task 1's `<files>` element named `expr_ops.rs` and the test; the check is useless unless the method arm calls it.

## TDD Gate Compliance

| Task | RED | GREEN | REFACTOR |
|------|-----|-------|----------|
| 1 | `f10d75fdf` | `49271be8b` | — |
| 2 | `de2d08349` | `651a6267c` | — |

Cargo's console report is outside the RED classifier adapters (Rust gap). Semantic inspection:

- Task 1: `cargo test -p vox-compiler --test array_dtype_and_required_names` exited 101. 3 tests ran, 2 passed, 1 failed, 0 filtered out. `sort_requires_nulls_first_or_last` failed because positional `sort("first")` produced no diagnostic. `mixed_array_arithmetic_names_astype` and `std_requires_named_correction` already passed.
- Task 2: same command, 6 tests, 3 failed, 0 filtered out. `from_list` of `list[int]` reported `Cannot unify Float with Int`, `list[bool]` reported `Cannot unify Float with Bool`, and `mean`/`std` on Array of i64 reported no diagnostic.

## Issues Encountered

An early `return` from `check_expr` skipped the inferred-type insert, so `from_list` typechecked but had no span to assert. The match arm now yields the type and the insert runs. That fix is in `651a6267c`. A split borrow was required so `check_expr` and `uf.resolve` do not overlap.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ARR-02 compile errors for mixed dtypes, missing `correction=`, and missing `nulls=` are covered by `array_dtype_and_required_names`.
- Integer and bool arrays materialize, at the type level, to list of int and list of bool.
- Plan 04 and plan 08 were not started.

## Self-Check: PASSED

- FOUND: crates/vox-compiler/tests/array_dtype_and_required_names.rs
- FOUND: crates/vox-compiler/src/typeck/checker/expr_ops.rs
- FOUND: f10d75fdf
- FOUND: 49271be8b
- FOUND: de2d08349
- FOUND: 651a6267c

---
*Phase: 326-std-array-on-numpy-arr-01-04-k-01*
*Completed: 2026-11-03*
