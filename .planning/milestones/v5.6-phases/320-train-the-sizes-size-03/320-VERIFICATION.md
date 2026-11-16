---
phase: 320-train-the-sizes-size-03
status: passed
verified: 2026-11-01
requirements:
  - SIZE-03
---

# Phase 320 Verification: Train the Sizes (`SIZE-03`)

## Goal-Backward Verification

| Requirement / Must-Have | Evidence | Status |
|---|---|---|
| Measured `qwen3_code` QLoRA row whose snapshot is absent gets `size_train.v1` under `mens/runs/size_train/<BaseLabel::report_file_name>`, `status: "not_trained"`, `reason: "base_snapshot_absent"`, `floor_mb`, `free_mb` | `commands::mens::size_train::tests::size_train_absent_0_6b_record` | Passed |
| Every `qwen3_code` QLoRA size gets one record from one injected `free_mb`, smallest measured floor first (`0.6B`, `8B`, `14B`, `32B`, then `1.7B`, `4B`); 14B LoRA excluded | `commands::mens::size_train::tests::size_train_records_every_qlora_size_smallest_floor_first` | Passed |
| Ready snapshot + matching `adapter_manifest.json` -> `status: "trained"`; Ready snapshot without matching manifest -> `reason: "train_not_run"`; no `score` key | `size_train_ready_fixture_is_trained_when_manifest_names_the_pin`, `size_train_ready_without_adapter_is_train_not_run` | Passed |
| `vox mens size-train` registered on `PopuliMensTail`, takes one reading via `run_size_train_reading`, compiles with default and `gpu` features | `size_train_variant_parses`, `size_train_command_reads_free_memory_once`, `size_train_command_ready_snapshot_is_train_not_run` | Passed |

## Verification Output

- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --lib size_train_variant_parses -- --test-threads=8` → `1 passed; 0 failed`
- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-ml-cli --features gpu --lib size_train_ -- --test-threads=8` → `7 passed; 0 failed`
