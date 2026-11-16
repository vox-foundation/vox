---
phase: 329-escape-hatch-for-any-python-library
verified: 2026-11-13
status: passed
score: 5/5 must-haves verified
requirements: [PYX-01, PYX-02, PYX-03, PYX-04, K-01]
---

# Phase 329 Verification Report

## Scope & Verdict

**Status:** `passed` (`5/5` requirements `PYX-01..04`, `K-01` verified).

## Checks Executed

1. **Tier 2 Escape Hatch Integration Suite (`PYX-01..04`):**
   - Command: `timeout 900s cargo test -p vox-compiler --test py_escape_hatch_test -- --nocapture`
   - Outcome: `7 passed; 0 failed` (`0.16s`), including live `python3` worker execution (`runtime_escape_call_executes_unbound_stdlib_math_when_python_available`), stale `uv.lock` sidecar rejection, `PyObject` field/indexing/boundary opacity errors, `Denied` policy blocking, and `vox/python/unverified-call` diagnostic + `python.escape_call` hashed telemetry emission.
2. **Compiler Core Library Suite (`K-01`, `PYX-01`, `PYX-03`):**
   - Command: `timeout 900s cargo test -p vox-compiler --lib -- --test-threads=1`
   - Outcome: `642 passed; 0 failed; 1 ignored`.
3. **Python Bridge Parity Contract (`PYX-02`):**
   - Command: `timeout 900s cargo test -p vox-codegen --test python_bridge_parity_test`
   - Outcome: `4 passed; 0 failed` (12 Tier 1 ops parity preserved).
4. **Syntax-K Tier 1 / Tier 2 Counter (`PYX-03`, `K-01`):**
   - Command: `timeout 900s cargo test -p vox-codegen --lib -- count_python_tier_calls`
   - Outcome: `1 passed; 0 failed; 0 ignored` (`count_python_tier_calls_tracks_tier1_and_tier2`, `461 filtered out`).
5. **Ladder & Complexity / Source-Token Budgets (`K-01`):**
   - Command: `timeout 900s cargo run -p vox-cli -- ci source-token-budget --update`
   - Outcome: Exit `0` (`Source-token budget OK (16 ladder fixtures validated)`).
   - Command: `timeout 900s cargo test -p vox-cli-research --lib -- k_complexity_budget_tests`
   - Outcome: `5 passed; 0 failed; 0 ignored` (`all_ladder_fixtures_have_budget_entries_in_repo` passing with `py_frame_report`, `py_tensor_infer`, and `py_escape_psd`, `662 filtered out`).
6. **`vox py bind` & `Denied` Policy Unit Tests (`PYX-02`, `PYX-04`):**
   - Command: `timeout 900s cargo test -p vox-cli-research --lib -- py::env::tests`
   - Outcome: `7 passed; 0 failed; 0 ignored` (`bind_module_rejects_denied_and_writes_lock_stamped_sidecar` included, `660 filtered out`).
