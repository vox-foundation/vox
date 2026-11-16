---
phase: 328
status: fixed
reviewer: antigravity
reviewed_at: 2026-11-13T08:12:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/vox-compiler/src/eval/plot_plan.rs
  - crates/vox-compiler/src/eval/tensor_model.rs
  - crates/vox-python-bridge/worker/vox_bridge/plot_ops.py
  - crates/vox-python-bridge/worker/vox_bridge/tensor_ops.py
  - crates/vox-compiler/tests/py_plot_and_tensor_run.rs
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
---

# Phase 328 Code Review (`std.plot` & `std.tensor`)

### WR-328-01: Matplotlib figure leak on exception in `execute_plot_plan` (`plot_ops.py`)
- **Severity:** Warning
- **Location:** `crates/vox-python-bridge/worker/vox_bridge/plot_ops.py:129-146`
- **Issue:** `fig, ax = plt.subplots(...)` was followed directly by `ax.plot`/`scatter`/`bar`/`hist` and `fig.savefig(...)`, with `plt.close(fig)` outside a `try ... finally` block. If plotting, directory creation, or `fig.savefig` raised an exception before reaching the fallback PNG path, the figure remained open in matplotlib's global figure registry inside the long-lived Python worker process.
- **Fix Applied (see `328-REVIEW-FIX.md`):** Wrapped figure rendering and saving in `try: ... finally: plt.close(fig)`.
