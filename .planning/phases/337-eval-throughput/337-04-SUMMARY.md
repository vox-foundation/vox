# Phase 337 Plan 04 Summary: Side-Targeted Force Restart and Base Generation Reuse

## Overview
Implemented `ForceRestart { adapter: bool, base: bool }` to split the `--force-restart` behavior:
- `--force-restart` on `eval-reasoning-compare` and `eval-collateral-damage` discards only adapter checkpoint generations for the active decode settings.
- Added explicit `--force-restart-base` CLI flag to both commands (`mens_tail_subcommands.rs` and `dispatch.rs`) to discard base generations only when requested.
- In `reasoning_compare.rs`, `run_eval_reasoning_compare` maps `Side::Base` to `force.for_base()` and `Side::Adapter` to `force.for_adapter()`.
- In `eval_collateral.rs`, `run_collateral_damage` threads `ForceRestart` into `score_bench`, passing `force.for_base()` for the pre/base checkpoint and `force.for_adapter()` for the adapter checkpoint and non-reasoning evaluations.

## Tests & Invariants
- `force_restart_targets_only_its_side`: validates `for_adapter()` and `for_base()` flags independently.
- `an_adapter_restart_never_touches_the_base_file`: verifies on real files in a temp root that an adapter restart leaves the base checkpoint file byte-for-byte untouched.
- `a_second_adapter_decodes_no_base_items`: verifies that when evaluating adapter B on a base snapshot already decoded by adapter A, `infer` is called 0 times for the base side even if adapter B specifies `ForceRestart { adapter: true, base: false }`. When explicit `ForceRestart { adapter: false, base: true }` is specified, all base items are decoded again.

## Mutation Proof
- Grep presence check `grep -c 'fn for_base' crates/vox-ml-cli/src/commands/mens/item_checkpoint.rs`: 1 before, 1 after.
- Mutation: changed `for_base(&self)` to return `self.adapter || self.base`.
- Result: both `force_restart_targets_only_its_side` and `a_second_adapter_decodes_no_base_items` FAILED.
- Restored with editor undo and verified all tests pass green.
