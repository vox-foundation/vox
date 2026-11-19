---
phase: 324
phase_name: "effect-gate-determinism-eff-01-03"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 13
  lessons: 4
  patterns: 2
  surprises: 1
missing_artifacts:
  - "UAT.md"
---

# Phase 324 Learnings: effect-gate-determinism-eff-01-03

## Decisions

### @uses(...) and uses share parse_effect_item
`@uses(...)` and `uses` share `parse_effect_item`. An unknown name is a `Declaration` parse error that lists `python` and `python_unverified`. `mcp(search)` lowers to `HirCapability::Mcp`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-01-SUMMARY.md

---

### python and python_unverified round-trip AST → HirEffectKind → HirCapabil...
`python` and `python_unverified` round-trip AST → `HirEffectKind` → `HirCapability`, including on `activity` and `workflow`. A `uses net` caller of a `uses python` function is an effect violation; declaring `python` clears it.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-01-SUMMARY.md

---

### CapabilitySet::parse("py:allow") allows py only
`CapabilitySet::parse("py:allow")` allows `py` only. `py:unverified` allows both. `restrictive()` allows neither. `py:bogus` errors with `py takes allow|unverified`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-01-SUMMARY.md

---

### A workflow that declares python or calls a non-activity function that de...
A workflow that declares `python` or calls a non-activity function that declares it emits `lint.workflow.non_deterministic`. An `activity` callee stays exempt.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-01-SUMMARY.md

---

### One parser serves both effect spellings, and it consumes an unknown iden...
One parser serves both effect spellings, and it consumes an unknown identifier so recovery does not re-lex it as the next token.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-01-SUMMARY.md

---

### Denial uses the existing EvalError::CapabilityDenied and the existing py...
Denial uses the existing EvalError::CapabilityDenied and the existing py / py.unverified grants from 324-01

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 324-02-SUMMARY.md

---

### Endpoint functions record Python from HirEffectKind because HirEndpointF...
Endpoint functions record Python from HirEffectKind because HirEndpointFn has effects, not HirFn.capabilities

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 324-02-SUMMARY.md

---

### Pipe and apply_closure also call deny_declared_python so a declared Pyth...
Pipe and apply_closure also call deny_declared_python so a declared Python function cannot run through those paths

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 324-02-SUMMARY.md

---

### Nested secret() with uses python under env:ro returns CapabilityDenied {...
Nested `secret()` with `uses python` under `env:ro` returns `CapabilityDenied { ns: "py", method: "secret" }`. The same program under `py:allow` and under `developer_default()` returns `Int(7)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-02-SUMMARY.md

---

### An entry main that itself declares uses python is denied on Interpreter:...
An entry `main` that itself declares `uses python` is denied on `Interpreter::call` under `env:ro`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-02-SUMMARY.md

---

### uses python_unverified is denied for py.unverified under both env:ro and...
`uses python_unverified` is denied for `py.unverified` under both `env:ro` and `py:allow`, and returns `Int(7)` under `py:unverified`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-02-SUMMARY.md

---

### vox run --mode interp --caps env:ro on a program whose main calls that f...
`vox run --mode interp --caps env:ro` on a program whose `main` calls that function exits 77. Stderr contains `vox: capability denied: py.secret` and `--caps`. Stdout does not contain `LEAK`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-02-SUMMARY.md

---

### The check reads membership recorded from HirCapability at registration
The check reads membership recorded from `HirCapability` at registration. It does not scan source at the call.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 324-02-SUMMARY.md

## Lessons

### The first cargo test -p vox-compiler (fail-fast) stopped on distributed_...
The first `cargo test -p vox-compiler` (fail-fast) stopped on `distributed_policy_warns` and `subagent_chain_depth_error` before the spawn fix. After the fix, `cargo test -p vox-compiler --no-fail-fast` exited 0.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 324-01-SUMMARY.md

---

### EFF-02's requirement sentence also says vox run refuses a py binding wit...
EFF-02's requirement sentence also says `vox run` refuses a py binding without the cap. This plan adds the `CapabilitySet` tokens and the effect-compliance check. No interpreter call site calls `allows_namespace("py")` yet, so the REQUIREMENTS checkbox for EFF-02 stays pending. The plan's EFF-02 truths pass.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 324-01-SUMMARY.md

---

### The prior report's digest was stale because later commits edited covered...
The prior report's digest was stale because later commits edited covered evaluator files. Those edits did not remove the gate.

**Context:** Verified in phase acceptance checks.
**Source:** 324-VERIFICATION.md

---

### python.version (expr.rs) refuses without allows_namespace("py") before c...
`python.version` (`expr.rs`) refuses without `allows_namespace("py")` before `call_version`. `materialize_to_list` returns `_Denied("py.to_list")` without `py`; `materialize_item` and `materialize_shape` call that function. Those are extra gates on later Python calls. They do not skip `deny_declared_python` for a user function.

**Context:** Verified in phase acceptance checks.
**Source:** 324-VERIFICATION.md

## Patterns

### HirFn registration records Python and PythonUnverified; deny_declared_py...
HirFn registration records Python and PythonUnverified; deny_declared_python consults those sets

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 324-02-SUMMARY.md

---

### py:allow satisfies namespace py only; py.unverified is a separate allows...
py:allow satisfies namespace py only; py.unverified is a separate allows_namespace check

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 324-02-SUMMARY.md

## Surprises

### Estimate variance on 324-02
Estimate variance on 324-02: planned 28000 tokens vs 3409 actual (0.12x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 324-02-PLAN.md, 324-02-SUMMARY.md

