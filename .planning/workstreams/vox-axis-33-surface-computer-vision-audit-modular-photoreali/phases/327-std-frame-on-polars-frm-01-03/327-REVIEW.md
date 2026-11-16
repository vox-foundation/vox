---
phase: 327
status: clean
reviewer: antigravity
reviewed_at: 2026-11-13T08:12:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/vox-compiler/src/eval/frame_plan.rs
  - crates/vox-python-bridge/worker/vox_bridge/frame_ops.py
  - crates/vox-compiler/src/eval/builtins.rs
  - crates/vox-compiler/src/typeck/builtins.rs
findings:
  critical: 0
  warning: 0
  info: 1
  total: 1
---

# Phase 327 Code Review (`std.frame` on polars)

- **Clean (0 Critical, 0 Warning):**
  - `FramePlan` and `FrameExpr` in `crates/vox-compiler/src/eval/frame_plan.rs` stay pure lazy descriptions (`PLAN_WORKER_CALLS` untouched by constructors), forwarding only explicit user-permitted paths and operator descriptors to the worker.
  - `crates/vox-python-bridge/worker/vox_bridge/frame_ops.py` enforces strict allowlisted expression/step dispatch (`HANDLERS` and `_build_expr`), disabling path globbing (`pl.scan_csv(path, glob=False)`), coercion via `_coerce_literal`, and fail-closed `PlanError` on unknown ops.
  - **IN-327-01 (Info):** `py_frame_report` golden and host `--caps fs` boundary tests were already verified passing in `327-VERIFICATION.md` (`327-01`..`327-06`).
