---
phase: 324-effect-gate-determinism-eff-01-03
verified: 2026-11-04T01:47:32Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
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
covered_digest: "v3:sha256:0c4874e2609973936a4d82146dc127d8e669e984b465b04e9a0931da71631eaa"
decision_coverage:
  honored: 5
  total: 5
  not_honored: []
---

# Phase 324: Effect Gate & Determinism Verification Report

**Phase Goal:** The effect and capability gate that every later Python call depends on exists and cannot be silently bypassed.
**Verified:** 2026-11-04T01:47:32Z
**Status:** passed
**Re-verification:** Yes — stale fingerprint after later commits. No 324 plan was re-run. Production code was not modified.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `@uses(nonexistent)` and `uses nonexistent` both produce a compile error; `@uses(mcp(tool))` appears in HIR as `Mcp(tool)`. | ✓ VERIFIED | `head.rs` and `head_fn.rs` have no commits after `2026-11-03T10:26:00Z`. `parse_effect_item` is still the shared helper. This pass: `decorator_uses_unknown_effect_is_parse_error`, `clause_uses_unknown_effect_is_parse_error`, and `decorator_uses_mcp_tool_lowers_to_hir_mcp` passed. |
| 2 | `@uses(python_unverified)` survives into HIR and `HirCapability`; a materializer-shaped call without the effect fails typecheck. | ✓ VERIFIED | `effect_kind_to_cap` still maps `HirEffectKind::Python` and `PythonUnverified`. `check_effect_compliance` is still a typeck pass (`typeck/mod.rs`). `effect_check.rs` changed in `b8cf7109c` (phase 325) by adding the `python` stdlib-module capability; it did not remove the caller check. This pass: `decorator_uses_python_unverified_lowers_to_capability`, `caller_without_python_effect_is_violation`, and `caller_declaring_python_effect_is_clean` passed. |
| 3 | `CapabilitySet::parse("py:allow")` allows namespace `py` and denies `py.unverified`; `py:unverified` allows both; `restrictive()` allows neither. | ✓ VERIFIED | `caps.rs` has no commits after the prior pass. `GATED` still lists `py` and `py.unverified`. Parse arms are `("py","allow")`, `("py","unverified")`, and `("py", _) => "py takes allow\|unverified"`. `restrictive_denies_py` is still present and not ignored. Runtime tests this pass consume that grammar: `py:allow` returns `Int(7)`; `py:allow` still denies `python_unverified`. |
| 4 | `vox run --caps` without `py` refuses a Python-capability call at runtime with a caps error. | ✓ VERIFIED | `deny_declared_python` (`expr.rs`) still returns `CapabilityDenied` when `python_fns` lacks `allows_namespace("py")`, and when `python_unverified_fns` lacks `allows_namespace("py.unverified")`. The function body is unchanged since `620daa9d0`. It still runs from the `HirExpr::Call` arm before the activity early-return and before `apply_closure`, from the pipe arm, from the import-alias arm, at the start of `apply_closure`, and at the start of `Interpreter::call` before the body loop. `run.rs` still maps `CapabilityDenied` to stderr `vox: capability denied: {ns}.{method}` and `exit(77)`. `cargo test -p vox-cli --test run_interp_limits python_capability_call_without_py_exits_77 -- --exact` passed: 1 passed, 7 filtered, 0 ignored. |
| 5 | A workflow calling a Python-capability fn is rejected by capability; the same call in an activity compiles. | ✓ VERIFIED | `determinism_lint.rs` and `crates/vox-compiler/tests/determinism_lint.rs` have no commits after the prior pass. `check_workflow_determinism` still flags `HirCapability::Python` / `PythonUnverified` and is still registered from `typeck/mod.rs`. The four Python workflow tests are still present and not ignored. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Re-verification

| Field | Value |
| --- | --- |
| previous_status | passed |
| previous_score | 5/5 |
| gaps_closed | none this pass (the runtime gap was already closed) |
| gaps_remaining | none |
| regressions | none |

The prior report's digest was stale because later commits edited covered evaluator files. Those edits did not remove the gate.

Commits after `2026-11-03T10:26:00Z` on covered files:

| File | Commits |
| --- | --- |
| `crates/vox-compiler/src/eval/expr.rs` | `b8cf7109c` (325-02), `d6418f5f3` (326-06), `97d5ff2a5` (326-02) |
| `crates/vox-compiler/src/eval/mod.rs` | `b8cf7109c`, `64a990a0f` (326-01), `d6418f5f3` |
| `crates/vox-compiler/src/eval/builtins.rs` | `b8cf7109c`, `64a990a0f`, `d6418f5f3`, `97d5ff2a5`, `98cad7490` (326-04), `eb4ebe5f8` (326-04) |
| `crates/vox-cli/src/commands/run.rs` | `b8cf7109c` |
| `crates/vox-compiler/src/typeck/effect_check.rs` | `b8cf7109c` |

`python.version` (`expr.rs`) refuses without `allows_namespace("py")` before `call_version`. `materialize_to_list` returns `_Denied("py.to_list")` without `py`; `materialize_item` and `materialize_shape` call that function. Those are extra gates on later Python calls. They do not skip `deny_declared_python` for a user function.

Parser, HIR, caps grammar, determinism lint, `python_effects_test.rs`, and `run_interp_limits.rs` are unchanged since the prior pass.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | ----------- | ------ | ------- |
| `crates/vox-ast/src/decl/effect.rs` | `Python` and `PythonUnverified` | ✓ VERIFIED | Unchanged since the prior pass. |
| `crates/vox-compiler/src/parser/descent/decl/head.rs` | Shared `parse_effect_item` errors on unknown names | ✓ VERIFIED | Unchanged. Unknown-effect tests passed again. |
| `crates/vox-compiler/src/parser/descent/decl/head_fn.rs` | `@uses` uses the shared parser | ✓ VERIFIED | Unchanged. Decorator unknown-effect test passed again. |
| `crates/vox-compiler/src/hir/nodes/effect.rs` | `HirEffectKind` labels | ✓ VERIFIED | Unchanged since the prior pass. |
| `crates/vox-compiler/src/hir/nodes/decl.rs` | `HirCapability::as_str` | ✓ VERIFIED | Unchanged since the prior pass. |
| `crates/vox-compiler/src/hir/lower/decl.rs` | Lowering for fn, activity, workflow | ✓ VERIFIED | Unchanged. Activity/workflow lowering test passed again. |
| `crates/vox-compiler/src/typeck/effect_check.rs` | Effect compliance includes the new capabilities | ✓ VERIFIED | Python mapping intact. Caller-without-`python` test passed again after the 325-02 addition. |
| `crates/vox-compiler/src/typeck/determinism_lint.rs` | Capability-aware workflow lint | ✓ VERIFIED | Unchanged. Lint still names `lint.workflow.non_deterministic` for `Python` / `PythonUnverified`. |
| `crates/vox-compiler/src/eval/caps.rs` | `py:allow` / `py:unverified` grammar | ✓ VERIFIED | Unchanged. Runtime tests grant through `CapabilitySet::parse`. |
| `crates/vox-compiler/src/eval/expr.rs` | Denial before the body | ✓ VERIFIED | `verify.artifacts` passed (`allows_namespace("py")` present). `deny_declared_python` is called from `HirExpr::Call` (before the activity early-return and before `apply_closure`), the pipe arm, the import-alias arm, and the start of `apply_closure`. |
| `crates/vox-compiler/src/eval/mod.rs` | `python_fns` / `python_unverified_fns` | ✓ VERIFIED | `verify.artifacts` passed. Sets start empty in `Interpreter::new`. `record_python_capabilities` runs for module functions, tests, MCP tools/resources/examples, and imported `pub fn`s. Endpoint fns record `HirEffectKind::Python` / `PythonUnverified`. `Interpreter::call` denies before the body loop. |
| `crates/vox-compiler/tests/python_effects_test.rs` | Interpreter deny/allow cases | ✓ VERIFIED | Six `python_cap_runtime_*` tests still exported. 17 passed, 0 ignored. |
| `crates/vox-cli/tests/run_interp_limits.rs` | `vox run --caps` exits 77 | ✓ VERIFIED | `python_capability_call_without_py_exits_77` passed. |

`gsd-tools query verify.artifacts` on `324-02-PLAN.md`: 4/4 passed. `324-01-PLAN.md` declares no `must_haves.artifacts` (tool: no artifacts in frontmatter).

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `parse_uses_clause` / `Token::AtUses` | `parse_effect_item` | direct call | WIRED | Unchanged. Unknown-effect tests passed. |
| AST effects | `HirCapability` | `lower_declared_capabilities` | WIRED | Unchanged. Lowering tests passed. |
| `HirCapability` | effect kind and workflow lint | `effect_check.rs`, `determinism_lint.rs`, `typeck/mod.rs` | WIRED | Both lints still registered. |
| `HirFn` registration | `python_fns` / `python_unverified_fns` | `record_python_capabilities` and the endpoint effect loop | WIRED | Functions, tests, MCP, examples, imports, endpoints. |
| `HirExpr::Call` | `deny_declared_python` | before `apply_closure` | WIRED | `expr.rs` Call arm. `python_cap_runtime_without_py_is_denied` passed. |
| `Interpreter::call` | `deny_declared_python` | before the body loop | WIRED | `mod.rs` `VoxValue::Fn` branch. `python_cap_runtime_entry_main_without_py_is_denied` passed. |
| `EvalError::CapabilityDenied` | process exit 77 | `commands/run.rs` | WIRED | `eprintln!("vox: capability denied: {ns}.{method} — grant with --caps ...")` then `exit(77)`. CLI test asserts that stderr and status `Some(77)`. |

`gsd-tools query verify.key-links` on `324-02-PLAN.md` returned `total: 0`, `all_verified: false`. The plan's `key_links` are prose strings, not `{from, to, via}` objects, so the tool checked nothing. The rows above are from the current source and the passing tests.

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| Unknown-effect diagnostic | `ParseError` | `parse_effect_item` | Yes — unknown-effect tests passed this pass | ✓ FLOWING |
| Python capability | `HirFn.capabilities` | `uses python` / `@uses(python_unverified)` | Yes — lowering tests passed this pass | ✓ FLOWING |
| Runtime denial | `EvalError::CapabilityDenied` | `python_fns` membership and `caps.allows_namespace` | Yes — the six `python_cap_runtime_*` tests passed this pass | ✓ FLOWING |
| CLI caps error | stderr + exit code | `CapabilityDenied` in `run.rs` | Yes — `python_capability_call_without_py_exits_77` passed this pass | ✓ FLOWING |
| Workflow diagnostic | `lint.workflow.non_deterministic` | declared `HirCapability::Python*` | Yes — lint source unchanged; four tests still present | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Compile-time effect gate plus runtime deny/allow | `cargo test -p vox-compiler --test python_effects_test -- --test-threads=8` | 17 passed, 0 failed, 0 ignored (includes the six `python_cap_runtime_*` cases) | ✓ PASS |
| `vox run --mode interp --caps env:ro` refuses `uses python` | `cargo test -p vox-cli --test run_interp_limits python_capability_call_without_py_exits_77 -- --exact --test-threads=8` | 1 passed, 0 failed, 7 filtered, 0 ignored (`python_capability_call_without_py_exits_77 ... ok`) | ✓ PASS |
| Workflow lint (truth 5) | not re-run | `determinism_lint.rs` and its test file have no commits since the prior pass; four Python tests are present and not `#[ignore]` | ✓ PASS (regression only) |
| Caps grammar unit tests (truth 3) | not re-run | `caps.rs` has no commits since the prior pass; `py_allow_grants_py_but_not_unverified`, `py_unverified_grants_both_tiers`, and `restrictive_denies_py` are still present | ✓ PASS (regression only) |

### Probe Execution

No phase plan or summary declares a `probe-*.sh`. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| EFF-01 | `324-01-PLAN.md` | Unknown `@uses` / `uses` name is a compile error; `@uses(mcp(tool))` reaches HIR as `Mcp(tool)`; one shared helper | ✓ SATISFIED | Truth 1. |
| EFF-02 | `324-01-PLAN.md`, `324-02-PLAN.md` | A materializer-shaped call without `uses python` fails typecheck; `vox run` without `--caps py` / `py:unverified` refuses at runtime | ✓ SATISFIED | Truth 2 and truth 4. `python_cap_runtime_unverified_without_grant_is_denied` passed. Tier 1 `std.array` names remain phase 326 work (D-03); they are not a 324 success criterion. |
| EFF-03 | `324-01-PLAN.md` | A workflow whose capabilities include `python` / `python_unverified` is rejected; the same call inside an activity compiles | ✓ SATISFIED | Truth 5. |

No requirement mapped to phase 324 in `REQUIREMENTS.md` is absent from the plans. The EFF-02 traceability row is still Pending. This verification did not edit `REQUIREMENTS.md`. The 324 implementation still meets the phase success criteria.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|-----------|-----------|--------|---------|----------|-----------------|---------|
| `crates/vox-compiler/tests/python_effects_test.rs` | EFF-01, EFF-02 | 17 | 0 | no | Behavioral (parse error, capability equality, `CapabilityDenied` ns/method, `Int(7)` on the allow paths) | PASS |
| `crates/vox-compiler/tests/determinism_lint.rs` | EFF-03 | 4 Python-specific tests present, unchanged | 0 | no | Behavioral (`lint.workflow.non_deterministic`; activity path empty) — not re-executed this pass | PASS |
| `crates/vox-cli/tests/run_interp_limits.rs` | EFF-02 runtime | 1 case executed | 0 | no | Behavioral (exit 77, stderr `vox: capability denied: py.secret`, stdout has no `LEAK`) | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0

`python_cap_runtime_py_allow_runs_body` and `python_cap_runtime_developer_default_runs_body` can pass without the deny helper. The deny cases cannot: they expect `CapabilityDenied` with a specific namespace and method. The CLI test fails if the body prints, because `print` uses `println!`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| — | — | No `TBD`, `FIXME`, or `XXX` in `expr.rs`, `eval/mod.rs`, `effect_check.rs`, or `commands/run.rs` | — | Debt-marker gate clear. |

### Advisory (New Scope, Unevidenced)

None. Later phase 325 and 326 commits changed covered evaluator files. The declared-Python denial, the exit 77 arm, and the effect-compliance tests still hold. No new-scope blocker was raised.

### Decision Coverage

`workflow.context_coverage_gate` is unset (query: key not found), so the gate ran. `check.decision-coverage-verify`: honored 5 / total 5, `not_honored: []`, non-blocking. Message: all trackable CONTEXT.md decisions are honored by shipped artifacts.

D-04's bridge worker remains outside this phase. `developer_default()` still allows every gated namespace; `python_cap_runtime_developer_default_runs_body` passed.

### Human Verification Required

N/A — infrastructure/foundation phase. The runtime refusal is exercised by the compiler interpreter tests and by one `vox run --caps` test. No user-facing behavior is left unchecked.

### Gaps Summary

No gaps. Later commits added `python.version` and `std.array` materialization on top of the phase 324 gate. A user function that declares `python` or `python_unverified` is still refused with `CapabilityDenied` before its body runs unless the granted set allows `py` or `py.unverified`. `py:allow` does not satisfy `python_unverified`. `vox run --mode interp --caps env:ro` on a program whose `main` calls that function still exits 77 and does not print `LEAK`.

---

_Verified: 2026-11-04T01:47:32Z_
_Verifier: Claude (gsd-verifier)_
