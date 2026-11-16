---
phase: 320-train-the-sizes-size-03
plan: 02
subsystem: ml-cli
tags: [mens, cli, clap, size-train]
provides:
  - vox mens size-train subcommand on PopuliMensTail
  - run_size_train_reading one-reading CLI seam
affects:
  - crates/vox-ml-cli/src/commands/mens/size_train.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
tech-stack:
  added: []
  patterns:
    - Single FnOnce free-memory reading seam for deterministic CLI testing
key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/mens/size_train.rs
    - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
key-decisions:
  - "D-02/D-06: Expose only --hub-root and --runs-root on vox mens size-train (no --free-mb or --adapter flags) so production callers always take one live reading via read_accelerator_free_mb."
patterns-established:
  - "PopuliMensTail::SizeTrain compiles under both default and gpu feature sets."
requirements-completed:
  - SIZE-03
duration: 15min
completed: 2026-11-01
---

# Phase 320 Plan 02: `vox mens size-train` Subcommand & One-Reading Seam Summary

Registered `vox mens size-train` on `PopuliMensTail`, wired `PopuliMensTail::SizeTrain` in `dispatch.rs`, and added `run_size_train_reading` + `run_size_train` in `crates/vox-ml-cli/src/commands/mens/size_train.rs`.

## Tasks Completed

1. **Task 1: `size-train` writes the absent 0.6B record from one injected reading**
   - Added `PopuliMensTail::SizeTrain { hub_root, runs_root }` in `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs` with unit test `size_train_variant_parses`.
   - Added `SizeTrainArgs`, `run_size_train_reading`, and `run_size_train` in `crates/vox-ml-cli/src/commands/mens/size_train.rs` with unit test `size_train_command_reads_free_memory_once`.
   - Wired `PopuliMensTail::SizeTrain` in `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` for both default and `gpu` builds.
2. **Task 2: A Ready snapshot with no adapter is `train_not_run`**
   - Added and verified `size_train_command_ready_snapshot_is_train_not_run` in `crates/vox-ml-cli/src/commands/mens/size_train.rs`.

## Verification

- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --lib size_train_variant_parses -- --test-threads=8` (`1 passed; 0 failed`)
- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu --lib size_train_ -- --test-threads=8` (`7 passed; 0 failed`)
- `CARGO_INCREMENTAL=0 timeout 900s cargo check -p vox-ml-cli --lib` (`0 errors`)
