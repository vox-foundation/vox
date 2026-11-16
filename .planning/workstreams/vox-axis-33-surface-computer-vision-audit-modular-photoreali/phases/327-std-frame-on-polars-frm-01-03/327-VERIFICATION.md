---
phase: 327-std-frame-on-polars-frm-01-03
verified: 2026-11-13T07:04:00Z
status: passed
score: 3/3 must-haves verified
---

# Phase 327 Verification (`std.frame` on Polars: `FRM-01`, `FRM-02`, `FRM-03`)

**Phase Goal:** Expose `std.frame` lazy dataframe pipelines backed by `polars` through a single `execute_plan` RPC, enforcing compile-time/runtime schema & null safety and host `fs:ro` capability boundaries.

## Requirement Matrix

| Requirement | Description | Verification Command | Outcome |
|---|---|---|---|
| `FRM-01` | `std.frame` lazy pipeline (`scan_csv`, `filter`, `group_by`, `agg`, `sort`, `collect`) compiles and executes via one `execute_plan` RPC on `polars==2.1.0`; `Records` exposes `num_rows()` and `column(...)` | `timeout 900s cargo test -p vox-compiler --test py_frame_report_run` | Passed (`3/3`, `43.17s`, `execute_plan == 1`) |
| `FRM-02` | Null cells and unsupported Arrow types fail closed with `py-plan:` diagnostic (`frame_ops.py` & Arrow IPC `decode_frame_columns`) | `timeout 900s cargo test -p vox-python-bridge --lib frame` & `py_frame_report_run::frame_null_csv_is_py_plan` | Passed (`2/2` unit + integration) |
| `FRM-03` | Host `fs:ro` path confinement & missing column checks (compile-time and runtime `py-plan:`) | `timeout 900s cargo test -p vox-compiler --test scan_csv_fs_cap` & `py_frame_report_run::frame_missing_column_runtime_is_py_plan` | Passed (`1/1` + integration) |

## Plans Completed

- `327-01` through `327-06` (`6/6` complete, operator approved `polars==2.1.0` in `327-05`).
