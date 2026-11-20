---
phase: 550-high-capacity-qwen3-8-27b-recipe-r64-rslora-chunked-lm-head
workstream: mens-eval-memory-scaling-v94
milestone: v9.4
status: executing
---

# Phase 550 Context: High-Capacity `Qwen3.8-27B` Recipe (`r64` rsLoRA, Chunked `lm_head`, `seq 2048` & Verified Distillation)

## Goal
Beat the `r16` `dogfood_v4` `Qwen/Qwen3.8-27B` baseline locally on 128 GB Apple Silicon by:
1. Moving `lm_head` projection + autograd backward inside the 256-row CE chunk loop (`O(256 * V)` logits memory instead of `[seq_len, 248320]`).
2. Expanding `mens/data/verified_vox_codegen_v2.jsonl` from `1,012` to `>= 3,000` compiler+interpreter-verified rows with zero held-out leakage.
3. Proving the `r64` rsLoRA + chunked `lm_head` + expanded corpus recipe on the fast `Qwen3.5-0.8B` canary (`> 14.1%` / `9/64` on `heldout_bench_v2_canary`) before running full `27B` (`CAP-27B-04`, post-`PID 34157`).

## Smart Discuss Decisions (User-Confirmed)
- **A1 — Corpus & API Spend (`CAP-27B-02`):** Hybrid `>= 3,000` rows — deterministic zero-cost execution-verified synthesis (`~1,800+` rows across 10 Vox problem families) + clean local pool harvest + MIT frontier teacher distillation (`<= $3.00` hard cap via OpenRouter through `vox_actor_runtime::llm`).
- **A2 — GPU Sequencing vs `PID 34157` (`CAP-27B-03..04`):** Execute `CAP-27B-01..02` on CPU and run the fast `0.8B` `r64` rsLoRA + chunked `lm_head` canary in an isolated `--target-dir` + `VOX_PLUGINS_DIR` leaving `target/release/` and `~/Library/Application Support/vox/plugins/` untouched while `PID 34157` runs; defer the multi-hour `Qwen3.8-27B` run (`CAP-27B-04`) as `human_needed` / `verification_deferred_human` while `PID 34157` occupies Metal.
- **A3 — MoE Horizon (`Phases 551–552`):** MoE handling (`35B-A3B` / `80B-A3B`) is **not needed now** (`Qwen3.8-27B` beats both on LCB v6 `90.3` and SWE Pro `61.7`, Phase 547 already rejects MoE cleanly at `config.json`, and MoE adds `1.5k–2.5k` duplicated LoC). Halt `/gsd-autonomous` after Phase `550` (`--to 550`).

## Locked Constraints
- Never rebuild into default `target/release/` or mutate `~/Library/Application Support/vox/plugins/` while `PID 34157` runs.
- Route all LLM synthesis calls strictly through `vox_actor_runtime::llm` (`llm_provider_call` audit clean).
- Zero leakage against `mens/data/heldout_bench/manifest.json` and `mens/data/heldout_bench_v2/manifest.json`.
- For `r64` + `--use-rslora`, use `--alpha 16` (`effective_scale = 16 / sqrt(64) = 2.0`, matching standard `alpha = 2*r` dynamics) rather than `--alpha 64` (`scale = 8.0`).
