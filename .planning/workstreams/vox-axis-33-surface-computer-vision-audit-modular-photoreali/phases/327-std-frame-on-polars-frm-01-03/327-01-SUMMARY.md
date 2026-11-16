---
phase: 327-std-frame-on-polars-frm-01-03
plan: "01"
subsystem: compiler
tags: [std.frame, polars, frame-plan, allowlist]

requires:
  - phase: 326
    provides: std.array host plan, ecosystem-support allowlist, and execute_plan
provides:
  - Pure FramePlan that records scan_csv, filter, group_by, agg, and sort with zero worker calls
  - Sibling std.frame/polars allowlist of twenty ops, with collect as the only materializer
  - Generated python_frame_ops table from frame-support.yaml
affects: [327-02, 327-03, 327-04, 327-05, 327-06]

actuals:
  tokens: 6016
  tasks: 1
  commits: 1

tech-stack:
  added: []
  patterns: [sibling allowlist emitted beside the array table, host plan copies a worker counter and never increments it]

key-files:
  created:
    - contracts/python/frame-support.yaml
    - contracts/python/frame-plan-ir.v1.schema.json
    - crates/vox-compiler/src/eval/frame_plan.rs
    - crates/vox-compiler/src/python_frame_support.rs
  modified:
    - contracts/index.yaml
    - crates/vox-compiler/build.rs
    - crates/vox-compiler/src/lib.rs
    - crates/vox-compiler/src/eval/mod.rs

key-decisions:
  - "std.frame is a sibling allowlist. ecosystem-support.yaml and plan-ir.v1.schema.json stay the twelve array ops."
  - "FramePlan stores the path it is given and does not store a collect step. Constructors copy the worker counter and do not call the worker."
  - "sort stores the column string and the nulls string separately. nulls is recorded only for first and last."

patterns-established:
  - "Pattern: emit_python_frame_ops mirrors the array emitter and panics on an empty signature, an empty handler, a handler that differs from name, a non-mapping expected, or an unknown kind."
  - "Pattern: collect is the only PythonFrameOpKind::Materializer."

requirements-completed: [FRM-01]

coverage:
  - id: D1
    description: A host FramePlan built through scan_csv, filter, group_by, agg, and sort records zero worker calls and does not store a collect step.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_plan_collect_is_one_round_trip
        status: pass
    human_judgment: false
  - id: D2
    description: The generated std.frame table has the twenty op names, each handler equals its name, and collect is the only materializer.
    requirement: FRM-01
    verification:
      - kind: unit
        ref: cargo test -p vox-compiler --lib frame_support_collect_is_only_materializer
        status: pass
      - kind: unit
        ref: cargo test -p vox-compiler --lib array_plan_zscore_is_pure
        status: pass
    human_judgment: false

duration: 23min
completed: 2026-11-04
status: complete
plan_head_before: c6996849ed7fb9ae845171613867ac196f59e851
plan_head_after: 38c4b58d7b3132958f8c84e01852ba853ad57d9a
---

# Phase 327 Plan 01: Pure frame plan and sibling allowlist Summary

**Host `FramePlan` records scan, filter, group, agg, and sort with zero worker calls, and the sibling allowlist marks `collect` as the only materializer.**

## Performance

- **Duration:** 23 min
- **Started:** 2026-11-04T04:41:13Z
- **Completed:** 2026-11-04T05:04:22Z
- **Tasks:** 1
- **Files modified:** 8

## Accomplishments

- `FramePlan::scan_csv`, `filter`, `group_by`, `agg`, and `sort` return another plan, copy the process-global worker counter, and do not store a `collect` step.
- `sort` stores the column and `nulls` separately. Only `first` and `last` are recorded.
- `contracts/python/frame-support.yaml` lists the twenty `std.frame` ops. `collect` is the only `materializer`. `build.rs` emits `python_frame_ops` from that file.
- The array plan-ir name enum was left as the twelve stats-slice ops. `execute_plan` was not called.

## Task Commits

Each task was committed atomically:

1. **Task 1: Sibling allowlist and a pure FramePlan** - `38c4b58d7` (feat)

## Files Created/Modified

- `contracts/python/frame-support.yaml` - twenty `std.frame` ops on polars, handler equal to name
- `contracts/python/frame-plan-ir.v1.schema.json` - plan IR envelope whose name enum is those twenty ops
- `contracts/index.yaml` - `python-frame-support` and `python-frame-plan-ir` after `python-plan-ir`
- `crates/vox-compiler/build.rs` - `emit_python_frame_ops` beside the array emitter
- `crates/vox-compiler/src/python_frame_support.rs` - `PythonFrameOp` wrapper and `frame_support_collect_is_only_materializer`
- `crates/vox-compiler/src/eval/frame_plan.rs` - `FramePlan`, `FrameExpr`, and `frame_plan_collect_is_one_round_trip`
- `crates/vox-compiler/src/eval/mod.rs` - `pub mod frame_plan`
- `crates/vox-compiler/src/lib.rs` - `pub mod python_frame_support`

## Decisions Made

- The frame allowlist is a new document. `ecosystem-support.yaml`, its schema, and `plan-ir.v1.schema.json` were not edited.
- `FramePlan::scan_csv` stores the path string it is given. Filesystem canonicalization stays with the later `fs` cap check.
- Expression ops (`col`, `eq`, `sum`, `.alias`) live on `FrameExpr`. They are not extra steps on the plan.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

The first `cargo test -p vox-compiler --lib frame_plan_collect_is_one_round_trip` was killed with exit 137 while `vox-compiler` was still compiling. The same command was retried once. That run exited 101 because `FramePlan` and `python_frame_support` were not defined. After the implementation, the test exited 0 with 1 passed. `frame_support_collect_is_only_materializer` and `array_plan_zscore_is_pure` also exited 0.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02 can typecheck against `python_frame_ops`. Plan 03 can refuse a path in `scan_csv` and send `collect` through the existing `execute_plan`. Polars is not installed.

## Self-Check: PASSED

- FOUND: contracts/python/frame-support.yaml
- FOUND: contracts/python/frame-plan-ir.v1.schema.json
- FOUND: crates/vox-compiler/src/eval/frame_plan.rs
- FOUND: crates/vox-compiler/src/python_frame_support.rs
- FOUND: 38c4b58d7

---
*Phase: 327-std-frame-on-polars-frm-01-03*
*Completed: 2026-11-04*
