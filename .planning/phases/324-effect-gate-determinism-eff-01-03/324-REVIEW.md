---
phase: 324-effect-gate-determinism-eff-01-03
reviewed: 2026-11-03T09:36:04Z
depth: deep
files_reviewed: 13
files_reviewed_list:
  - crates/vox-ast/src/decl/effect.rs
  - crates/vox-compiler/src/eval/caps.rs
  - crates/vox-compiler/src/hir/lower/decl.rs
  - crates/vox-compiler/src/hir/lower/mod.rs
  - crates/vox-compiler/src/hir/nodes/decl.rs
  - crates/vox-compiler/src/hir/nodes/effect.rs
  - crates/vox-compiler/src/parser/descent/decl/head.rs
  - crates/vox-compiler/src/parser/descent/decl/head_fn.rs
  - crates/vox-compiler/src/required_capabilities.rs
  - crates/vox-compiler/src/typeck/determinism_lint.rs
  - crates/vox-compiler/src/typeck/effect_check.rs
  - crates/vox-compiler/tests/determinism_lint.rs
  - crates/vox-compiler/tests/python_effects_test.rs
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 324: Code Review Report

**Reviewed:** 2026-11-03T09:36:04Z
**Depth:** deep
**Files Reviewed:** 13
**Status:** issues_found

## Summary

Reviewed the plan 324-01 commits (`03e07f96e`, `dc6d1f538`, `ac1540516`, `53065172f`) for the new `python` / `python_unverified` effects, unknown-name parse errors, `py` caps, and the workflow determinism lint.

Unknown `@uses` / `uses` names are `Declaration` errors and fail the parse. `python` and `python_unverified` lower on `fn`, `activity`, and `workflow`. `py:allow` grants `py` only; `py:unverified` grants both; `restrictive()` grants neither. A direct workflow call to a non-activity function that declares either capability is rejected, and an `activity` callee is not.

Three gaps sit outside those direct paths. Placement does not treat the new capabilities as native. The determinism walk misses function references. `vox run` still does not enforce `py` caps: the set is stored on the interpreter and no call site reads `allows_namespace("py")` or `"py.unverified"`. That runtime clause stays pending (roadmap success criterion 3, EFF-02). There is no Python dispatch to fail open today.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: Python effects do not force native placement

**File:** `crates/vox-compiler/src/typeck/placement.rs:28-44`
**Issue:** `is_native_capability` is an allowlist, so adding `HirCapability::Python` and `PythonUnverified` did not make the match non-exhaustive. A function whose only declared effect is `python` or `python_unverified` stays `Placement::Shared` (`seed_fn` at lines 56-63). `@place(gui)` and `@place(shared)` are also accepted: `check_override` (lines 294-307) sets `needs_native` from that same allowlist, so it will not report "native-only effects" for a Python capability. Every other host effect in the list (`net`, `fs`, `spawn`, `mcp`, …) is rejected for GUI. The Python bridge is a host worker, so a GUI-placed `uses python` function is the wrong tier.
**Fix:**

```rust
fn is_native_capability(cap: &HirCapability) -> bool {
    matches!(
        cap,
        HirCapability::Net
            | HirCapability::Db
            | HirCapability::Fs
            | HirCapability::Env
            | HirCapability::Clock
            | HirCapability::Random
            | HirCapability::Spawn
            | HirCapability::GpuCompute
            | HirCapability::Vcs
            | HirCapability::Spend
            | HirCapability::Write
            | HirCapability::ExternalApi
            | HirCapability::Python
            | HirCapability::PythonUnverified
            | HirCapability::Mcp(_)
    )
}
```

Add a placement test: `fn f() uses python` seeds `Native`, and `@place(gui) fn f() uses python_unverified` is unsatisfiable.

### WR-02: Determinism lint misses Python passed by reference

**File:** `crates/vox-compiler/src/typeck/determinism_lint.rs:172-216`
**Issue:** The new check runs only after `callee_path` resolves a `Call` or `MethodCall` to a `HirFn` name (lines 189-209). Bare identifiers are ignored (lines 307-316). `walk_arg` (lines 320-327) does not treat an argument identifier as a callee. These two workflows are not rejected, and effect compliance does not save them because an unannotated workflow returns before the cap check (`effect_check.rs` `check_fn`, lines 156-158):

```text
fn py_helper() uses python to int { return 1 }
fn apply(f: fn() to int) to int { return f() }
workflow w() to int { return apply(py_helper) }

fn py_helper() uses python to int { return 1 }
workflow w() to int {
    let f = py_helper
    return f()
}
```

`placement::callee_names` already records a bare identifier in argument position as a callee edge (`placement.rs` `walk_arg`, lines 236-245). EFF-03 is worded as inferred capabilities. Direct calls and the `outer_plain → inner_py` chain are covered; this shape is not. An activity callee must stay exempt.
**Fix:**

```rust
fn walk_arg<'a>(
    arg: &'a HirArg,
    by_name: &HashMap<&'a str, &'a HirFn>,
    visited: &mut HashSet<&'a str>,
    diags: &mut Vec<Diagnostic>,
) {
    if let HirExpr::Ident(name, span) = &arg.value {
        if let Some(callee) = by_name.get(name.as_str()).copied() {
            if callee.durability != Some(DurabilityKind::Activity) {
                if let Some(cap) = declared_python_capability(callee) {
                    emit_diag(cap, *span, diags);
                }
            }
        }
    }
    walk_expr(&arg.value, by_name, visited, diags);
}
```

Also flag a `Let` / `Assign` whose value is a resolved non-activity Python function, or stop treating a later call through that binding as pure. Add the two workflows above next to `workflow_calling_plain_fn_using_python_is_rejected`.

### WR-03: `vox run` does not enforce `py` caps

**File:** `crates/vox-cli/src/commands/run.rs:77-96`
**Issue:** `vox run` parses `--caps` into a `CapabilitySet` and stores it on the interpreter. Nothing in the interpreter consults `py` or `py.unverified`. `allows_namespace` is only called for those namespaces from the new unit tests in `caps.rs`. The builtin gate (`crates/vox-compiler/src/eval/builtins.rs:1072-1074`) denies a namespace that is actually dispatched, and the following `match ns` has no `py` arm. `developer_default()` allows both new namespaces, and a restrictive set such as `--caps net:allow` records the denial, but both are inert. Roadmap phase 324 success criterion 3 (`vox run --caps` without `py` refuses a Python-capability call) is unmet. There is no Python call to fail open yet; the gate is disconnected, not bypassed. Phase context D-04 deferred the bridge call to phase 325, and the EFF-02 checkbox is still open. The grammar that did land is `py:allow` and `py:unverified`. A bare `--caps py` token is `expected ns:value`, not a verified-tier grant.
**Fix:** Keep EFF-02's runtime sentence unchecked. When a Python dispatch exists, refuse before the call:

```rust
if !interp.caps.allows_namespace("py") {
    return Err(/* caps error naming `py` */);
}
// Tier 2 escape hatch:
if !interp.caps.allows_namespace("py.unverified") {
    return Err(/* caps error naming `py.unverified` */);
}
```

Do not treat `developer_default()` allowing `py` as that check. Default `vox run` is the trusted embedder; the refusal is for a `--caps` set that omits `py`.

## Info

### IN-01: Capability diagnostic says the workflow called a stdlib function

**File:** `crates/vox-compiler/src/typeck/determinism_lint.rs:139-157`
**Issue:** `declared_python_capability` reuses `emit_diag`, whose message is `workflow body calls non-deterministic stdlib function \`{path}\``. For a workflow that only declares `uses python` and returns a constant, the text names a call that does not exist, and the hint says to wrap the `python` call. The code `lint.workflow.non_deterministic` is right, and the capability name is in the message, which is what the tests assert.
**Fix:** Use a separate message when the hit is a declared capability: ``workflow declares non-deterministic capability `python` `` (or ``calls `{callee}` which declares `python` `` at a call site). Keep the stdlib wording for `NON_DETERMINISTIC_CALLS`.

---

_Reviewed: 2026-11-03T09:36:04Z_
_Reviewer: gsd-code-reviewer_
_Depth: deep_
