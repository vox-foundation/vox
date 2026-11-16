---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 06
subsystem: runtime
tags: [numpy, pyarrow, arrow-ipc, python-bridge, std.array]

requires:
  - phase: 326-05
    provides: typechecked z-score golden and the seven-op plan
provides:
  - one execute_plan host call that materializes std.array to_list as f64
  - worker HANDLERS for from_list, astype, sub, div, mean, std, to_list
affects: [326-04, 326-07]

actuals:
  tokens: 8824
  tasks: 2
  commits: 1
commits: 1
plan_head_before: fb66ce4044dfa7170ec7cd311b3528303ac2416b
plan_head_after: d6418f5f3f97264fc4af181dcab822a17f0daf24

tech-stack:
  added: [arrow-array 60.1.0, arrow-ipc 60.1.0, numpy, pyarrow]
  patterns: [one Bridge::call per to_list, plan JSON without host numbers, f64 Arrow column named values]

key-files:
  created:
    - crates/vox-compiler/tests/py_array_stats_run.rs
    - crates/vox-python-bridge/src/array_ipc.rs
    - crates/vox-python-bridge/worker/vox_bridge/array_ops.py
  modified:
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/src/eval/python.rs
    - crates/vox-python-bridge/worker/vox_bridge/__main__.py

key-decisions:
  - "Task 1 PyPI approval (2026-11-03) is exactly numpy and pyarrow. No other Python project was installed."
  - "mean and std stay inside the plan. The host logs one execute_plan line and does not log the plan JSON."
  - "Arrow encode/decode lives in vox-python-bridge so the compiler does not take an arrow dependency."

patterns-established:
  - "to_list is the only interpreter path that calls the existing Phase 325 bridge."
  - "Worker op dispatch is the HANDLERS dict. Unknown ops return py-plan:."

requirements-completed: [ARR-01]

coverage:
  - id: D1
    description: "vox run of the z-score golden exits 0, the assert accepts [-1.0, 1.0], and the debug log has one execute_plan line and no wire method named mean or std."
    requirement: ARR-01
    verification:
      - kind: integration
        ref: "crates/vox-compiler/tests/py_array_stats_run.rs#py_array_stats_run"
        status: pass
    human_judgment: false
  - id: D2
    description: "PyPI projects approved for the temp project are exactly numpy and pyarrow."
    requirement: ARR-01
    verification: []
    human_judgment: true
    rationale: "Blocking-human package check. The user approved those two names on 2026-11-03. The executor did not re-ask and did not install any other Python project."

duration: 62min
completed: 2026-11-03
status: complete
---

# Phase 326 Plan 06: std.array on numpy Summary

**The z-score golden runs through one `execute_plan` call. mean and std stay in the plan, and the f64 column comes back as `[-1.0, 1.0]`.**

## PyPI approval

Task 1 (blocking-human) was approved by the user on 2026-11-03. The approved projects are exactly the PyPI names `numpy` (NumPy, author Travis E. Oliphant et al.) and `pyarrow` (Apache Arrow's Python package). No other Python project was installed. The temp project used by `py_array_stats_run` depends on `numpy>=2.1` and `pyarrow>=17` only, under `requires-python = ">=3.12"`, synced with uv and Python 3.12.

## Performance

- **Duration:** 62 min
- **Started:** 2026-11-03T22:02:06Z
- **Completed:** 2026-11-03T23:04:35Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `cargo test -p vox-compiler --test py_array_stats_run` exited 0. Result line: `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.85s`.
- The golden assert accepts `[-1.0, 1.0]`. The debug log has one `execute_plan` line and no wire method named mean or std.
- `materialize_to_list` is the only host entry that calls `Bridge::call` for this golden. Denial without the `py` capability returns `_Denied("py.to_list")`.

## Task Commits

Each task was committed atomically:

1. **Task 1: PyPI check** - no code commit. Approved 2026-11-03 for `numpy` and `pyarrow` only.
2. **Task 2: Run the z-score golden** - `d6418f5f3` (feat)

## Files Created/Modified

- `crates/vox-compiler/tests/py_array_stats_run.rs` - fail-closed worker run of the golden
- `crates/vox-compiler/src/eval/builtins.rs` - `materialize_to_list`, plan JSON, `from_list` / `astype` / `mean` / `std`
- `crates/vox-compiler/src/eval/expr.rs` - array sub/div, dtype idents, `to_list` intercept
- `crates/vox-compiler/src/eval/mod.rs` - `import std.array` binds the `array` namespace
- `crates/vox-compiler/src/eval/python.rs` - `ExecutePlan` on the existing host bridge
- `crates/vox-python-bridge/src/array_ipc.rs` - uncompressed f64 IPC column `values`
- `crates/vox-python-bridge/src/lib.rs` - re-exports the column codec
- `crates/vox-python-bridge/Cargo.toml` - `arrow-array` and `arrow-ipc` at `=60.1.0`, default features
- `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` - `HANDLERS` for the seven ops
- `crates/vox-python-bridge/worker/vox_bridge/__main__.py` - `execute_plan` dispatch and reply body
- `Cargo.lock` - arrow 60.1.0 and the existing compiler → bridge edge

## Decisions Made

- Host floats travel only in the Arrow body. `plan_params` emits op names, `astype` dtype `f64`, and `std` `correction`. It does not emit the host list.
- The debug line is `eprintln!("execute_plan")` after a successful call when `VOX_BRIDGE_DEBUG_OPS=1`. The plan JSON is not printed on that line.
- `arrow-ipc` and `arrow-array` are pinned `=60.1.0` with no feature list, so lz4 and zstd stay off.
- i64::MIN, uint64, and f32 stay out of this decoder. That slice is plan 04.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The interpreter could not build or run an ArrayPlan**
- **Found during:** Task 2 (run the z-score golden)
- **Issue:** `import std.array` did not bind `array`. `astype(f64)` looked up an ident. Sub and div had no ArrayPlan arms. `call_builtin_method` has no `Interpreter`, so it cannot call the bridge.
- **Fix:** Bind `array` on the module scope. Treat dtype idents as strings. Add ArrayPlan sub/div. Intercept `to_list` in `expr.rs` and call `materialize_to_list` on the existing host.
- **Files modified:** `crates/vox-compiler/src/eval/mod.rs`, `expr.rs`, `builtins.rs`, `python.rs`
- **Verification:** `py_array_stats_run` passed. Before the wiring, `vox run` of the golden exited 1 with `UndefinedVariable("array")`.
- **Committed in:** `d6418f5f3`

**2. [Rule 2 - Missing Critical] Same-file tests for the new public functions, and `py-plan:` must survive the worker error prefix**
- **Found during:** Task 2
- **Issue:** `materialize_to_list`, `encode_f64_column`, and `decode_f64_column` are public. A worker `except` that always prefixes `py:` would hide `py-plan:`.
- **Fix:** Added `materialize_to_list_denies_without_py`, `plan_json_keeps_host_numbers_out_of_params`, `execute_plan_missing_config_does_not_spawn`, and `f64_column_roundtrip`. The worker prefixes `py:` only when the message does not already start with `py-plan:` or `py:`.
- **Files modified:** `builtins.rs`, `python.rs`, `array_ipc.rs`, `__main__.py`
- **Verification:** those four tests passed.
- **Committed in:** `d6418f5f3`

**3. [Rule 3 - Blocking] The compiler must not take a new arrow crate edge**
- **Found during:** Task 2
- **Issue:** Decoding the reply in the compiler would mean a new arrow dependency on `vox-compiler`. The plan forbids editing `crates/vox-compiler/Cargo.toml` and forbids a new crate-edges row.
- **Fix:** Encode and decode the f64 column in `vox-python-bridge`, which already is a compiler dependency. `arrow-ipc` and `arrow-array` are `=60.1.0` with default features.
- **Files modified:** `crates/vox-python-bridge/src/array_ipc.rs`, `lib.rs`, `Cargo.toml`, `Cargo.lock`
- **Verification:** `f64_column_roundtrip` passed, and the golden round-tripped through pyarrow.
- **Committed in:** `d6418f5f3`

---

**Total deviations:** 3 auto-fixed (2 Rule 3, 1 Rule 2)
**Impact on plan:** Required for the golden to reach the existing bridge. No new HirType, no `array` capability, no second Bridge, no pickle or torch, no add/mul/sort/item/shape.

## TDD Gate

Plan type is `execute`. Task 2 sets `tdd="true"`. RED was the existing `vox` binary: `vox run examples/golden/py_array_stats.vox` exited 1 with `UndefinedVariable("array")`. The dispatch asked for one commit for the task, so the test and the implementation landed together in `d6418f5f3` after green. The test is not ignored.

## Issues Encountered

Parallel `rustc` processes stopped making progress (CPU frozen, state sleeping) during `cargo build -p vox-cli`. The build that produced `target/debug/vox` used `CARGO_INCREMENTAL=0`, `RUSTC_WRAPPER=/usr/bin/env`, and `-j1`. That is a local compile workaround, not a source change.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 06 is complete. 326-02, 326-04, 326-07, and 326-08 were not started.
- `STATE.md`, `ROADMAP.md`, and `REQUIREMENTS.md` were left untouched.
- Plan 04 still owns i64::MIN, uint64, and f32.

---
*Phase: 326-std-array-on-numpy-arr-01-04-k-01*
*Completed: 2026-11-03*

## Self-Check: PASSED

- FOUND: `d6418f5f3`
- FOUND: `crates/vox-compiler/tests/py_array_stats_run.rs`
- FOUND: `crates/vox-python-bridge/src/array_ipc.rs`
- FOUND: `crates/vox-python-bridge/worker/vox_bridge/array_ops.py`
- FOUND: `.planning/phases/326-std-array-on-numpy-arr-01-04-k-01/326-06-SUMMARY.md`
