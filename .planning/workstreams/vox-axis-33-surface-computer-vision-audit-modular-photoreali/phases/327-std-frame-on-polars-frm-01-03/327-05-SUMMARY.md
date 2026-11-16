---
phase: 327-std-frame-on-polars-frm-01-03
plan: "05"
subsystem: compiler-python-bridge
tags: [std.frame, polars, py_frame_report, integration-test]
requires:
  - phase: 327-02
    provides: std.frame typecheck & compile-time CSV header check
  - phase: 327-03
    provides: host fs:ro path denial before worker dispatch
  - phase: 327-04
    provides: frame_ops.py handlers and decode_frame_columns
  - phase: 327-06
    provides: VoxValue::Records and examples/golden/py_frame_report.vox
provides:
  - Approved polars==2.1.0 golden integration verification (py_frame_report_run, frame_null_csv_is_py_plan, frame_missing_column_runtime_is_py_plan)
affects: []
tech-stack:
  added: [polars==2.1.0 (temp uv venv test fixture only)]
  patterns: [fail-closed uv worker integration tests]
key-files:
  created: []
  modified:
    - crates/vox-compiler/tests/py_frame_report_run.rs
key-decisions:
  - "User approved polars==2.1.0 on PyPI (resume signal: approved for polars)."
  - "Real worker execution against frame_ops.py succeeded without changes to frame_ops.py."
requirements-completed: [FRM-01, FRM-02, FRM-03]
completed: 2026-11-13
---

# Phase 327 Plan 05: `std.frame` Golden Run on `polars==2.1.0` Summary

**User-approved `polars==2.1.0` runs `examples/golden/py_frame_report.vox` end-to-end via `vox run` with one `execute_plan` call, and fail-closed `py-plan:` errors on nulls and missing runtime columns.**

## Checkpoint Resolution

- **Task 1 (`Confirm polars 2.1.0 on PyPI before any install`):** Operator approved (`approved for polars`).

## Verification

- `timeout 900s cargo test -p vox-compiler --test py_frame_report_run`: **3 passed; 0 failed; 0 ignored** (`43.17s`)
  - `py_frame_report_run`: synced `numpy==2.5.4`, `pyarrow==26.0.1`, `polars==2.1.0` in a temporary `uv` Python 3.12 environment, ran `examples/golden/py_frame_report.vox` with `VOX_BRIDGE_DEBUG_OPS=1`, confirmed `num_rows() == 2`, `revenue == [10, 5]`, and exactly `1` `execute_plan` line.
  - `frame_null_csv_is_py_plan`: verified null cell in `amount` fails closed with `py-plan:`, `amount`, and `scan_csv`.
  - `frame_missing_column_runtime_is_py_plan`: verified runtime-variable CSV path with missing column fails closed with `py-plan:`.
- `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py`: required zero edits against `polars==2.1.0`.
