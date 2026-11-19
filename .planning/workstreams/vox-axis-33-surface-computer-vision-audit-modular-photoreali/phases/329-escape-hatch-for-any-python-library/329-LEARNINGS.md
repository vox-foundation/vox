---
phase: 329
phase_name: "escape-hatch-for-any-python-library"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts: []
---

# Phase 329 Learnings: escape-hatch-for-any-python-library

## Decisions

### Added ImportPathKind::PyModule { alias, module } in crates/vox-ast/src/d...
Added `ImportPathKind::PyModule { alias, module }` in `crates/vox-ast/src/decl/types.rs` and contextual `try_parse_py_module_import` in `crates/vox-compiler/src/parser/descent/decl/head_import.rs` without modifying `LEXER_KEYWORDS`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 329-01-SUMMARY.md

---

### Added formatter support (py {alias} from "{module}") in crates/vox-compi...
Added formatter support (`py {alias} from "{module}"`) in `crates/vox-compiler/src/fmt/printer.rs` and lowered to `HirPyImport` / `HirModule::py_imports` in `crates/vox-compiler/src/hir/{nodes/decl.rs, lower/mod.rs}`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 329-01-SUMMARY.md

---

### Created crates/vox-compiler/src/python_escape_support.rs (exported via c...
Created `crates/vox-compiler/src/python_escape_support.rs` (exported via `crates/vox-compiler/src/lib.rs`) implementing `is_denied_python_target`, `sanitize_sidecar_filename`, `parse_vox_type_str`, `parse_sidecar_text`, `load_sidecar` (`uv.lock` BLAKE3 verification via `vox_crypto::secure_hash`), `format_escape_telemetry_event` (`module_hash`, `symbol_hash`, `tier`), and `json_to_vox_value`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 329-01-SUMMARY.md

---

### Created curated sidecars under contracts/python/sidecars/ for scipy.sign...
Created curated sidecars under `contracts/python/sidecars/` for `scipy.signal`, `scipy.stats`, `sklearn.cluster`, `PIL.Image`, and `pandas`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 329-01-SUMMARY.md

---

### Wired crates/vox-compiler/src/typeck/ (diagnostics.rs, registration.rs, ...
Wired `crates/vox-compiler/src/typeck/` (`diagnostics.rs`, `registration.rs`, `checker/expr_field.rs`, `checker/expr_ops.rs`, `checker/expr.rs`, `checker/mod.rs`) to enforce `Denied` module rejection, lock-matched sidecar resolution, `PyObject` opacity (disallowing field access, indexing, unary/binary ops, and boundary crossing on `pub fn`/`@activity`/`workflow`), `.decode(...)`, `uses python_unverified`, and warning diagnostic `codes::PYTHON_UNVERIFIED_CALL = "vox/python/unverified-call"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 329-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

