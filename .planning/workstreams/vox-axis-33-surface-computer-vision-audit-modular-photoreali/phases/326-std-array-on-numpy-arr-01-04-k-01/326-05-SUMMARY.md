---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 05
subsystem: compiler
tags: [numpy, std.array, typeck, array-plan, contracts]

requires:
  - phase: 326-std-array-on-numpy-arr-01-04-k-01
    provides: py_array_stats.vox, python_array_ops(), and the seven-row allowlist
provides:
  - Ty::Array lowered through HirType::Generic, with z-score methods typechecked
  - Named std correction and a python capability required only on to_list
  - Indexed python allowlist, its schema, and the stats-slice plan IR schema
affects: [326-06, 326-07]

actuals:
  tokens: 8627
  tasks: 3
  commits: 3
commits: 3
plan_head_before: 121d83bb9570ae6a842859c29ec307f29b610cc2
plan_head_after: 6f4b5459decba9756632cf85f16a3cec94d359ff

tech-stack:
  added: []
  patterns:
    - "Array method types are parsed from python_array_ops(); Array[...] on those methods is GenericParam(0)"
    - "to_list is the only Array method that requires HirCapability::Python"

key-files:
  created:
    - contracts/python/ecosystem-support.schema.json
    - contracts/python/plan-ir.v1.schema.json
  modified:
    - crates/vox-compiler/src/typeck/ty.rs
    - crates/vox-compiler/src/typeck/builtins.rs
    - crates/vox-compiler/src/typeck/checker/expr.rs
    - crates/vox-compiler/src/typeck/checker/expr_ops.rs
    - crates/vox-compiler/src/builtin_registry.rs
    - crates/vox-compiler/src/typeck/checker/expr_field.rs
    - contracts/index.yaml
    - examples/golden/py_array_stats.vox

key-decisions:
  - "Array of f64 is Ty::Array(Named f64), displayed as Array of f64, and lowered as HirType::Generic Array"
  - "std accepts only the named argument correction and does not default it"
  - "Plan methods add no capability; to_list on Ty::Array requires uses python"

patterns-established:
  - "Pattern: std.array signatures are read from python_array_ops, not a second match of op names"
  - "Pattern: Sub and Div of Ty::Array run before host numeric promotion"

requirements-completed: [ARR-01, ARR-02]

coverage:
  - id: D1
    description: "py_array_stats.vox typechecks. to_list on Array of f64 is Result of list of float and requires uses python. Plan methods do not."
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/typeck/builtins.rs#zscore_golden_typechecks"
        status: pass
    human_judgment: false
  - id: D2
    description: "std(correction=0) typechecks. std() and std(0) are diagnostics. Same-dtype sub and div, including a rank-0 plan against a rank-1 plan, are Array of f64. A mismatch names astype."
    requirement: ARR-02
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/typeck/builtins.rs#zscore_golden_typechecks"
        status: pass
    human_judgment: false
  - id: D3
    description: "std.array.from_list of a list of float is Array of f64, and the allowlist, its schema, and plan IR are indexed."
    requirement: ARR-01
    verification:
      - kind: unit
        ref: "crates/vox-compiler/src/builtin_registry.rs#std_array_field_resolves"
        status: pass
      - kind: other
        ref: "cargo run -p vox-cli -- ci contracts-index"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-11-03
status: complete
---

# Phase 326 Plan 05: Typecheck the z-score golden Summary

**`Ty::Array` typechecks the z-score golden: sub, div, mean, named `std(correction=)`, and `to_list` returning `Result[list[float]]` only under `uses python`, with the three python contracts indexed.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-03T13:52:43Z
- **Completed:** 2026-11-03T14:17:18Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- `examples/golden/py_array_stats.vox` typechecks. `to_list` on Array of f64 is `Result` of `list` of `float`, and that call requires `HirCapability::Python`. Plan methods do not.
- Sub and div of two `Ty::Array` values with the same inner dtype yield Array of that dtype, including mean/std rank-0 plans against the rank-1 input. A mismatched pair diagnoses and names `astype`, and does not use host numeric promotion. `1 + 1.0` stays float.
- `a.std(correction=0)` typechecks. `a.std()` and `a.std(0)` are diagnostics. `correction` is not defaulted.
- `std.array.from_list` of a list of float is Array of f64. `contracts-index` exits 0 with ids `python-ecosystem-support`, `python-ecosystem-support-schema`, and `python-plan-ir`. The plan-ir name enum is the seven stats-slice ops.

## Task Commits

Each task was committed atomically:

1. **Task 1: Typecheck the z-score golden** - `ea108e9d1` (feat)
2. **Task 2: Resolve std.array as a field** - `702a4a24e` (feat)
3. **Task 3: Schema and contracts index for the seven rows** - `6f4b5459d` (feat)

## Files Created/Modified

- `crates/vox-compiler/src/typeck/ty.rs` - `Ty::Array` signature `Array of <inner>` and `HirType::Generic("Array", …)`
- `crates/vox-compiler/src/typeck/builtins.rs` - `zscore_golden_typechecks`, signature parser, `StdArrayNs.from_list`, dtype token `f64`
- `crates/vox-compiler/src/typeck/checker/expr.rs` - Array instantiation, named `std`, `to_list` python capability
- `crates/vox-compiler/src/typeck/checker/expr_ops.rs` - Array sub/div before host promotion
- `crates/vox-compiler/src/typeck/unify.rs` - resolve, instantiate, occurs, and unify for `Ty::Array`
- `crates/vox-compiler/src/typeck/registration.rs` - `Array[T]` resolves to `Ty::Array`
- `crates/vox-compiler/src/builtin_registry.rs` - `std.array` root field and `std_array_field_resolves`
- `crates/vox-compiler/src/typeck/checker/expr_field.rs` - `StdArrayNs` short name
- `contracts/python/ecosystem-support.schema.json` - op object requires name, kind, signature, handler, expected
- `contracts/python/plan-ir.v1.schema.json` - same envelope; name enum is the seven stats-slice ops
- `contracts/index.yaml` - the three python contract rows
- `examples/golden/py_array_stats.vox` - `pub` before `@uses(python)` so the golden parses

## Decisions Made

- Array dtype `f64` is `Ty::Named("f64")`, not host `Ty::Float`. `to_list` still returns host `list[float]`.
- Signatures are parsed from `python_array_ops()`. On `Ty::Array` methods, `Array[...]` becomes `GenericParam(0)` and `MethodCall` substitutes the receiver inner type. `from_list` stays a concrete Array of f64 on `StdArrayNs`.
- `std` is checked before the positional argument zip and accepts only `correction` by name.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Golden `pub` sat between the decorator and `fn`**
- **Found during:** Task 1 (Typecheck the z-score golden)
- **Issue:** `@uses(python)` then `pub fn` does not parse. The parser accepts `pub` and then decorators, and reports `Expected fn, found pub`.
- **Fix:** Moved `pub` in front of `@uses(python)`. `fn zscore` and the single `to_list` are unchanged.
- **Files modified:** `examples/golden/py_array_stats.vox`
- **Verification:** `zscore_golden_typechecks` parses the golden and passes.
- **Committed in:** `ea108e9d1` (Task 1 commit)

**2. [Rule 2 - Missing Critical] `astype(f64)` had no dtype binding**
- **Found during:** Task 1 (Typecheck the z-score golden)
- **Issue:** The allowlist signature types the `astype` argument as `f64`. An unbound ident is an undefined-variable diagnostic, so the golden could not typecheck.
- **Fix:** Pre-registered root binding `f64` as `Ty::Named("f64")`. It is a dtype token, not host float.
- **Files modified:** `crates/vox-compiler/src/typeck/builtins.rs`
- **Verification:** `zscore_golden_typechecks` passes, including `astype(f64)`.
- **Committed in:** `ea108e9d1` (Task 1 commit)

**3. [Rule 3 - Blocking] `Ty::Array` had to survive instantiate, resolve, and unify**
- **Found during:** Task 1 (Typecheck the z-score golden)
- **Issue:** `instantiate_inner` and `resolve` did not walk a new variant, so `GenericParam(0)` inside `Array` would not become the receiver dtype.
- **Fix:** Added `Ty::Array` arms in `unify.rs` and `resolve_hir_type` for `Array[T]`.
- **Files modified:** `crates/vox-compiler/src/typeck/unify.rs`, `crates/vox-compiler/src/typeck/registration.rs`
- **Verification:** `array_to_hir_type_is_generic_named_array` and `array_annotation_resolves_to_ty_array` pass.
- **Committed in:** `ea108e9d1` (Task 1 commit)

**4. [Rule 3 - Blocking] tdd-guard rejects a staged file that has `pub fn` and no in-file test**
- **Found during:** Task 1 (Typecheck the z-score golden)
- **Issue:** Editing `expr.rs`, `checker/mod.rs`, and `registration.rs` failed the pre-commit hook. Those files already exposed `pub fn` and had no `#[test]`.
- **Fix:** Added one focused test in each file: mismatch names `astype`, an empty module typechecks, and `Array[f64]` resolves to `Ty::Array`.
- **Files modified:** `crates/vox-compiler/src/typeck/checker/expr.rs`, `crates/vox-compiler/src/typeck/checker/mod.rs`, `crates/vox-compiler/src/typeck/registration.rs`
- **Verification:** The three tests pass, and the Task 1 commit hook reported no issues.
- **Committed in:** `ea108e9d1` (Task 1 commit)

---

**Total deviations:** 4 auto-fixed (1 bug, 1 missing critical, 2 blocking)
**Impact on plan:** The golden parses and typechecks. No new `HirType`, no lexer keyword, no crate edge, no ladder row.

## TDD Gate Compliance

Plan type is `execute`. Tasks 1 and 2 set `tdd="true"`. RED was observed, then each task was committed once after green:

- Task 1: `zscore_golden_typechecks` failed because `from_list` was not found on `ImportPlaceholder("array")`.
- Task 2: `std_array_field_resolves` failed with `Unknown std submodule or field 'array'`.

There is no separate `test(326-05)` commit. The dispatch asked for one commit per task.

## Issues Encountered

None beyond the deviations above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The z-score golden typechecks. The worker that runs `to_list` is still plan 06.
- Add, mul, sort, item, and shape are not in this plan. Plan IR does not name them.
- `STATE.md`, `ROADMAP.md`, and `REQUIREMENTS.md` were left untouched.

---
*Phase: 326-std-array-on-numpy-arr-01-04-k-01*
*Completed: 2026-11-03*

## Self-Check: PASSED

- FOUND: `ea108e9d1`
- FOUND: `702a4a24e`
- FOUND: `6f4b5459d`
- FOUND: `.planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-05-SUMMARY.md`
- FOUND: `contracts/python/ecosystem-support.schema.json`
- FOUND: `contracts/python/plan-ir.v1.schema.json`
- FOUND: `examples/golden/py_array_stats.vox`
