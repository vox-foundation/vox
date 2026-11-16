---
phase: 329
status: fixed
reviewer: antigravity
reviewed_at: 2026-11-13T08:12:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - crates/vox-compiler/src/python_escape_support.rs
  - crates/vox-python-bridge/worker/vox_bridge/__main__.py
  - crates/vox-cli-research/src/py/env.rs
  - crates/vox-compiler/tests/py_escape_hatch_test.rs
findings:
  critical: 0
  warning: 2
  info: 0
  total: 2
---

# Phase 329 Code Review (Tier 2 Python Escape Hatch)

### WR-329-01: Denied-tier submodule/alias gaps & permissive sidecar filename sanitization
- **Severity:** Warning
- **Location:** `crates/vox-compiler/src/python_escape_support.rs:35-113`, `crates/vox-python-bridge/worker/vox_bridge/__main__.py:114-132`
- **Issue:**
  1. `is_denied_python_target` and `_is_denied_escape_target` blocked `torch.load` and `os.{system,popen,spawn*,exec*}`, but missed `torch.serialization.load` (`torch.*.load`), `posix`/`nt` OS module aliases, and CPython low-level bypass modules `_posixsubprocess`, `_imp`, and `imp`.
  2. `sanitize_sidecar_filename` rejected `/` / `\` / `..`, but allowed whitespace or leading/trailing dots (`.hidden`).
- **Fix Applied (see `329-REVIEW-FIX.md`):** Extended host + worker deny checks across `torch.*.load`, `posix`/`nt`, `_posixsubprocess`, `_imp`, and `imp`, and tightened `sanitize_sidecar_filename` to valid ASCII identifier/dot segments (`[A-Za-z0-9_.]`, no leading/trailing `.`).

### WR-329-02: Directory-path truncation in `load_sidecar` & stale local sidecar blocking `vox py bind` refresh
- **Severity:** Warning
- **Location:** `crates/vox-compiler/src/python_escape_support.rs:304-348`, `crates/vox-cli-research/src/py/env.rs:286-302`
- **Issue:**
  1. Passing a directory path (`Some(&root)`) into `load_sidecar` fell through `p.is_file()` into `p.parent()`, stripping the directory itself before searching `.vox/python/bindings/` and `contracts/python/sidecars/`.
  2. `bind_module` (`vox py bind <module>`) called `load_sidecar` to inspect curated signatures, which failed on `verify_sidecar_freshness` whenever an outdated local `.vox/python/bindings/<module>.vox.pyi` already existed — preventing `vox py bind` from refreshing a stale sidecar with the curated signatures.
- **Fix Applied (see `329-REVIEW-FIX.md`):** Fixed `resolve_start_dir` to preserve directory paths (`Some(p) if p.is_dir()`), added `load_curated_sidecar`, and switched `bind_module` to `load_curated_sidecar`.
