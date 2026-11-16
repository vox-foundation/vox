---
phase: 324-effect-gate-determinism-eff-01-03
plan: 02
subsystem: compiler
tags: [effects, python, caps, interpreter, eff-02]

requires:
  - 324-01
provides:
  - Runtime denial of a declared Python user function before its body runs
  - CapabilityDenied for python_unverified unless the grant includes py.unverified
affects: [325]

actuals:
  tokens: 3409
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "HirFn registration records Python and PythonUnverified; deny_declared_python consults those sets"
    - "py:allow satisfies namespace py only; py.unverified is a separate allows_namespace check"

key-files:
  created: []
  modified:
    - crates/vox-compiler/tests/python_effects_test.rs
    - crates/vox-cli/tests/run_interp_limits.rs
    - crates/vox-compiler/src/eval/expr.rs
    - crates/vox-compiler/src/eval/mod.rs

key-decisions:
  - "Denial uses the existing EvalError::CapabilityDenied and the existing py / py.unverified grants from 324-01"
  - "Endpoint functions record Python from HirEffectKind because HirEndpointFn has effects, not HirFn.capabilities"
  - "Pipe and apply_closure also call deny_declared_python so a declared Python function cannot run through those paths"

requirements-completed: [EFF-02]

coverage:
  - id: D4
    description: A nested or entry Python call under env:ro is CapabilityDenied and vox run --caps env:ro exits 77 without running the body
    requirement: EFF-02
    verification:
      - kind: integration
        ref: crates/vox-compiler/tests/python_effects_test.rs#python_cap_runtime_without_py_is_denied
        status: pass
      - kind: integration
        ref: crates/vox-cli/tests/run_interp_limits.rs#python_capability_call_without_py_exits_77
        status: pass
    human_judgment: false

duration: 24m
completed: 2026-11-03
status: complete
plan_head_before: f7c6bc99157852004729213ac88260914c3348ce
plan_head_after: 620daa9d0e8a37376a5c14941390fb8aeac9bf6e
---

# Phase 324 Plan 02: Runtime Python capability refusal Summary

**A user function that declares `python` or `python_unverified` is refused with `CapabilityDenied` before its body runs unless the granted caps allow `py` or `py.unverified`.**

## Performance

- **Duration:** 24m
- **Started:** 2026-11-03T09:54:27Z
- **Completed:** 2026-11-03T10:18:52Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- Nested `secret()` with `uses python` under `env:ro` returns `CapabilityDenied { ns: "py", method: "secret" }`. The same program under `py:allow` and under `developer_default()` returns `Int(7)`.
- An entry `main` that itself declares `uses python` is denied on `Interpreter::call` under `env:ro`.
- `uses python_unverified` is denied for `py.unverified` under both `env:ro` and `py:allow`, and returns `Int(7)` under `py:unverified`.
- `vox run --mode interp --caps env:ro` on a program whose `main` calls that function exits 77. Stderr contains `vox: capability denied: py.secret` and `--caps`. Stdout does not contain `LEAK`.

## Task Commits

1. **Task 1: Failing runtime refusal tests** - `ea13019f9` (test)
2. **Task 2: Deny declared Python capabilities before the call** - `620daa9d0` (feat)

## Files Created/Modified

- `crates/vox-compiler/tests/python_effects_test.rs` - interpreter cases for deny, allow, unverified, and developer_default
- `crates/vox-cli/tests/run_interp_limits.rs` - one `vox run --caps env:ro` invocation that expects exit 77
- `crates/vox-compiler/src/eval/mod.rs` - `python_fns` / `python_unverified_fns` filled when a HirFn is registered; `Interpreter::call` denies before the body
- `crates/vox-compiler/src/eval/expr.rs` - `deny_declared_python` on `HirExpr::Call`, import-alias calls, pipe, and `apply_closure`

## Decisions Made

- The check reads membership recorded from `HirCapability` at registration. It does not scan source at the call.
- `allows_namespace("py")` and `allows_namespace("py.unverified")` stay as two string-literal calls inside `deny_declared_python`.
- `developer_default()`, the caps grammar, and `GATED` were left as 324-01 shipped them.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Type] Endpoint registration is `HirEndpointFn`, not `HirFn`**
- **Found during:** Task 2
- **Issue:** `module.endpoint_fns` items have `effects: HirEffectSet`, so `record_python_capabilities(&HirFn)` did not compile at that loop.
- **Fix:** Record `HirEffectKind::Python` and `PythonUnverified` from `f.effects` into the same sets. The HirFn helper still covers functions, tests, MCP tools, resources, examples, and imported pub fns.
- **Files modified:** `crates/vox-compiler/src/eval/mod.rs`
- **Commit:** `620daa9d0`

**2. [Rule 2 - Missing gate] Pipe and `apply_closure` skipped the Call-arm check**
- **Found during:** Task 2
- **Issue:** `lhs |> f` and collection methods apply a `VoxValue::Fn` without entering the `HirExpr::Call` arm, so a declared Python function could run under `env:ro`.
- **Fix:** `deny_declared_python` also runs on the pipe arm and at the start of `apply_closure`, in addition to the Call arm, the alias method path, and `Interpreter::call`.
- **Files modified:** `crates/vox-compiler/src/eval/expr.rs`
- **Commit:** `620daa9d0`

## Verification

- `cargo test -p vox-compiler --test python_effects_test -- --test-threads=8` — 17 passed.
- `cargo test -p vox-cli --test run_interp_limits python_capability_call_without_py_exits_77 -- --exact --test-threads=8` — 1 passed.
- Task 1 deny tests failed first with `expected capability denied: Int(7)` before the check existed. The three allow cases passed in that same run.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: crates/vox-compiler/tests/python_effects_test.rs
- FOUND: crates/vox-cli/tests/run_interp_limits.rs
- FOUND: crates/vox-compiler/src/eval/expr.rs
- FOUND: crates/vox-compiler/src/eval/mod.rs
- FOUND: ea13019f9
- FOUND: 620daa9d0
