---
phase: 328
status: all_fixed
findings_in_scope: 1
fixed: 1
skipped: 0
iteration: 1
fixed_at: 2026-11-13T08:19:00Z
---

# Phase 328 Code Review Fix Report

## Fixed

### WR-328-01: Matplotlib figure leak on exception in `execute_plot_plan`
- **File:** `crates/vox-python-bridge/worker/vox_bridge/plot_ops.py:129-148`
- **Change:** Wrapped chart plotting, directory creation, and `fig.savefig(out_path, format="png")` in `try: ... finally: plt.close(fig)` so figures are deterministically released even when plotting or disk writes raise before reaching the fallback PNG renderer.

## Verification
- `python3 -m py_compile crates/vox-python-bridge/worker/vox_bridge/__main__.py crates/vox-python-bridge/worker/vox_bridge/plot_ops.py`: exit `0`.
- `timeout 900s cargo test -p vox-compiler --test py_plot_and_tensor_run`: `4 passed; 0 failed` (`0.12s`).
