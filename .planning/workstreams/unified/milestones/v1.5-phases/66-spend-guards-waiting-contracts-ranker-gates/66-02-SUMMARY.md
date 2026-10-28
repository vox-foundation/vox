# Summary 66-02: `@ranker` Multi-Factor Transparent Provenance & `@eval_shadow` Ordering Gates

## Status
Completed (`RANK-03`, `RANK-04`).

## Key Changes
1. **Multi-Factor Transparent Provenance (`RANK-03`):**
   - Added `FactorProvenance`, `RankerCandidateEvaluation`, `RankerFactorInput`, and `evaluate_ranker_factors` in [`crates/vox-actor-runtime/src/builtins/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/builtins/mod.rs).
   - Missing optional signals (`raw: None`) are recorded as `absent_neutral = true` with `normalized = 0.5` and `contribution = 0.0` (never penalized).
2. **`@eval_shadow` Ordering Regression Gates (`RANK-04`):**
   - Added `verify_ordering_gate(evaluations, higher_id, lower_id)` in [`crates/vox-actor-runtime/src/builtins/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/builtins/mod.rs) to assert relative ordering invariants (`gold > bronze`) and fail with explicit score diagnostics when violated.
