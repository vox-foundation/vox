---
phase: 550-high-capacity-qwen3-8-27b-recipe-r64-rslora-chunked-lm-head
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: human_needed
requirements-completed:
  - CAP-27B-01
  - CAP-27B-02
requirements-partial:
  - CAP-27B-03
requirements-deferred:
  - CAP-27B-04
---

# Phase 550 Summary: High-Capacity `Qwen3.8-27B` Recipe (`r64` rsLoRA, Chunked `lm_head`, Verified Corpus & Canary Validation)

## Outcome
- **Completed `CAP-27B-01` & `CAP-27B-02`**, completed **isolated `CAP-27B-03` canary ablations (partial / step-capped at `8.0/64` in `205` optimizer steps / `30m` cap)** without disturbing live Metal eval `PID 34157` (`10:16:06` elapsed, `1,616,432` KiB RSS), and cleanly **deferred `CAP-27B-04`** (`Qwen/Qwen3.8-27B` full training + `heldout_bench_v2` eval) until `PID 34157` completes.
- **Halted `/gsd-autonomous` after Phase `550` per user decision `A3`:** Phases `551–552` (MoE support for `35B-A3B` / `80B-A3B`) were not started and zero MoE trainer code was written.

## Key Engineering & Measurement Results

### 1. `CAP-27B-01`: 256-Row Detached-Boundary Chunked `lm_head` + CE
- Implemented `256`-row detached-boundary `lm_head` projection + CE forward/backward across `vox-plugin-mens-candle-core`, `vox-plugin-mens-candle-metal`, `vox-plugin-mens-candle-cuda`, and `vox-populi` without disturbing default `target/release/` or `~/Library/Application Support/vox/plugins/` (staged isolated release dylib at `target/v94_canary_plugins/mens-candle-metal/0.6.0/libvox_plugin_mens_candle_metal.dylib`, `33,233,648` bytes).
- **Verified parity:** `vox-plugin-mens-candle-core` (`93 passed; 0 failed` + `sameness_gate` `1 passed; 0 failed`), `vox-plugin-mens-candle-metal --lib candle_qlora_train` (`13 passed; 0 failed`), `cargo check -p vox-plugin-mens-candle-cuda` (exit `0`), `vox-populi --features mens --lib mens::tensor` (`212 passed; 0 failed; 1 ignored`).
- **Measured `0.8B` `seq_len=1024` Metal memory savings:** `18,241,520,638` B (`18.24 GB`) legacy `r16` vs `9,154,657,838` B (`9.15 GB`) chunked `r64` with `lm_head` LoRA included (**`-49.81%`**, `-9,086,862,800` B).

### 2. `CAP-27B-02`: Expanded Execution-Verified Corpus (`3,338` Rows) + OOM Guard
- Added `crates/vox-ml-cli/src/commands/mens/v94_synth_exec.rs` (`2,120` synthetic `.vox` tasks across 10 families -> `2,109` execution-verified), `crates/vox-ml-cli/src/commands/mens/v94_teacher_distill.rs` (`180` seeds routed through `FacadeBackend` / `vox_actor_runtime::llm` -> `deepseek/deepseek-chat` [`MIT`], `0.1001 USD` total spend <= `$3.00` cap, `98` execution-verified rows emitted in `mens/data/mix_sources/mit_teacher_rft_v94.jsonl` / `contracts/reports/mens-mit-teacher-distill-v94.json`), and clean pool harvest (`dogfood_v4_verified: 675`, `repo_source_pool: 204`, `humaneval: 133`, `train_full_backup_verified: 105`, `validated_clean_verified: 14`).
- Updated `v94_eval_corpus.rs` and verified via `build_and_verify_v94_eval_and_corpus`: **`3,338` emitted rows** in `target/v94_canary_data/train.jsonl` (**`2,340` execution-verified**), **`0` heldout overlap** (`heldout_bench_v1_passed: true`, `heldout_bench_v2_passed: true`, `output_blake3: 765a8f83604e5eec776dda928629456abb5b927935dd764ce654a7a86324d1b0`) in `contracts/reports/mens-verified-corpus-v2.json`.
- Fixed an infinite string-doubling OOM hang uncovered during `eval-local` (`to_hex` completion loop) by adding `max_string_bytes: Option<usize>` in `crates/vox-compiler/src/eval/mod.rs` and `expr.rs` (wired to `Some(1_048_576)` in `crates/vox-ml-cli/src/commands/mens/metrics.rs`; verified by `harness_aborts_exponential_string_doubling_without_oom` and `vox-compiler --lib eval::` `112 passed; 0 failed`).

### 3. `CAP-27B-03`: `Qwen/Qwen3.5-0.8B` `r64` Canary Ablations (Isolated Plugin)
Full JSON comparison recorded in `contracts/reports/mens-canary-0.8b-r64-chunked-v2.json`:
- **Base (`base_0_8b_v2_canary.json`):** `3.0/64` (`4.69%`).
- **Legacy `r16` full run (`mens/runs/qwen3.5-0.8b`, `437` opt steps / `12,100.92s`, `seq=1024`):** `9.0/64` (`14.06%`), `18.24 GB` peak (`avg_train_loss=0.5410`, `final_loss_ema=0.5121`).
- **Run 1 (`qwen3.5-0.8b-v94-r64-canary`, `r64, alpha=16, lr=1.5e-4, seq=1024, lm_head=included`, `58` opt steps / `15m`):** `1.0/64` (`1.56%`, collapsed; `9.15 GB` peak).
- **Run 2 (`qwen3.5-0.8b-v94-r64-canary-s512`, `r64, alpha=16, lr=2.0e-4, seq=512, lm_head=included`, `108` opt steps / `15m`):** `0.0/64` (`0.0%`, collapsed; `6.17 GB` peak).
- **Root cause found:** At `r=64, alpha=16`, AdamW step magnitude in logit space scales as `scale * sqrt(r) * lr`, giving `3.4x–4.4x` larger updates than legacy `r16, alpha=8, lr=9e-5`; additionally `Qwen3.5-0.8B` has `"tie_word_embeddings": true`, so updating tied `lm_head` corrupts base embeddings (unlike `Qwen3.8-27B`, where `"tie_word_embeddings": false`).
- **Run 3 (`qwen3.5-0.8b-v94-r64-canary-stable`, `r64, alpha=8, lr=5e-5, --lora-exclude-lm-head, seq=512`, `108` opt steps / `15m`):** `6.0/64` (`9.38%`), `final_loss_ema=0.3855`, `5.91 GB` peak.
- **Run 4 (`qwen3.5-0.8b-v94-r64-canary-matched`, `r64, alpha=8, lr=9e-5, warmup=20, --lora-exclude-lm-head, seq=512`, `205` opt steps = `25.7%` of Epoch 1 / `1808.59s` capped):** **`8.0/64` (`12.5%`)** (`adt_match: 4/6`, `humaneval-vox: 2/16` double legacy `1/16`, `algorithm: 1/7`, `function: 1/8`), `avg_train_loss=0.4469`, `final_loss_ema=0.3981`, `peak_metal_allocated_bytes=5,933,166,510` (`5.93 GB`), `0` skips. Kept `CAP-27B-03` **partial / step-capped** (`8/64 <= 9/64` under the `1800s` cap vs legacy's `201.7m` / `437` opt steps).

## Deferred Execution (`CAP-27B-04`)
After live `PID 34157` completes:

```text
cargo build --release -p vox-ml-cli --features gpu,execution-api
cargo build --release -p vox-plugin-mens-candle-metal
VOX_PLUGINS_DIR=/Users/brbrainerd/dev/vox/target/v94_canary_plugins ./target/release/vox-ml-cli mens train --model Qwen/Qwen3.8-27B --device metal --backend qlora --tokenizer hf --data-dir target/v94_canary_data --fast-corpus --output-dir mens/runs/qwen3.8-27b-v94-r64 --rank 64 --alpha 8 --use-rslora --seq-len 2048 --batch-size 1 --grad-accum 8 --epochs 2 --lr 2.0e-5 --warmup 25 --seed 42 --min-rating 3 --gradient-checkpointing --sequence-packing --token-normalized-grad-accum --vram-limit-fraction 0.92 --no-auto-heal --no-hf-sync --force-restart --license-class apache-2.0
```
