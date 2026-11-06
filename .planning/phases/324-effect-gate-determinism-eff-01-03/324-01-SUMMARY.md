---
phase: 324-effect-gate-determinism-eff-01-03
plan: 01
subsystem: compiler
tags: [effects, python, caps, determinism, parser]

requires: []
provides:
  - Shared effect-item parser for `@uses` and `uses` that errors on unknown names
  - `python` and `python_unverified` effects from AST through HirCapability
  - `py:allow` and `py:unverified` capability tokens
  - Workflow determinism lint that rejects non-activity Python effects
affects: [325, 330]

actuals:
  tokens: 10056
  tasks: 4
  commits: 4

tech-stack:
  added: []
  patterns:
    - "parse_effect_item is the only effect-name grammar for decorator and clause forms"
    - "Keyword tokens env and spawn map to the same effect names as identifiers"

key-files:
  created:
    - crates/vox-compiler/tests/python_effects_test.rs
  modified:
    - crates/vox-ast/src/decl/effect.rs
    - crates/vox-compiler/src/parser/descent/decl/head.rs
    - crates/vox-compiler/src/parser/descent/decl/head_fn.rs
    - crates/vox-compiler/src/hir/nodes/effect.rs
    - crates/vox-compiler/src/hir/nodes/decl.rs
    - crates/vox-compiler/src/hir/lower/decl.rs
    - crates/vox-compiler/src/typeck/determinism_lint.rs
    - crates/vox-compiler/src/eval/caps.rs

key-decisions:
  - "@uses and uses share parse_effect_item; unknown names are Declaration errors"
  - "python and python_unverified lower on fn, activity, and workflow"
  - "py:allow grants py; py:unverified grants py and py.unverified"
  - "spawn stays a keyword token and is mapped to the spawn effect"

patterns-established:
  - "Effect lists accept Ident, Token::Env, Token::Spawn, and mcp(tool)"

requirements-completed: [EFF-01, EFF-03]

coverage:
  - id: D1
    description: Unknown @uses and uses names are Declaration errors, and mcp(search) lowers to HirCapability::Mcp
    requirement: EFF-01
    verification:
      - kind: integration
        ref: crates/vox-compiler/tests/python_effects_test.rs#decorator_uses_unknown_effect_is_parse_error
        status: pass
      - kind: integration
        ref: crates/vox-compiler/tests/python_effects_test.rs#decorator_uses_mcp_tool_lowers_to_hir_mcp
        status: pass
    human_judgment: false
  - id: D2
    description: python and python_unverified round-trip to HirCapability and propagate through effect compliance
    requirement: EFF-02
    verification:
      - kind: integration
        ref: crates/vox-compiler/tests/python_effects_test.rs#caller_without_python_effect_is_violation
        status: pass
      - kind: unit
        ref: crates/vox-compiler/src/eval/caps.rs#py_allow_grants_py_but_not_unverified
        status: pass
    human_judgment: false
  - id: D3
    description: A workflow that reaches a non-activity python effect is rejected; an activity is not
    requirement: EFF-03
    verification:
      - kind: integration
        ref: crates/vox-compiler/tests/determinism_lint.rs#workflow_calling_plain_fn_using_python_is_rejected
        status: pass
      - kind: integration
        ref: crates/vox-compiler/tests/determinism_lint.rs#workflow_calling_activity_using_python_is_allowed
        status: pass
    human_judgment: false

duration: 1h 48m
completed: 2026-11-03
status: complete
plan_head_before: 2cbd9b2200424553c56e708062fd5741bd4af3e8
plan_head_after: 53065172f607e30bb9f80e782a85c6a24418af57
---

# Phase 324 Plan 01: Effect Gate & Determinism Summary

**Unknown `@uses` names are parse errors, and `python` / `python_unverified` effects flow through HIR, `py` caps, and the workflow determinism lint.**

## Performance

- **Duration:** 1h 48m
- **Started:** 2026-11-03T07:37:02Z
- **Completed:** 2026-11-03T09:25:02Z
- **Tasks:** 4
- **Files modified:** 13

## Accomplishments

- `@uses(...)` and `uses` share `parse_effect_item`. An unknown name is a `Declaration` parse error that lists `python` and `python_unverified`. `mcp(search)` lowers to `HirCapability::Mcp`.
- `python` and `python_unverified` round-trip AST → `HirEffectKind` → `HirCapability`, including on `activity` and `workflow`. A `uses net` caller of a `uses python` function is an effect violation; declaring `python` clears it.
- `CapabilitySet::parse("py:allow")` allows `py` only. `py:unverified` allows both. `restrictive()` allows neither. `py:bogus` errors with `py takes allow|unverified`.
- A workflow that declares `python` or calls a non-activity function that declares it emits `lint.workflow.non_deterministic`. An `activity` callee stays exempt.

## Task Commits

1. **Task 1: failing tests** - `03e07f96e` (test)
2. **Task 2: shared effect-item parser and python effects** - `dc6d1f538` (feat)
3. **Task 3: py caps and capability-aware determinism lint** - `ac1540516` (feat)
4. **Task 4: spawn keyword regression** - `53065172f` (fix)

## Files Created/Modified

- `crates/vox-compiler/tests/python_effects_test.rs` - EFF-01/EFF-02 pipeline tests
- `crates/vox-compiler/tests/determinism_lint.rs` - EFF-03 workflow tests
- `crates/vox-ast/src/decl/effect.rs` - `Python` and `PythonUnverified`
- `crates/vox-compiler/src/parser/descent/decl/head.rs` - `parse_effect_item`, including `Token::Spawn`
- `crates/vox-compiler/src/parser/descent/decl/head_fn.rs` - `@uses` calls the shared parser
- `crates/vox-compiler/src/hir/nodes/effect.rs` - `HirEffectKind` labels
- `crates/vox-compiler/src/hir/nodes/decl.rs` - `HirCapability` `as_str`
- `crates/vox-compiler/src/hir/lower/decl.rs` - lowering for fn, activity, and workflow
- `crates/vox-compiler/src/hir/lower/mod.rs` - capability to effect-kind
- `crates/vox-compiler/src/typeck/effect_check.rs` - effect-kind to capability
- `crates/vox-compiler/src/required_capabilities.rs` - exhaustive maps
- `crates/vox-compiler/src/typeck/determinism_lint.rs` - Python capability walk
- `crates/vox-compiler/src/eval/caps.rs` - `py` / `py.unverified` grants

## Decisions Made

- One parser serves both effect spellings, and it consumes an unknown identifier so recovery does not re-lex it as the next token.
- `activity` and `workflow` `uses` clauses lower through the same capability map as `fn`.
- `to_tokens` emits `py:allow` or `py:unverified` so a parsed grant round-trips.
- `Token::Spawn` maps to the `spawn` effect. The lexer already keywords that word; no new keyword was added.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing lowering] Activity and workflow dropped declared effects**
- **Found during:** Task 2
- **Issue:** `lower_activity` and `lower_workflow` always stored an empty capability list, so `activity a() uses python` lowered to `[]`.
- **Fix:** `lower_declared_capabilities` feeds fn, activity, and workflow.
- **Files modified:** `crates/vox-compiler/src/hir/lower/decl.rs`
- **Verification:** `activity_and_workflow_uses_clause_lowers_to_capability` passed
- **Committed in:** `dc6d1f538`

**2. [Rule 2 - Caps round-trip] `to_tokens` omitted py grants**
- **Found during:** Task 3
- **Issue:** The plan specified parse arms. The caps tests also rebuild a set from `to_tokens`, which would drop `py`.
- **Fix:** Emit `py:unverified` when that namespace is present, otherwise `py:allow`.
- **Files modified:** `crates/vox-compiler/src/eval/caps.rs`
- **Verification:** `py_grants_round_trip_through_tokens` passed
- **Committed in:** `ac1540516`

**3. [Rule 3 - Hook] tdd-guard rejected the determinism file**
- **Found during:** Task 3
- **Issue:** The installed hook flags `check_workflow_determinism` unless the same file contains a `#[test]`. Integration tests already covered it.
- **Fix:** Added `workflow_python_capability_emits_non_deterministic_diagnostic` in the lint module, and named the function from `tests/determinism_lint.rs`.
- **Files modified:** `crates/vox-compiler/src/typeck/determinism_lint.rs`, `crates/vox-compiler/tests/determinism_lint.rs`
- **Verification:** unit test passed; pre-commit tdd-guard passed
- **Committed in:** `ac1540516`

**4. [Rule 1 - Bug] `spawn` keyword rejected as an unknown effect**
- **Found during:** Task 4
- **Issue:** `@uses(net, spawn)` lexes `spawn` as `Token::Spawn`. The new item parser only special-cased `Token::Env`, so two AI fixture tests failed parse.
- **Fix:** Map `Token::Spawn` to `EffectAnnotation::Spawn`.
- **Files modified:** `crates/vox-compiler/src/parser/descent/decl/head.rs`
- **Verification:** `ai_fixture_typeck_diagnostics` and `mens_decorators` passed; full `vox-compiler` suite passed
- **Committed in:** `53065172f`

**Total deviations:** 4 auto-fixed (1 bug, 2 missing behavior, 1 hook)
**Impact on plan:** Required for the plan's tests and for existing `@uses(net, spawn)` fixtures. No new effect keyword and no new `HirType`.

## Issues Encountered

- The first `cargo test -p vox-compiler` (fail-fast) stopped on `distributed_policy_warns` and `subagent_chain_depth_error` before the spawn fix. After the fix, `cargo test -p vox-compiler --no-fail-fast` exited 0.
- EFF-02's requirement sentence also says `vox run` refuses a py binding without the cap. This plan adds the `CapabilitySet` tokens and the effect-compliance check. No interpreter call site calls `allows_namespace("py")` yet, so the REQUIREMENTS checkbox for EFF-02 stays pending. The plan's EFF-02 truths pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Effect declarations, HIR capabilities, and the determinism lint are in place for a later plan to gate actual Python bridge calls on `py` / `py.unverified`. Phases 325–331 were not started.

## Test tails

RED (`cargo test -p vox-compiler --test python_effects_test --test determinism_lint`), before implementation:

```
error[E0599]: no variant named `Python` found for enum `HirEffectKind`
error: could not compile `vox-compiler` (test "python_effects_test") due to 8 previous errors
EXIT:101
```

`cargo test -p vox-compiler --test determinism_lint`: 8 passed, 4 failed. New tests panicked on `Unknown effect `python``.

`cargo test -p vox-compiler --lib py_`: 4 failed. `CapsParseError { token: "py:allow", why: "unknown namespace" }` and `GATED` did not contain `"py"`.

GREEN:

```
cargo test -p vox-compiler --test python_effects_test --test determinism_lint
test result: ok. 12 passed (determinism_lint)
test result: ok. 11 passed (python_effects_test)
EXIT_INT:0

cargo test -p vox-compiler --lib caps
test result: ok. 25 passed; 0 failed
EXIT_CAPS:0
```

Regression:

```
timeout 1800s cargo test -p vox-compiler --no-fail-fast -- --test-threads=8
test result: ok. 1 passed (doc-tests)
EXIT_COMPILER:0

timeout 900s cargo test -p vox-ast -- --test-threads=8
test result: ok. 50 passed
Doc-tests vox_ast: 0 passed
EXIT_AST:0
```

## Self-Check: PASSED
