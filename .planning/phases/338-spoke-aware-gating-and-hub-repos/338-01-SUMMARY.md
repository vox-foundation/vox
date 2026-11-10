# Phase 338-01 Summary: Collateral Evidence & Gate Alignment

## Completed Scope
1. **`vox-eval::collateral_evidence`**:
   - Pure shared checker `check_collateral_report(report, req)` replacing all duplicate hand-rolled collateral verification logic.
   - Enforces strict execution order:
     1. Status check (`"pass"` required; status mismatch yields `CollateralRefusal::Failed("the collateral report status is \`{status}\` on \`{failed_on}\`")`)
     2. `require_rule_alpha` (`reasoning_slices.rule == req.rule` and `alpha == req.alpha`)
     3. `require_slice_pins` (all required slices present in `slice_blake3` / `slices` matching pinned BLAKE3 hashes)
     4. `require_grader` (report `grader` equals `req.grader`)
     5. `verify_reasoning_slices` (paired McNemar metric recount and check)
   - 3 mutation proofs recorded: dropping power-of-two/rounding or altering guards causes test failures.

2. **`vox-populi` publish, pull, and hub card**:
   - `vox_populi::mens::gate_policy::ladder_requirement` reads `mens/config/eval-gates-ladder.yaml` or embedded fallback.
   - `verify_pulled_adapter_dir` rewired to `verify_publish_gates_with(dir, false, &req)`.
   - On a gated pull, older published bundles lacking valid current collateral evidence (such as published 1.7B v5, 4B v5, 8B v4) are refused and deleted per the fail-closed contract, outputting a clear remedy pointing to `--allow-ungated`.
   - Removed dead standin module `crates/vox-populi/src/mens/hub_card/collateral_standin.rs`.
   - Hub card facts render `needs re-gate` (`GateState::NeedsReGate`) when reports are stale.

3. **`vox-ml-cli` serve gate & `eval-gate check_run`**:
   - `assert_collateral_gate` in `populi/dispatch.rs` delegates to `check_collateral_report`.
   - `check_run` in `eval_gate/check_run.rs` uses `cd.to_requirement()` and `check_collateral_report`.
   - `eval_gate::policy::load_policy_value` implemented and tested.
   - Added `writer_output_satisfies_shared_check` in `eval_collateral.rs`.
   - `train_arm.rs:676-693` maps a gate refusal to `force_ungated = true`, meaning stale or regressed runs publish strictly as ungated, never as passed.

## Verification
- `cargo test -p vox-eval --lib collateral_evidence`: 6 passed, 0 failed.
- `cargo test -p vox-populi --features mens-hf-hub --lib hub_sync::tests::stale`: 3 passed, 0 failed.
- `cargo test -p vox-populi --features mens-hf-hub --lib hub_card`: 113 passed, 0 failed.
- `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate_tests eval_gate eval_collateral`: 172 passed, 0 failed.
- `cargo test -p vox-ml-cli --features gpu --lib -- writer_output_satisfies_shared_check`: 1 passed, 0 failed.
- `vox ci crate-edges`: 0 violations (703 live in-tree edges within baseline).
