---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
plan: 02
subsystem: infra
tags: [cargo-features, build-contention, vox-orchestrator, vox-terminal-core, vox-compiler, route-policy]

# Dependency graph
requires: []
provides:
  - "Clean --no-default-features and --no-default-features --features runtime compilation for vox-orchestrator (--lib and --tests)"
  - "Fail-closed VOX_ROUTE_POLICY_PROFILE, VOX_ROUTE_ALLOW_NET, VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP, and VOX_ROUTE_ALLOW_PROVIDER_NETWORK fallback in vox-orchestrator::route_policy when runtime is disabled"
  - "Opt-in vox-lang feature gating ~47k LoC vox-compiler in vox-terminal-core and vox-term (off by default, enabled in vox-orchestrator-d and vox-gui)"
  - "Aligned default-features = false vox-orchestrator dependency on vox-terminal-core eliminating split-feature recompilation"
affects:
  - 447-03-PLAN
  - vox-terminal-core
  - vox-term
  - vox-orchestrator
  - vox-orchestrator-d
  - vox-gui

actuals:
  tokens: 14800
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Closure-injected environment lookup (exclusion_reason_without_runtime_with) for race-free unit testing without unsafe std::env::set_var"
    - "Compound #[cfg(all(feature = \"runtime\", feature = \"toestub-gate\"))] gating when optional subsystems cross-reference two Cargo features"
    - "Opt-in heavy compiler feature (vox-lang = [\"dep:vox-compiler\"]) off by default in shared core/TUI crates and enabled only in shipping daemon/GUI binaries"

key-files:
  created: []
  modified:
    - crates/vox-orchestrator/src/victory.rs
    - crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs
    - crates/vox-orchestrator/src/route_policy.rs
    - crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs
    - crates/vox-orchestrator/tests/scaling_test.rs
    - crates/vox-orchestrator/tests/context_compaction_wiring_test.rs
    - crates/vox-terminal-core/Cargo.toml
    - crates/vox-terminal-core/src/vox_interp.rs
    - crates/vox-terminal-core/tests/vox_interp.rs
    - crates/vox-terminal-core/tests/session.rs
    - crates/vox-term/Cargo.toml
    - crates/vox-orchestrator-d/Cargo.toml
    - crates/vox-gui/Cargo.toml

key-decisions:
  - "Mirrored all four VOX_ROUTE_* environment variables fail-closed in route_policy::exclusion_reason_without_runtime_with via closure-injected lookup so --no-default-features builds never bypass restricted network/provider policy and unit tests require zero std::env::set_var calls"
  - "Preserved 1-arg eval_line(src: &str) -> anyhow::Result<String> and literal Interpreter::new(100_000) + CapabilitySet::parse(\"\") tokens in crates/vox-terminal-core/src/vox_interp.rs so vox-compiler's production_embedders_assign_caps_explicitly source-audit test passes unchanged"
  - "Kept vox-compiler out of vox-terminal-core [dev-dependencies] so default cargo test -p vox-terminal-core runs lean without compiling 47k LoC of vox-compiler"

patterns-established:
  - "Explicit Vox keyword diagnostic assertion (let x = 42) in session tests under #[cfg(not(feature = \"vox-lang\"))] so ambiguous non-keyword expressions continue falling back to shell"

requirements-completed:
  - BUILD-CONTENTION-02

duration: 10min
completed: 2026-11-15
status: complete
---

# Phase 447 Plan 02: Lean `vox-orchestrator` Feature Guards & `vox-terminal-core` `vox-lang` Decoupling Summary

**Fixed all 5 `--no-default-features` / `--features runtime` `#[cfg]` blockers in `vox-orchestrator` with fail-closed `VOX_ROUTE_*` policy parity, gated `vox-compiler` behind opt-in `vox-lang` in `vox-terminal-core` and `vox-term`, and unified `vox-terminal-core` onto `vox-orchestrator` `default-features = false`.**

## Performance

- **Duration:** 10 min
- **Started:** 2026-11-15T08:49:30Z
- **Completed:** 2026-11-15T08:59:30Z
- **Tasks:** 2
- **Files modified:** 13

## Accomplishments

- Fixed all `--no-default-features` and `--no-default-features --features runtime` blockers in `vox-orchestrator`:
  - Gated `VictoryEvaluator::evaluate` (and its `ObservationReport`/`VictoryVerdict` import) on `#[cfg(feature = "toestub-gate")]` while keeping `VictoryEvaluator::new`, `VictoryConfig`, `VictoryReport`, and `VictoryTiers` unconditional.
  - Added a `#[cfg(not(feature = "toestub-gate"))]` fallback `handle_ai_check` in `dei_dispatch.rs` preserving `"params.path required"` validation and returning `"ai.check requires feature `toestub-gate`"`.
  - Implemented `exclusion_reason_without_runtime_with` + `exclusion_reason_without_runtime` in `route_policy.rs` enforcing `VOX_ROUTE_POLICY_PROFILE`, `VOX_ROUTE_ALLOW_NET`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP`, and `VOX_ROUTE_ALLOW_PROVIDER_NETWORK` fail-closed when `runtime` is disabled, covered by pure closure-driven unit tests.
  - Tightened `Orchestrator::apply_vox_healing` to `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]` with an unused-binding suppressor in the `not(...)` arm so `--no-default-features --features runtime` compiles cleanly without `vox-lsp`.
  - Added `#![cfg(feature = "runtime")]` to `tests/scaling_test.rs` and `tests/context_compaction_wiring_test.rs`.
- Decoupled `vox-terminal-core` and `vox-term` from default `vox-compiler` (~47k LoC) and heavyweight `vox-orchestrator` defaults:
  - Wired `vox-lang = ["dep:vox-compiler"]` (off by default) and `vox-orchestrator = { workspace = true, default-features = false }` in `crates/vox-terminal-core/Cargo.toml`.
  - Preserved `pub fn eval_line(src: &str) -> Result<String>` and literal `Interpreter::new(100_000)` + `CapabilitySet::parse("")` in `crates/vox-terminal-core/src/vox_interp.rs`, returning an actionable diagnostic error when `vox-lang` is disabled.
  - Updated `crates/vox-terminal-core/tests/vox_interp.rs` and `tests/session.rs` (testing explicit `"let x = 42"` `BlockKind::VoxNative` failure under `not(feature = "vox-lang")` and `"40 + 2"` `Ok` evaluation under `feature = "vox-lang"`).
  - Forwarded `vox-lang = ["vox-terminal-core/vox-lang"]` (default off) in `vox-term` and opted `vox-orchestrator-d` and `vox-gui` into `vox-terminal-core` `features = ["vox-lang"]`.

## Task Commits

Git commits and the Cargo verification matrix are deferred to the parent orchestrator's unified single-pass batch across Phase 442 and Phase 447 (per orchestrator execution rules: zero git mutations inside subagent, unified Cargo graph build once).

**Queued Cargo verification matrix for orchestrator batch:**
1. `timeout 600s cargo test -p vox-orchestrator --no-default-features --lib --tests`
2. `timeout 600s cargo check -p vox-orchestrator --no-default-features --features runtime`
3. `timeout 300s cargo test -p vox-terminal-core`
4. `timeout 300s cargo test -p vox-terminal-core --features vox-lang`
5. `timeout 300s cargo check -p vox-term`
6. `timeout 300s cargo test -p vox-compiler --lib eval::caps`

## Files Created/Modified

- `crates/vox-orchestrator/src/victory.rs` - Gated `evaluate` and `ObservationReport`/`VictoryVerdict` import on `toestub-gate`; added `VictoryEvaluator::new` unit test.
- `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` - Gated `handle_ai_check` on `toestub-gate` with path-validating fallback and unit tests.
- `crates/vox-orchestrator/src/route_policy.rs` - Gated `vox_actor_runtime` import on `runtime`; added `exclusion_reason_without_runtime_with` fail-closed `VOX_ROUTE_*` fallback and closure-injected unit tests.
- `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs` - Guarded `vox_lsp` healing block with `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]`.
- `crates/vox-orchestrator/tests/scaling_test.rs` - Added `#![cfg(feature = "runtime")]`.
- `crates/vox-orchestrator/tests/context_compaction_wiring_test.rs` - Added `#![cfg(feature = "runtime")]`.
- `crates/vox-terminal-core/Cargo.toml` - Made `vox-compiler` optional behind `vox-lang` and set `vox-orchestrator` `default-features = false`.
- `crates/vox-terminal-core/src/vox_interp.rs` - Dual `#[cfg(feature = "vox-lang")]` / `#[cfg(not(feature = "vox-lang"))]` `eval_line` preserving `Interpreter::new` and `CapabilitySet::parse("")`.
- `crates/vox-terminal-core/tests/vox_interp.rs` - Feature-gated compiler tests and added disabled-feature diagnostic test.
- `crates/vox-terminal-core/tests/session.rs` - Feature-gated `"40 + 2"` `VoxNative` assertion and added `"let x = 42"` `BlockStatus::Failed` diagnostic assertion when `vox-lang` is disabled.
- `crates/vox-term/Cargo.toml` - Added optional `vox-lang = ["vox-terminal-core/vox-lang"]` feature (off by default).
- `crates/vox-orchestrator-d/Cargo.toml` - Enabled `vox-terminal-core` `features = ["vox-lang"]`.
- `crates/vox-gui/Cargo.toml` - Enabled `vox-terminal-core` `features = ["vox-lang"]`.

## Decisions Made

- Also gated `use vox_db::store::types::{ObservationReport, VictoryVerdict};` in `crates/vox-orchestrator/src/victory.rs` behind `#[cfg(feature = "toestub-gate")]` so `--no-default-features` and `--no-default-features --features runtime` emit zero `unused_imports` warnings.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Guarded private `ObservationReport`/`VictoryVerdict` import in `victory.rs` behind `toestub-gate`**
- **Found during:** Task 1 (`crates/vox-orchestrator/src/victory.rs`)
- **Issue:** `ObservationReport` and `VictoryVerdict` in `victory.rs` are referenced exclusively by `VictoryEvaluator::evaluate`; leaving line 2 unguarded would emit an `unused_imports` warning under `--no-default-features` and `--no-default-features --features runtime`.
- **Fix:** Added `#[cfg(feature = "toestub-gate")]` on `use vox_db::store::types::{ObservationReport, VictoryVerdict};` alongside `pub fn evaluate`.
- **Files modified:** `crates/vox-orchestrator/src/victory.rs`
- **Verification:** `rustfmt --edition 2024` clean; Cargo `--no-default-features` check queued in orchestrator batch.
- **Committed in:** Pending orchestrator batch commit

---

**Total deviations:** 1 auto-fixed (1 Rule 1 unused-import warning prevention)
**Impact on plan:** Necessary to meet the zero-warnings criterion under `--no-default-features`. Zero scope creep.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- All 13 target files for Plan `447-02` are edited, formatted with `rustfmt --edition 2024`, and verified `< 500` LoC with zero `Cargo.lock` or `layers.toml` modifications.
- Ready for the parent orchestrator's unified Cargo verification batch and Plan `447-03`.

---
*Phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c*
*Completed: 2026-11-15*
