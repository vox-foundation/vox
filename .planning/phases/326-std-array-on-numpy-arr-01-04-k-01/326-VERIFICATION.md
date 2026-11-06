---
phase: 326-std-array-on-numpy-arr-01-04-k-01
verified: 2026-11-04T01:36:00Z
status: passed
score: 8/8 must-haves verified
behavior_unverified: 0
overrides_applied: 0
decision_coverage:
  honored: 3
  total: 3
  not_honored: []
covered_files:
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-01-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-01-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-02-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-02-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-03-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-03-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-04-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-04-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-05-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-05-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-06-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-06-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-07-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-07-SUMMARY.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-08-PLAN.md
  - .planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-08-SUMMARY.md
  - contracts/eval/complexity-budget.v1.json
  - contracts/eval/source-token-budget.v1.json
  - contracts/index.yaml
  - contracts/operations/catalog.v1.yaml
  - contracts/pipeline/canonical-ladder.v1.yaml
  - contracts/python/ecosystem-support.schema.json
  - contracts/python/ecosystem-support.yaml
  - contracts/python/plan-ir.v1.schema.json
  - contracts/python/signature-manifest-budget.v1.json
  - crates/vox-cli-ci/src/cmd_enums.rs
  - crates/vox-cli/src/commands/ci/run_body.rs
  - crates/vox-codegen/src/canonical_ladder.rs
  - crates/vox-compiler/src/eval/array_plan.rs
  - crates/vox-compiler/src/eval/builtins.rs
  - crates/vox-compiler/src/eval/expr.rs
  - crates/vox-compiler/src/eval/python.rs
  - crates/vox-compiler/src/python_array_support.rs
  - crates/vox-compiler/src/typeck/builtins.rs
  - crates/vox-compiler/src/typeck/checker/expr.rs
  - crates/vox-compiler/src/typeck/checker/expr_ops.rs
  - crates/vox-compiler/src/typeck/ty.rs
  - crates/vox-compiler/tests/array_dtype_and_required_names.rs
  - crates/vox-compiler/tests/golden_interp_test.rs
  - crates/vox-compiler/tests/py_array_stats_run.rs
  - crates/vox-compiler/tests/python_bridge_parity_test.rs
  - crates/vox-language-surface/src/lib.rs
  - crates/vox-python-bridge/src/array_numeric.rs
  - crates/vox-python-bridge/worker/vox_bridge/array_ops.py
  - docs/src/reference/cli.md
  - examples/golden/py_array_stats.vox
covered_digest: "v3:sha256:745c8550768d0d76290d8f4479de6c0dc2a0197f0e896775e6c85b70922ccf3e"
---

# Phase 326: std.array on numpy Verification Report

**Phase Goal:** Vox code computes with numpy through a closed, typed, standards-named vocabulary and pure lazy plans.
**Verified:** 2026-11-04T01:36:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `examples/golden/py_array_stats.vox` runs under `vox run` with asserted output, one `execute_plan` per `to_list` (three frames), and no wire method named `mean` or `std`. | ✓ VERIFIED | Golden asserts `z is [-1.0, 1.0]`, `nan isnt nan`, both infinities, and `9223372036854775807`. `cargo test -p vox-compiler --test py_array_stats_run` passed (`py_array_stats_run ... ok`, 7.26s). The test requires three `execute_plan` lines and rejects a wire `mean`/`std`. |
| 2 | Mixed int/float array arithmetic without `.astype`, `std` without `correction=`, and `sort` without `nulls=` are compile errors, including add/sub/mul/div and the positional forms. | ✓ VERIFIED | `expr_ops.rs` `array_same_dtype_op` names `astype` for Add, Sub, Mul, and Div. `check_array_std_args` and `check_array_sort_args` require the named arguments. `cargo test -p vox-compiler --test array_dtype_and_required_names`: 6 passed, 0 ignored, including `mixed_array_arithmetic_names_astype`, `std_requires_named_correction`, `sort_requires_nulls_first_or_last`. |
| 3 | NaN, ±Inf, and i64::MAX round-trip through `to_list`. i64::MIN round-trips through `materialize_to_list`'s decoder. A uint64 above i64::MAX is a `py:` error and is not saturated. | ✓ VERIFIED | Golden path is truth 1. `array_numeric_roundtrip` calls `materialize_to_list`, which calls `decode_column`. `cargo test -p vox-compiler --lib array_numeric_roundtrip` passed. `decode_column` returns `py: uint64 {value} does not fit i64`. The golden does not contain `9223372036854775808`. |
| 4 | `vox ci python-bridge-parity` fails when an allowlisted op lacks a signature, a worker handler, or an expected value. A non-zero cargo status returns Err. | ✓ VERIFIED | `synthetic_row_missing_signature_handler_or_expected_is_err` passed. `CiCmd::PythonBridgeParity` in `run_body.rs` shells `cargo test -p vox-compiler --test python_bridge_parity` and returns `python_bridge_parity_from_status`, which is `Err` when `status.success()` is false. Catalog id `ci.python-bridge-parity` and the `cli.md` row exist. |
| 5 | Ladder, complexity-budget, and source-token-budget rows exist for `py_array_stats` only. `LEXER_KEYWORDS` stays 54. The K2 ceiling is in `signature-manifest-budget.v1.json` and is not stored in `complexity-budget.v1.json`. | ✓ VERIFIED | `ladder_loads_thirteen_fixtures` passed and asserts the three later `py_*` ids are absent. `lexer_keywords_freeze_std_array_idents` passed (`len == 54`, first `fn`, last `form`; `array`, `astype`, `f32`, `f64`, `correction`, `nulls` absent). `signature_manifest_stays_within_its_ceiling` passed (ceiling 135, twelve D-01 names). The complexity map has `py_array_stats: 120` and does not contain `signature-manifest` or `"ceiling"`. |
| 6 | The allowlist is exactly the twelve D-01 ops. Each row has a signature, a column-0 worker handler, and a non-null expected object equal to the behavior cases. | ✓ VERIFIED | `contracts/python/ecosystem-support.yaml` lists `from_list`, `astype`, `add`, `sub`, `mul`, `div`, `mean`, `std`, `sort`, `to_list`, `item`, `shape`. `on_disk_expected_matches_behavior_cases` passed, including add `[2.0, 4.0]`, mul `[8.0, 15.0]`, sort nulls first/last, item `2.0`, and shape `[2]`. `array_ops.py` defines each name at column 0 and maps it in `HANDLERS`. |
| 7 | `add`, `mul`, and `sort` build `ArrayPlan` values and record zero worker calls. `item` and `shape` each call `materialize_to_list` once and require `uses python`. `sort` does not require that effect. | ✓ VERIFIED | `array_plan_zscore_is_pure`, `sort_is_a_plan`, and `item_shape_require_python` each passed. Interpreter `+`/`*`/`-`/`/` on two `ArrayPlan`s call `add`/`mul`/`sub`/`div`. `to_list`/`item`/`shape` are the materializers and call `execute_plan`. |
| 8 | `from_list` of `list[int]` is `Array[i64]` and of `list[bool]` is `Array[bool]`. `mean` and `std` are diagnostics on `Array[i64]` and typecheck on `f32` and `f64`. `i32` and `f32` exist only after `astype`. | ✓ VERIFIED | `from_list_int_and_bool_map_to_host_lists`, `mean_and_std_only_on_f32_and_f64`, and `i32_and_f32_exist_only_after_astype` passed in the dtype suite. |

**Score:** 8/8 truths verified (0 present, behavior-unverified)

### Prohibitions checked

Plan frontmatter left these `verification: flagged`. They hold in the tree, and the ones with tests were re-run:

| Prohibition | Evidence |
| --- | --- |
| No Vox literal for i64::MIN in the golden | Golden spells `9223372036854775807` only. `py_array_stats_run` asserts the source does not contain `9223372036854775808`. |
| NaN is not compared with `is` | Golden uses `assert(nan isnt nan)`. The same test asserts the source contains `isnt`. |
| Signature ceiling is not in `complexity-budget.v1.json` | File has fixture integers only. `signature_manifest_stays_within_its_ceiling` asserts the absence and passed. |
| Budget map was not replaced | Prior fixture keys remain beside `py_array_stats` in both budget files. |
| No ladder or budget rows for `py_frame_report`, `py_tensor_infer`, or `py_escape_psd` | `ladder_loads_thirteen_fixtures` passed. Those ids are absent from the ladder and both budget maps. |
| `LEXER_KEYWORDS` length stays 54 | `lexer_keywords_freeze_std_array_idents` passed. |

### Required Artifacts

| Artifact | Expected | Status | Details |
| --- | --- | --- | --- |
| `examples/golden/py_array_stats.vox` | Z-score plus NaN, ±Inf, i64::MAX; three `to_list` calls | ✓ VERIFIED | `zscore`, `isnt`, max i64 literal. Ran under `vox run` in the integration test. |
| `contracts/python/ecosystem-support.yaml` | Twelve closed ops with signature, handler, expected | ✓ VERIFIED | Parity test passed against this file. |
| `crates/vox-compiler/src/eval/array_plan.rs` | Pure `ArrayPlan`; zero worker calls while building | ✓ VERIFIED | `array_plan_zscore_is_pure` and `sort_is_a_plan` passed. |
| `crates/vox-compiler/src/eval/builtins.rs` | `materialize_to_list` is the host entry | ✓ VERIFIED | Calls `execute_plan`, then `decode_column`. `array_numeric_roundtrip` passed. |
| `crates/vox-python-bridge/src/array_numeric.rs` | Arrow decoder; uint64 overflow is `py:` | ✓ VERIFIED | `decode_column` rejects uint64 above i64::MAX and widens f32 with `f64::from`. |
| `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` | Numpy handlers behind one `execute_plan` | ✓ VERIFIED | Twelve `def`s and `HANDLERS`. Unknown names raise `py-plan:`. |
| `contracts/python/signature-manifest-budget.v1.json` | K2 ceiling 135 for the twelve signatures | ✓ VERIFIED | Ceiling test passed. Indexed as `python-signature-manifest-budget`. |
| `contracts/pipeline/canonical-ladder.v1.yaml` | One new fixture, count 13 | ✓ VERIFIED | `ladder_loads_thirteen_fixtures` passed. |
| `crates/vox-language-surface/src/lib.rs` | `LEXER_KEYWORDS` frozen at 54 | ✓ VERIFIED | Freeze test passed. |
| `crates/vox-cli/src/commands/ci/run_body.rs` | Parity arm returns the cargo status | ✓ VERIFIED | Arm returns `python_bridge_parity_from_status`. Non-zero maps to `Err`. |

### Key Link Verification

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| Golden `to_list` | Worker `execute_plan` | `materialize_to_list` → `python::execute_plan` → `Bridge::call("execute_plan", …)` | WIRED | Debug line is `execute_plan`. Golden run counted three. |
| `materialize_to_list` | `decode_column` | `host_list_from_body` | WIRED | Numeric test injects an Arrow body and still goes through `materialize_to_list`. |
| `ecosystem-support.yaml` | Generated signature table | `python_array_ops()` compared in the parity test | WIRED | `on_disk_expected_matches_behavior_cases` passed. |
| Typeck `Array` arm | Diagnostics | `expr_ops.rs` before host int/float promotion | WIRED | Dtype suite passed for add, sub, mul, div, `std`, and `sort`. |
| `CiCmd::PythonBridgeParity` | `python_bridge_parity` test | `cargo test -p vox-compiler --test python_bridge_parity` then `python_bridge_parity_from_status` | WIRED | No second branch that returns `Ok` after a failed status. |
| Ladder yaml | `canonical_ladder.rs` | `CanonicalLadder::load_from_repo_root` | WIRED | Count test asserts 13 and the `py_array_stats` path. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `py_array_stats.vox` | z-score list | `from_list` host column → plan JSON ops → numpy `execute_plan` → Arrow body → `decode_column` | Yes | ✓ FLOWING |
| `array_numeric_roundtrip` | i64::MIN, uint64 error, f32 widen | Test encodes an Arrow body; `materialize_to_list` decodes it | Yes, host codec | ✓ FLOWING |
| Allowlist `expected` objects | add/mul/sort/item/shape cases | Handwritten yaml, checked against literals in the parity test | Conformance fixtures, not a second numpy oracle | ✓ FLOWING |

`i64::MIN` never enters as a Vox literal. The host test encodes `i64::MIN` and decodes it back to `VoxValue::Int(i64::MIN)`. That is the plan-04 contract.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Golden numpy run, three `execute_plan` frames | `cargo test -p vox-compiler --test py_array_stats_run` | `py_array_stats_run ... ok` (7.26s) | ✓ PASS |
| i64::MIN / uint64 / f32 widen | `cargo test -p vox-compiler --lib array_numeric_roundtrip` | 1 passed | ✓ PASS |
| Parity, synthetic missing row, K2 ceiling | `cargo test -p vox-compiler --test python_bridge_parity` | 3 passed, 0 ignored | ✓ PASS |
| Dtype and required names | `cargo test -p vox-compiler --test array_dtype_and_required_names` | 6 passed, 0 ignored | ✓ PASS |
| Pure z-score plan, sort plan, item/shape effect | `cargo test -p vox-compiler --lib` those three names | each 1 passed | ✓ PASS |
| Lexer freeze and 13-fixture ladder | `cargo test -p vox-language-surface --lib lexer_keywords_freeze_std_array_idents`; `cargo test -p vox-codegen --lib ladder_loads_thirteen_fixtures` | each 1 passed | ✓ PASS |

`python_bridge_parity_nonzero_status_is_err` was not re-run. Compiling `vox-cli` for that one lib test was killed before the test binary started. The function is the `if status.success()` branch in `run_body.rs` and the unit test is present beside it.

### Probe Execution

No `scripts/*/tests/probe-*.sh` is declared for this phase. Step 7c is not applicable.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| --- | --- | --- | --- | --- |
| ARR-01 | 01, 02, 04, 05, 06 | Every allowlisted `std.array` op typechecks and runs on numpy; the golden runs under `vox run` | ✓ SATISFIED | Twelve-op yaml and handlers. Golden `vox run` passed. Binary ops and materializers are wired through `ArrayPlan` and `execute_plan`. |
| ARR-02 | 05, 07 | Mixed int/float without `.astype` is a type error; `std` requires `correction=`; `sort` requires `nulls=` | ✓ SATISFIED | Dtype suite, 6 passed. |
| ARR-03 | 04 | NaN, ±Inf, and i64 extremes round-trip through `to_list`; uint64 above i64::MAX is a runtime error | ✓ SATISFIED | Golden run plus `array_numeric_roundtrip`. |
| ARR-04 | 08 | `vox ci python-bridge-parity` fails when a row lacks a signature, handler, or conformance expected value | ✓ SATISFIED | Synthetic-row test passed. CI arm returns that cargo status. |
| K-01 | 03, 04 | Ladder and complexity-budget rows; `LEXER_KEYWORDS` gains 0; per-module signature ceiling enforced | ✓ SATISFIED | This phase's contract is the one golden (roadmap criterion 5). `py_array_stats` is on the ladder and both budget files. Lexer length stayed 54. K2 ceiling is enforced. The other three `py_*` goldens named in the requirement sentence are the exit files of later phases and are absent here on purpose. |

No requirement ID mapped to phase 326 in `REQUIREMENTS.md` is missing from plan frontmatter. ARR-01, ARR-02, ARR-03, ARR-04, and K-01 are all claimed. None are orphaned.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-compiler/tests/py_array_stats_run.rs` | ARR-01, ARR-03 | 1 | 0 | No | Behavioral (`vox run` exit plus log counts; golden asserts values) | PASS |
| `crates/vox-compiler/tests/array_dtype_and_required_names.rs` | ARR-02 | 6 | 0 | No | Value (diagnostic text and HIR types) | PASS |
| `crates/vox-compiler/tests/python_bridge_parity_test.rs` | ARR-04, K-01 | 3 | 0 | No | Value (yaml cases, `Err` on dropped fields, compressed size ≤ ceiling) | PASS |
| `array_numeric_roundtrip` in `builtins.rs` | ARR-03 | 1 | 0 | No | Value (`VoxValue::Int(i64::MIN)`, `py:` prefix, `f64::from`) | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0. Expected objects are handwritten and compared to literals. They are not generated by running the worker.
**Insufficient assertions:** 0 blocking. `add`, `mul`, `sort`, `item`, and `shape` are not each given a separate numpy process test. They share `execute_plan` and have real numpy handlers. D-03 makes the golden the one end-to-end materialization. That is the phase contract, not a missing requirement.

### Decision Coverage

All trackable CONTEXT.md decisions are honored by shipped artifacts. 3/3. Non-blocking.

D-01 is the twelve-name allowlist. D-02 is `Array[f32]`/`Array[f64]` plans with `to_list` returning host floats, including NaN and both infinities. D-03 is the table-driven row check plus one golden materialization.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| `crates/vox-compiler/src/eval/array_plan.rs` | `astype` | The dtype argument is stored as `_dtype` and dropped. `plan_params` always writes `"dtype": "f64"`. The worker rejects any other dtype. | ℹ️ Info | `astype(i32)` and `astype(f32)` typecheck (truth 8) and do not change the plan payload. Plan 07 required the identifiers at typecheck, not a numpy cast to those dtypes. The golden uses `astype(f64)`. |
| `crates/vox-compiler/src/eval/builtins.rs` | `array_from_list` | Runtime `from_list` accepts ints and floats. A bool list is a panic path. | ℹ️ Info | Bool `from_list` is proven at typecheck only (`from_list_int_and_bool_map_to_host_lists`). |

No `TBD`, `FIXME`, or `XXX` markers in the phase implementation files that were scanned.

### Human Verification Required

N/A — compiler, worker, and contract phase. Acceptance criteria are covered by the tests above. The plan 06 PyPI checkpoint is an execution gate that already unblocked the golden run; this verification re-ran that run.

### Gaps Summary

No gaps. The phase goal holds in the codebase: a closed twelve-op `std.array` vocabulary typechecks, plans stay lazy until `to_list`/`item`/`shape`, and the z-score golden computes on numpy in one `execute_plan` per materialization.

---

_Verified: 2026-11-04T01:36:00Z_
_Verifier: Claude (gsd-verifier)_
