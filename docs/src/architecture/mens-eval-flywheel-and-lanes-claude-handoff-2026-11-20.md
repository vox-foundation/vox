---
title: "MENS Eval Flywheel, Architecture & Training Lanes — Claude Handoff (2026-11-20)"
description: "Engineering handoff for Claude Code covering post-Phase-550 state, architecture expansion for codegen vs hub-and-spokes, truthful training-lane taxonomy resolution, time-to-train+eval instrumentation, missing flywheel metrics, and a gated 50 USD data-imputation plan."
category: "Architecture SSOTs"
status: "current"
topic: "mens-training"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "google-deepmind-agent"
training_eligible: false
---

# MENS Eval Flywheel, Architecture & Training Lanes — Claude Handoff (2026-11-20)

<!-- kb:summary -->
Engineering handoff for Claude Code summarizing v9.4 Phases 547–550, removal of MoE Phases 551–552, and the post-PID 34157 plan. It recommends reusing `qwen3` and `qwen3_5` kernels for hub-and-spoke adapters (`MiMo-V2.6-Distill-Qwen-9B`, `FrogNano-4B`, `Qwen3Guard`) with `Granite-4.2` `aLoRA` as the sole near-term new architecture, resolves four overloaded meanings of "training lane" into `backend`, `spoke`, and `data_slice`, specifies time-to-train-and-eval telemetry plus shared-prefix KV caching and sequential gates, lists missing failure-funnel and repair-loop KPIs, and allocates an optional gated 50 USD data-imputation budget.
<!-- /kb:summary -->

> **Session & Safety Invariants:**
> - **Do not touch live `PID 34157`** while running (`./target/release/vox-ml-cli mens eval-reasoning-compare` on `Qwen/Qwen3.8-27B`; check with `ps -p 34157 -o pid,etime,rss,command` first). Isolated builds from Phase `550` live in `target/v94_canary_rel` + `target/v94_canary_plugins` (`33,233,648` B dylib) and `target/debug/vox-ml-cli`, leaving default `target/release/` and `~/Library/Application Support/vox/plugins/` intact.
> - **Uncommitted working tree:** Phase `547–550` implementation edits, verified reports under `contracts/reports/`, `max_string_bytes` OOM guard in `vox-compiler` + `vox-ml-cli`, this handoff doc, and the `gsd-tools query phase.remove` updates (`552`, `551`) are in the working tree for Claude Code to review, test end-to-end, and commit cleanly once ready.

---

## 1. Where We Are & Why (Post-Phase `550` Snapshot)

### Landed in Workstream `mens-eval-memory-scaling-v94` (`v9.4`)

1. **Phases `547–549` (Completed):**
   - **Early MoE / Unsupported Arch Guard (`vox-hf-layout`):** `validate_supported_hf_config` and `classify_hf_architecture` in `crates/vox-hf-layout/src/lib.rs` (`lines 440–520`) fail closed at `config.json` parse for `qwen3_5_moe*`, `qwen3_next`, `gpt_oss`, `glm4_moe`, `gemma3*`, `gemma4*`, `qwen4*`, `mamba`, and `rwkv`.
   - **Throughput Reconciliation (`telem_helpers.rs`):** Fixed the `grad_accum` omission (`~4–6 tok/s` bug) and separated raw dataset tokens (`30.77 tok/s`), packed tokens (`28.39 tok/s`), and assistant-only supervised tokens (`3.85 tok/s`) on the historical `108,813.49s` `Qwen3.8-27B` run (`3,347,740` / `3,089,400` / `418,896` tokens, `13.56%` supervised ratio).
   - **Harder Execution-Verified Eval (`heldout_bench_v2`):** Replaced saturated compile-only scoring (`94.2%` on `52` v1 tasks) with `166` self-verified execution tasks (`mens/data/heldout_bench_v2/manifest.json`) + `64`-task canary slice (`mens/data/heldout_bench_v2_canary/manifest.json`), plus 5-rung CPU rescore (`0.8B 21.7%`, `2B 28.3%`, `4B 47.8%`, `9B 54.3%`, `27B 69.6%` strict / `93.5%` signature-rescued across `46` runnable v1 items in `contracts/reports/mens-execution-eval-rescore-v2.json`).
   - **Multi-Term Metal Memory Predictor (`contracts/mens/memory-model.v1.yaml`):** Fitted across 11 Apple Silicon runs (`1.000x` exact on `27B` lean `43.28 GB`, `1.012x` on `27B r32 + lm_head` `77.99 GB` vs `77.07 GB`, removing the `1.71x` single-slope over-prediction).

2. **Phase `550` (`CAP-27B-01..03` landed; `CAP-27B-04` queued post-`PID 34157`):**
   - **`CAP-27B-01` (256-row chunked `lm_head` + CE):** Implemented detached-boundary chunked projection across `vox-plugin-mens-candle-{core,metal,cuda}` and `vox-populi` (`93/93` + `sameness_gate` `1/1` in `candle-core`, `13/13` in `candle-metal`, `cuda` check exit `0`, `212/212` `mens::tensor`). Cut `0.8B` `seq_len=1024` Metal peak by **`49.81%`**: `18,241,520,638` B (`18.24 GB`) legacy `r16` -> `9,154,657,838` B (`9.15 GB`) chunked `r64` with `lm_head` LoRA included.
   - **`CAP-27B-02` (`3,338`-row Verified Corpus + String OOM Guard):** Expanded `target/v94_canary_data/train.jsonl` / `verified_vox_codegen_v2.jsonl` to **`3,338` unique rows** (**`2,340` execution-verified**: `2,109` synthetic across 10 Vox families + `133` `humaneval-vox` + `98` MIT teacher RFT rows from `deepseek/deepseek-chat` for **`0.1001 USD`** total OpenRouter spend via `vox_actor_runtime::llm`), with zero held-out leakage (`heldout_bench_v1_passed: true`, `heldout_bench_v2_passed: true`; `contracts/reports/mens-verified-corpus-v2.json` and `contracts/reports/mens-mit-teacher-distill-v94.json`). Also added `max_string_bytes = Some(1_048_576)` in `crates/vox-compiler/src/eval/{mod,expr}.rs` and `crates/vox-ml-cli/src/commands/mens/metrics.rs` after `eval-local` exposed an exponential string-doubling hang on `to_hex`.
   - **`CAP-27B-03` (`0.8B` `r64` Canary Ablations, `contracts/reports/mens-canary-0.8b-r64-chunked-v2.json`):**
     - `alpha=16, lr=1.5e-4..2.0e-4` + updates to tied `lm_head` (`"tie_word_embeddings": true` on `Qwen3.5-0.8B`) collapsed (`1.0/64` at `seq=1024`, `0.0/64` at `seq=512`) due to `3.4x–4.4x` larger AdamW logit step norms (`scale * sqrt(r) * lr`) and embedding corruption.
     - Stabilized `r=64, alpha=8` (`scale=1.0`), `lr=9e-5`, `--lora-exclude-lm-head`, `seq=512` (`mens/runs/qwen3.5-0.8b-v94-r64-canary-matched`, `1808.59s` cap = `205` optimizer steps = `25.7%` of Epoch 1) converged cleanly (`avg_train_loss=0.4469`, `final_loss_ema=0.3981`, `0` skips, `5.93 GB` peak) to **`8.0/64` (`12.5%`)**, doubling `humaneval-vox` (`2/16` vs `1/16`) relative to `3.0/64` (`4.69%`) base and approaching the full `437`-step (`12,100.92s`) legacy `r16` run (`9.0/64`, `14.06%`). Marked **partial / step-capped** (`[ ]` in `REQUIREMENTS.md`).
   - **`CAP-27B-04` (Deferred):** Ready post-`PID 34157` command targets untied `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` (`"tie_word_embeddings": false`) with `r=64, alpha=8, lr=2.0e-5, seq=2048` and `lm_head` LoRA included:

```zsh
cargo build --release -p vox-ml-cli --features gpu,execution-api
cargo build --release -p vox-plugin-mens-candle-metal
VOX_PLUGINS_DIR=/Users/brbrainerd/dev/vox/target/v94_canary_plugins ./target/release/vox-ml-cli mens train --model Qwen/Qwen3.8-27B --device metal --backend qlora --tokenizer hf --data-dir target/v94_canary_data --fast-corpus --output-dir mens/runs/qwen3.8-27b-v94-r64 --rank 64 --alpha 8 --use-rslora --seq-len 2048 --batch-size 1 --grad-accum 8 --epochs 2 --lr 2.0e-5 --warmup 25 --seed 42 --min-rating 3 --gradient-checkpointing --sequence-packing --token-normalized-grad-accum --vram-limit-fraction 0.92 --no-auto-heal --no-hf-sync --force-restart --license-class apache-2.0
```

3. **Phases `551–552` Removed (`gsd-tools query phase.remove`):**
   - Removed MoE Phases `551` (`Qwen3.6-35B-A3B`) and `552` (`80B`-class `qwen3_next`) from `.planning/workstreams/mens-eval-memory-scaling-v94/ROADMAP.md` and marked them removed in `REQUIREMENTS.md`. Neither beats `Qwen3.8-27B` on verified coding benchmarks (`73.4` vs `75.0` SWE-bench Verified on Qwen's comparative card; `Qwen3.8-27B` hits `61.7` SWE Pro / `90.3` LiveCodeBench v6 / `73.0` Terminal-Bench 2.1).

---

## 2. Open Questions for Claude to Critically Assess & Challenge

1. **Should we land eval timing + prefix KV cache + failure-funnel telemetry *before* launching `CAP-27B-04`?** A full `27B` train+eval cycle costs `~30h` train + `10–16h` collateral + `eval-local` (`40+` GPU hours). Measuring time and fixing redundant prefill recomputation first converts every subsequent run into comparable flywheel data.
2. **Should `Qwen/Qwen3.5-2B` replace `Qwen/Qwen3.5-0.8B` as the Tier-1 canary?** As documented in `mens-model-selection-nov-2026-research-2026-11-13.md` (§4.2), `0.8B` puts `~30%` of its parameters into tied `248,320`-vocab embeddings (`GPQA 11.9`, `TAU2 11.6`, `BFCL-V4 25.3`), whereas `Qwen3.5-2B` (`18,740` MiB peak at `seq=1024`) scores `51.6` GPQA / `48.8` TAU2 / `78.6` IFEval — enough capacity to separate recipe improvements from model starvation in `30–45m`.
3. **Should `CAP-27B-04` run a `30m` `seq=2048` preflight probe before the 2-epoch run?** Verifying steady-state `peak_metal_allocated_bytes`, initial loss finite checks, and `grad_norm` with untied `lm_head` LoRA over 15–20 optimizer steps avoids discovering a `seq=2048` spill cliff hours into the run.
4. **Are `eval-gates-ladder.yaml` thresholds (`min_pass_rate_at_1: 0.25`) aligned with `heldout_bench_v2`?** `eval-gates-ladder.yaml:8` (`0.25`) was written for compile-only v1 (`52` tasks); `0.8B` drops from `73.1%` v1 to `14.1%` v2 canary. Calibrate v2 per-size floors separately and consider reserving a small rotating v3 blind slice (`40–50` tasks) so synthetic data never overfits `heldout_bench_v2`.

---

## 3. Architecture Expansion Decision (Vox Codegen vs Hub+Spokes)

| Role / Spoke | Recommended Base & Pin (`mens/config/gpu-specs.yaml`) | License | New Candle Arch Needed? | Evidence & Rationale |
| :--- | :--- | :--- | :--- | :--- |
| **`vox-lang` (Workstation Hub)** | **`Qwen/Qwen3.8-27B`** (`@37561816...`) | Apache-2.0 | **No (`qwen3_5_text`)** | `61.7` SWE-bench Pro, `90.3` LiveCodeBench v6, `73.0` Terminal-Bench 2.1, untied `lm_head`, fits 128 GB Mac cleanly. |
| **Tier-1 Fast Canary / Laptop** | **`Qwen/Qwen3.5-2B`** (`@15852e8c...`) / **`4B`** (`@851bf6e8...`) | Apache-2.0 | **No (`qwen3_5_text`)** | Avoids `0.8B` `248K` tied-vocab collapse (`43.6` / `50.3` BFCL-V4; `48.8` / `79.9` TAU2-Bench). Keep `Qwen3-0.6B` only for CPU/WebGPU ONNX. |
| **`tool-selection` & `argument-generation` + Skill Dispatch** | **`XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B`** (`@1f09ddcc...`, `gpu-specs.yaml:349`) | MIT (+ attr.) | **No (`qwen3_5_text`)** | `+12.6` SWE Pro (`44.6` vs `32.0`), `+10.1` Terminal-Bench 2.1 (`37.1`), `+9.3` Toolathlon (`35.2`), `+25.3` AutomationBench (`30.3` vs `5.0`) over `Qwen3.5-9B`. |
| **Lightweight Tool-Loop / CLI Controller** | **`microsoft/FrogNano-4B-2609`** (`@1844d060...`, `gpu-specs.yaml:348`) | Apache-2.0 | **No (`qwen3_5_text`)** | Trained via TaskPilot executable SWE RL (`61.5%` SWE Verified, `37.6%` SWE Pro at 4B); high signal for `read/write/edit/glob/bash` control. |
| **Pre-Tool Safety / Policy / Effort Judge** | **`Qwen/Qwen3Guard-Gen-0.6B`** (`@fada3b2f...`, `gpu-specs.yaml:347`) | Apache-2.0 | **No (`qwen3`)** | Drop-in 0.6B guardrail / loop-judge spoke replacing brittle regex heuristics without touching Candle kernels. |
| **Skill / Tool Retrieval Embedder (`hub.embedder`)** | **`Qwen/Qwen3-Embedding-0.6B`** (`@97b0c614...`, `domain-profiles.yaml:12`) | Apache-2.0 | **Embedding wire-up only** | Replace causal LM / 256-dim FNV-1a `local_embed_text()` fallback in `vox-search` with real neural embeddings (`75.41` MTEB Code v1). |
| **Next Worthwhile Architecture (Post-Flywheel)** | **`ibm-granite/granite-4.2-8b`** + **`granite-switch-4.1-8b-preview`** | Apache-2.0 | **Yes (~150–250 LoC)** | Pure dense GQA (zero DeltaNet recurrence), `73.24` LiveCodeBench v6 + `79.33` IFBench at 8B, and **`aLoRA` intrinsic switching** over a shared base KV cache for multi-spoke / skill / guardrail routing (`mens-model-selection-nov-2026-research-2026-11-13.md` §4.4–4.5). |

### Models & Paths Explicitly Excluded (and Why)

- **Sparse MoE (`Qwen3.6-35B-A3B`, `Qwen3-Coder-Next-80B`, `GLM-4.5-Air`, `DeepSeek-V2/V3`):** Zero coding quality advantage over `Qwen3.8-27B` today (`~1,000–2,500` LoC across duplicated Metal/CUDA backends). Only revisit for **serving throughput (`3B`-active speed)** or once a permissive open model cleanly beats `Qwen3.8-27B`, and prefer an external `mlx-lm` / `vox-python-bridge/worker` sidecar emitting standard LoRA adapters before porting MoE kernels to Rust.
- **License Traps for Redistribution/Automation:** Never use `Salesforce/xLAM-2` or `MadeAgents/Hammer2.1` (`cc-by-nc`), `nvidia/Nemotron-Orchestrator-8B` (R&D-only — copy ToolOrchestra reward design only), `katanemo/Plano-Orchestrator` / `Arch-Agent` (`license: other`), `Devstral-2-123B` (Modified MIT), `Qwen3.8-Flash-Next` (`qwen-community-1.0`), or gated `Gemma` / `Llama` weights as shipped bases. `google/gemma-4-*` is Apache-2.0, but needs a heavy `gemma4_unified` port while `31B` trails `Qwen3.8-27B` (`35.7` vs `61.7` SWE Pro) — use via API only when native audio+vision is required.
- **Note on Existing XML `<tool_call>` Drift:** `Qwen3.8-27B`, `MiMo-V2.6-Distill`, and `Granite-4.2` emit XML `<tool_call><function=NAME><parameter=K>V</parameter></function></tool_call>` whereas `vox-eval` / salvage paths historically assumed JSON envelopes (`mens-model-selection-nov-2026-research-2026-11-13.md` §6.2 item 2). Ensure both `vox-eval::episode` and `eval_local.rs:524-526` grade the exact XML format trained for `27B`/`9B` tool spokes.

---

## 4. Resolving Training Lanes So Naming Reflects Truth

### Verified Drift & Naming Overload Today

The word `"lane"` is currently overloaded across **four incompatible concepts** and mixed configs:

1. **Compute Backend (`contracts/mens/memory-model.v1.yaml:36-41`):** uses `lane: candle-cuda` and `lane: candle-metal`.
2. **Row Dataset Slice (`crates/vox-corpus/src/corpus/mix/mod.rs:459-478`, `mens/config/mix*.yaml`):** uses `lane: vox_codegen`, `vox_tooling`, `vox_docs_qa`, `vox_lang_tier_b`, `vox_tool_selection`, `vox_argument_generation`, `vox_rust_authoring`, `vox_rust_review`, `vox_rust_expert_cross`, `vox_research_expert`.
3. **Arbitrary Letter Alias (`mens/config/domain-profiles.yaml:184`):** describes `research-expert` (a non-QLoRA profile) as `"Lane G"`.
4. **Orchestrator Execution Lanes:** `Sync` / `Background` / `Plan` in `modern-model-selection-and-cost-accuracy-ssot-2026.md`.

Even worse, configuration truth diverges from documentation and active code:
- **Docs vs `mix.yaml` contradiction:** `docs/src/reference/mens-training-data-contract.md:40,67` claims `mens/config/mix.yaml` uses `include_lanes: [vox_codegen]` (code-only, no docs QA), yet `mens/config/mix.yaml:83-88` actually includes `vox_tooling`, `vox_docs_qa`, `vox_lang_tier_b`, and `vox_research_expert`, and still marks `target/dogfood/train.jsonl` as the sole `optional: false` source.
- **Unwired Phase `550` corpus:** `mens/config/mix.yaml` and `mens/config/mix-vox-lang.yaml` do **not** yet point to `mens/data/verified_vox_codegen_v2.jsonl`; `Phase 550` required `--fast-corpus --data-dir target/v94_canary_data` to bypass `mix.yaml`. Also every source in `mix-vox-lang.yaml` is `optional: true`.
- **Duplicate / dead harness mixes:** `mens/config/mix-agents.yaml` overlaps `mix-harness.yaml`, `mix-tool-selection.yaml`, and `mix-argument-generation.yaml`, while `mix-rust.yaml:16` includes `vox_rust_review` even though `domain-profiles.yaml:83` routes review signals to the base model.
- **Stale spoke defaults:** `domain-profiles.yaml` (`lines 57-61, 105-108, 132-135`) still points `vox-lang` at `small_code_default` / `qwen3_16g` (`8B`) and `tool-selection`/`argument-generation` at `agentic_default` / `qwen3_16g`, while `mens/config/hub-cards.yaml:13` hardcodes `default_spoke: vox-lang` because run manifests lack a `spoke_id` + data-slice histogram (`facts.rs` / `mens-models.md:61`).

### Target 3-Axis Schema (Unambiguous Separation)

Replace overloaded `"lane"` uses cleanly while preserving `serde(alias = "lane")` during migration:

| Axis | Canonical Field & Type | Valid Values | Owned By / Enforced By |
| :--- | :--- | :--- | :--- |
| **1. Hardware Execution Backend** | `backend: ComputeBackend` | `candle-metal`, `candle-cuda`, `candle-cpu` | `contracts/mens/memory-model.v1.yaml` (rename `lane:` -> `backend:` with alias), `vox-plugin-mens-candle-*` |
| **2. Adapter Target (`spoke`)** | `spoke_id: String` | Fine-tuned QLoRA spokes: `vox-lang`, `harness-tools` *(mono until `tool-selection` + `argument-generation` beat it on `agent_episodes`)*, `harness-skills`, `rust-authoring` | `mens/config/domain-profiles.yaml` + `crates/vox-populi/src/mens/tensor/domain_profiles.rs` (`validate_all_profiles`) & `domain_router.rs` + `training_manifest.json` (`spoke_id`, derived automatically for Hub cards!) |
| **3. Corpus Supervision Slice** | `data_slice: String` + `verification_tier: enum` | `<domain>.<kind>`: `vox.exec_verified`, `vox.compile_verified`, `vox.repair_trace`, `vox.docs_grounded_qa`, `harness.tool_select`, `harness.tool_args`, `harness.skill_dispatch`, `rust.authoring_verified`, `replay.reasoning` | `mens/config/mix-<spoke>.yaml`, `vox_corpus::corpus::mix`, stamped per row + aggregated into `training_manifest.json` (`data_slice_histogram`, `verification_histogram`) |

### Concrete Cleanup Plan

1. **Retire ambiguous names:** Remove `"Lane G"` (`domain-profiles.yaml:184`), replace `"vox_lang_tier_b"` (`vox.repair_trace` / `vox.exec_verified` instead), delete or archive duplicate `mix-agents.yaml`, and rename non-QLoRA profiles (`rocks`, `research`, `populi-meta`, `research-expert`) under explicit `mode: retrieval_only` / `mode: eval_only`.
2. **One canonical mix per spoke + strict `optional: false` inputs:**
   - Unify `mix.yaml` and `mix-vox-lang.yaml` so `vox-lang` trains from ONE canonical mix reading `mens/data/verified_vox_codegen_v2.jsonl` (`optional: false`, `vox.exec_verified` / `vox.compile_verified`).
   - Update `mix-rust.yaml` to drop `vox_rust_review` (since review routes to base) or keep `vox_rust_authoring` strict-only.
   - Compare `harness-tools` mono (`mix-harness.yaml`) against decomposed `tool-selection` + `argument-generation` using `eval_spoke::run_agent_suite` (`agent_episodes`, `n=72`), keeping whichever wins rather than guessing.
3. **Stamp truth into run artifacts:** Have `run_mens_training` write `spoke_id`, `backend`, `base_preset`, `data_slice_histogram`, and `verification_histogram` into `training_manifest.json` and `train-record.json`, and have `crates/vox-populi/src/mens/hub_card/facts.rs` read `spoke_id` directly instead of `hub-cards.yaml:13`'s hardcoded `default_spoke: vox-lang`.
4. **Update `domain-profiles.yaml` base pointers:** Point `vox-lang` / `small_code_default` / `agentic_default` to `qwen3_5_code` + `MiMo-V2.6-Distill-Qwen-9B` / `FrogNano-4B` / `Qwen3.8-27B` **after** `CAP-27B-04` and spoke gates pass (preserving current `spoke_base_resolver` gate discipline).

---

## 5. Measuring & Accelerating Time-to-Train + Eval (Flywheel Design)

### Verified Measurement Blind Spot Today

Searching `crates/vox-plugin-mens-candle-metal/src/inference.rs` (`lines 900–988`), `crates/vox-ml-cli/src/commands/mens/eval_local.rs` (`lines 512–542`), `reasoning_eval.rs`, `reasoning_compare.rs`, and `eval_spoke.rs` confirmed **zero eval timing instrumentation**:
- `inference.rs:909-913` returns ONLY `{"generated_text", "prompt_tokens"}` — **not** `completion_tokens`, prefill duration, decode duration, tokens/sec, stop reason, or KV prefix cache hits.
- `eval_local.rs` records `passed_at_1`, `pass_rate_at_1`, `construct_richness_mean`, and `checks`, with **no** `eval_wall_secs`, `model_load_secs`, per-item latency, or cache-hit breakdown (`eval-reasoning-compare` ran `>10h 16m` on `27B` blind).
- **Hidden Prefix-Cache Reset Bug:** `eval_local.rs:221` sorts prompts lexicographically and prints `"Prompt order: lexicographic by full prompt (prefix-cache friendly)"`, **but `inference.rs:945` calls `model.empty_cache()` at the start of every single `generate()` call**, recomputing the shared `~1.1k`-token Vox system prompt on every item across `166` eval tasks!

### Required Timing Schema Additions (Preserve Checkpoint Keys)

Do **not** touch `item_checkpoint.rs` cache-key inputs (`checkpoint_inputs`, `reasoning_inputs`) so all existing `.ckpt.jsonl` caches remain valid. Add additive fields only:

1. **In `crates/vox-plugin-mens-candle-{metal,cuda}/src/inference.rs` JSON response:**
   - `completion_tokens` (`usize`)
   - `prefill_ms` (`u64`), `decode_ms` (`u64`), `total_ms` (`u64`)
   - `prefill_tok_s` (`f64`), `decode_tok_s` (`f64`)
   - `stop_reason` (`"eos"` | `"max_tokens"`)
   - `kv_prefix_cache_hit_tokens` (`usize`)
2. **In `eval_local.rs`, `reasoning_eval.rs`, `reasoning_compare.rs`, and `eval_spoke.rs` report headers + `contracts/reports/`:**
   - `model_load_ms`, `generation_wall_secs`, `verify_wall_secs`, `eval_wall_secs`
   - `cached_items`, `generated_items` (so cached resumes never distort tok/s or wall comparisons!)
   - `latency_ms_p50`, `latency_ms_p95`, `mean_decode_tok_s`, `truncation_rate` (`stop_reason == "max_tokens"`)
   - **Combined Flywheel Metrics:** Read `train_wall_secs` + `peak_metal_allocated_bytes` from `run_summary.json` / `train-record.json` and emit:
     - `cycle_wall_secs = train_wall_secs + eval_wall_secs`
     - `eval_share_of_cycle = eval_wall_secs / cycle_wall_secs`
     - `pass_gain_per_gpu_hour = delta_pass_at_1_pp / ((train_wall_secs + eval_wall_secs) / 3600.0)`

### Three High-ROI Eval + Training Speedups

1. **True System-Prompt Prefix KV + DeltaNet State Caching (`inference.rs`):** Keep one cached prefix snapshot keyed by `token_ids[..lcp_len]` for the static system prompt across `eval_local` / `reasoning_eval` calls instead of resetting `model.empty_cache()` per item. On `heldout_bench_v2` (`166` items * `~1.1k` shared system tokens), this skips `~180K` redundant prefill token passes per run!
2. **Activate Group-Sequential Collateral Mode for Fast Inner Loops (`eval-gates-ladder.yaml:33-35`):** `eval-gates-ladder.yaml` *already* specifies `sequential.looks_fraction: [0.25, 0.5, 0.75, 1.0]`, `look_alpha: [0.0005, 0.002, 0.005, 0.04795]`, and `prefix_n: 32` (`lines 33–47`), which `reasoning_compare` / `eval_collateral_damage` can expose as `--sequential` for fast early accept/reject (`32–64` items first -> `~4x` faster screening) while keeping full `128 x 3` `fixed_n` for release Hub promotion. Also track `elapsed` per slice and expose a `--max-tokens` override audit.
3. **3-Tier Flywheel Pyramid:**
   - **Tier 0 (`<15s`, CPU):** `cargo test` corpus leakage (`assert_no_text_leakage`), static compile/anti-stub checks, and failure-funnel regression diff.
   - **Tier 1 (`<=30m`, Fast Canary):** `Qwen/Qwen3.5-2B` (replacing `0.8B` once validated) on `heldout_bench_v2_canary` (`64` tasks) + `agent_episodes` (`n=72`, <2m once wired to model inference) with `--sequential` collateral look 1 (`32` items). Target total Tier-1 cycle: **`<45 minutes`**.
   - **Tier 2 (Release Promotion):** `Qwen/Qwen3.8-27B` (or target spoke `MiMo-9B`) full `heldout_bench_v2` (`166` tasks) + `n=5` pass@k / repair@2 + full `128 x 3` `fixed_n` collateral McNemar gate.

---

## 6. What We Aren't Tracking Yet (Missing Telemetry & KPIs)

Per `docs/src/reference/mens-measurement-gap-analysis.md` plus Phase `550` empirical findings:

| Category | Missing Metric(s) | Where to Capture | Why It Matters |
| :--- | :--- | :--- | :--- |
| **Eval Failure Funnel** | `no_code_rate`, `parse_fail_rate`, `typecheck_fail_rate`, `anti_stub_fail_rate`, `step_or_string_limit_rate`, `assertion_fail_rate`, `pass_exec_rate` | `eval_local.rs` aggregate summary | Shows *where* completions die (`matrix_row_sums` died on truncated `if` at `512` tokens vs `is_sorted_unique` on wrong logic). |
| **Decode Failure Modes** | `truncation_rate` (`hit_max_tokens`), `unclosed_think_rate`, `mean_completion_tokens`, `prompt_tokens_mean` | `inference.rs` + `eval_local.rs` | Detects whether failures need more tokens (`max_tokens=1024`), stop-token fixes, or syntax training rather than blind rank bumps. |
| **Repair-Loop Economics** | `pass@1`, `pass@1+repair1`, `pass@1+repair2`, `mean_repairs_to_pass`, `time_to_first_valid_vox_ms`, `tokens_to_valid_vox` | New `eval_local --repair-turns 2` feeding compiler diagnostics back | Real IDE / agent workflows run `vox check` and repair in 1–2 turns; measures actual developer latency + token cost. |
| **Training Health & True Data** | Uncontaminated `avg_val_loss_last_epoch` (`null` in all `v94` runs!), non-null `truncated_rows` & `prompt_trimmed_rows` (both `null` in `train-record.json`!), `grad_norm_pre_clip`, `clip_fraction`, `lora_delta_frob_ratio`, `supervised_ratio` per `data_slice` | `train-record.json`, `telemetry.jsonl`, `training_skip_stats.json` | Would have caught the `0.8B r64 alpha=16` gradient explosion at step 10 instead of after a 15m eval! Also proves `--seq-len 512` wasn't clipping long answers. |
| **Cross-Slice Collateral Matrix** | Per-category deltas (`humaneval-vox` vs `adt_match` vs `endpoint_handler` vs `state_machine`) + prose/reasoning drift per training slice mix | `contracts/reports/` + `eval_gate` | Verifies synthetic Python-like algorithmic rows help `humaneval-vox` without hurting `@table`/`query`/`mutation`/`workflow` idioms. |
| **Harness & Skill-Use Spokes** | XML `<tool_call>` vs JSON parse compliance, tool-selection accuracy vs argument-schema validity breakdown, multi-turn recovery after tool error, `SKILL.md` discovery trigger F1 (`vox_skill_search` / tier-2 use), `cost_per_success_step` | `eval_spoke.rs` + `mens/data/agent_episodes` | `eval_spoke::run_agent_suite` currently only has deterministic `never_call`/`oracle` unit tests (`eval_spoke.rs:159-203`) — no live MENS model inference wired yet! |
| **Flywheel & Budget Efficiency** | `cycle_wall_secs`, `delta_pass_at_1_per_gpu_hour`, `usd_per_verified_row`, `usd_per_pp_gain` | `mens_scorecard` + teacher distill reports | Prevents spending GPU hours or API USD on low-yield ablations. |

---

## 7. Careful `50 USD` Data-Imputation Budget Plan (Gated, Optional)

In Phase `550` (`CAP-27B-02`), `deepseek/deepseek-chat` (`MIT`) yielded **`98` execution-verified rows for `0.1001 USD`** (`~0.00102 USD/verified row`, `98/180` = `54.4%` net yield after 1 compiler repair retry). At that cost efficiency, **`50 USD` is large enough to fund over `40,000` raw attempts** if left uncurated — so the right discipline is to **spend slowly, only on measured failure buckets, and leave unused budget unspent if local self-play suffices**.

### Non-Negotiable Rules Before Spending Any Dollar

1. **Zero spend before funnel diagnosis:** Measure baseline failure funnels (`CAP-27B-04` `27B` + Tier-1 `2B` canary) on `heldout_bench_v2` first so every teacher prompt targets concrete zero-pass categories (currently on `0.8B` matched canary: `endpoint_handler 0/4`, `state_machine 0/4`, `matrix_grid 0/6`, `data_structures 0/7`, `string_parsing 0/6`).
2. **MIT / Apache-2.0 teachers only & single facade:** Route strictly via `vox_actor_runtime::llm` (`llm_provider_call` audit clean), enforce `max_budget_usd` hard caps per run, and log exact token/USD ledgers under `contracts/reports/`.
3. **100% Verifier + Anti-Stub + Dual Leakage Gate:** Every emitted row must pass `verify_completion_with_harness` (real assertions, not compile-only) and `assert_no_text_leakage` against `heldout_bench`, `heldout_bench_v2`, and any new `v3` blind eval.
4. **Prevent Synthetic Collapse (`Mix Ceiling`):** Tag `origin`, `teacher_model`, and `data_slice`; cap distilled synthetic share with mandatory human `.vox` golden anchors (`examples/golden/` + fixed workspace `.vox`) and reasoning replay.
5. **Save Both Successes and Repair Trajectories:** Every failed first attempt that compiles/passes after `vox check` error feedback becomes a high-value `vox.repair_trace` pair alongside the clean `vox.exec_verified` pair!

### Staged Allocation (`50 USD` Ceiling — Optional Progression)

| Stage | Target Cap | Gate to Unlock | Data Imputed & Verifier | Expected Output |
| :--- | :---: | :--- | :--- | :--- |
| **Stage 1: Zero-Cost Local Harvest** | **`0 USD`** | Immediate | Fix the `49` compile-failing workspace `.vox` files (`22/51` in `scripts/` per `mens-multispoke-corpus-and-tuning-research-2026.md:23`), generate cohesive multi-construct `.vox` files (`type`+`@table`+`query`/`mutation`+`@test`), and mine local `vox check` repair logs. | `+400–800` authentic human `.vox` & repair pairs at `0 USD`. |
| **Stage 2: Weak-Category Execution RFT + Repair Traces** | **`5–12 USD`** | Tier-1 failure funnel shows `<20%` on `endpoint_handler`, `state_machine`, `matrix_grid`, `data_structures`, or `string_parsing` | Targeted seeds via `deepseek/deepseek-chat` / `DeepSeek-V4.1-Flash` / `GLM-5.3-Flash` (`MIT`) with up to 2 `vox check` + interpreter repair turns; save passing solutions (`vox.exec_verified`) AND failed->fixed diffs (`vox.repair_trace`). | `~2,500–5,000` verified full-stack & algorithmic `.vox` + repair rows (`~10–25%` budget). |
| **Stage 3: Hard-Failure Preference / DPO Pairs** | **`5–10 USD`** | Post-`CAP-27B-04` `27B` `heldout_bench_v2` exposes persistent `27B` semantic bugs (`pass_compile=true, pass_exec=false`) | Sample `N` completions from our own `27B`/`9B` model (`0 USD` local) + teacher critique/repair (`5–10 USD`) for contrastive `(chosen, rejected)` DPO pairs on subtle semantics, `is` vs `==`, `@test` edge cases. | `~1,000–2,000` high-leverage diagnostic DPO / RFT pairs (`~20–45%` cumulative budget). |
| **Stage 4: Harness Tool, Arg & `SKILL.md` Episode Imputation** | **`5–10 USD`** | When activating `harness-tools` / `harness-skills` spokes with live `eval_spoke` inference wired | Multi-turn tool selection, XML `<tool_call>` schema recovery on bad args, and `SKILL.md` selection episodes scored by `vox_eval::episode::run_episode` + JSON-Schema validation (`eval_spoke.rs`). | `~1,500–3,000` validated agentic harness episodes (`~30–65%` cumulative budget). |
| **Stage 5: Unallocated Reserve** | **`18–35 USD`** | Keep **unspent** unless measurable Tier-2 (`heldout_bench_v2` + `agent_episodes`) deltas justify scaling | Reserve for higher-cost teacher synthesis or `v3` blind eval authoring only when Stage 2–4 measurably move `delta_pass_at_1_per_gpu_hour`. | Preserves budget discipline; target **total spend <= 20 USD** unless ROI is proven. |

---

## 8. Recommended Claude Execution Order

1. Inspect `ps -p 34157` (`eval-reasoning-compare`); once finished, save its report, and review/commit the pending Phase `547–550` + `gsd remove` changes (`551`, `552` removed).
2. Add inference + eval timing (`inference.rs`, `eval_local.rs`, `reasoning_eval.rs`) + prefix KV cache reuse + failure-funnel summary **first**, then verify zero checkpoint cache invalidation.
3. Run a fast `30m` `Qwen/Qwen3.8-27B` `seq=2048` memory/stability preflight (and evaluate promoting `Qwen/Qwen3.5-2B` as the Tier-1 canary); if clean, run `CAP-27B-04` (`Qwen/Qwen3.8-27B` full run) and score Before/After on `heldout_bench_v2` (`166` tasks).
4. Execute the 3-axis training-lane cleanup (`backend` / `spoke` / `data_slice`), wire `verified_vox_codegen_v2.jsonl` into the single canonical `vox-lang` mix, and run **Stage 1 (`0 USD`)** + targeted **Stage 2 (`5–12 USD`)** distillation on whatever categories still fail in `heldout_bench_v2`'s failure funnel.
5. Wire live model inference into `eval_spoke::run_agent_suite`, add `MiMo-V2.6-Distill-Qwen-9B` / `FrogNano-4B` / `Qwen3Guard` spokes + `Qwen3-Embedding-0.6B` retrieval, and only revisit `Granite-4.2 aLoRA` or sidecar MoE after the dense flywheel stabilizes.
