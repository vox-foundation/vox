---
phase: 326-std-array-on-numpy-arr-01-04-k-01
reviewed: 2026-11-04T01:45:00Z
depth: deep
files_reviewed: 46
files_reviewed_list:
  - contracts/cli/command-registry.yaml
  - contracts/eval/complexity-budget.v1.json
  - contracts/eval/source-token-budget.v1.json
  - contracts/index.yaml
  - contracts/operations/catalog.v1.yaml
  - contracts/pipeline/canonical-ladder.v1.yaml
  - contracts/policy/policy-registry.v1.yaml
  - contracts/python/ecosystem-support.schema.json
  - contracts/python/ecosystem-support.yaml
  - contracts/python/plan-ir.v1.schema.json
  - contracts/python/signature-manifest-budget.v1.json
  - crates/vox-cli-ci/src/cmd_enums.rs
  - crates/vox-cli/src/commands/ci/run_body.rs
  - crates/vox-codegen/src/canonical_ladder.rs
  - crates/vox-compiler/build.rs
  - crates/vox-compiler/src/builtin_registry.rs
  - crates/vox-compiler/src/eval/array_plan.rs
  - crates/vox-compiler/src/eval/builtins.rs
  - crates/vox-compiler/src/eval/expr.rs
  - crates/vox-compiler/src/eval/mod.rs
  - crates/vox-compiler/src/eval/python.rs
  - crates/vox-compiler/src/eval/value.rs
  - crates/vox-compiler/src/lib.rs
  - crates/vox-compiler/src/python_array_support.rs
  - crates/vox-compiler/src/typeck/builtins.rs
  - crates/vox-compiler/src/typeck/checker/expr.rs
  - crates/vox-compiler/src/typeck/checker/expr_field.rs
  - crates/vox-compiler/src/typeck/checker/expr_ops.rs
  - crates/vox-compiler/src/typeck/checker/mod.rs
  - crates/vox-compiler/src/typeck/registration.rs
  - crates/vox-compiler/src/typeck/ty.rs
  - crates/vox-compiler/src/typeck/unify.rs
  - crates/vox-compiler/tests/array_dtype_and_required_names.rs
  - crates/vox-compiler/tests/golden_interp_test.rs
  - crates/vox-compiler/tests/py_array_stats_run.rs
  - crates/vox-compiler/tests/python_bridge_parity.rs
  - crates/vox-compiler/tests/python_bridge_parity_test.rs
  - crates/vox-language-surface/src/lib.rs
  - crates/vox-python-bridge/Cargo.toml
  - crates/vox-python-bridge/src/array_ipc.rs
  - crates/vox-python-bridge/src/array_numeric.rs
  - crates/vox-python-bridge/src/lib.rs
  - crates/vox-python-bridge/worker/vox_bridge/__main__.py
  - crates/vox-python-bridge/worker/vox_bridge/array_ops.py
  - docs/src/reference/cli.md
  - examples/golden/py_array_stats.vox
findings:
  critical: 3
  warning: 3
  info: 2
  total: 8
status: issues_found
---

# Phase 326: Code Review Report

**Reviewed:** 2026-11-04T01:45:00Z
**Depth:** deep
**Files Reviewed:** 46
**Status:** issues_found

## Summary

Reviewed the eight plan commits through `98fd7eba3` (parent `fbe41d5ee`). The single-host z-score plan is coherent: `mean` and `std` stay inside the plan JSON, `np.std(..., correction=)` matches NumPy 2.x, a bare int `from_list`/`to_list` keeps `i64`, and `python_bridge_parity_from_status` returns `Err` when cargo's status is non-zero. That cargo-status check is fail-closed. The pre-existing `vox ci command-compliance` gap (missing `py` reachability row and missing `vox_context_handoff` registry entry) is not a phase 326 defect.

Three product bugs sit beside that path. Binary plans replay every `from_list` against the left operand's host, so two distinct arrays do not add. `astype` and the numeric workers then force `f64` even when the type lattice says otherwise. `list[bool]` typechecks and aborts in the interpreter.

## Critical Issues

### CR-01: Binary plans drop the right-hand host

**File:** `crates/vox-compiler/src/eval/array_plan.rs:220-234`
**Issue:** `combine` clones the left plan, appends the right plan's step names, and keeps only `self.host`. `encode_host_column` sends that one column. The worker's `from_list` always copies that same host (`array_ops.py:12-13`). `from_list([1.0, 3.0]) + from_list([1.0, 1.0])` therefore runs as `[1.0, 3.0] + [1.0, 3.0]` and materializes `[2.0, 6.0]`. The registry's expected value is `[2.0, 4.0]`. The same drop breaks `sub`, `mul`, `div`, and `a + b.mean()` whenever the two plans were not built from one host. Z-score survives only because both sides are the same `base`. `add_mul_are_plans` and the parity test check op names and YAML text, not the numbers.
**Fix:**

```rust
fn combine(&self, other: &Self, op: &'static str) -> Self {
    // Carry `other.host` as its own column (or as literal steps).
    // Replaying `other.steps` against `self.host` computes left op left.
    let mut next = self.clone();
    next.steps.extend(other.steps.iter().copied());
    next.steps.push(PlanStep { op, correction: None, nulls: None });
    next
}
```

`from_list` in the worker must load the column that belongs to that step, not column 0 on every call. Add an execute_plan assertion that `[1.0, 3.0] + [1.0, 1.0]` is `[2.0, 4.0]`.

### CR-02: Typed dtypes materialize as float64

**File:** `crates/vox-compiler/src/eval/array_plan.rs:76-77`
**Issue:** `astype` discards `_dtype`. `plan_params` then writes `"dtype": "f64"` for every astype step (`builtins.rs:2801-2802`). The worker rejects anything except `"f64"` and casts with `np.float64` (`array_ops.py:22-24`). `add`, `sub`, `mul`, `div`, `sort`, and `item` also force `float64` (`array_ops.py:34`, `44`, `53`, `63`, `95`, `117`). Typecheck does the opposite: `array_same_dtype_op` returns `Array` of the shared inner dtype (`expr_ops.rs:161-163`), `check_array_astype` returns `Array` of `i32`/`i64`/`f32`/`f64`/`bool` (`expr_ops.rs:311-312`), and `sort`'s signature is rewritten to the receiver dtype (`builtins.rs` `array_op_fn_ty` with `generic_array: true`). `from_list([9223372036854775807]).astype(i64).to_list()` is typed `Result[list[int]]` and comes back as the rounded f64 `9223372036854775808`. `sort` on `Array[i64]` is the same lie. A bare int `from_list`/`to_list` with no cast still preserves `i64`; that path is fine. `item()` is typed `Result[float]` in the registry, so its float cast matches that signature and is not a second defect.
**Fix:**

```rust
pub fn astype(&self, dtype: &str) -> Self {
    self.push_with_dtype("astype", dtype)
}

// plan_params
} else if step.op == "astype" {
    op.insert("dtype".into(), serde_json::Value::String(step.dtype.to_string()));
}
```

Honor that token in the worker (`i64` stays `np.int64`, `f32` narrows, `bool` is 0/1). Do not cast integer `add`/`mul`/`sort` through `float64`. `to_list` of an `Array[i64]` must still decode `DecodedColumn::Ints`.

### CR-03: `list[bool]` typechecks, then the interpreter aborts

**File:** `crates/vox-compiler/src/typeck/checker/expr_ops.rs:264-274`
**Issue:** `check_array_from_list` maps `list[bool]` to `Array` of `bool`, and `array_to_list_ty` types `to_list` as `Result[list[bool]]`. Eval only keeps a non-empty all-int list as `i64`. Any bool hits `_Panic("from_list expects floats, found Bool")` (`builtins.rs:2833-2840`), which `expr.rs` turns into `EvalError::AssertionFailed`. There is no bool Arrow column. A program the lattice accepts cannot materialize, and the failure is not a `Result` the caller can match.
**Fix:**

```rust
VoxValue::Bool(_) => {
    return VoxValue::Result(Err(crate::eval::value::err_str(
        "py-plan: from_list bool is not implemented".to_string(),
    )));
}
```

Until a bool column exists, reject it in `check_array_from_list` as well so the type does not promise `list[bool]`. When it is implemented, encode a bool column and decode it to `VoxValue::Bool`, not `0.0`/`1.0`.

## Warnings

### WR-01: Interp-gap test still requires one `to_list`

**File:** `crates/vox-compiler/tests/golden_interp_test.rs:272`
**Issue:** `py_array_stats_is_an_interp_gap` asserts `src.matches("to_list").count() == 1`. The golden now calls `to_list` three times (z-score, infinities, `i64::MAX`). The assertion fails. Plan 04 updated `py_array_stats_run` to expect three `execute_plan` frames and did not update this count.
**Fix:** Assert `count() == 3`, and keep the gap entry so the in-process sweep still skips the worker golden.

### WR-02: Numeric roundtrip never enters the production reply arm

**File:** `crates/vox-compiler/src/eval/builtins.rs:2639-2642`
**Issue:** Under `cfg(test)`, `materialize_to_list` returns `host_list_from_body` as soon as `take_reply_body_for_test()` is set, before `execute_plan` and before the production `reply.body` match. `array_numeric_roundtrip` only exercises that seam. Deleting `decode_column` from `host_list_from_body` still fails the test, but a change that decodes on the seam and drops or ignores `reply.body` on the real path stays green. That is a fail-open lock on the host decoder the plan said this test would pin.
**Fix:** Inject the bytes as `Reply.body` and let the production arm call `host_list_from_body`. Delete the early return.

### WR-03: Arrow decode panics on a zero-column batch

**File:** `crates/vox-python-bridge/src/array_ipc.rs:75-83`
**Issue:** `read_column` uses `batch.column(0)` with no `num_columns` check. `array_numeric.rs:100-108` does the same. Arrow's `column` panics when the index is out of range. An empty stream is already an `Err`. A batch with zero columns, which a bad worker reply can produce, aborts the host instead of returning `py:`. The worker's own decoder rejects `num_columns < 1`.
**Fix:**

```rust
if batch.num_columns() < 1 {
    return Err("py: arrow decode: empty batch".to_string());
}
Ok(batch.column(0).clone())
```

Apply the same guard in both `read_column` functions.

## Info

### IN-01: Plan-IR index still says seven nodes

**File:** `contracts/index.yaml:945`
**Issue:** The `python-plan-ir` description says the schema covers seven stats-slice nodes. `plan-ir.v1.schema.json` enumerates twelve names.
**Fix:** Say twelve D-01 names.

### IN-02: An empty list is always an f64 plan

**File:** `crates/vox-compiler/src/eval/builtins.rs:2818`
**Issue:** The int column is taken only when `!items.is_empty()`. `from_list` of an empty `list[int]` is stored as `HostColumn::F64`. Typecheck still says `Array` of `i64` for a known empty int list. `to_list` then returns an empty float list.
**Fix:** Treat an empty list whose static element type is int as `from_ints(Vec::new())`, or reject an untyped empty list before building a float plan.

---

_Reviewed: 2026-11-04T01:45:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
