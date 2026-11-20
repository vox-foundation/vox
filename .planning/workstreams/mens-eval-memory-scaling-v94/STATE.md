---
gsd_state_version: "1.0"
milestone: v9.4
current_phase_name: "High-Capacity `Qwen3.8-27B` Recipe (`r64` rsLoRA, Chunked `lm_head`, `seq 2048` & Verified Distillation)"
status: human_needed
last_updated: "2026-11-20T09:40:00Z"
last_activity: "2026-11-20T09:40:00Z — Phase 550 CAP-27B-01..03 landed (CAP-27B-04 deferred post-PID 34157; Phases 551–552 halted per A3)."
state_head: 19d94c2ee23c35284d344cd9ff48318b4d4319f5
milestone_name: MENS Harder Execution Eval, Verified Corpus Expansion, Multi-Term Memory Predictor, Early MoE Guard & Canary Proof
workstream: mens-eval-memory-scaling-v94
current_phase: 550
---

# Project State: mens-eval-memory-scaling-v94 (Milestone v9.4)

## Current Position

Phase: Phase 550 (`CAP-27B-01` & `CAP-27B-02` PASS; `CAP-27B-03` PARTIAL / step-capped `8.0/64`; `CAP-27B-04` deferred as `human_needed` awaiting `PID 34157`). Workstream halted after Phase 550 per decision `A3` (Phases `551–552` MoE not needed).
Status: `human_needed` — chunked `256`-row `lm_head`, `3,338`-row verified corpus (`$0.1001` MIT distillation), isolated `0.8B` `r64` canary ablations (`18.24 GB -> 9.15 GB` at `seq=1024`; `3/64 -> 8/64` in `205` opt steps / `30m` cap), and compiler string-OOM guard verified while `PID 34157` (`10:16:06` elapsed) remained undisturbed.
Last activity: 2026-11-20T09:40:00Z — Completed `CAP-27B-01..03` and paused `CAP-27B-04` on live `PID 34157`.

## Deferred Verification (`CAP-27B-04`)

Live Metal eval `PID 34157` (`./target/release/vox-ml-cli mens eval-reasoning-compare ...`) was still running (`10:16:06` elapsed, `1,616,432` KiB RSS). Once `PID 34157` exits, build release artifacts and launch full `Qwen/Qwen3.8-27B` `CAP-27B-04` training + `heldout_bench_v2` evaluation:

```text
cargo build --release -p vox-ml-cli --features gpu,execution-api
cargo build --release -p vox-plugin-mens-candle-metal
VOX_PLUGINS_DIR=/Users/brbrainerd/dev/vox/target/v94_canary_plugins ./target/release/vox-ml-cli mens train --model Qwen/Qwen3.8-27B --device metal --backend qlora --tokenizer hf --data-dir target/v94_canary_data --fast-corpus --output-dir mens/runs/qwen3.8-27b-v94-r64 --rank 64 --alpha 8 --use-rslora --seq-len 2048 --batch-size 1 --grad-accum 8 --epochs 2 --lr 2.0e-5 --warmup 25 --seed 42 --min-rating 3 --gradient-checkpointing --sequence-packing --token-normalized-grad-accum --vram-limit-fraction 0.92 --no-auto-heal --no-hf-sync --force-restart --license-class apache-2.0
```

## Accumulated Context

- **Live Metal Eval Guard:** `PID 34157` runs `./target/release/vox-ml-cli mens eval-reasoning-compare` on `Qwen3.8-27B`. Never rebuild release `vox-ml-cli` or reinstall `mens-candle-metal` dylib while running.
- **Audit Reality (2026-11-19):**
  - `94.2%` (`49/52`) on `heldout_bench` is compile+anti-stub only (`pass` ignores `semantic_expected_contains`, and `tests.vox` assertions are unused for `humaneval-vox` problems).
  - `mens/runs/qwen3.8-27b/run_summary.json` recorded `41,274 MiB` (`43.28 GB`) peak for `r16 --lora-exclude-lm-head`, vs `73.8 GB` single-slope prediction (`1.79x` conservative) and `77.07 GB` (`73,503 MiB`) on earlier `r32` + `lm_head` LoRA.
  - `telem_helpers.rs` omitted `grad_accum` from `token_throughput_proxy`, causing ~`4-6 tok/s` step telemetry vs `30.77 raw tok/s` (`3,347,740 / 108,813.49s`), `28.39 packed tok/s` (`3,089,400 / 108,813.49s`), and `3.85 supervised tok/s` (`418,896 / 108,813.49s`).
  - `vox-hf-layout` buckets `qwen3_next` and `qwen3_5_moe` as dense `HfArchitecture::Qwen35` and `gpt_oss` as `Gpt2`.

### Roadmap Evolution

- Phase 550 edited: replaced Qwen3-32B dense check with High-Capacity Qwen3.8-27B recipe (r64 rsLoRA, chunked lm_head, seq 2048, verified distillation)
