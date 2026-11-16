---
phase: 329
status: all_fixed
findings_in_scope: 2
fixed: 2
skipped: 0
iteration: 1
fixed_at: 2026-11-13T08:19:00Z
---

# Phase 329 Code Review Fix Report

## Fixed

### WR-329-01: Hardened denied-tier alias coverage & sidecar filename sanitization
- **Files:** `crates/vox-compiler/src/python_escape_support.rs`, `crates/vox-python-bridge/worker/vox_bridge/__main__.py`, `crates/vox-compiler/tests/py_escape_hatch_test.rs`
- **Change:**
  - Extended host `is_denied_python_target` and worker `_is_denied_escape_target` to reject `_posixsubprocess`, `_imp`, `imp`, `torch.*.load` (e.g. `torch.serialization.load`), and `posix`/`nt` process spawning (`system`, `popen`, `spawn*`, `exec*`).
  - Restricted `sanitize_sidecar_filename` to ASCII `[A-Za-z0-9_.]` without leading/trailing `.` or `..`.
  - Added regression assertions in `denied_escape_aliases_and_dir_sidecar_resolution_hold`.

### WR-329-02: Preserved directory paths in `load_sidecar` & fixed stale sidecar refresh in `vox py bind`
- **Files:** `crates/vox-compiler/src/python_escape_support.rs`, `crates/vox-cli-research/src/py/env.rs`, `crates/vox-compiler/tests/py_escape_hatch_test.rs`
- **Change:**
  - Fixed `resolve_start_dir` (`Some(p) if p.is_dir() => p.to_path_buf()` before `is_file()` / `parent()`) so passing a directory path checks `.vox/python/bindings/` inside that directory instead of skipping to its parent.
  - Added `load_curated_sidecar` and switched `bind_module` to it so running `vox py bind <module>` refreshes an existing stale local sidecar instead of failing on stale lock verification.
  - Added regression test `py::env::tests::bind_module_refreshes_existing_stale_local_sidecar`.

## Verification
- `timeout 900s cargo test -p vox-compiler --test py_escape_hatch_test -- --nocapture`: `8 passed; 0 failed` (`0.21s`).
- `timeout 900s cargo test -p vox-cli-research --lib -- py::env::tests`: `8 passed; 0 failed; 660 filtered out` (`0.13s`).
- `python3 -m py_compile crates/vox-python-bridge/worker/vox_bridge/__main__.py crates/vox-python-bridge/worker/vox_bridge/plot_ops.py`: exit `0`.
