---
phase: 328-charts-and-model-inference-from-vox
plan: "01"
subsystem: python-interop
tags: [std.plot, std.tensor, pt2, seed, workflow-journal]
requires:
  - phase: 327
    provides: FramePlan and worker bridge
provides:
  - "`std.plot` (`line`, `scatter`, `bar`, `hist`, `x`, `y`, `title`, `legend`, `save`)"
  - "`std.tensor` (`load_exported` for `.pt2`, `Model.{run, device, to_device}`)"
  - "`Array.{reshape, softmax, astype(f32)}`"
  - "Seeded worker determinism and `ActivityCompleted.python_env` journal provenance"
affects:
  - crates/vox-compiler
  - crates/vox-python-bridge/worker
  - crates/vox-workflow-runtime
key-files:
  created:
    - crates/vox-compiler/src/eval/plot_plan.rs
    - crates/vox-compiler/src/eval/tensor_model.rs
    - crates/vox-python-bridge/worker/vox_bridge/plot_ops.py
    - crates/vox-python-bridge/worker/vox_bridge/tensor_ops.py
    - examples/golden/fixtures/tiny_linear.pt2
    - examples/golden/py_plot_report.vox
    - examples/golden/py_tensor_infer.vox
    - crates/vox-compiler/tests/py_plot_and_tensor_run.rs
  modified:
    - crates/vox-compiler/src/eval/array_plan.rs
    - crates/vox-compiler/src/eval/builtins.rs
    - crates/vox-compiler/src/eval/expr.rs
    - crates/vox-compiler/src/eval/python.rs
    - crates/vox-compiler/src/eval/activity_hook.rs
    - crates/vox-compiler/src/typeck/builtins.rs
    - crates/vox-compiler/src/typeck/checker/expr.rs
    - crates/vox-workflow-runtime/src/workflow/host.rs
    - crates/vox-workflow-runtime/src/workflow/run.rs
key-decisions:
  - "Registered `reshape` and `softmax` in `typeck/builtins.rs` for `Ty::Array(_)` without altering `D01_OPS` in `contracts/python/ecosystem-support.yaml`."
  - "Implemented worker PNG fallback via stdlib `zlib` alongside matplotlib `Agg` so headless/minimal Python environments render valid PNGs without returning frame data to the host."
  - "Refused `.pt`, `.pth`, `.pkl`, `.pickle`, and pickle magic `\\x80` on both host (`builtins.rs`) and worker (`tensor_ops.py`)."
requirements-completed: [PLT-01, TNS-01, TNS-02]
completed: 2026-11-13
---

# Phase 328 Plan 01 Summary

Implemented `std.plot` (`line`, `scatter`, `bar`, `hist`, `x`, `y`, `title`, `legend`, `save`), `std.tensor` (`load_exported` for `.pt2`, `Model.run`, `device`, `to_device`), `Array.{reshape, softmax, astype(f32)}`, seed forwarding (`--caps random:seed=N`), and durable activity journal `python_env` provenance (`python`, `packages`, `uv_lock_blake3`).

## Verification Results
- `cargo test -p vox-python-bridge`: 20 passed (13 unit + 7 lifecycle), 0 failed.
- `cargo test -p vox-compiler --test py_plot_and_tensor_run`: 4 passed, 0 failed (`plot_png_and_tensor_pt2_cpu_seeded_end_to_end` skipped live worker numpy/polars sub-branch when absent; stdlib PNG & `.pt2` zip checks verified directly with `python3`).
- `cargo test -p vox-workflow-runtime --lib`: 82 passed (including `activity_completed_records_python_env_metadata`), 0 failed, 1 ignored.
- `cargo test -p vox-compiler --lib -- --test-threads=1`: 642 passed, 0 failed, 1 ignored.
