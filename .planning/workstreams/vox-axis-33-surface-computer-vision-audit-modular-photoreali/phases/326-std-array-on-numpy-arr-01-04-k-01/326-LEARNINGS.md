---
phase: 326
phase_name: "std-array-on-numpy-arr-01-04-k-01"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 65
  lessons: 12
  patterns: 33
  surprises: 9
missing_artifacts:
  - "UAT.md"
---

# Phase 326 Learnings: std-array-on-numpy-arr-01-04-k-01

## Decisions

### mean and std return rank-0 plans and copy a worker-call counter that sta...
mean and std return rank-0 plans and copy a worker-call counter that stays at zero

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-01-SUMMARY.md

---

### build.rs panics unless every row has a signature, a handler equal to its...
build.rs panics unless every row has a signature, a handler equal to its name, a known kind, and a non-null expected object

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-01-SUMMARY.md

---

### py_array_stats.vox is an in-process interp gap until plan 06 runs it thr...
py_array_stats.vox is an in-process interp gap until plan 06 runs it through the worker

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-01-SUMMARY.md

---

### ArrayPlan::zscore builds from_list of [1.0, 3.0], astype f64, subtract m...
`ArrayPlan::zscore` builds from_list of `[1.0, 3.0]`, astype f64, subtract mean, divide std with correction 0. Mean and std stay rank 0. The plan's worker-call count stays 0.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-01-SUMMARY.md

---

### contracts/python/ecosystem-support.yaml lists exactly from_list, astype,...
`contracts/python/ecosystem-support.yaml` lists exactly `from_list`, `astype`, `sub`, `div`, `mean`, `std`, `to_list`. Each row has a signature, a handler equal to its name, and a non-null expected object. `std` requires named `correction`. Empty mean and empty std are NaN. `to_list` records host `list_f64` for `[1.0, 3.0]` and the z-score list `[-1.0, 1.0]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-01-SUMMARY.md

---

### build.rs writes OUT_DIR/python_array_ops.rs and panics when signature, h...
`build.rs` writes `OUT_DIR/python_array_ops.rs` and panics when signature, handler, or expected is missing, or the kind is unknown. The rows are not folded into `GENERATED_RUST_INTEROP_POLICY`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-01-SUMMARY.md

---

### examples/golden/py_array_stats.vox calls to_list once and asserts [-1.0,...
`examples/golden/py_array_stats.vox` calls `to_list` once and asserts `[-1.0, 1.0]`. `KNOWN_INTERP_GAPS` contains `("py_array_stats.vox", "needs the uv python worker")`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-01-SUMMARY.md

---

### Plan construction copies plan_worker_calls() and never increments it
Plan construction copies `plan_worker_calls()` and never increments it. Mean and std therefore record zero worker calls.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-01-SUMMARY.md

---

### item's generated signature is Result[float], the f64 host scalar, matchi...
item's generated signature is Result[float], the f64 host scalar, matching to_list's float mapping.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-02-SUMMARY.md

---

### add and mul are excluded from Ty::Array method lookup
add and mul are excluded from Ty::Array method lookup. Their signatures still come from python_array_ops.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-02-SUMMARY.md

---

### The cargo target python_bridge_parity is a one-line path module over pyt...
The cargo target python_bridge_parity is a one-line path module over python_bridge_parity_test.rs so the plan command works without a Cargo.toml edit.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-02-SUMMARY.md

---

### The yaml ops list is from_list, astype, add, sub, mul, div, mean, std, s...
The yaml ops list is `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. The plan-ir name enum is those twelve names.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-02-SUMMARY.md

---

### sort appends a plan step with nulls of first or last
`sort` appends a plan step with `nulls` of `first` or `last`. The worker partitions NaNs, sorts the finite values with `np.sort`, and concatenates. `nulls=first` puts NaNs first.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-02-SUMMARY.md

---

### item and shape append their op and call materialize_to_list
`item` and `shape` append their op and call `materialize_to_list`. They require `HirCapability::Python`. `sort` does not. `add` and `mul` broadcast a rank-0 operand the same way `sub` and `div` do.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-02-SUMMARY.md

---

### on_disk_expected_matches_behavior_cases locks add [2.0, 4.0], mul [8.0, ...
`on_disk_expected_matches_behavior_cases` locks add `[2.0, 4.0]`, mul `[8.0, 15.0]`, sort nulls=first `[NaN, 1.0, 3.0]`, sort nulls=last `[1.0, 3.0, NaN]`, item host float `2.0`, and shape `[2]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-02-SUMMARY.md

---

### item() is Result[float] in the generated table
`item()` is `Result[float]` in the generated table. That is the f64 host scalar, the same float mapping `to_list` uses for this slice.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-02-SUMMARY.md

---

### py_array_stats targets rust-script only, because the golden is in KNOWN_...
py_array_stats targets rust-script only, because the golden is in KNOWN_INTERP_GAPS

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-03-SUMMARY.md

---

### Complexity-budget and source-token-budget integers stay in plan 04
Complexity-budget and source-token-budget integers stay in plan 04

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-03-SUMMARY.md

---

### The freeze test also checks the slice runs from fn through form
The freeze test also checks the slice runs from fn through form

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-03-SUMMARY.md

---

### contracts/pipeline/canonical-ladder.v1.yaml has one new fixture, id py_a...
`contracts/pipeline/canonical-ladder.v1.yaml` has one new fixture, id `py_array_stats`, path `examples/golden/py_array_stats.vox`, `proves: [py_array_stats]`, `targets: [rust-script]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-03-SUMMARY.md

---

### CanonicalLadder::load_from_repo_root returns 13 fixtures
`CanonicalLadder::load_from_repo_root` returns 13 fixtures. Loaded ids do not include `py_frame_report`, `py_tensor_infer`, or `py_escape_psd`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-03-SUMMARY.md

---

### lexer_keywords_freeze_std_array_idents asserts LEXER_KEYWORDS.len() == 5...
`lexer_keywords_freeze_std_array_idents` asserts `LEXER_KEYWORDS.len() == 54`, endpoints `fn` and `form`, and the absence of `array`, `astype`, `f32`, `f64`, `correction`, and `nulls`. The partition test still passes on the same slice.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-03-SUMMARY.md

---

### The new fixture targets rust-script only
The new fixture targets `rust-script` only. `py_array_stats.vox` is listed in `KNOWN_INTERP_GAPS`, so `interp` is not a ladder target.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-03-SUMMARY.md

---

### Budget maps were not edited and the budget updater was not run
Budget maps were not edited and the budget updater was not run. A red `vox ci k-complexity-budget` is expected until plan 04 inserts the measured key.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-03-SUMMARY.md

---

### i64::MAX is an integer host column; f64 cannot round-trip 2^63-1
i64::MAX is an integer host column; f64 cannot round-trip 2^63-1

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-04-SUMMARY.md

---

### uint64 above i64::MAX errors with a py
uint64 above i64::MAX errors with a py: prefix and is not wrapped

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-04-SUMMARY.md

---

### f32 widens with f64::from, not from_bits
f32 widens with f64::from, not from_bits

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-04-SUMMARY.md

---

### Signature ceiling is contracts/python/signature-manifest-budget.v1.json,...
Signature ceiling is contracts/python/signature-manifest-budget.v1.json, not the WebIR map

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-04-SUMMARY.md

---

### The golden py_array_stats.vox materializes NaN, +Inf, -Inf, and 92233720...
The golden `py_array_stats.vox` materializes NaN, +Inf, -Inf, and `9223372036854775807` with three `to_list` calls (three `execute_plan` frames). `mean` and `std` stay inside the plan.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-04-SUMMARY.md

---

### decode_column keeps i64::MIN, returns py
`decode_column` keeps `i64::MIN`, returns `py: uint64 {value} does not fit i64` above `i64::MAX`, widens f32 with `f64::from`, and rejects an over-cap body before reading the stream.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-04-SUMMARY.md

---

### Measured budgets, not guessed
Measured budgets, not guessed: complexity `fixtures.py_array_stats` = 120, source tokens = 280, source bytes = 1223, signature ceiling = 135.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-04-SUMMARY.md

---

### Integer lists take an int64 host column
Integer lists take an int64 host column. An f64 host rounds `i64::MAX` to `2^63`, and cross-numeric `is` would still pass, so the golden also asserts `(n - 9223372036854775806) is 1`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-04-SUMMARY.md

---

### i64::MIN is not a Vox literal
`i64::MIN` is not a Vox literal. The compiler test injects Arrow bytes from `encode_i64_column` through a `cfg(test)` reply seam. Production still calls the worker.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-04-SUMMARY.md

---

### Array of f64 is Ty::Array(Named f64), displayed as Array of f64, and low...
Array of f64 is Ty::Array(Named f64), displayed as Array of f64, and lowered as HirType::Generic Array

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-05-SUMMARY.md

---

### std accepts only the named argument correction and does not default it
std accepts only the named argument correction and does not default it

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-05-SUMMARY.md

---

### Plan methods add no capability; to_list on Ty::Array requires uses pytho...
Plan methods add no capability; to_list on Ty::Array requires uses python

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-05-SUMMARY.md

---

### examples/golden/py_array_stats.vox typechecks
`examples/golden/py_array_stats.vox` typechecks. `to_list` on Array of f64 is `Result` of `list` of `float`, and that call requires `HirCapability::Python`. Plan methods do not.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-05-SUMMARY.md

---

### Sub and div of two Ty::Array values with the same inner dtype yield Arra...
Sub and div of two `Ty::Array` values with the same inner dtype yield Array of that dtype, including mean/std rank-0 plans against the rank-1 input. A mismatched pair diagnoses and names `astype`, and does not use host numeric promotion. `1 + 1.0` stays float.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-05-SUMMARY.md

---

### a.std(correction=0) typechecks
`a.std(correction=0)` typechecks. `a.std()` and `a.std(0)` are diagnostics. `correction` is not defaulted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-05-SUMMARY.md

---

### std.array.from_list of a list of float is Array of f64
`std.array.from_list` of a list of float is Array of f64. `contracts-index` exits 0 with ids `python-ecosystem-support`, `python-ecosystem-support-schema`, and `python-plan-ir`. The plan-ir name enum is the seven stats-slice ops.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-05-SUMMARY.md

---

### Array dtype f64 is Ty::Named("f64"), not host Ty::Float
Array dtype `f64` is `Ty::Named("f64")`, not host `Ty::Float`. `to_list` still returns host `list[float]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-05-SUMMARY.md

---

### Task 1 PyPI approval (2026-11-03) is exactly numpy and pyarrow
Task 1 PyPI approval (2026-11-03) is exactly numpy and pyarrow. No other Python project was installed.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-06-SUMMARY.md

---

### mean and std stay inside the plan
mean and std stay inside the plan. The host logs one execute_plan line and does not log the plan JSON.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-06-SUMMARY.md

---

### Arrow encode/decode lives in vox-python-bridge so the compiler does not ...
Arrow encode/decode lives in vox-python-bridge so the compiler does not take an arrow dependency.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-06-SUMMARY.md

---

### cargo test -p vox-compiler --test py_array_stats_run exited 0
`cargo test -p vox-compiler --test py_array_stats_run` exited 0. Result line: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.85s`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-06-SUMMARY.md

---

### The golden assert accepts [-1.0, 1.0]
The golden assert accepts `[-1.0, 1.0]`. The debug log has one `execute_plan` line and no wire method named mean or std.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-06-SUMMARY.md

---

### materialize_to_list is the only host entry that calls Bridge::call for t...
`materialize_to_list` is the only host entry that calls `Bridge::call` for this golden. Denial without the `py` capability returns `_Denied("py.to_list")`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-06-SUMMARY.md

---

### Host floats travel only in the Arrow body
Host floats travel only in the Arrow body. `plan_params` emits op names, `astype` dtype `f64`, and `std` `correction`. It does not emit the host list.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-06-SUMMARY.md

---

### The debug line is eprintln!("execute_plan") after a successful call when...
The debug line is `eprintln!("execute_plan")` after a successful call when `VOX_BRIDGE_DEBUG_OPS=1`. The plan JSON is not printed on that line.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-06-SUMMARY.md

---

### Add and Mul already shared the same-dtype Array arm; this plan locks the...
Add and Mul already shared the same-dtype Array arm; this plan locks them in array_dtype_and_required_names and adds the nulls check

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-07-SUMMARY.md

---

### from_list of list[int] is Array of i64, list[bool] is Array of bool, and...
from_list of list[int] is Array of i64, list[bool] is Array of bool, and list[float] stays Array of f64

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-07-SUMMARY.md

---

### mean and std typecheck only on Array of f32 or f64 and the diagnostic na...
mean and std typecheck only on Array of f32 or f64 and the diagnostic names astype

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-07-SUMMARY.md

---

### astype accepts i32, i64, f32, f64, and bool so each plan dtype can be sp...
astype accepts i32, i64, f32, f64, and bool so each plan dtype can be spelled; from_list still never yields i32 or f32

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-07-SUMMARY.md

---

### array_dtype_and_required_names covers add, sub, mul, and div of Array of...
`array_dtype_and_required_names` covers add, sub, mul, and div of Array of i64 with Array of f64, and add of Array of i32 with Array of i64. Each diagnostic names `astype`. Same-dtype add and mul, including `a + a.mean()`, stay Array of f64. Host scalar promotion is not asserted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-07-SUMMARY.md

---

### a.std() and a.std(0) are diagnostics
`a.std()` and `a.std(0)` are diagnostics. `a.std(correction=0)` typechecks on Array of f64.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-07-SUMMARY.md

---

### a.sort() and a.sort("first") are diagnostics that name nulls
`a.sort()` and `a.sort("first")` are diagnostics that name `nulls`. `nulls="first"` and `nulls="last"` typecheck. `nulls="middle"` is a diagnostic. `nulls` is not defaulted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-07-SUMMARY.md

---

### from_list of list[int] is Array of i64 and of list[bool] is Array of boo...
`from_list` of `list[int]` is Array of i64 and of `list[bool]` is Array of bool. `to_list` on those dtypes returns Result of list of int and Result of list of bool. `mean` and `std` diagnose on Array of i64 and typecheck on Array of f32 and f64. `astype(i32)` and `astype(f32)` produce those dtypes. `from_list` does not.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-07-SUMMARY.md

---

### Add and Mul already used array_same_dtype_op
Add and Mul already used `array_same_dtype_op`. The new test locks that behavior, including a 0-d `mean` operand. This plan's new arithmetic work is the i32/i64 and four-op diagnostic coverage, not a second arm.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-07-SUMMARY.md

---

### A comment that names an op is not a handler
A comment that names an op is not a handler. The checker skips comment lines before looking for a column-0 def or a HANDLERS key.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-08-SUMMARY.md

---

### The policy registry gained only the python-bridge-parity entry
The policy registry gained only the python-bridge-parity entry. The full generator rewrite was not committed.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 326-08-SUMMARY.md

---

### synthetic_row_missing_signature_handler_or_expected_is_err builds the ro...
`synthetic_row_missing_signature_handler_or_expected_is_err` builds the row in memory. Dropping signature, handler, or expected returns Err. A handler that is not a column-0 def and not a HANDLERS key returns Err. A comment that names the op returns Err. A complete `mean` row against the worker returns Ok. The yaml bytes are unchanged.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-08-SUMMARY.md

---

### CiCmd::PythonBridgeParity (python-bridge-parity, catalog id ci.python-br...
`CiCmd::PythonBridgeParity` (`python-bridge-parity`, catalog id `ci.python-bridge-parity`) runs `cargo test -p vox-compiler --test python_bridge_parity` through `cargo_bin()` and returns `python_bridge_parity_from_status`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-08-SUMMARY.md

---

### A non-zero status is Err with the reproducer string
A non-zero status is Err with the reproducer string. A zero status prints `python-bridge-parity OK` and returns Ok. There is no second branch that returns Ok after a failed status.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-08-SUMMARY.md

---

### Comment lines are ignored when deciding that a worker defines a handler
Comment lines are ignored when deciding that a worker defines a handler. A column-0 `def` and a HANDLERS key are both required.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-08-SUMMARY.md

---

### policy-registry --write also rewrote unrelated descriptions and inserted...
`policy-registry --write` also rewrote unrelated descriptions and inserted `ci.capability-index`. Only the python-bridge-parity entry was kept.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 326-08-SUMMARY.md

## Lessons

### None remaining
None remaining. Both scoped tests passed on the committed tree's sources.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-01-SUMMARY.md

---

### The first golden run failed closed on from_list expects floats, found In...
The first golden run failed closed on `from_list expects floats, found Int` after NaN and Inf had already materialized. The int64 host path fixed that. `target/debug/vox` was rebuilt before the run test, because that test spawns the debug binary.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-04-SUMMARY.md

---

### None beyond the deviations above.
None beyond the deviations above.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-05-SUMMARY.md

---

### Parallel rustc processes stopped making progress (CPU frozen, state slee...
Parallel `rustc` processes stopped making progress (CPU frozen, state sleeping) during `cargo build -p vox-cli`. The build that produced `target/debug/vox` used `CARGO_INCREMENTAL=0`, `RUSTC_WRAPPER=/usr/bin/env`, and `-j1`. That is a local compile workaround, not a source change.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-06-SUMMARY.md

---

### Blocking-human package check
Blocking-human package check. The user approved those two names on 2026-11-03. The executor did not re-ask and did not install any other Python project.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-06-SUMMARY.md

---

### The sort call site is in expr.rs, which the plan's files_modified alread...
The sort call site is in `expr.rs`, which the plan's `files_modified` already lists. Task 1's `<files>` element named `expr_ops.rs` and the test; the check is useless unless the method arm calls it.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-07-SUMMARY.md

---

### An early return from check_expr skipped the inferred-type insert, so fro...
An early `return` from `check_expr` skipped the inferred-type insert, so `from_list` typechecked but had no span to assert. The match arm now yields the type and the insert runs. That fix is in `651a6267c`. A split borrow was required so `check_expr` and `uf.resolve` do not overlap.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-07-SUMMARY.md

---

### cargo run -p vox-cli -- ci command-compliance exits 1
`cargo run -p vox-cli -- ci command-compliance` exits 1. `check_ref_cli` passed, including the new `python-bridge-parity` row. The next check is a pre-existing gap: `docs/src/reference/cli.md` has no `| `py` |` reachability row for top-level `py` (`reachability_required` is null). A diagnostic row, not committed, moved the failure to `vox_context_handoff` missing from `TOOL_REGISTRY`. No reachability row was added for `python-bridge-parity` or for `py`.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-08-SUMMARY.md

---

### command-sync --write also updated docs/src/reference/env-vars.generated....
`command-sync --write` also updated `docs/src/reference/env-vars.generated.md` (pre-existing `VOX_REASONING_EVAL_MAX_TOKENS` drift) and the working tree already had `docs/src/index.mdx` and `contracts/ci/fan-in-snapshot.v1.json` changes. Those files were not staged.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-08-SUMMARY.md

---

### command-compliance exits 1 on this branch before it finishes
command-compliance exits 1 on this branch before it finishes. The first error is a pre-existing missing reachability row for top-level py. A diagnostic row then surfaced an unrelated missing TOOL_REGISTRY entry for vox_context_handoff. python-bridge-parity itself is reachability_required false and its cli.md row passed check_ref_cli.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 326-08-SUMMARY.md

---

### N/A — compiler, worker, and contract phase
N/A — compiler, worker, and contract phase. Acceptance criteria are covered by the tests above. The plan 06 PyPI checkpoint is an execution gate that already unblocked the golden run; this verification re-ran that run.

**Context:** Verified in phase acceptance checks.
**Source:** 326-VERIFICATION.md

---

### No gaps. The phase goal holds in the codebase: a closed twelve-op std.ar...
No gaps. The phase goal holds in the codebase: a closed twelve-op `std.array` vocabulary typechecks, plans stay lazy until `to_list`/`item`/`shape`, and the z-score golden computes on numpy in one `execute_plan` per materialization.

**Context:** Verified in phase acceptance checks.
**Source:** 326-VERIFICATION.md

## Patterns

### Pattern: python array allowlist is contracts/python/ecosystem-support.ya...
Pattern: python array allowlist is contracts/python/ecosystem-support.yaml, emitted to OUT_DIR/python_array_ops.rs

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-01-SUMMARY.md

---

### Pattern: VoxValue::ArrayPlan wraps the pure plan; materializers are not ...
Pattern: VoxValue::ArrayPlan wraps the pure plan; materializers are not called in this plan

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-01-SUMMARY.md

---

### std.array ops are a generated signature table, separate from GENERATED_R...
std.array ops are a generated signature table, separate from GENERATED_RUST_INTEROP_POLICY

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-01-SUMMARY.md

---

### ArrayPlan records op names and a worker-call count that plan builders ne...
ArrayPlan records op names and a worker-call count that plan builders never increment

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-01-SUMMARY.md

---

### Plan builders for add, mul, and sort copy plan_worker_calls and never in...
Plan builders for add, mul, and sort copy plan_worker_calls and never increment it.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-02-SUMMARY.md

---

### Materializers item and shape require uses python
Materializers item and shape require uses python. sort does not.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-02-SUMMARY.md

---

### sort stores nulls on the plan step; the worker partitions NaNs and does ...
sort stores nulls on the plan step; the worker partitions NaNs and does not pass nulls to np.sort

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-02-SUMMARY.md

---

### item and shape append one op and call materialize_to_list; the wire meth...
item and shape append one op and call materialize_to_list; the wire method stays execute_plan

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-02-SUMMARY.md

---

### add and mul are binary operators on ArrayPlan, with the same 0-d broadca...
add and mul are binary operators on ArrayPlan, with the same 0-d broadcast as sub and div

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-02-SUMMARY.md

---

### Pattern: one ladder id per phase golden; later-phase py_* ids stay off t...
Pattern: one ladder id per phase golden; later-phase py_* ids stay off the file

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-03-SUMMARY.md

---

### A new golden joins the canonical ladder as one rust-script fixture when ...
A new golden joins the canonical ladder as one rust-script fixture when it is an interp gap

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-03-SUMMARY.md

---

### LEXER_KEYWORDS length and excluded identifiers are asserted beside the c...
LEXER_KEYWORDS length and excluded identifiers are asserted beside the category partition

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-03-SUMMARY.md

---

### Three to_list calls are three execute_plan frames; mean and std stay ins...
Three to_list calls are three execute_plan frames; mean and std stay inside the plan

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-04-SUMMARY.md

---

### i64::MIN is injected as Arrow bytes in the Rust test, never as a Vox lit...
i64::MIN is injected as Arrow bytes in the Rust test, never as a Vox literal

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-04-SUMMARY.md

---

### Integer host columns stay Arrow int64 until decode; floats stay float64
Integer host columns stay Arrow int64 until decode; floats stay float64

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-04-SUMMARY.md

---

### K2 signature ceiling lives outside the WebIR complexity map
K2 signature ceiling lives outside the WebIR complexity map

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-04-SUMMARY.md

---

### Pattern: std.array signatures are read from python_array_ops, not a seco...
Pattern: std.array signatures are read from python_array_ops, not a second match of op names

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-05-SUMMARY.md

---

### Pattern: Sub and Div of Ty::Array run before host numeric promotion
Pattern: Sub and Div of Ty::Array run before host numeric promotion

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-05-SUMMARY.md

---

### Array method types are parsed from python_array_ops(); Array[...] on tho...
Array method types are parsed from python_array_ops(); Array[...] on those methods is GenericParam(0)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-05-SUMMARY.md

---

### to_list is the only Array method that requires HirCapability::Python
to_list is the only Array method that requires HirCapability::Python

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-05-SUMMARY.md

---

### to_list is the only interpreter path that calls the existing Phase 325 b...
to_list is the only interpreter path that calls the existing Phase 325 bridge.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-06-SUMMARY.md

---

### Worker op dispatch is the HANDLERS dict
Worker op dispatch is the HANDLERS dict. Unknown ops return py-plan:.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-06-SUMMARY.md

---

### one Bridge::call per to_list
one Bridge::call per to_list

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-06-SUMMARY.md

---

### plan JSON without host numbers
plan JSON without host numbers

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-06-SUMMARY.md

---

### f64 Arrow column named values
f64 Arrow column named values

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-06-SUMMARY.md

---

### Pattern: required array names are checked on HirArg.name before check_ar...
Pattern: required array names are checked on HirArg.name before check_arguments zips positionally

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-07-SUMMARY.md

---

### Pattern: host-list dtype mapping lives in typeck, not in a new yaml row
Pattern: host-list dtype mapping lives in typeck, not in a new yaml row

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-07-SUMMARY.md

---

### Array add, sub, mul, and div share array_same_dtype_op and do not fall t...
Array add, sub, mul, and div share array_same_dtype_op and do not fall through to host promotion

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-07-SUMMARY.md

---

### sort checks HirArg.name before any positional zip; nulls is first or las...
sort checks HirArg.name before any positional zip; nulls is first or last and is not defaulted

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-07-SUMMARY.md

---

### Host list int and bool map to plan dtypes i64 and bool; i32 and f32 are ...
Host list int and bool map to plan dtypes i64 and bool; i32 and f32 are astype tokens

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-07-SUMMARY.md

---

### CiCmd::PythonBridgeParity shells out through cargo_bin() and returns pyt...
CiCmd::PythonBridgeParity shells out through cargo_bin() and returns python_bridge_parity_from_status.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-08-SUMMARY.md

---

### python_bridge_parity_from_status maps ExitStatus to Result, and the CI a...
python_bridge_parity_from_status maps ExitStatus to Result, and the CI arm returns that Result

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-08-SUMMARY.md

---

### A handler counts only as a column-0 def plus a non-comment HANDLERS key
A handler counts only as a column-0 def plus a non-comment HANDLERS key

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 326-08-SUMMARY.md

## Surprises

### Estimate variance on 326-01
Estimate variance on 326-01: planned 42000 tokens vs 5694 actual (0.14x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-01-PLAN.md, 326-01-SUMMARY.md

---

### Estimate variance on 326-02
Estimate variance on 326-02: planned 40000 tokens vs 9352 actual (0.23x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-02-PLAN.md, 326-02-SUMMARY.md

---

### Estimate variance on 326-03
Estimate variance on 326-03: planned 16000 tokens vs 682 actual (0.04x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-03-PLAN.md, 326-03-SUMMARY.md

---

### Estimate variance on 326-04
Estimate variance on 326-04: planned 36000 tokens vs 8228 actual (0.23x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-04-PLAN.md, 326-04-SUMMARY.md

---

### Estimate variance on 326-05
Estimate variance on 326-05: planned 38000 tokens vs 8627 actual (0.23x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-05-PLAN.md, 326-05-SUMMARY.md

---

### None beyond the deviations above.
None beyond the deviations above.

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 326-05-SUMMARY.md

---

### Estimate variance on 326-06
Estimate variance on 326-06: planned 40000 tokens vs 8824 actual (0.22x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-06-PLAN.md, 326-06-SUMMARY.md

---

### Estimate variance on 326-07
Estimate variance on 326-07: planned 22000 tokens vs 5368 actual (0.24x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-07-PLAN.md, 326-07-SUMMARY.md

---

### Estimate variance on 326-08
Estimate variance on 326-08: planned 28000 tokens vs 6227 actual (0.22x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 326-08-PLAN.md, 326-08-SUMMARY.md

