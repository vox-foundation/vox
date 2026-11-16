---
phase: 320-train-the-sizes-size-03
plan: 01
subsystem: ml-cli
tags: [mens, qlora, size-train, vram]
provides:
  - size_train.v1 record writer per qwen3_code QLoRA size
  - pub(crate) snapshot_complete and adapter_manifest_names_snapshot helpers
affects:
  - crates/vox-ml-cli/src/commands/mens/size_train.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
tech-stack:
  added: []
  patterns:
    - Model-free snapshot completeness and manifest pin verification
key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/size_train.rs
  modified:
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
key-decisions:
  - "D-01: Filter qwen3_code overlay rows to methods containing qlora; sort measured rows ascending by floor_mb (2000, 12000, 20000, 60000) followed by unmeasured rows (1.7B, 4B) in Qwen3 size order."
  - "D-03/D-04/D-05: Strict decision precedence (floor_unmeasured -> does_not_fit -> base_snapshot_absent -> trained / train_not_run) with check_out_path containment guard."
patterns-established:
  - "Injected free_mb and adapter map allow full unit testing of size_train.v1 records without GPU hardware, network calls, or weight downloads."
requirements-completed:
  - SIZE-03
duration: 15min
completed: 2026-11-01
---

# Phase 320 Plan 01: `size_train.v1` Record Writer Summary

Implemented `write_one_size_train` and `write_size_train_records` in `crates/vox-ml-cli/src/commands/mens/size_train.rs` to emit one `size_train.v1` JSON record per `qwen3_code` QLoRA size from a single injected free-memory reading.

## Tasks Completed

1. **Task 1: End-to-end absent 0.6B record**
   - Exposed `pub(crate) fn snapshot_complete` in `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` and `pub(crate) mod detect` in `reasoning_compare.rs`.
   - Registered `#[cfg(feature = "gpu")] pub(crate) mod size_train;` in `crates/vox-ml-cli/src/commands/mens/mod.rs`.
   - Implemented `write_one_size_train` with `check_out_path` containment verification and verified `size_train_absent_0_6b_record`.
2. **Task 2: Every QLoRA size, smallest floor first**
   - Implemented `write_size_train_records` filtering `qwen3_code` QLoRA rows, ordering measured rows by `floor_mb` ascending (`0.6B`, `8B`, `14B`, `32B`) and unmeasured rows (`1.7B`, `4B`) in Qwen3 size order.
   - Verified `size_train_records_every_qlora_size_smallest_floor_first`.
3. **Task 3: Ready fixture trains only when the manifest names the pin**
   - Added `pub(crate) fn adapter_manifest_names_snapshot` in `detect.rs`.
   - Completed the Ready snapshot branch (`trained` when `adapter_manifest_names_snapshot` matches, `train_not_run` otherwise) and verified `size_train_ready_fixture_is_trained_when_manifest_names_the_pin` and `size_train_ready_without_adapter_is_train_not_run`.

## Verification

- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu --lib size_train_ -- --test-threads=8` (`7 passed; 0 failed`)
- Implementation line count before `#[cfg(test)]`: `227` non-blank lines (`< 450` budget).
