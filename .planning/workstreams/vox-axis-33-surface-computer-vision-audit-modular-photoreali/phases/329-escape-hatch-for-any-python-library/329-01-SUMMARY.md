---
phase: 329-escape-hatch-for-any-python-library
plan: 01
subsystem: python-escape-hatch
tags: [python, escape-hatch, sidecar, vox-crypto, typeck, telemetry, syntax-k]
requires: [328-01]
provides:
  - "Contextual `import py <alias> from \"<module>\"` AST/Parser/Fmt/HIR support with 0 new lexer keywords"
  - "`python_escape_support.rs`: `.vox.pyi` sidecar parser, `uv.lock` BLAKE3 check via `vox-crypto`, `Denied` policy guard, hashed `python.escape_call` telemetry"
  - "Typeck `PyModule`/`PyObject` opacity, `.decode(...)`, `uses python_unverified`, `vox/python/unverified-call` diagnostic"
  - "Worker `family == \"escape_call\"` + `_is_denied_escape_target`, `vox py bind <module>`, `count_python_tier_calls`, and curated sidecars"
affects:
  - crates/vox-ast/src/decl/types.rs
  - crates/vox-compiler/src/
  - crates/vox-python-bridge/worker/vox_bridge/__main__.py
  - crates/vox-cli-research/src/
  - crates/vox-codegen/src/syntax_k.rs
  - contracts/
  - examples/golden/py_escape_psd.vox
requirements-completed: [PYX-01, PYX-02, PYX-03, PYX-04, K-01]
completed: 2026-11-13
---

# Phase 329 Plan 01 Summary: Tier 2 Escape Hatch (`PYX-01..04`, `K-01`)

## What Was Built

- **Contextual Syntax & HIR (`PYX-01`, `K-01`):**
  - Added `ImportPathKind::PyModule { alias, module }` in `crates/vox-ast/src/decl/types.rs` and contextual `try_parse_py_module_import` in `crates/vox-compiler/src/parser/descent/decl/head_import.rs` without modifying `LEXER_KEYWORDS`.
  - Added formatter support (`py {alias} from "{module}"`) in `crates/vox-compiler/src/fmt/printer.rs` and lowered to `HirPyImport` / `HirModule::py_imports` in `crates/vox-compiler/src/hir/{nodes/decl.rs, lower/mod.rs}`.
- **Sidecar Contracts, Opacity & Security (`PYX-01`, `PYX-03`, `PYX-04`):**
  - Created `crates/vox-compiler/src/python_escape_support.rs` (exported via `crates/vox-compiler/src/lib.rs`) implementing `is_denied_python_target`, `sanitize_sidecar_filename`, `parse_vox_type_str`, `parse_sidecar_text`, `load_sidecar` (`uv.lock` BLAKE3 verification via `vox_crypto::secure_hash`), `format_escape_telemetry_event` (`module_hash`, `symbol_hash`, `tier`), and `json_to_vox_value`.
  - Created curated sidecars under `contracts/python/sidecars/` for `scipy.signal`, `scipy.stats`, `sklearn.cluster`, `PIL.Image`, and `pandas`.
  - Wired `crates/vox-compiler/src/typeck/` (`diagnostics.rs`, `registration.rs`, `checker/expr_field.rs`, `checker/expr_ops.rs`, `checker/expr.rs`, `checker/mod.rs`) to enforce `Denied` module rejection, lock-matched sidecar resolution, `PyObject` opacity (disallowing field access, indexing, unary/binary ops, and boundary crossing on `pub fn`/`@activity`/`workflow`), `.decode(...)`, `uses python_unverified`, and warning diagnostic `codes::PYTHON_UNVERIFIED_CALL = "vox/python/unverified-call"`.
- **Runtime Bridge, CLI Binding, Telemetry & Syntax-K (`PYX-02`, `PYX-03`, `PYX-04`, `K-01`):**
  - Added `VoxValue::PyModule`, `VoxValue::PyObject`, `python_escape_telemetry`, `materialize_escape_call`, and `.decode(...)` evaluation in `crates/vox-compiler/src/eval/` alongside worker `family == "escape_call"` and `_is_denied_escape_target` in `crates/vox-python-bridge/worker/vox_bridge/__main__.py`.
  - Added `PyCmd::Bind { module }` (`vox py bind`) and `bind_module` in `crates/vox-cli-research/src/py/{mod.rs, env.rs}`.
  - Registered `contracts/telemetry/python-escape-call.v1.schema.json` and `python.escape_call` in `contracts/telemetry/events.v1.yaml`.
  - Added `count_python_tier_calls` in `crates/vox-codegen/src/syntax_k.rs` and wired `py_tier1_calls` / `py_tier2_calls` into `support_metrics.llm_surface` in `crates/vox-cli-research/src/ci_helpers/syntax_k.rs`.
  - Created `examples/golden/py_escape_psd.vox` and updated `contracts/pipeline/canonical-ladder.v1.yaml`, `contracts/eval/complexity-budget.v1.json`, and `contracts/eval/source-token-budget.v1.json` (`py_frame_report`, `py_tensor_infer`, `py_escape_psd`).

## Verification Highlights

- `timeout 900s cargo test -p vox-compiler --test py_escape_hatch_test -- --nocapture`: `7 passed; 0 failed` (including live `python3` worker execution in `runtime_escape_call_executes_unbound_stdlib_math_when_python_available`).
- `timeout 900s cargo run -p vox-cli -- ci source-token-budget --update`: exit `0` (`Source-token budget OK (16 ladder fixtures validated)`).
- `timeout 900s cargo test -p vox-compiler --lib -- --test-threads=1`: `642 passed; 0 failed; 1 ignored`.
- `timeout 900s cargo test -p vox-codegen --test python_bridge_parity_test`: `4 passed; 0 failed`.
- `timeout 900s cargo test -p vox-codegen --lib -- count_python_tier_calls`: `1 passed; 0 failed` (`461 filtered out`).
- `timeout 900s cargo test -p vox-cli-research --lib -- k_complexity_budget_tests`: `5 passed; 0 failed` (`662 filtered out`).
- `timeout 900s cargo test -p vox-cli-research --lib -- py::env::tests`: `7 passed; 0 failed` (`660 filtered out`).
