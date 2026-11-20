---
phase: 550-high-capacity-qwen3-8-27b-recipe-r64-rslora-chunked-lm-head
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: human_needed
verified: "2026-11-20T09:40:00Z"
---

# Phase 550 Verification Report

## Requirement Verification Matrix

| Requirement | Status | Evidence & Measured Output |
| :--- | :--- | :--- |
| **CAP-27B-01** | **PASS** | `256`-row detached-boundary chunked `lm_head` + CE implemented across `vox-plugin-mens-candle-core`, `metal`, `cuda`, and `vox-populi`. `vox-plugin-mens-candle-core`: `93 passed; 0 failed` + `sameness_gate` `1 passed; 0 failed`; `vox-plugin-mens-candle-metal --lib candle_qlora_train`: `13 passed; 0 failed`; `cargo check -p vox-plugin-mens-candle-cuda`: exit `0`; `vox-populi --features mens --lib mens::tensor`: `212 passed; 0 failed; 1 ignored`. Live Metal `0.8B` `seq=1024` peak dropped `-49.8%` (`18,241,520,638` B -> `9,154,657,838` B). |
| **CAP-27B-02** | **PASS** | `build_and_verify_v94_eval_and_corpus` passed in `126.55s` emitting `3,338` unique verified rows (`2,340` execution-verified: `2,109` synthetic + `133` `humaneval-vox` + `98` `deepseek/deepseek-chat` [`MIT`] at `0.1001 USD` <= `$3.00` cap) with `heldout_bench_v1_passed: true` and `heldout_bench_v2_passed: true` (`contracts/reports/mens-verified-corpus-v2.json`, `contracts/reports/mens-mit-teacher-distill-v94.json`, `target/v94_canary_data/train.jsonl` `3,338` lines). |
| **CAP-27B-03** | **PARTIAL / STEP-CAPPED** | Isolated canary built into `target/v94_canary_plugins` (`PID 34157` untouched). Stabilized `r64, alpha=8, lr=9e-5, --lora-exclude-lm-head, seq=512` (`mens/runs/qwen3.5-0.8b-v94-r64-canary-matched`) converged cleanly (`avg_train_loss=0.4469`, `final_loss_ema=0.3981`, `0` skips, `5.93 GB` peak), scoring **`8.0/64` (`12.5%`, `humaneval-vox` `2/16`)** at `205` optimizer steps (`1800s` cap = `25.7%` of Epoch 1) vs `3.0/64` (`4.69%`) base and `9.0/64` (`14.06%`) full `437`-step / `201.7m` legacy `r16`. Full ablation JSON: `contracts/reports/mens-canary-0.8b-r64-chunked-v2.json`. |
| **CAP-27B-04** | **human_needed** | Deferred intentionally per decision `A2`: live Metal eval `PID 34157` (`./target/release/vox-ml-cli mens eval-reasoning-compare ...`) remained active at `10:16:06` elapsed (`1,616,432` KiB RSS). Ready `Qwen3.8-27B` command recorded in `STATE.md`, `550-SUMMARY.md`, and `docs/src/architecture/larger-local-training-candidates-research-2026-11.md`. |
| **Phases 551–552 (`MOE-35B-*`, `NEXT-80B-*`)** | **STOPPED (`A3`)** | Not executed; zero MoE trainer code added per user confirmation (`A3`). |

## Workspace & Regression Checks
- `cargo run -p vox-arch-check`: exit `0`
- `cargo test -p vox-code-audit --lib llm_provider_call`: `11 passed; 0 failed` (`0.01s`)
- `cargo test -p vox-compiler --lib eval::`: `112 passed; 0 failed` (`1.10s`) — verifies `max_string_bytes` OOM guard integration with compiler evaluator
- `cargo test -p vox-ml-cli --lib harness_aborts_exponential_string_doubling_without_oom`: `1 passed; 0 failed` (`0.01s`)
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/larger-local-training-candidates-research-2026-11.md`: exit `0` (`no hard errors`)
