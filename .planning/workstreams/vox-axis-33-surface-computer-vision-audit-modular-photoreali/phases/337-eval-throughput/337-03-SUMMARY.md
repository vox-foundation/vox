# 337-03 SUMMARY

**Completed:** 2026-11-07
**Requirements:** THRU-10

## What shipped

- `mens/config/eval-gates-ladder.yaml`:
  - Added pre-registered stopping rule keys under `collateral_damage:`
    ```yaml
    rule: fixed_n
    sequential:
      looks_fraction: [0.25, 0.5, 0.75, 1.0]
      look_alpha: [0.0005, 0.002, 0.005, 0.04795]
    ```
  - Documented proven worst-case false-rejection rate $0.04884 \le \alpha$ (exact enumeration via `vox_eval::sequential_mcnemar`).
  - Active rule remains `fixed_n` until explicit user activation (deferred receipt decision).
- `crates/vox-ml-cli/src/commands/mens/eval_gate/policy.rs`:
  - `CollateralRule`: enum with `FixedN` (default) and `SequentialMcnemarV1`.
  - `SequentialBlock`: struct with `looks_fraction` and `look_alpha`, annotated with `#[serde(deny_unknown_fields)]` so any extra keys (e.g. `futility`) are refused at parse time.
  - `CollateralDamageGate`: added fields `rule: CollateralRule` and `sequential: Option<SequentialBlock>` with serde defaults.
  - `CollateralDamageGate::sequential_rule`: returns `Ok(None)` for `FixedN`; for `SequentialMcnemarV1`, builds the rule via `SequentialRule::from_fractions` using `SLICE_SIZE` (128) and the gate's `alpha`, and executes `rule.check_size()`.
  - `validate_collateral_rule`: called on every `load_policy`. Validates structural integrity (monotonicity, fractions ending at 1.0, look levels $\le \alpha$) whenever `sequential` is present, and verifies exact size via `sequential_rule()` when the rule is active.
  - `load_policy`: contextually attributes all parse and validation errors to the policy file path.

## Verification

- `cargo test -p vox-ml-cli --features gpu --lib -- eval_gate::policy`: 19 passed, 0 failed in 2.45s.
- `load_policy_reads_real_ladder_yaml_with_sequential_block`: confirms real ladder policy loads as `FixedN` with sequential fractions `[0.25, 0.5, 0.75, 1.0]` and alphas `[0.0005, 0.002, 0.005, 0.04795]`, and returns `None` for `sequential_rule()`.
- `committed_ladder_sequential_rule_passes_check_size`: parses the real ladder file and proves looks evaluate to `[32, 64, 96, 128]` and worst-case size is $0.04884 \pm 10^{-5} \le 0.05$.
- Comprehensive load refusal tests for malformed policies naming the file path and specific error:
  - unknown key inside sequential (e.g. `futility`)
  - mismatched fraction and alpha vector lengths
  - non-increasing fraction sequences
  - fractions not terminating at 1.0
  - look alpha exceeding gate alpha
  - active `sequential_mcnemar_v1` without a sequential block
  - unknown rule identifier
  - `oversized_active_schedule_is_refused`: schedule with first look alpha equal to 0.05 (producing worst-case size ~0.096) fails when rule is active, reporting computed size and alpha.
  - `oversized_inactive_schedule_is_allowed`: the same schedule loads cleanly when `rule: fixed_n`.

## Mutation Proof

- **Load-time size check enforcement in `sequential_rule`**:
  - Target function: `fn validate_collateral_rule` in `crates/vox-ml-cli/src/commands/mens/eval_gate/policy.rs`
  - Presence count before mutation (`grep -c 'fn validate_collateral_rule'`): 1
  - Mutation applied: removed `rule.check_size().map_err(...)` inside `CollateralDamageGate::sequential_rule`.
  - Observed failure: `oversized_active_schedule_is_refused` panicked with `called Result::unwrap_err() on an Ok value` because the oversized rule was accepted without running `check_size`.
  - Reversion: restored `rule.check_size()`.
  - Presence count after restoration: 1.
  - Re-run test suite: 19 passed, 0 failed.
