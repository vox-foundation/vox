---
phase: 69
phase_name: "decorator-parity-deterministic-skill-engine"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 69 Learnings: decorator-parity-deterministic-skill-engine

## Decisions

### Audit and promote newly verified decorators (@form, @rate_limit) in crat...
Audit and promote newly verified decorators (`@form`, `@rate_limit`) in `crates/vox-compiler/src/feature_matrix.rs`, and provide a deterministic `decorator_parity_summary()` audit helper proving that all 56 decorators across all 4 targets (`Interpreter`, `RustAxum`, `RustTauri`, `TypeScript`) either emit or return a registered diagnostic code (`silent_noop_cells == 0`).

**Rationale:** Primary phase objective established in plan.
**Source:** 69-01-PLAN.md

---

### Provide the deterministic simulation runtime primitives required by the ...
Provide the deterministic simulation runtime primitives required by the `idle-skill-rpg` canonical reference template (`SIM-03`, `SIM-04`): bounded offline tick progression (`simulate_offline_progress`) with exact parity against step-by-step live tick loops, and the 3-axis skill progression model (`Level`, `Mastery`, `Resonance`) with checkpoint discovery at `effective_power = 25, 50, 75, 100` and `NearnessSplash` XP transfer across skill edges.

**Rationale:** Primary phase objective established in plan.
**Source:** 69-02-PLAN.md

## Lessons

### Verified in [crates/vox-compiler/src/feature_matrix.rs](file:///Users/br...
Verified in [`crates/vox-compiler/src/feature_matrix.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-compiler/src/feature_matrix.rs) via `DecoratorParitySummary`, `decorator_parity_summary()`, and `is_ladder_proven_decorator` (promoting `Form` and `RateLimit`).

**Context:** Verified in phase acceptance checks.
**Source:** 69-VERIFICATION.md

---

### Test feature_matrix::tests::decorator_parity_sweep_promotes_form_and_rat...
Test `feature_matrix::tests::decorator_parity_sweep_promotes_form_and_ratelimit_and_has_zero_silent_noops` passed (`11 passed; 0 failed` in `feature_matrix::tests`).

**Context:** Verified in phase acceptance checks.
**Source:** 69-VERIFICATION.md

## Patterns

## Surprises

