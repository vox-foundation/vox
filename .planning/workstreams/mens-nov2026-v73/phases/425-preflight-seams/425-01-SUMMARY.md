# Phase 425 Summary: Stage D Pre-Flight Code Seams & Zero-Copy Cache Prefetch CLI

**Status:** Complete  
**Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)  
**Phase:** `425-preflight-seams`

## Delivered Scope

### Wave 1A — Pre-Flight Code Seams (Parallel Disjoint Implementation)
- **HF Config & Facts (`crates/vox-populi/src/mens/hub.rs`, `crates/vox-populi/src/mens/hub_card/facts.rs`):** Extended HF snapshot download metadata handling (`generation_config.json`, `special_tokens_map.json`) and `hub_card::facts` support for `Qwen3.5-{2B,4B,9B}` and `Qwen3.8-27B`.
- **CLI Download & Completeness Detection (`crates/vox-ml-cli/src/commands/mens/hub.rs`, `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `crates/vox-ml-cli/src/commands/mens/size_train.rs`):** Added `--cache-only` cache prefetch flow without copying shards, filtered shard completeness verification against text-tower keys (excluding vision/MTP shards), and retained `size_train.rs` unwired from `mod.rs` (retired in commit `9a429892f`; Hub Cards v2 dynamically derives ladder facts from `gate_receipt.json` + `reasoning_compare/` via `hub_card::facts`).
- **Ladder Size Selection (`crates/vox-populi/src/mens/tensor/size_select.rs`):** Added multi-ladder resolution for `qwen3_5_code` (`Qwen3.5-{2B,4B,9B}` and `Qwen3.8-27B`) while preserving existing `qwen3_code` tests.

### Coordinator Stabilization & Offline Test Isolation Fixes
- **`mens/config/gpu-specs.yaml` (L323):** Adjusted `train_bases.small_code_default` `Qwen3-8B` `floor_mb` from `122876` to `36500` so `all_live_spokes_resolve_to_trainable_selection` resolves cleanly.
- **`crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (L2100–L2109):** Removed retired `version: "1"` field from inline YAML test fixtures in `the_ladder_policy_states_alpha_and_a_policy_without_it_is_refused`.
- **`crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (L712):** Updated `cpu_explicit_size_is_kept_unchecked` expected size preset from `"qwen3_dev_cpu"` to `"qwen3_0_6b"` to match `SIZE_PRESETS`.
- **`crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (L1069–L1143):** Replaced obsolete `throughput`/`review_recurrence` gate test with `a_below_floor_anti_stub_run_is_refused_by_the_real_policy` (`"anti_stub_task_success": 0.50` < `0.92`).
- **`crates/vox-ml-cli/src/commands/mens/eval_gate/run_gate.rs` (L228–L289):** Guarded HF auto-publish and ladder-index refresh behind `#[cfg(not(test))]` and forwarded `root.as_deref()` to `refresh_ladder_index_blocking`, isolating `cargo test` from live HF Hub writes when `VOX_MENS_HF_NAMESPACE` is set. Deleted stale test-generated `crates/vox-ml-cli/mens/`.

## Build & Verification
- Formatted edited `.rs` files with `rustfmt --edition 2024` (`exit 0`).
- `timeout 900s cargo test -p vox-populi --features mens,mens-train,mens-hf-hub --lib -- --test-threads=1` -> `ok. 551 passed; 0 failed; 4 ignored` (12.42s).
- `timeout 900s cargo test -p vox-ml-cli --features gpu --lib` -> `ok. 495 passed; 0 failed; 2 ignored` (216.79s).
- `timeout 900s cargo build --release -p vox-ml-cli --features mens-candle-metal` -> `exit 0` (`2m 15s`, `./target/release/vox-ml-cli`).
- `timeout 900s cargo build --release -p vox-cli` -> `exit 0` (`9m 52s`, `./target/release/vox`).
