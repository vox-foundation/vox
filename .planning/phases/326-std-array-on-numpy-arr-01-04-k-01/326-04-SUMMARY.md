---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 04
subsystem: python-bridge
tags: [std.array, numpy, arrow, k-complexity, signature-ceiling]

requires:
  - phase: 326-06
    provides: z-score golden that materializes through one execute_plan on the uv worker
provides:
  - NaN, ±Inf, and i64::MAX round-trip through three to_list materializers
  - Host decode of i64::MIN, uint64 overflow, and f32 widening
  - Measured py_array_stats budget keys and a separate signature-manifest ceiling
affects: [326-verify, std.array]

actuals:
  tokens: 8228
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Integer host columns stay Arrow int64 until decode; floats stay float64"
    - "K2 signature ceiling lives outside the WebIR complexity map"

key-files:
  created:
    - crates/vox-python-bridge/src/array_numeric.rs
    - contracts/python/signature-manifest-budget.v1.json
  modified:
    - examples/golden/py_array_stats.vox
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/tests/python_bridge_parity_test.rs
    - contracts/eval/complexity-budget.v1.json
    - contracts/eval/source-token-budget.v1.json

key-decisions:
  - "i64::MAX is an integer host column; f64 cannot round-trip 2^63-1"
  - "uint64 above i64::MAX errors with a py: prefix and is not wrapped"
  - "f32 widens with f64::from, not from_bits"
  - "Signature ceiling is contracts/python/signature-manifest-budget.v1.json, not the WebIR map"

patterns-established:
  - "Three to_list calls are three execute_plan frames; mean and std stay inside the plan"
  - "i64::MIN is injected as Arrow bytes in the Rust test, never as a Vox literal"

requirements-completed: [ARR-01, ARR-03, K-01]

coverage:
  - id: D1
    description: Golden materializes NaN, ±Inf, and i64::MAX through three to_list calls
    requirement: ARR-03
    verification:
      - kind: e2e
        ref: "cargo test -p vox-compiler --test py_array_stats_run (1 passed, 0 ignored)"
        status: pass
    human_judgment: false
  - id: D2
    description: Host decode keeps i64::MIN, rejects uint64 past i64::MAX, and widens f32
    requirement: ARR-03
    verification:
      - kind: unit
        ref: "cargo test -p vox-compiler --lib array_numeric_roundtrip (1 passed); cargo test -p vox-python-bridge --lib decode_column (6 passed)"
        status: pass
    human_judgment: false
  - id: D3
    description: Measured py_array_stats budgets and the twelve-signature K2 ceiling
    requirement: K-01
    verification:
      - kind: other
        ref: "cargo run -p vox-cli -- ci k-complexity-budget; cargo run -p vox-cli -- ci source-token-budget; cargo test -p vox-compiler --test python_bridge_parity (3 passed)"
        status: pass
    human_judgment: false

duration: 44min
completed: 2026-11-04
status: complete
commits: 3
plan_head_before: 23d2fc815b07b41bbf556a19c48535903f270a03
plan_head_after: 4edebdcf18ca5277768c073c49c758b48b1b7692
---

# Phase 326 Plan 04: Numeric Extremes and K Budgets Summary

**std.array round-trips NaN, infinities, and i64::MAX through the numpy worker, and the host rejects uint64 overflow while widening f32 with a numeric cast.**

## Performance

- **Duration:** 44 min
- **Started:** 2026-11-04T00:34:11Z
- **Completed:** 2026-11-04T01:18:23Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- The golden `py_array_stats.vox` materializes NaN, +Inf, -Inf, and `9223372036854775807` with three `to_list` calls (three `execute_plan` frames). `mean` and `std` stay inside the plan.
- `decode_column` keeps `i64::MIN`, returns `py: uint64 {value} does not fit i64` above `i64::MAX`, widens f32 with `f64::from`, and rejects an over-cap body before reading the stream.
- Measured budgets, not guessed: complexity `fixtures.py_array_stats` = 120, source tokens = 280, source bytes = 1223, signature ceiling = 135.

## Task Commits

Each task was committed atomically:

1. **Task 1: NaN, Inf, and i64::MAX in the golden** - `98cad7490` (feat)
2. **Task 2: i64::MIN, uint64, and f32 widening on the to_list host** - `eb4ebe5f8` (feat)
3. **Task 3: Budget keys and the K2 signature ceiling** - `4edebdcf1` (feat)

## Files Created/Modified

- `examples/golden/py_array_stats.vox` — NaN/`isnt`, infinities, and i64::MAX, plus a discriminating subtraction assert
- `crates/vox-compiler/tests/py_array_stats_run.rs` — three `execute_plan` frames; source must contain the MAX literal and `isnt`, not `9223372036854775808`
- `crates/vox-compiler/src/eval/array_plan.rs` — `HostColumn::{F64,I64}`
- `crates/vox-compiler/src/eval/builtins.rs` — int `from_list`, `decode_column` on the materialize path, test reply seam
- `crates/vox-python-bridge/src/array_ipc.rs` — int64 column encode/decode
- `crates/vox-python-bridge/src/array_numeric.rs` — capped `decode_column`, f32 and uint64 encoders
- `crates/vox-python-bridge/worker/vox_bridge/array_ops.py` — preserve `np.int64` through the plan
- `contracts/eval/complexity-budget.v1.json` — `py_array_stats`: 120
- `contracts/eval/source-token-budget.v1.json` — tokens 280, bytes 1223
- `contracts/python/signature-manifest-budget.v1.json` — ceiling 135 for the twelve signatures
- `contracts/index.yaml` — `python-signature-manifest-budget`
- `crates/vox-compiler/tests/python_bridge_parity_test.rs` — ceiling test; WebIR map must not store `ceiling`

## Decisions Made

- Integer lists take an int64 host column. An f64 host rounds `i64::MAX` to `2^63`, and cross-numeric `is` would still pass, so the golden also asserts `(n - 9223372036854775806) is 1`.
- `i64::MIN` is not a Vox literal. The compiler test injects Arrow bytes from `encode_i64_column` through a `cfg(test)` reply seam. Production still calls the worker.
- uint64 above `i64::MAX` is an error starting with `py:`. It is not saturated, truncated, or wrapped.
- f32 widens with `f64::from`. `0.1f32` widened is not `0.1f64`.
- The signature ceiling is not a key in `complexity-budget.v1.json`. Budget commands were not passed `--update`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Integer host column so the golden can run**
- **Found during:** Task 1 (NaN, Inf, and i64::MAX in the golden)
- **Issue:** `from_list` rejected ints (`from_list expects floats, found Int`), and an f64 host cannot round-trip `i64::MAX`.
- **Fix:** `HostColumn::I64`, `encode_i64_column`, and a worker path that keeps `np.int64` until the final encode.
- **Files modified:** `array_plan.rs`, `builtins.rs`, `array_ipc.rs`, `array_ops.py`
- **Verification:** `cargo test -p vox-compiler --test py_array_stats_run` — 1 passed, 0 ignored
- **Committed in:** `98cad7490`

**2. [Rule 2 - Correctness] Discriminating i64::MAX assert**
- **Found during:** Task 1
- **Issue:** `Int(i64::MAX) is Float(2^63)` is true, so a lone `is` against the literal accepts the rounded float.
- **Fix:** `assert((n - 9223372036854775806) is 1)` in the golden.
- **Files modified:** `examples/golden/py_array_stats.vox`
- **Verification:** same run test, 1 passed
- **Committed in:** `98cad7490`

**3. [Rule 2 - Correctness] `main` needs `@uses(python)`**
- **Found during:** Task 2
- **Issue:** `to_list` in `main` requires the python capability. `zscore` stays the public decorated function so the effect-strip test still sees the first `@uses(python)`.
- **Fix:** `@uses(python) fn main()` without `pub`.
- **Files modified:** `examples/golden/py_array_stats.vox`
- **Verification:** `cargo test -p vox-compiler --lib zscore_golden_typechecks` passed with the run test
- **Committed in:** `98cad7490` (decorator); host decode in `eb4ebe5f8`

**4. [Rule 2 - Correctness] Test seam for bodies the compiler cannot build**
- **Found during:** Task 2
- **Issue:** `vox-compiler` must not depend on arrow, and `i64::MIN` cannot be a Vox integer literal.
- **Fix:** `set_reply_body_for_test` feeds bytes from bridge encoders into `materialize_to_list`, which calls `decode_column`.
- **Files modified:** `builtins.rs`, `array_numeric.rs`
- **Verification:** `cargo test -p vox-compiler --lib array_numeric_roundtrip` — 1 passed; `cargo test -p vox-python-bridge --lib decode_column` — 6 passed
- **Committed in:** `eb4ebe5f8`

---

**Total deviations:** 4 auto-fixed (1 Rule 3, 3 Rule 2)
**Impact on plan:** Required for the golden e2e and for the numeric cases the plan forbids as Vox literals. No new packages, no arrow features, no new crate edge.

## Test commands

| Command | Result |
| --- | --- |
| `cargo test -p vox-python-bridge --lib column_roundtrip` | 2 passed |
| `cargo test -p vox-compiler --lib zscore_golden_typechecks` | passed (with `from_list_keeps_i64_max` and `array_plan_zscore_is_pure`: 3 passed) |
| `cargo test -p vox-compiler --test py_array_stats_run` | 1 passed, 0 failed, 0 ignored |
| `cargo test -p vox-python-bridge --lib decode_column` | 6 passed |
| `cargo test -p vox-compiler --lib array_numeric_roundtrip` | 1 passed |
| `cargo test -p vox-compiler --test python_bridge_parity` | 3 passed, 0 failed |
| `cargo run -p vox-cli -- ci k-complexity-budget` | exit 0, `K-complexity budget OK (13 ladder fixtures validated)` |
| `cargo run -p vox-cli -- ci source-token-budget` | exit 0, `Source-token budget OK (13 ladder fixtures validated)` |

Measured integers: complexity 120, tokens 280, bytes 1223, signature ceiling 135 (concatenated signature bytes 354).

## Issues Encountered

The first golden run failed closed on `from_list expects floats, found Int` after NaN and Inf had already materialized. The int64 host path fixed that. `target/debug/vox` was rebuilt before the run test, because that test spawns the debug binary.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 04 is the last plan in phase 326. ARR-03 numeric extremes and K-01 budgets are in place. Verification, phase 327, and phase 331 were not started.

## Self-Check: PASSED

- FOUND: `98cad7490`, `eb4ebe5f8`, `4edebdcf1` (ancestors of HEAD at summary write)
- FOUND: golden, run test, host decode, budget files, signature ceiling
- TEMP_GONE: `crates/vox-compiler/tests/measure_326_04_tmp.rs`

## Known Stubs

None. The reply seam is `cfg(test)` only. Production `materialize_to_list` still encodes the host column and calls `execute_plan`.
