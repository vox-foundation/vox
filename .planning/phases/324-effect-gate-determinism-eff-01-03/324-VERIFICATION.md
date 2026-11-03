---
phase: 324-effect-gate-determinism-eff-01-03
verified: 2026-11-03T10:26:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: yes
previous_status: gaps_found
previous_score: 4/5
gaps_closed:
  - "vox run --caps without py refuses a Python-capability call at runtime with a caps error."
gaps_remaining: []
regressions: []
covered_files:
  - ".planning/phases/324-effect-gate-determinism-eff-01-03/324-01-PLAN.md"
  - ".planning/phases/324-effect-gate-determinism-eff-01-03/324-01-SUMMARY.md"
  - ".planning/phases/324-effect-gate-determinism-eff-01-03/324-02-PLAN.md"
  - ".planning/phases/324-effect-gate-determinism-eff-01-03/324-02-SUMMARY.md"
  - "crates/vox-ast/src/decl/effect.rs"
  - "crates/vox-cli/src/commands/run.rs"
  - "crates/vox-cli/tests/run_interp_limits.rs"
  - "crates/vox-compiler/src/eval/builtins.rs"
  - "crates/vox-compiler/src/eval/caps.rs"
  - "crates/vox-compiler/src/eval/expr.rs"
  - "crates/vox-compiler/src/eval/mod.rs"
  - "crates/vox-compiler/src/hir/lower/decl.rs"
  - "crates/vox-compiler/src/hir/lower/mod.rs"
  - "crates/vox-compiler/src/hir/nodes/decl.rs"
  - "crates/vox-compiler/src/hir/nodes/effect.rs"
  - "crates/vox-compiler/src/parser/descent/decl/head.rs"
  - "crates/vox-compiler/src/parser/descent/decl/head_fn.rs"
  - "crates/vox-compiler/src/required_capabilities.rs"
  - "crates/vox-compiler/src/typeck/determinism_lint.rs"
  - "crates/vox-compiler/src/typeck/effect_check.rs"
  - "crates/vox-compiler/src/typeck/mod.rs"
  - "crates/vox-compiler/tests/determinism_lint.rs"
  - "crates/vox-compiler/tests/python_effects_test.rs"
covered_digest: "v3:sha256:0ee8049986bad4fe77c3d30981f1ef5c63b138b1a8342bfab8f202baef946725"
decision_coverage:
  honored: 5
  total: 5
  not_honored: []
---

# Phase 324: Effect Gate & Determinism Verification Report

**Phase Goal:** The effect and capability gate that every later Python call depends on exists and cannot be silently bypassed.
**Verified:** 2026-11-03T10:26:00Z
**Status:** passed
**Re-verification:** Yes — after gap closure plan 324-02

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `@uses(nonexistent)` and `uses nonexistent` both produce a compile error; `@uses(mcp(tool))` appears in HIR as `Mcp(tool)`. | ✓ VERIFIED | Unchanged since the prior pass (`head.rs`, `head_fn.rs` have no commits after `2026-11-03T09:32:09Z`). Re-ran `python_effects_test`: `decorator_uses_unknown_effect_is_parse_error`, `clause_uses_unknown_effect_is_parse_error`, and `decorator_uses_mcp_tool_lowers_to_hir_mcp` passed. |
| 2 | `@uses(python_unverified)` survives into HIR and `HirCapability`; a materializer-shaped call without the effect fails typecheck. | ✓ VERIFIED | `effect_check.rs` still maps `HirEffectKind::Python` / `PythonUnverified`, and `check_effect_compliance` is still a typeck pass (`typeck/mod.rs`). Same test run: `decorator_uses_python_unverified_lowers_to_capability`, `caller_without_python_effect_is_violation`, `caller_declaring_python_effect_is_clean` passed. |
| 3 | `CapabilitySet::parse("py:allow")` allows namespace `py` and denies `py.unverified`; `py:unverified` allows both; `restrictive()` allows neither. | ✓ VERIFIED | `caps.rs` unchanged since the prior pass. `GATED` still lists `py` and `py.unverified`. Parse arms and `developer_default()` are the same grammar 324-01 shipped. Runtime tests now consume that grammar (`py:allow` runs the body; `py:allow` still denies `python_unverified`). |
| 4 | `vox run --caps` without `py` refuses a Python-capability call at runtime with a caps error. | ✓ VERIFIED | `deny_declared_python` (`expr.rs`) returns `CapabilityDenied` when `python_fns` lacks `allows_namespace("py")`, and when `python_unverified_fns` lacks `allows_namespace("py.unverified")`. It runs before `apply_closure` and before the `Interpreter::call` body loop. `cargo test -p vox-cli --test run_interp_limits python_capability_call_without_py_exits_77 -- --exact` passed: exit 77, stderr `vox: capability denied: py.secret`, stdout does not contain `LEAK`. `print` is `println!`, so that stdout check is the body. |
| 5 | A workflow calling a Python-capability fn is rejected by capability; the same call in an activity compiles. | ✓ VERIFIED | `determinism_lint.rs` and `crates/vox-compiler/tests/determinism_lint.rs` have no commits after the prior pass. `check_workflow_determinism` still flags `HirCapability::Python` / `PythonUnverified` and is still registered from `typeck/mod.rs`. The four Python workflow tests are still present and not ignored. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Re-verification

| Field | Value |
| --- | --- |
| previous_status | gaps_found |
| previous_score | 4/5 |
| gaps_closed | `vox run --caps` without `py` refuses a Python-capability call at runtime with a caps error. |
| gaps_remaining | none |
| regressions | none on the previously passing truths |

The prior miss was the absence of a production `allows_namespace("py")` caller. `deny_declared_python` is that caller. Membership comes from `HirCapability` (and endpoint `HirEffectKind`) recorded at registration, not from a source scan at the call.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | ----------- | ------ | ------- |
| `crates/vox-ast/src/decl/effect.rs` | `Python` and `PythonUnverified` | ✓ VERIFIED | Unchanged. Prior pass confirmed both variants. |
| `crates/vox-compiler/src/parser/descent/decl/head.rs` | Shared `parse_effect_item` errors on unknown names | ✓ VERIFIED | Unchanged. Unknown-effect tests passed again. |
| `crates/vox-compiler/src/parser/descent/decl/head_fn.rs` | `@uses` uses the shared parser | ✓ VERIFIED | Unchanged. Decorator unknown-effect test passed again. |
| `crates/vox-compiler/src/hir/nodes/effect.rs` | `HirEffectKind` labels | ✓ VERIFIED | Unchanged from the prior pass. |
| `crates/vox-compiler/src/hir/nodes/decl.rs` | `HirCapability::as_str` | ✓ VERIFIED | Unchanged from the prior pass. |
| `crates/vox-compiler/src/hir/lower/decl.rs` | Lowering for fn, activity, workflow | ✓ VERIFIED | Unchanged. Activity/workflow lowering test passed again. |
| `crates/vox-compiler/src/typeck/effect_check.rs` | Effect compliance includes the new capabilities | ✓ VERIFIED | Unchanged. Caller-without-`python` test passed again. |
| `crates/vox-compiler/src/typeck/determinism_lint.rs` | Capability-aware workflow lint | ✓ VERIFIED | Unchanged. Lint still names `lint.workflow.non_deterministic` for `Python` / `PythonUnverified`. |
| `crates/vox-compiler/src/eval/caps.rs` | `py:allow` / `py:unverified` grammar | ✓ VERIFIED | Unchanged. Runtime tests grant through `CapabilitySet::parse`. |
| `crates/vox-compiler/src/eval/expr.rs` | Denial before the body | ✓ VERIFIED | `verify.artifacts` passed (`allows_namespace("py")` present). `deny_declared_python` is called from `HirExpr::Call` (before the activity early-return and before `apply_closure`), the pipe arm, the import-alias arm, and the start of `apply_closure`. |
| `crates/vox-compiler/src/eval/mod.rs` | `python_fns` / `python_unverified_fns` | ✓ VERIFIED | `verify.artifacts` passed. Sets start empty in `Interpreter::new`. `record_python_capabilities` runs for module functions, tests, MCP tools/resources/examples, and imported `pub fn`s. Endpoint fns record `HirEffectKind::Python` / `PythonUnverified`. `Interpreter::call` denies before the body loop. |
| `crates/vox-compiler/tests/python_effects_test.rs` | Interpreter deny/allow cases | ✓ VERIFIED | Exports the six `python_cap_runtime_*` tests. 17 passed, 0 ignored. |
| `crates/vox-cli/tests/run_interp_limits.rs` | `vox run --caps` exits 77 | ✓ VERIFIED | `python_capability_call_without_py_exits_77` passed. |

`gsd-tools query verify.artifacts` on `324-02-PLAN.md`: 4/4 passed, exit 0. `324-01-PLAN.md` still declares no `must_haves.artifacts`.

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `parse_uses_clause` / `Token::AtUses` | `parse_effect_item` | direct call | WIRED | Unchanged. Unknown-effect tests passed. |
| AST effects | `HirCapability` | `lower_declared_capabilities` | WIRED | Unchanged. Lowering tests passed. |
| `HirCapability` | effect kind and workflow lint | `effect_check.rs`, `determinism_lint.rs`, `typeck/mod.rs` | WIRED | Both lints still registered. |
| `HirFn` registration | `python_fns` / `python_unverified_fns` | `record_python_capabilities` and the endpoint effect loop | WIRED | Functions, tests, MCP, examples, imports, endpoints. Nested `secret` is denied with method `secret`, so registration is what the call sees. |
| `HirExpr::Call` | `deny_declared_python` | before `apply_closure` | WIRED | `expr.rs` Call arm. `python_cap_runtime_without_py_is_denied` returns `CapabilityDenied { ns: "py", method: "secret" }` under `env:ro`. |
| `Interpreter::call` | `deny_declared_python` | before the body loop | WIRED | `mod.rs` `VoxValue::Fn` branch. Entry `main` under `env:ro` is `CapabilityDenied { ns: "py", method: "main" }`. |
| `EvalError::CapabilityDenied` | process exit 77 | `commands/run.rs` | WIRED | `eprintln!("vox: capability denied: {ns}.{method} — grant with --caps ...")` then `exit(77)`. CLI test asserts that stderr and status `Some(77)`. |

`gsd-tools query verify.key-links` on `324-02-PLAN.md` returned `total: 0`, `all_verified: false`, exit 1. The plan's `key_links` are prose strings, not `{from, to, via}` objects, so the tool checked nothing. The rows above are from the source and the passing tests, not from that empty result.

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| Unknown-effect diagnostic | `ParseError` | `parse_effect_item` | Yes — same tests as the prior pass, re-run | ✓ FLOWING |
| Python capability | `HirFn.capabilities` | `uses python` / `@uses(python_unverified)` | Yes — lowering tests passed again | ✓ FLOWING |
| Runtime denial | `EvalError::CapabilityDenied` | `python_fns` membership and `caps.allows_namespace` | Yes — `env:ro` yields `ns: "py"`; `py:allow` on an unverified fn yields `ns: "py.unverified"`; `py:allow` and `developer_default()` return `Int(7)` | ✓ FLOWING |
| CLI caps error | stderr + exit code | `CapabilityDenied` in `run.rs` | Yes — exit 77 and `vox: capability denied: py.secret`; `LEAK` is not on stdout | ✓ FLOWING |
| Workflow diagnostic | `lint.workflow.non_deterministic` | declared `HirCapability::Python*` | Yes — lint source unchanged | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Compile-time effect gate plus runtime deny/allow | `cargo test -p vox-compiler --test python_effects_test -- --test-threads=8` | 17 passed, 0 failed, 0 ignored (includes the six `python_cap_runtime_*` cases and the original 11) | ✓ PASS |
| `vox run --mode interp --caps env:ro` refuses `uses python` | `cargo test -p vox-cli --test run_interp_limits python_capability_call_without_py_exits_77 -- --exact --test-threads=8` | 1 passed, 7 filtered | ✓ PASS |
| Workflow lint (truth 5) | not re-run | `determinism_lint.rs` and its test file have no commits since the prior pass | ✓ PASS (regression only) |

### Probe Execution

No phase plan or summary declares a `probe-*.sh`. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| EFF-01 | `324-01-PLAN.md` | Unknown `@uses` / `uses` name is a compile error; `@uses(mcp(tool))` reaches HIR as `Mcp(tool)`; one shared helper | ✓ SATISFIED | Truth 1. |
| EFF-02 | `324-01-PLAN.md`, `324-02-PLAN.md` | A materializer-shaped call without `uses python` fails typecheck; `vox run` without `--caps py` / `py:unverified` refuses at runtime | ✓ SATISFIED | Truth 2 and truth 4. `python_cap_runtime_unverified_without_grant_is_denied` refuses under both `env:ro` and `py:allow`. Tier 1 module names stay in phase 326 per D-03; that was not a 324 success criterion on the prior pass and the effect-check file is unchanged. |
| EFF-03 | `324-01-PLAN.md` | A workflow whose capabilities include `python` / `python_unverified` is rejected; the same call inside an activity compiles | ✓ SATISFIED | Truth 5. |

No requirement mapped to phase 324 in `REQUIREMENTS.md` is absent from the plans. The EFF-02 checkbox in `REQUIREMENTS.md` is still open. The implementation meets the requirement; the checkbox was not updated by this verification.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `crates/vox-compiler/tests/python_effects_test.rs` | EFF-01, EFF-02 | 17 | 0 | no | Behavioral (parse error, capability equality, `CapabilityDenied` ns/method, `Int(7)` on the allow paths) | PASS |
| `crates/vox-compiler/tests/determinism_lint.rs` | EFF-03 | 12 (4 Python-specific), unchanged | 0 | no | Behavioral (`lint.workflow.non_deterministic`; activity path empty) — not re-executed this pass | PASS |
| `crates/vox-cli/tests/run_interp_limits.rs` | EFF-02 runtime | 1 new case executed | 0 | no | Behavioral (exit 77, stderr marker, stdout has no `LEAK`) | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0

`python_cap_runtime_py_allow_runs_body` and `python_cap_runtime_developer_default_runs_body` can pass without the deny helper. The deny cases cannot: they `expect_err("expected capability denied")` and match `ns` and `method`. Removing `allows_namespace("py")` makes the `env:ro` nested call return `Ok(Int(7))` and fail those tests. The CLI test fails if the body prints, because `print` uses `println!`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| — | — | No `TBD`, `FIXME`, or `XXX` in `expr.rs` or `eval/mod.rs` | — | Debt-marker gate clear. |

`expr.rs` comments that say "placeholder" describe pre-existing builtin function values, not the Python gate.

### Advisory (New Scope, Unevidenced)

None. The prior gap is closed in the files 324-02 edited, and the re-run tests pass. No new-scope blocker was raised.

### Decision Coverage

`workflow.context_coverage_gate` is unset (query: key not found), so the gate ran. `check.decision-coverage-verify`: honored 5 / total 5, `not_honored: []`, non-blocking. Message: all trackable CONTEXT.md decisions are honored by shipped artifacts.

D-04's bridge worker remains phase 325. This phase gates the declared effect. `developer_default()` still allows every gated namespace; `python_cap_runtime_developer_default_runs_body` passed.

### Human Verification Required

N/A — infrastructure/foundation phase. The runtime refusal is exercised by the compiler interpreter tests and by one `vox run --caps` test. No user-facing behavior is left unchecked.

### Gaps Summary

No gaps. The prior failure was that `CapabilitySet` understood `py` and nothing in the interpreter read the grant. A user function that declares `python` or `python_unverified` is now refused with `CapabilityDenied` before its body runs unless the granted set allows `py` or `py.unverified`. `py:allow` does not satisfy `python_unverified`. `vox run --mode interp --caps env:ro` on a program whose `main` calls that function exits 77 and does not print `LEAK`.

---

_Verified: 2026-11-03T10:26:00Z_
_Verifier: Claude (gsd-verifier)_
