# Phase 324: Effect Gate & Determinism - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Mode:** Smart discuss (infrastructure phase — decisions locked by approved design SSOT `docs/src/architecture/vox-python-interop-design-2026-11-01.md` §6)

<domain>
## Phase Boundary

The effect + capability gate every later Python-bridge call depends on: no silent effect drops, two new effects (`python`, `python_unverified`) carried AST→HIR→capability, interpreter `--caps` support, and workflow determinism enforced by capability. No bridge runtime, no std modules (those are Phases 325–326).

</domain>

<decisions>
## Implementation Decisions

- **D-01 (EFF-01):** One shared parser helper parses a single effect item (`<ident>` | `env` keyword | `mcp(<tool>)`) and is used by BOTH `parse_uses_clause` (`parser/descent/decl/head.rs:517+`) and the `Token::AtUses` arm (`parser/descent/decl/head_fn.rs:668-713`). Unknown names are a `ParseError` (`ParseErrorClass::Declaration`) in both forms — the decorator form currently drops them silently, and drops `mcp(x)` because `from_keyword` has no `mcp` arm.
- **D-02 (EFF-02):** Flat effects `python` and `python_unverified` (no parameterized form). Add to `EffectAnnotation` (+ `from_keyword`/`as_str`), `HirEffectKind` (+ `label`), `HirCapability` (+ `as_str`), the lowering map in `hir/lower/decl.rs`, and every exhaustive match the compiler flags. Unknown-effect error message lists the new names.
- **D-03 (EFF-02 typecheck):** Effect propagation uses the existing `check_effect_compliance` fixpoint (`typeck/effect_check.rs`). A fn declaring `uses net` (or any explicit set) that calls a fn declaring `python` without itself declaring `python` must error, same as existing effects. `stdlib_module_capability` gains `"py" => PythonUnverified` only if needed by tests; Tier 1 module names (`array|frame|plot|tensor`) are mapped in Phase 326 when the modules exist (YAGNI now).
- **D-04 (caps):** `GATED` (`eval/caps.rs:16`) gains `"py"` and `"py.unverified"`. `parse` accepts `py:allow` (grants `py`) and `py:unverified` (grants both). `developer_default` keeps allowing all GATED (unchanged semantics). Runtime enforcement on an actual bridge call lands in Phase 325; this phase unit-tests the caps grammar.
- **D-05 (EFF-03):** `check_workflow_determinism` (`typeck/determinism_lint.rs`) additionally flags, inside a workflow body (and transitively through plain helper fns, same walk), any call to a user fn whose declared capabilities include `Python`/`PythonUnverified` and which is not an `activity`; and a workflow that itself declares a Python effect. Code: existing `lint.workflow.non_deterministic` category; message names the capability. Name-based blocklist stays.

### the agent's Discretion
Helper name/location, diagnostic wording, test file placement (prefer existing `crates/vox-compiler/tests/determinism_lint.rs`, `parser/descent/tests.rs`).

</decisions>

<code_context>
## Existing Code Insights

- `EffectAnnotation` enum + `from_keyword`: `crates/vox-ast/src/decl/effect.rs:8-80`.
- `uses` clause parser (already errors on unknown): `head.rs:517-596`. Decorator parser (silently drops): `head_fn.rs:668-713`.
- HIR: `HirEffectKind` `hir/nodes/effect.rs:9`; `HirCapability` `hir/nodes/decl.rs:786`; AST→HIR map `hir/lower/decl.rs:22-44`.
- Other exhaustive users: `typeck/placement.rs`, `typeck/boilerplate_grafts.rs`, `required_capabilities.rs`, `hir/lower/mod.rs`, `semcov_wave17_tests.rs`.
- Determinism lint walk: `typeck/determinism_lint.rs` (321 lines), tests `crates/vox-compiler/tests/determinism_lint.rs`.
- Caps: `eval/caps.rs` (`PURE`, `GATED`, `parse`, `developer_default`).

</code_context>

<specifics>
## Specific Ideas
Goldens/tests required by ROADMAP success criteria: unknown-effect error in both forms; `@uses(mcp(tool))` → HIR `Mcp(tool)`; `@uses(python_unverified)` → HIR + capability; caller-without-effect error; workflow→python-fn rejected, workflow→activity-with-python accepted; caps parse `py:allow`/`py:unverified`.
</specifics>

<deferred>
## Deferred Ideas
Runtime refusal of an actual bridge call (Phase 325); Tier 1 module→capability mapping (Phase 326).
</deferred>
