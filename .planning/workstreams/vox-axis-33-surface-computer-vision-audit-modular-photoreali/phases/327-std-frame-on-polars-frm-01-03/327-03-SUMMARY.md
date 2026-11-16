---
phase: 327-std-frame-on-polars-frm-01-03
plan: "03"
subsystem: compiler
tags: [std.frame, scan_csv, fs:ro, collect, execute_plan, frame_ops, polars]

requires:
  - phase: 327-01
    provides: FramePlan, frame allowlist, python_frame_ops
  - phase: 327-06
    provides: VoxValue::Records and the golden CSV fixture
  - phase: 327-02
    provides: StdFrameNs typeck and .and( / .or( method parse
provides:
  - scan_csv_outside_fs_ro_never_calls_worker host path denial
  - import std.frame bind and namespace scan_csv / col
  - materialize_collect calling existing execute_plan with family frame
  - frame_ops.py HANDLERS and __main__ family dispatch
affects: [327-04, 327-05]

actuals:
  tokens: 8184
  tasks: 2
  commits: 2

plan_head_before: 0b6f57fd6ab9987f15471d3f2fa29095eccf279e
plan_head_after: d2a3f32d4db770ddd32192f469fc2d78238c33f9

tech-stack:
  added: []
  patterns: [fs_resolve_allowed before FramePlan store, family frame on execute_plan, polars import inside execute_plan]

key-files:
  created:
    - crates/vox-compiler/tests/scan_csv_fs_cap.rs
    - crates/vox-python-bridge/worker/vox_bridge/frame_ops.py
  modified:
    - crates/vox-compiler/src/eval/mod.rs
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/src/eval/expr.rs
    - crates/vox-compiler/src/eval/frame_plan.rs
    - crates/vox-compiler/tests/python_bridge_parity_test.rs
    - crates/vox-python-bridge/worker/vox_bridge/__main__.py

key-decisions:
  - "scan_csv denies with fs.scan_csv via fs_resolve_allowed before any FramePlan is stored."
  - "collect is the only execute_plan call; params carry family frame. No execute_frame."
  - "Python keywords and/or use def and_/or_ with HANDLERS keys still named and/or."

patterns-established:
  - "Pattern: materialize_collect mirrors materialize_to_list (py denial, canned-body hook, one execute_plan)."
  - "Pattern: __main__ routes family frame to frame_ops and leaves array_ops on every other plan."

requirements-completed: [FRM-01, FRM-03]

coverage:
  - id: D1
    description: scan_csv outside fs:ro and a parent-dir path are CapabilityDenied for fs.scan_csv and do not spawn a worker.
    requirement: FRM-03
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --test scan_csv_fs_cap
        status: pass
    human_judgment: false
  - id: D2
    description: collect without py is py.collect once; frame_ops handlers match frame-support.yaml; array twelve-name parity still passes.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_collect_without_python_is_denied
        status: pass
      - kind: unit
        ref: cargo test -p vox-compiler --test python_bridge_parity
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-11-04
status: complete
---

# Phase 327 Plan 03: Path denial, one collect, frame worker module Summary

**Host `fs:ro` blocks `scan_csv` before any worker request; `collect` is the single `execute_plan` with `family: frame` and a closed `frame_ops.py` handler set.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-11-04T06:12:59Z
- **Completed:** 2026-11-04T06:27:48Z
- **Tasks:** 2/2
- **Files modified:** 8

## Accomplishments

- `scan_csv` uses `fs_resolve_allowed`; outside grant and `..` paths return `_Denied("fs.scan_csv")` without storing a plan.
- `import std.frame` binds the namespace; plan methods only append steps; `frame_plan_params` emits `family: frame`.
- `materialize_collect` is the only frame worker entry; missing `py` is `_Denied("py.collect")`.
- `frame_ops.py` owns HANDLERS (polars imported inside `execute_plan`); `__main__.py` routes `family == "frame"`.

## Task Commits

1. **Task 1: scan_csv refuses a path before any worker request** - `1c6423f45` (feat)
2. **Task 2: collect calls execute_plan once, on the frame family** - `d2a3f32d4` (feat)

## Files Created/Modified

- `crates/vox-compiler/tests/scan_csv_fs_cap.rs` - FRM-03 denial integration test
- `crates/vox-compiler/src/eval/mod.rs` - `import std.frame` bind
- `crates/vox-compiler/src/eval/builtins.rs` - frame namespace, FramePlan/FrameExpr methods, `materialize_collect`, `frame_plan_params`
- `crates/vox-compiler/src/eval/frame_plan.rs` - `param_steps` / public `FrameStep`
- `crates/vox-compiler/src/eval/expr.rs` - `collect` intercept on `FramePlan`
- `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` - allowlisted handlers
- `crates/vox-python-bridge/worker/vox_bridge/__main__.py` - family frame dispatch
- `crates/vox-compiler/tests/python_bridge_parity_test.rs` - frame allowlist parity

## Decisions Made

- Worker stub for the denial test is an empty `.venv/bin/python` so `venv_python` selects that path instead of `python3` on PATH; a spawn would not look like `CapabilityDenied`.
- `and`/`or` HANDLERS keys stay Vox names; defs are `and_`/`or_` because they are Python keywords.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `param_steps` on FramePlan**
- **Found during:** Task 1 (`frame_plan_params`)
- **Issue:** Serializing ops required step access; `FrameStep` was private.
- **Fix:** Made `FrameStep` public and added `param_steps`.
- **Files modified:** `crates/vox-compiler/src/eval/frame_plan.rs`
- **Committed in:** `1c6423f45`

**2. [Rule 2 - Missing Critical] Keyword-safe frame HANDLERS check**
- **Found during:** Task 2 (parity test)
- **Issue:** `def and(` / `def or(` are SyntaxErrors in Python.
- **Fix:** `and_`/`or_` defs with HANDLERS keys `"and"`/`"or"`; frame parity helper accepts that shape. Array twelve-name check unchanged.
- **Files modified:** `frame_ops.py`, `python_bridge_parity_test.rs`
- **Committed in:** `d2a3f32d4`

---

**Total deviations:** 2 auto-fixed (both Rule 2)
**Impact on plan:** Required for correctness; no scope creep into plans 04/05.

## Issues Encountered

None beyond the keyword HANDLERS shape above.

## User Setup Required

None - no external service configuration required. Polars install remains deferred (D-12).

## Next Phase Readiness

- Plan 04 can decode canned Arrow bodies into `Records` via the existing reply-body hook.
- Plan 05 can count `execute_plan` lines; `python.rs` was not edited.
- mid.rs leftover remains unstaged by design.

## Self-Check: PASSED

- `crates/vox-compiler/tests/scan_csv_fs_cap.rs` — FOUND
- `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` — FOUND
- commits `1c6423f45`, `d2a3f32d4` — FOUND as ancestors of HEAD
- `python.rs` unchanged; no `execute_frame`; mid.rs / STATE / ROADMAP / REQUIREMENTS not in task commits

---
*Phase: 327-std-frame-on-polars-frm-01-03*
*Completed: 2026-11-04*
