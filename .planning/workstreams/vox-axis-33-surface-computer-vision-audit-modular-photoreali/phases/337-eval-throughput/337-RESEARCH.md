# Phase 337: Eval throughput - Research

**Researched:** 2026-11-06 (read-only: no model process started, no GPU use, no cargo build; a 0.6B `eval-reasoning-compare` was running on Metal throughout)
**Domain:** Candle/Metal batched decode throughput, decode-protocol identity, sequential exact McNemar
**Confidence:** MEDIUM on the diagnosis and on the statistics (exact, enumerated); LOW on absolute s/item projections (no clean per-step measurement exists above 0.6B; the plan's first checkpoint produces one)

<user_constraints>
## User Constraints (from 337-CONTEXT.md)

### Locked Decisions
- **D-01 [USER]:** Batched decode is done (7ccb4adaa: per-sequence batch=1 prefill, shared decode steps, per-row attention over unpadded caches, byte-identical to batch=1, `VOX_REASONING_EVAL_BATCH_SIZE`, default derived from free memory). 337 does not redo it.
- **D-02 [USER]:** Before writing a new kernel, diagnose the sublinear scaling (0.6B Metal, 512 cap: 26.60 / 16.28 / 14.20 / 10.75 s/item at batch 1/4/8/16) across: per-row attention loops in `vox-plugin-mens-candle-metal/src/model.rs` `attend`, per-step host sync, sampler, cache concatenation, and anything else measured. The research decides whether a fused/batched attention kernel is the next win.
- **D-05 [USER]:** bf16/quantized decode changes scores, so it runs under its own protocol tag with a fresh base and adapter baseline, and is never compared with f32 results.
- **D-08 [USER]:** Base generations are already cached per snapshot (31c692401). 337 only fixes the remaining waste research finds (keys that ignored decode settings, `--force-restart` discarding the base cache) and does not rebuild the cache.
- **D-10 [USER]:** Sequential testing keeps the false-fail rate at or below alpha (alpha-spending or an exact sequential design) and is pre-registered in `mens/config/eval-gates-ladder.yaml`.
- **D-14 [USER]:** Phase 335 is actively editing `crates/vox-eval`, `crates/vox-ml-cli/src/commands/mens/eval_*` and `reasoning_eval.rs`. Plans that touch those files carry an `ls` precondition on 335's SUMMARY and re-read the files at execution time.
- **D-15 [USER]:** Phase 338 is planned and depends on 337. 337's outputs (decode settings, protocol tag, checkpoint key, collateral report fields) must stay compatible with 338's plans.
- **D-16 [USER]:** AGENTS.md rules: test-first, mutation proofs for guards, no new crate edges without a proposed exception, no hardcoded counts (batch sizes and look schedules derive from memory or config), per-crate `cargo check`/`cargo test` only.
- **D-17 [USER]:** Every run a plan proposes states its expected eval wall-clock. Any real-model benchmark or eval is a non-autonomous checkpoint (`autonomous: false`). No plan starts a model process while another eval is running on Metal.
- **D-18 [USER]:** Success evidence: measured per-item wall-clock for 0.6B-14B before and after, and a 4B collateral run at 128 items/slice that fits overnight.

### Claude's Discretion (and the [ASSUMED] defaults the research may refine)
- Exact timing instrumentation format, plan granularity and wave layout.
- Where the sequential-boundary computation lives, provided it reuses the existing exact McNemar code and adds no crate edge.
- D-03 [ASSUMED] decode changes keep batch=1 byte-identity under f32 (else new tag). D-04 [ASSUMED] cheapest instrumented win first; custom kernel only as its own plan if attention stays dominant. D-06 [ASSUMED] f32 stays default, bf16 opt-in, quantized deferred. D-07 [ASSUMED] tag hashed into the checkpoint key, recorded in the report, gate hard-errors on mixed tags. D-09 [ASSUMED] `--force-restart` invalidates adapter side only by default. D-11 [ASSUMED] few pre-registered looks (e.g. 32/64/96/128), exact alpha-spending, deterministic curtailment. D-12 [ASSUMED] pinned slice order. D-13 [ASSUMED] fixed-n 128 test stays available; report records the rule. D-19 [ASSUMED] "overnight" = full 4B collateral gate (base + adapter, all slices) <= 10 h. D-20 [ASSUMED] "before" numbers from existing records; only sizes with no usable record get a short benchmark.

### Deferred Ideas (OUT OF SCOPE)
- Quantized (int8/q4) decode, unless research shows bf16 alone does not fit (D-06).
- Making bf16 the default ladder protocol (orphans every f32 baseline and receipt; raise with the user first).
- Spoke-aware gating, receipts, dataset repos (338), cards (339), selector (340), debt sweep (341).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| THRU-01 | Instrument the decode step and run a step-time decomposition benchmark (fixed part F, per-row part A, per size and batch width), recording the "before" per-item wall-clock for 0.6B-14B | §Sublinear-scaling diagnosis, §Timing evidence |
| THRU-02 | Byte-identical attention fast path: preallocated KV cache (`slice_set`), GQA attention without `repeat_kv` and without the K^T copy | §Diagnosis (traffic model), §Architecture Pattern 1 |
| THRU-03 | Row-independent pre/post-attention ops (q/k norm, RoPE, gate) batched across rows, bit-identical | §Architecture Pattern 2 |
| THRU-04 | Inference linear without the always-zero LoRA branch, bit-identical to the current NF4-dequantized `QuantizedLinear::forward` | §Diagnosis H3, Pattern 3 |
| THRU-05 | Batch width derived from memory with the exact preallocated-KV size, plus a byte-identity proof at the widest width actually used, and a verified-width cap | §Batch invariance |
| THRU-06 | Collateral/compare load each model once per side and decode all pending slice items as one continuous stream (no per-slice reload or per-slice tail); stream cancel hook | §Code map, §Host-side waste |
| THRU-07 | `DecodeProtocol` type (`f32` default, `bf16`), env var, checkpoint-input and report `decode` object, hard refusal on mixed-tag pairing; default leaves every existing checkpoint key unchanged | §Protocol-tag design |
| THRU-08 | bf16 block-linear decode path in the plugin (LM head, norms, attention math stay f32), fresh base+adapter baseline under the tag, never compared with f32 | §bf16 on Metal |
| THRU-09 | Remaining base-reuse gaps: protocol in key, `--force-restart` scope split, base-never-discarded proven by mutation, base reuse test across two adapters | §Base reuse audit |
| THRU-10 | Pre-registered sequential exact-McNemar rule (looks, per-look alpha) in `eval-gates-ladder.yaml`, pure module in `vox-eval`, exact worst-case-over-all-paths type-I enumeration test, mutation proofs, load-time guard | §Sequential McNemar |
| THRU-11 | Early-stop wiring: stream cancel on a crossed boundary, report records the rule and the look it stopped at, offline re-judge from checkpoint | §Sequential McNemar, §Code map |
| THRU-12 | Measured before/after per-item wall-clock for 0.6B-14B and a 4B 128/slice collateral gate inside the D-19 budget (non-autonomous checkpoints) | §Projections, §Validation |
</phase_requirements>

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Test-first:** new `pub fn` in Rust needs a `#[test]` in the same file; the failing test comes first. `[CITED: AGENTS.md "Test-First Policy"]`
- **No new crate edges** without a proposed `exceptions` entry (never write one). `vox-eval` is layer 2 and already a dependency of `vox-plugin-mens-candle-metal` and `vox-ml-cli`, so the pure modules proposed here add no edge. `[VERIFIED: crates/vox-plugin-mens-candle-metal/Cargo.toml: vox-eval = { workspace = true }; docs/src/architecture/layers.toml:186 "vox-eval                = { layer = 2 }"]`
- **Crypto:** hashing goes through `vox-crypto` (`hash_json` in `item_checkpoint.rs` already does). No new crypto.
- **Env vars** are registered in `contracts/config/env-vars.v1.yaml`; generated reference docs are regenerated by CI (`ssot-autoregen`), never by hand.
- **No hardcoded CPU/job counts:** batch width and looks derive from memory/config; the YAML list of looks is pre-registration data, code defaults must not exist.
- **Local verification:** per-crate `cargo check` / `cargo test -p <crate>` only; real-model runs are non-autonomous checkpoints. Background every build/test (user memory "Never block on cargo"; "GSD lean verification": executors run `cargo check` + new tests only).
- **Glue scripts are `.vox`**, not `.py`/`.sh`. Research scratch scripts used here (python via `uv`) are not shipped; plan verification must be Rust tests.
- Rust: never `cargo fmt --all`; use `vox run scripts/fmt.vox` / `cargo fmt -p <crate>`.

## Summary

**The batched decode stalls at 2.47x because the per-row attention path is expensive in two independent ways, and both are cheaper to fix than a new kernel.** (1) Per row and layer, `attend` issues roughly 60-70 small device ops, so a B=16 step is dominated by host-side encode cost that scales with B. (2) Per row and layer it moves about 18x the KV bytes it needs (`Tensor::cat` of both caches every step, `repeat_kv` materialising n_rep copies, a `.contiguous()` copy of K^T, then the matmuls reading the inflated copies), and that traffic grows with context (up to 2300 tokens in thinking mode) and with B. The weights are read once per step and amortise perfectly; this per-row part does not. Calibrating on the four measured points gives roughly F ~ 50 ms fixed plus A ~ 19 ms per row per step at 0.6B / 500-token context, which reproduces 2.47x. Everything needed to remove most of A already exists in the vendored candle: `Tensor::slice_set` (preallocated cache), GQA by treating `n_rep` as the matmul M dimension (Metal gemm accepts the transposed K view directly, so no copies), and row-independent elementwise ops that can be batched without touching numerics. **Recommendation: do not write a fused/padded-mask kernel now.** Do the byte-identical fixes, measure, and only revisit padded+masked batching if the post-fix profile still shows per-row attention above ~40% of a B>=32 step.

Two further findings change the plan. First, the "f32" protocol is not what the name suggests: the non-quantised load path runs **NF4 quantise, dequantise-and-cache to f32, then every linear also runs a LoRA branch whose weights are zero** (adapter deltas are folded into the weight *before* NF4). The zero-LoRA branch costs ~4 extra dispatches and a host-to-device scalar upload per linear (~800 per step) and can be removed bit-identically. Second, the thinking-mode cost is a long tail: **39% of 759 recorded generations hit the 2048-token cap and carry 55% of all generated tokens** (math500 66-78% at 4B+). Only 27% of the capped generations are verbatim repetition loops (6% of all tokens), so a loop-stop rule would not pay for a new protocol tag. Throughput therefore comes from wider batches with a cheap per-row cost, not from shortening generations.

**The sequential McNemar test is not a throughput lever for adapters that pass.** Exact enumeration shows an efficacy-only design that holds the false-fail rate at or below 0.05 for every possible discordant-count path saves about 0 items under H0 and 4-9% at a 20% relative regression; it saves ~45-50% only for catastrophic regressions. Curtailment for the pass direction cannot fire at alpha = 0.05 (five unreplied losses already reject). A pre-registered *futility* stop would save 16-30% of items on the pass path at essentially no loss of per-slice power, but it interacts with the pooled test and is a user decision (off by default). It is still required by D-10, so it should be built, small and last, as a decode-stream cancel plus a pure module; but the plan must not count on it for the overnight budget.

**Primary recommendation:** Wave order = (1) instrument and measure F/A per size, (2) byte-identical attention/linear fixes proven on Metal, (3) after 335 lands: protocol tag + `decode` report object + keyed reuse fixes, one model load per side with a continuous stream, (4) bf16 opt-in path, (5) sequential rule, (6) measured before/after and the 4B overnight gate. Because 338-02 needs only the `decode` object and the sequential-rule YAML, schedule those plans first so 338 need not wait for the throughput work.

## Architectural Responsibility Map

| Capability | Primary tier | Secondary tier | Rationale |
|------------|-------------|----------------|-----------|
| KV cache, attention, linears, sampler, batch scheduler | Decode plugin (`vox-plugin-mens-candle-metal`) | - | numerics live here; byte-identity is judged here |
| Decode protocol identity (`f32`/`bf16`), decode-settings object | `vox-eval` (pure types + hashing input) | plugin reads the same type from env at load | one source for host header and plugin behaviour; no new edge |
| Per-item checkpoint, keys, reuse, `--force-restart` scope | Host CLI (`vox-ml-cli` `item_checkpoint.rs`, `reasoning_eval.rs`) | - | already owns generation identity |
| Model load/unload, continuous stream across slices, cancel | Host CLI `with_backend` / `decode_batched` | plugin batch session (`batch_cancel`) | load scope is a host decision |
| Sequential boundary, worst-case size proof, replay from counts | `vox-eval` (new pure module) | `eval_collateral.rs` wiring | reuses `mcnemar_one_sided_p`; testable without a GPU |
| Pre-registration (looks, per-look alpha, rule id) | `mens/config/eval-gates-ladder.yaml` | policy loader in `eval_gate/policy.rs` | D-10 |
| Timing evidence | Ignored hardware tests (existing `batch_bench.rs`, `decode_dtype_bench.rs` pattern) | planning docs | measurement-only, `#[ignore]`d |

## Standard Stack

No new external packages. Everything is already in the lockfile.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| candle-core / candle-nn | 0.10.2 | tensors, Metal ops, `Tensor::slice_set`, `KvCache` | already the engine `[VERIFIED: Cargo.lock "candle-core" version = "0.10.2"; ~/.cargo/registry candle-core-0.10.2/src/tensor_cat.rs:246 "pub fn slice_set"]` |
| candle-metal-kernels (vendored patch) | 0.10.2 | MLX-style steel GEMM, tile selection | `[VERIFIED: patches/candle-metal-kernels-0.10.2/src/kernels/mlx_gemm.rs:56-176]` |
| qlora-rs (vendored patch) | 1.0.5 | NF4 quantise/dequantise, `QuantizedLinear`, `ComputeDType` | `[VERIFIED: patches/qlora-rs-1.0.5/src/qlora.rs:276-333, 417-496]` |
| vox-crypto | workspace | BLAKE3 (`hash_json`) | repo crypto policy |

### Supporting
| Library | Purpose | When to Use |
|---------|---------|-------------|
| `vox-eval::reasoning_slice::mcnemar_one_sided_p` | exact one-sided p, log-space | boundary for every look; do not reimplement `[VERIFIED: reasoning_slice.rs:80-97]` |
| serde_yaml-based policy loader (`eval_gate::load_policy`) | reads `collateral_damage.*` | add the `sequential:` block there |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Byte-identical cheap fixes | Padded+masked batched attention | one launch per layer, but changes softmax/P.V reduction lengths; identity unproven, would need its own tag if it differs (the 7ccb4adaa comment already records this risk) |
| candle fused ops (`softmax_last_dim`, fused `rms_norm`, `rope`) | decomposed ops kept | fused kernels reduce dispatches further but change last-bit numerics; bundle them into a future `kernels` tag, not into f32 |
| Hand-written Metal kernel | none | not justified by the cost model (see below) |

**Installation:** none. **Package Legitimacy Audit:** no external package is added; `slice_set`, `KvCache`, `softmax_last_dim` are in the already-vendored candle 0.10.2. Research scratch used `numpy`, `tokenizers`, `blake3` via `uv run --with` for analysis only; none is shipped. Packages removed: none. Flagged: none.

## Current-state code map (all `[VERIFIED]` by reading this session)

Repo root `/Users/brbrainerd/dev/vox-mens-compare`, branch `gsd/mens-compare-316`.

### Decode loop (plugin) - `crates/vox-plugin-mens-candle-metal/src/`
| Item | Location | What it does |
|------|----------|--------------|
| Single-sequence loop `InferenceEngine::generate` | `inference.rs:718-800` | prefill once, then `forward_cached` per token; **`logits.to_vec1::<f32>()` at :787** reads the whole 151,936-float row, host `sample_next_token` |
| Request entry `generate_from_json` | `inference.rs:689-716` | routes `{"batch":..}`, `{"batch_next":true}`, else single |
| Sampler `sample_next_token` | `inference.rs:1063-` | greedy = host argmax (first max wins on ties), else top-k/top-p |
| Batch session (`BatchSession`, `Running`) | `inference/batch.rs:28-56` | continuous batching: finished sequences leave, next prompt prefilled into the slot |
| `batch_start` | `inference/batch.rs:61` | width = `batch_size` or `fit_width(free_bytes/2 / (kv_bytes_per_token*(longest)))` |
| `batch_next` | `inference/batch.rs:106` | returns finished `{index,response}`; calls `admit` / `step` |
| `kv_bytes_per_token` | `inference/batch.rs:146` | full-attention layers only, `2*n_kv_heads*head_dim*elem` |
| `admit` (serial batch=1 prefill) | `inference/batch.rs:163` | `forward_cached` over the prompt, `to_vec1` of last logits at :189 |
| `step` | `inference/batch.rs:230` | `forward_decode_rows` then **`.to_vec2::<f32>()` at :239** ([B, 151936] readback), per-row `take_token` |
| `take_token` | `inference/batch.rs:216` | sample, stop-token check, **`tokenizer.decode(&[t])` per token**, `max_tokens` check |
| `forward_decode_rows` | `model.rs:942-975` | embed, per layer `Qwen35Layer::forward_rows` (:734), `head_forward` |
| `Qwen2Attention::forward_rows` | `model.rs:193-228` | one batched `project`, then **a per-row loop calling `attend`**, `Tensor::cat(&ys,0)`, `o_proj` |
| `Qwen2Attention::attend` | `model.rs:279-371` | q/k norm (`rms_norm_f32`), RoPE (`apply_rotary_emb` :373), **KV `Tensor::cat` at :320-321**, `repeat_kv` :344-345, decode branch :359-360 `(q.contiguous()?.matmul(&k.transpose(2,3)?.contiguous()?)? * scale)` then `candle_nn::ops::softmax` then `.matmul(&v.contiguous()?)` |
| `repeat_kv` | `model.rs:118-126` | `unsqueeze(2).expand(..).reshape(..)`: the reshape of an expanded view copies |
| `rms_norm_f32` | `model.rs:109-116` | `norm.forward_diff(x)`: decomposed (non-fused) RMSNorm |
| `QuantizedLinear::forward` | `model.rs:71-86` | `QLora` arm calls `qlora_rs::QuantizedLinear::forward` |
| qlora `forward` | `patches/qlora-rs-1.0.5/src/qlora.rs:417-496` | base matmul on cached weight, then **always** `linear_no_bias_t(input,&a)`, `linear_no_bias_t(.,&b)`, `Tensor::new(scaling)`, `broadcast_mul`, `broadcast_add` (LoRA branch) |
| Load: NF4 for non-quantised checkpoints | `inference.rs:297-299`, `:430-436` | `qlora_cfg.quantization.compute_dtype = compute_dtype_for_device(&_device); qlora_cfg.cache_dequantized = true;` and `QuantizedLinear::from_weight(unquantized_weight, None, &qlora_cfg, &_device)` |
| Folded adapter delta | `inference.rs:369-375`, `fold_lora_delta` | delta added to the base weight **before** `from_weight` (so NF4 sees W+dW) |
| `compute_dtype_for_device` | `crates/vox-plugin-mens-candle-core/src/inference_device.rs:32-38` | `if device.is_cuda() { qlora_rs::ComputeDType::BF16 } else { qlora_rs::ComputeDType::F32 }` |
| Existing measurement harnesses | `inference/batch_bench.rs` (`batched_decode_wall_clock_on_a_real_model`), `decode_dtype_bench.rs` (`decode_linears_by_weight_dtype`) | both `#[ignore]`, need Metal; the second times linears only (f32/bf16/f16/q8_0/q4k) and **has no recorded result anywhere in `.planning`** |

### Host orchestration - `crates/vox-ml-cli/src/commands/mens/`
| Item | Location |
|------|----------|
| `decode_batched` (one batch per *bench*, records each finished item) | `reasoning_eval.rs:295-352` |
| `score_slice_checkpointed` (per bench; `VOX_REASONING_EVAL_BATCH_SIZE` via `parse_batch_size` :355) | `reasoning_eval.rs:248-282` |
| `prompt_request` = `{"prompt","max_tokens","temperature":0.0,"seed":0}` | `reasoning_eval.rs:370-378` |
| `with_backend` (lazy load, **unload at return**) | `reasoning_eval.rs:199-243` |
| `score_base_file`, `score_reasoning_file`, `parse_max_tokens` (default 2048) | `reasoning_eval.rs:121`, `:39`, `:154` |
| Compare: model loaded **once per side for all slices** (`score_side_with_backend`) | `reasoning_compare.rs:310-341` |
| Collateral: `score_bench` -> `cached_reasoning_report` per bench key, so `with_backend` runs **per slice per side** (3 slices x 2 sides = up to 6 loads) | `eval_collateral.rs:575-629` |
| Base is passed `force_restart = false`; only the adapter side honours `--force-restart` | `eval_collateral.rs:628-629` |
| Fail-fast order, `run_collateral_damage_fail_fast_with` | `eval_collateral.rs:196-345` |
| Alpha read from policy | `eval_collateral.rs:60-77` |
| Checkpoint: `reasoning_checkpoint`, `generations_for_other_item_sets`, `reasoning_checkpoint_path`, `model_identity`, `hash_json` | `item_checkpoint.rs:35`, `:70`, `:113`, `:148`, `:139` |
| `ItemCheckpoint::{open,infer,has,record}` | `item_checkpoint.rs` (`infer` :283, `has` :302, `record` :309) |

### Statistics - `crates/vox-eval/src/`
`mcnemar_one_sided_p` (`reasoning_slice.rs:80`), `mcnemar_power` (:112), `PairedMetric`/`paired_metric` (:205/:280), `verify_reasoning_slices` (:477). Quoted constants: `[VERIFIED: reasoning_slice.rs:22]` `pub const PAIRED_RULE: &str = "exact one-sided McNemar on paired items, fail when p < alpha";` `[VERIFIED: qwen_reasoning.rs:153]` `pub const GRADER_ID: &str = "grade_final_answer v2";` `[VERIFIED: reasoning_bench.rs:15]` `pub const SLICE_SIZE: usize = 128;`

### Checkpoint identity (verbatim)
`[VERIFIED: item_checkpoint.rs:23]` `pub(crate) const ITEM_CHECKPOINT_SCHEMA: &str = "vox_eval_item_checkpoint.v1";` `[VERIFIED: item_checkpoint.rs:28]` `pub(crate) const REASONING_CHECKPOINT_ROOT: &str = "mens/runs/reasoning_compare/checkpoints";` `[VERIFIED: item_checkpoint.rs:31-32]` `const REASONING_SCORER: &str = "reasoning_eval::score_loaded thinking_mode_prompt grade_final_answer v1";` The key is `hash_json` of `{scorer, model, bench, max_tokens, temperature, seed, items_blake3}` (:35-60). Note `GRADER_ID` says v2 while `REASONING_SCORER` says v1: the header string names the prompt builder and is inert for grading (the raw text is stored, graded on read). **Do not "fix" the string: it would orphan every cached generation.**

## Sublinear-scaling diagnosis

### Evidence available
Measured, 0.6B (snapshot c1899de2) Metal f32 greedy, first 16 gsm8k_think items, cap 512, ~7.9k generated tokens, host load average 18-44 from other agents' builds `[VERIFIED: 335-RESEARCH.md §7 table]`:

| batch | wall s | s/item | aggregate tok/s | derived ms per step (wall / (7.9k/B)) |
|---|---|---|---|---|
| 1 | 425.7 | 26.60 | 18.6 | 54 |
| 4 | 260.4 | 16.28 | 30.3 | 132 |
| 8 | 227.2 | 14.20 | 34.8 | 230 |
| 16 | 172.0 | 10.75 | 45.9 | 348 |

Least squares on T(B) = F + A*B over these four points: **F ~ 51 ms, A ~ 19 ms per row-step** (residuals within +-25 ms; the curve is mildly concave, so the per-row cost falls from ~26 to ~15 ms as B grows). The asymptote 54/22 = 2.5x is the measured ceiling: with A unchanged, no batch width can beat F-amortisation. The ceiling is set by A, not by F. (Load caveat: dispatch encoding is host-CPU-bound, so a loaded host inflates A more than F; the true quiet-host split may differ. This is why THRU-01 comes first.)

### Per-row cost model (from the code, `[ASSUMED]` constants for the arithmetic)
Per row and layer in `attend` (seq_len = 1), counting device ops from the code at model.rs:279-371:
- q norm + k norm: `rms_norm_f32` = decomposed RMSNorm, ~6 ops each = 12
- RoPE: `arange`, cast, `matmul`, `cat`, `cos`, `sin` (6) + for each of q,k: `broadcast_mul`, `rotate_half` (2 contiguous copies, `neg`, `cat`), `broadcast_mul`, add (7) = 14 -> ~20
- KV `Tensor::cat` (K and V) = 2; `repeat_kv` copies = 2; `k.transpose.contiguous` = 1; QK matmul = 1; scale = 1; softmax = 5 (`max`, `sub`, `exp`, `sum`, `div`, composed in `candle_nn::ops::softmax`, `ops.rs:22-29`); PV matmul = 1; output transpose/contiguous = 1; optional gate silu/mul = 2
- Total ~55-70 dispatches per row-layer, i.e. ~1.5-2k per row-step at 28 layers. At ~5-8 us encode each (host-bound) that is 8-16 ms per row-step, on the order of the measured A.

KV traffic per row-step (bytes, C = one layer's K tensor = ctx x n_kv_heads x head_dim x 4 B; V is the same). Reading the code: `cat` reads+writes 2C each (4C), `repeat_kv` reads 2C and writes 2C*n_rep, the K^T `.contiguous()` reads+writes the repeated K (2*C*n_rep), the matmuls read the repeated K and V (2*C*n_rep). For n_rep = 2 (0.6B/1.7B: 16 q heads, 8 kv) that is 4C + 6C + 4C + 4C = **18C versus the 2C actually needed**; for n_rep = 4 (4B/8B: 32/8) 30C; n_rep = 5 (14B: 40/8) 36C `[Qwen3 head counts: ASSUMED from config, verify in plan]`. At 0.6B, ctx 1000: C = 4 MB, 28 layers x 18C = 2 GB per row-step (~10 ms at an effective 200 GB/s); at the thinking-mode context of ~2300 it is ~4.7 GB (~24 ms). At 4B/8B/14B the per-row traffic at ctx 2300 is ~10-13 GB, comparable to the whole weight read (16/33/59 GB) *per row*, so at B=16-32 the KV traffic exceeds the weight traffic and widening the batch stops helping. So the sublinear scaling has two physical causes (dispatch count, KV copy traffic), and the second gets worse with context and with model size. `[ASSUMED: effective bandwidth; constants are order-of-magnitude]`

### Hypotheses ranked, with a test for each (all measured in THRU-01, none requires new kernels)
| # | Hypothesis | Verdict from code | Expected share of A | How to measure |
|---|---|---|---|---|
| H1 | per-row op count (dispatch/encode bound) | confirmed by code reading | 40-60% | count ops per `forward_rows`; time with `device.synchronize()` between phases at B=1,8,16,32 |
| H2 | KV copy traffic (cat + repeat + K^T copy) | confirmed by code reading | 30-50% at ctx 1000+, grows with ctx | time the same step at ctx 256 vs 2048 at fixed B |
| H3 | always-on zero-LoRA branch in every linear (197 linears, ~4 dispatches + a scalar upload each) | confirmed (`qlora.rs:468-496`; adapters are folded before NF4 so the branch is zero) | 15-30% of **F**, not A | time a step with LoRA branch removed (bit-compare logits) |
| H4 | candle Metal allocator: `find_available_buffer` scans every size bucket per op, `drop_unused_buffers` runs per command-buffer flush (every 50 ops) | read `candle-core-0.10.2/src/metal_backend/device.rs:179-200, 326-338` | unknown, scales with live buffer count (~B x layers x 2 KV buffers) | op-count x pool-size counter in the bench |
| H5 | host sync per step: `to_vec2` of [B,151936] f32 (9.7 MB at B=16, 19 MB at B=32) plus host argmax over B x 152k floats | small: ~2-5 ms/step = 1-3% | measure `to_vec2` + argmax separately |
| H6 | tokenizer decode per token | negligible (< 1 ms/step at B=32) | skip |
| H7 | serial batch=1 prefill on admission (all active rows idle) | 0.6B ~0.3-1 s per prompt vs 512+ steps; 14B ~1-2 s per prompt x ~770 prompts ~ 20 min of ~8 h | measure in bench |
| H8 | batch tail (batch runs to the longest row) | **mostly not applicable**: batching is continuous (`batch.rs:106-141`); only the end of each *bench* has a tail. Simulation on the recorded length distribution (4B): per-slice batches cost +1-3%, 12 barrier chunks +3-14% | simulate (done) |

H1+H2 are the dominant, removable terms; H3 inflates F; H5/H6/H7/H8 are secondary (<= ~5% each).

### Decision: fused/padded batched attention vs cheaper fixes
**Cheaper fixes first, no new kernel.** After the three byte-identical fixes the per-row core is `slice_set` K and V, QK matmul, scale, softmax (5 ops), PV matmul = ~11 dispatches (from ~60-70) and ~2C KV traffic (from 18-36C): a ~5-9x cut of A, hence A' ~ A/6 as the central estimate (range A/3 to A/10). A padded+masked batched attention would merge the remaining per-row core into one launch per layer but (a) changes reduction lengths (softmax over ctx_max with -inf pads, P.V over ctx_max), so identity with batch=1 is unproven and the existing design note in `model.rs:187-192` explicitly rejects it, and (b) is only needed if A' still dominates. **Trigger for revisiting:** post-fix profile shows per-row attention > 40% of a B >= 32 step. If that happens it is its own plan, byte-identity decides f32-vs-new-tag, and a custom Metal kernel stays out of scope.

### Byte-identity: why the fixes are safe, and where the risk is (read from the GEMM source)
`[VERIFIED: patches/candle-metal-kernels-0.10.2/src/kernels/mlx_gemm.rs:56-76]` `if m < 16 { return TILE_32_32_16_2_2; }`. Every decode matmul at M < 16 uses the 32x32x16 tile (BK = 16), so the K-loop accumulation order is the same whether the operand is batch-1, has `n_rep` rows (GQA as M), or a transposed K view (`b_trans` is a supported stride layout, `call_mlx_gemm` lines ~282-310 accept any `(lhs_m1,lhs_m2)` that is row-major or transposed; a narrowed cache view has the right strides). For M >= 16 and F32 the Max/Medium branch picks `TILE_32_64_16_1_2` (nt, small) or `TILE_64_64_16_2_2` (large) `[VERIFIED: mlx_gemm.rs:143-157]`, both BK = 16, which is consistent with the measured identity at B = 16. **Caveats the plan must test, not assume:** (i) the identity argument depends on the device being classified `Max`/`Medium` (`metal/device.rs:152-160`, last character of the architecture name; M5 Max classification `[ASSUMED]`; on a `BasePro` device nt f32 would use BK = 32); (ii) the argument is about f32 only: bf16 at M >= 16 picks `TILE_64_32_32_2_2` (BK = 32) for nt, so **bf16 is not batch-width invariant by construction** and the bf16 protocol must either pin the width or prove invariance empirically; (iii) the claim is per kernel, so each rewritten op needs a bitwise test.

## Recommended fixes (all plugin-side, all must pass a bitwise test against the current path)

### Pattern 1: preallocated KV cache + GQA attention without copies (THRU-02)
**What:** Replace `Option<(Tensor,Tensor)>` cat-per-step with a per-layer preallocated `[1, n_kv, cap, hd]` buffer pair (cap = prompt tokens + `max_tokens`, known at admit), append with `Tensor::slice_set(&k, 2, pos)`, attend over `narrow(2,0,len)` views. In decode, reshape q `[1,n_heads,1,hd]` to `[1,n_kv,n_rep,hd]` (head index = kv*n_rep + r, the order `repeat_kv`'s expand produces), `matmul` against the transposed K view `[1,n_kv,hd,len]` (no `.contiguous()`), scale, **keep the existing 5-op softmax**, matmul against the V view, reshape back. The prefill path keeps `chunked_causal_attention` and writes the prompt K/V into the buffer once.
**Why:** removes `cat` (4C), `repeat_kv` (2C + 2C*n_rep), the K^T copy and n_rep-fold matmul reads; KV traffic 18-36C -> 2C. `fit_width` becomes exact (cap is the full per-sequence size it already assumes).
**Constraints verified in candle:** `slice_set` requires both tensors contiguous and distinct storage (`tensor_cat.rs:246-253`); `candle_nn::kv_cache::KvCache`/`Cache::append` does exactly this (`kv_cache.rs:58-79`) but grows by `cat` when exceeding capacity, so use fixed capacity and refuse overflow rather than regrow.
**Anti-pattern:** do not switch to `softmax_last_dim` or fused `rms_norm`/`rope` in this plan: fused reductions change last-bit results. They are a candidate for a separate `kernels` protocol tag.

### Pattern 2: batch the row-independent ops across rows (THRU-03)
q-norm, k-norm, RoPE and the output gate are elementwise or reduce only over `head_dim`, so they can run once on the `[B,...]` tensors with per-row `cos/sin` built from a positions tensor (`[B,1] x [1,half]` matmul with K = 1 is a single exact product). Only cache write, QK matmul, scale, softmax and PV stay per row. Expected per-row op count ~11 from ~60. Bit-identical because every element sees the same arithmetic; prove by comparing `attend_reference` (kept `#[cfg(test)]`) with the new path bitwise.

### Pattern 3: inference linear without the zero LoRA branch (THRU-04)
Add a dense variant that performs the **same op sequence as qlora's base path** (reshape `[b*s,in]`, `matmul(&w.t())` with the *non-contiguous transposed view* so the same `nt` kernel runs, reshape back) and skips the LoRA arm. Do **not** reuse `QuantizedLinear::Unquantized`: it pre-transposes and makes the weight contiguous (`model.rs:60-69`), which selects the `nn` kernel and is not guaranteed bit-identical. Obtain the dequantised weight from the same NF4 path (`dequantize_nf4_with_dtype`, or add a one-line `pub fn base_weight()` accessor to the vendored qlora patch and record it in `VOX_PATCH.md`). Adding `+0.0` can only flip the sign of a zero, which no downstream op here observes; the test is a bitwise compare of logits modulo signed zero plus a greedy-token identity run on a fixed slice.

### Pattern 4: host orchestration (THRU-06)
One `with_backend` per side for all pending slice items, submitted as one continuous stream (the cross-bench batch removes the three per-bench tails, ~1-3% of makespan, and, more importantly, the 6 model loads of the collateral path become 2). Add `{"batch_cancel":true}` to the batch protocol so the host can stop consuming (needed for THRU-11). Sizing: `fit_width` already uses half the free memory; with exact KV allocation it can be raised to the full free budget minus a stated headroom, still derived at run time from `sysinfo` (no constant).

## Base reuse audit (D-08, D-09) - what is left to gain after 31c692401

Verified by reading `item_checkpoint.rs` and `eval_collateral.rs`:

| Question | Finding |
|---|---|
| Is the base keyed by snapshot only, so adapters share it? | Yes: `model_identity` returns `{"snapshot": canonical path}` for a non-adapter dir and the directory is `hash_json(&model)[..16]` (`item_checkpoint.rs:35-60, 148-165`); the HF cache path carries the revision. Base and adapter of `eval-reasoning-compare` and `eval-collateral-damage` share files. |
| Do decode settings alter the file? | Yes, since 335: the file is `<bench>-<key16>.jsonl`, a legacy `<bench>.jsonl` is reused only if its header key matches, and a grown slice seeds from sibling files with the same settings (`generations_for_other_item_sets`). |
| Does collateral `--force-restart` discard the base? | No: base is always passed `false` (`eval_collateral.rs:628`). Remaining gap: `eval-reasoning-compare --force-restart` applies `force_restart` to **both** sides (`reasoning_compare.rs:421-423`) and there is no flag to discard only the base. |
| What is missing from the key for 337? | The decode protocol (f32 vs bf16) and nothing else that varies today: `max_tokens`, `temperature`, `seed` are in the inputs; `top_k`/`top_p` are not sent, so they are the plugin's defaults and are not keyed (irrelevant under greedy but a latent hole if sampling is ever used for evals). |
| Waste that remains | (a) up to 6 model loads per collateral run instead of 2, (b) a decode tail per bench, (c) 32 of 128 items per slice already cached for 4B/8B/14B bases and adapters (prefix reuse works; no gain left), (d) no engine identity in the header: a numerics-changing plugin change would silently reuse stale text under an unchanged key. Mitigation for (d) is process, not code: any non-byte-identical decode change must use a new protocol tag (D-03). |
| Reuse gain from changing base handling | Essentially none beyond (a)/(b): once a size's base is complete, a second adapter costs only its own side. The protocol tag simply namespaces a second base. |

So THRU-09 is small: protocol in the key (only when non-default), split `--force-restart` scope, a two-adapter base-reuse test, and a mutation proof that removing the `false` at `eval_collateral.rs:628` makes a test fail.

## Protocol-tag design (compatible with 338)

338-02 hashes "the report's decode-settings object" into `inputs.decode_blake3` and its text lists "(max new tokens, temperature, thinking on or off, batch size)" `[CITED: 338-02-PLAN.md decode-settings paragraph]`. **337 must hand 338 one canonical `decode` object and 338-02 must drop "batch size" from it**: batch width is not a decode input (identity is proven per the f32 analysis above); including it would make receipts differ between runs that produced identical text.

```text
vox_eval::decode_protocol (new, pure, in vox-eval; no new edge)
  enum DecodeProtocol { F32, Bf16 }          // tags: "f32" (default), "bf16"
  fn from_env() -> Result<DecodeProtocol>    // VOX_MENS_DECODE_PROTOCOL; unset = F32; unknown value = hard error
  struct DecodeSettings { protocol, max_tokens, temperature, seed, thinking: bool, scorer: &str, batch_invariant: bool }
  fn to_checkpoint_inputs() -> serde_json::Map   // adds "decode_protocol": "<tag>" ONLY when protocol != F32
  fn to_report_object() -> serde_json::Value     // always includes "protocol"; this is what 338 hashes (decode_blake3)
```
- **Default must not change any existing key.** Adding a `decode_protocol` field to the checkpoint `inputs` for f32 would change `hash_json(inputs)` and orphan every cached generation (including the 0.6B 128-item base). So f32 omits the field; bf16 adds it. `generations_for_other_item_sets` already compares settings objects for equality, so bf16 and f32 files coexist and never seed each other.
- **Definitions (put in the YAML comment and docs):** `f32` = NF4-quantised weights, dequantised once and cached as f32, f32 matmuls (today's behaviour). `bf16` = same NF4 weights cached as bf16 for the **transformer-block linears only**; embedding, norms, attention math, KV cache, **LM head and logits stay f32** (a bf16 head would collapse near-tied logits and change greedy ties). The tag names a *set of numerics*, so a later `kernels` change (fused softmax/rms-norm/rope) or a "skip the NF4 round trip" change gets its own tag value, never a silent redefinition.
- **Plugin side:** `InferenceEngine::load` reads the same env var through the same `DecodeProtocol::from_env`; ABI of `MlBackend::load_model(path)` is unchanged. The plugin prints the applied protocol in its existing `load ...: <what> at <s>s` phase lines for audit. A shared function in one crate makes host header and plugin behaviour unable to disagree inside one process.
- **Pairing refusal (D-07):** the collateral gate reads the protocol from both sides' checkpoint headers (or from its own process value) and returns a hard `Err` (not a fail verdict) if base and adapter differ, or if the baseline file's recorded protocol differs from the run's. The report gains `decode: <to_report_object>` and `rule`/`stopped_at_look` fields (below).
- **Env registration:** `VOX_MENS_DECODE_PROTOCOL` in `contracts/config/env-vars.v1.yaml` (owner crate `vox-eval`), mirroring the entry style at lines 3206-3217.
- **338 interplay:** receipts must hash `to_report_object()`; `policy_blake3` includes the whole parsed YAML, so adding the `sequential:` block changes it. Land it in 337 (before 338-02), not after, otherwise every 338-era receipt is stale on arrival.

## bf16 on Metal (D-05/D-06): what exists and what it buys

- **Support exists in the stack.** Metal GEMM has a BF16 kernel family (`GemmDType::BF16`, tile branches at `mlx_gemm.rs:158-171`), qlora's `ComputeDType::{F32,F16,BF16}` already drives the cached-weight dtype and casts input to the weight dtype then back (`qlora.rs:419-460`), and `decode_dtype_bench.rs` was written to time bf16/f16/q8_0/q4k decode linears on Metal. Today `compute_dtype_for_device` returns F32 off CUDA with a comment that the Metal trainer dequantises to F32 and serving "must match" `[VERIFIED: inference.rs:1408-1414 test comment]`. **No result of `decode_linears_by_weight_dtype` is recorded anywhere** (phase 335 task 7 never ran it), so per-size bf16 speed is unmeasured.
- **Mechanism for the protocol:** add `compute_dtype_for_protocol(device, protocol)` next to the shared function (do not change the shared default; the CUDA plugin and trainer depend on it) and keep the LM head on an f32 path. Casting each linear's input to bf16 and back adds ~2 dispatches per linear (~400 per step), so **bf16 will not speed up 0.6B/1.7B**; it should help only where weight bytes dominate.
- **Memory:** block-linear weights halve. f32 14B is ~59-61 GB `[VERIFIED: CONTEXT + 335 context; my estimate 14.0B x 4 B + head ~ 59 GB]`; bf16 ~30 GB for the blocks, plus the f32 embedding/head (~3 GB at 14B) and the retained packed NF4 tensors. KV cache stays f32 (activations are f32).
- **Expected speedup (LOW confidence, to be replaced by the decode_dtype_bench numbers):** the weight-read share of a batch-1 step is ~70% at 14B, ~60% at 8B, ~40% at 4B, ~30% at 1.7B, ~25% at 0.6B (derived from the batch-1 records below). Halving it gives ~1.3x at 14B, ~1.25x at 8B, ~1.15x at 4B, ~1.05x at 1.7B, about 1.0x or slightly worse at 0.6B at batch 1; at B >= 32 the weight read is amortised and the gain shrinks by roughly half.
- **Fit analysis (D-06):** 4B fits the 10 h budget on f32 after the fixes (projection below) and 14B is borderline (7-11 h) on f32 at wide batches. bf16 is therefore *useful but not required*; quantised decode stays deferred. Recommendation: build bf16 as an opt-in tag in a late wave, gated on the decode_dtype_bench numbers showing >= 1.2x end to end at 8B/14B.
- **Fresh baseline cost under the tag:** a full base+adapter 128/slice run at the target size under `bf16` (new directory, nothing reused from f32): 0.6B ~1.5-2 h, 4B ~3.5-5 h after the fixes. Never compared with f32 verdicts; the gate refuses mixed pairs.

## Length distribution and cap-hit rates (read-only measurement)

Method: 759 generations in `mens/runs/reasoning_compare/checkpoints/*/*.jsonl` (cap 2048; plus 37 at cap 4096), `generated_text` tokenised with the Qwen3 tokenizer (`tokenizers` 0.23 via `uv`, 0.6B snapshot's `tokenizer.json`; Qwen3 sizes share the tokenizer). A generation counts as capped when its re-encoded length is >= 2036 tokens (12-token slack for re-encoding drift). Mapped adapters to sizes by BLAKE3 of `candle_qlora_adapter.safetensors` against `mens/runs/qwen3-*`. Scratch scripts: `scratchpad/lens2.py`, `loops.py` (not shipped).

| size | bench | n | mean tok | p50 | p75 | p90 | capped (>= 2036) |
|---|---|---|---|---|---|---|---|
| 0.6B | gsm8k_think | 160 | 1135 | 827 | 2048 | 2048 | 29% |
| 0.6B | math500_think | 44 | 1480 | 2048 | 2048 | 2048 | 50% |
| 0.6B | vox_think | 32 | 914 | 640 | 2048 | 2048 | 25% |
| 1.7B | gsm8k_think | 32 | 1533 | 1655 | 2048 | 2048 | 34% |
| 1.7B | math500_think | 32 | 1783 | 2048 | 2048 | 2048 | 66% |
| 1.7B | vox_think | 32 | 1292 | 1408 | 2029 | 2048 | 22% |
| 4B | gsm8k_think | 64 | 1590 | 1674 | 2048 | 2048 | 36% |
| 4B | math500_think | 64 | 1903 | 2048 | 2048 | 2048 | 73% |
| 4B | vox_think | 64 | 1393 | 1584 | 2048 | 2048 | 36% |
| 8B | gsm8k_think | 64 | 1502 | 1548 | 1980 | 2048 | 22% |
| 8B | math500_think | 64 | 1877 | 2048 | 2048 | 2048 | 67% |
| 8B | vox_think | 64 | 1287 | 1206 | 2005 | 2048 | 23% |
| 14B | gsm8k_think | 32 | 1467 | 1473 | 1941 | 2048 | 22% |
| 14B | math500_think | 11 | 1862 | 2046 | 2048 | 2048 | 55% |
| **all (cap 2048)** | | 759 | **1452** | 1584 | | | **39% (294/759)** |

- Capped generations hold **55% of all generated tokens**. At cap 4096, 0.6B gsm8k still hits the cap in 14/32 (44%): much of the tail is non-terminating reasoning, not a too-small cap.
- Repetition-loop check (200-char tail occurring earlier in the text): 78 of 294 capped (27%); truncating those at the loop onset would save **6% of all tokens**. Not worth a protocol tag. Lowering the cap or any early-stop heuristic changes scores and is out of scope (a new tag, user decision).
- Consequence for throughput: with continuous batching the makespan is set by total tokens, so a long tail costs little *if* rows are refilled; it costs a lot only for bench-boundary tails (+1-3%) and for any design with synchronisation barriers (+3-14%, see sequential section).

## Timing evidence ("before" s/item, D-20)

There is no timing field in any report JSON (`grep` for secs/elapsed/duration finds none). Timing comes from checkpoint file birth/mtime, which is the only wall-clock record: a bench's file is created when its header is written and last modified at its last item. Pauses, resumes and contention are included, so these are **upper-ish bounds**; the host was shared (load average 18-44 at last check, 31 at 01:06 MST today). Hardware: Apple M5 Max, 128 GB `[VERIFIED: sysctl hw.memsize=137438953472, machdep.cpu.brand_string "Apple M5 Max"]`.

Best-quality chain points (consecutive benches where the next file's birth equals the previous file's mtime, taking the 2nd/3rd bench to avoid load/pauses hidden in the first), batch 1, cap 2048, f32, tokens counted from the stored text:

| size | side/bench | items | s/item | tok/s | quality note |
|---|---|---|---|---|---|
| 0.6B | adapter math500 / vox | 32 / 32 | 144 / 75 | 11.9 / 12.1 | clean |
| 1.7B | adapter math500 / vox | 32 / 32 | 187 / 136 | 9.5 / 9.5 | clean |
| 4B | base math500 | 32 | 530 | 3.7 | single clean chain point; other 4B files 700-3700 s/item (resumed/contended) |
| 8B | base gsm8k / math / vox | 32 each | 550 / 718 / 526 | 2.8 / 2.7 / 2.8 | three consistent points |
| 14B | base gsm8k / math500 | 32 / 11 | 716 / 1021 | 2.05 / 1.8 | math500 file ends at 11 items (killed) |

Per-token step time at batch 1 follows: ~83 ms (0.6B), ~105 ms (1.7B), ~270 ms (4B), ~360 ms (8B), ~530 ms (14B), including attention over contexts that grow to ~2300. These are 3-5x what a bandwidth-bound step would be (effective ~35-170 GB/s of f32 weight traffic against a chip in the ~0.5 TB/s class `[ASSUMED]`), which fits the dispatch + non-optimal M = 1 GEMM story and is further evidence the decode is not memory-bound. The only batched number remains 0.6B (335 table above); **no batched measurement exists for 1.7B-14B**.

## Projections (LOW confidence: model on measured splits that must be re-measured)

Model: T(B) = F + A*B ms per step; per-item seconds = mean_tokens x T(B) / B x 1.05 (tail/barrier factor from the simulation). Calibration: 0.6B F=48, A=22 at the thinking-mode context; larger models split T1 = F + A with A scaled by layers and GQA repeat factor (A: 1.7B 22, 4B 43, 8B 43, 14B 57 ms; F = T1 - A). After the fixes: A' = A/6 (conservative A/3), F' = 0.85-0.97 F (LoRA branch share), optional bf16 removes ~35% of the weight-read share of F at >= 4B. Batch width 32 (derived width at 4B-14B is higher; 32 is also inside the f32 verified-identity regime to be proven).

| size | mean tokens | BEFORE batch=1 s/item (measured) | BEFORE batched (default width) | AFTER f32, B=32 expected [conservative] | AFTER bf16 (expected) | confidence |
|---|---|---|---|---|---|---|
| 0.6B | 1170 | 75-144 | ~25-40 (2.47x applied to batch-1; measured only at cap 512) | **6 [10.5]** | ~6 | LOW-MED (best calibrated size) |
| 1.7B | 1536 | 136-187 | unmeasured (model: 40-70) | **10 [16]** | ~9.5 | LOW |
| 4B | 1630 | ~530 (single clean) | unmeasured (model: ~35-60 at B=16-32) | **24 [36]** | ~21 | LOW |
| 8B | 1555 | 526-718 | unmeasured | **27 [39]** | ~23 | LOW |
| 14B | 1665 | 716-1021 | unmeasured | **42 [58]** | ~35 | LOW |

**4B 128/slice collateral gate (D-19, <= 10 h):** new generations needed = (3 slices x 96 new items) x 2 sides = 576 (the first 32 per slice, both sides, are cached prefix; the 4B base and adapter files exist at cap 2048 with the header inputs the prefix reuse requires), ~0.94 M tokens. Wall-clock at the AFTER rates: **~3.8 h expected, ~5.8 h conservative** (+ up to 2 model loads, unmeasured; today the collateral path would load up to 6 times). With no kernel changes at all (only wider batch), the same model gives ~13 h, i.e. the cheap fixes are what buy the margin. Pessimistic bound if A' ~ A/2: ~7.5 h. For scale, batch-1 would be ~70 h. **14B** gate (~700 new gens): ~8 h expected f32, ~11 h conservative, ~7 h bf16; do not promise 14B in 337, state it as "within reach, measured in THRU-12". All figures replaced by THRU-01/THRU-12 measurements.

## Sequential McNemar (D-10..D-13)

### Statistical model
Per slice, items i = 1..128 in the pinned order, each a loss (base right, adapter wrong; prob p_l), a gain (p_g) or concordant. Fixed-n test (current, `[VERIFIED: reasoning_slice.rs:80-97]`): fail iff `P(Bin(b+c, 1/2) >= b) < alpha`. At each look the statistic is the cumulative `(b_k, c_k)` over the first `n_k` items.

**The discordant count is random, so size must be proven path-wise.** Under the H0 boundary (p_l = p_g) the *directions* of the discordant items form an i.i.d. Bernoulli(1/2) sequence that is independent of *which* items are discordant. Therefore, conditioned on the discordant-count path `(d_1 <= d_2 <= ... <= d_K)` (d_k = b_k + c_k, increments bounded by the item increments), the loss counts at the looks are the partial sums of a fair random walk observed at times `d_k`. A rule whose conditional rejection probability is <= alpha **for every feasible path** has unconditional size <= alpha for every discordance rate, including any dependence of discordance on item difficulty, as long as direction is independent of discordance (the exact assumption the fixed-n test already makes). The proof obligation is therefore a finite enumeration: all `prod_k (n_k - n_{k-1} + 1)` paths (33^4 = 1,185,921 for looks 32/64/96/128).

### Rule (efficacy boundary, exact, per-look nominal levels)
Reject at look k iff `mcnemar_one_sided_p(b_k, c_k) < alpha_k`. `alpha_k` are pre-registered nominal levels (a fixed list in the YAML), not data-dependent. Boundary at look k: `crit_k(d) = min{ b : P(Bin(d,1/2) >= b) < alpha_k }`. The unused slack of the discrete exact test (worst-case conditional size of the fixed-n test is 0.0494 at alpha 0.05, `scratchpad/seq.py`) lets the interim levels be tiny while the final level stays near nominal.

### Recommended pre-registration (looks at 25/50/75/100% of `SLICE_SIZE` = 32/64/96/128)
`alpha_k = [0.0005, 0.002, 0.005, 0.04795]`. Exact worst case over all 1,185,921 paths: **0.04884 <= 0.05** (found by bisection on the final level with the interim levels fixed; script `scratchpad/seq.py`/`seq5.py`). Rejection boundary (min losses to fail by number of gains 0..8):

| look | alpha_k | gains = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|---|
| fixed-n (today) | 0.05 | 5 | 7 | 9 | 10 | 12 | 13 | 15 | 16 | 18 |
| 32 | 0.0005 | 11 | 14 | 17 | 19 | 21 | 23 | 25 | 27 | 29 |
| 64 | 0.002 | 9 | 12 | 15 | 17 | 19 | 21 | 22 | 24 | 26 |
| 96 | 0.005 | 8 | 11 | 13 | 15 | 17 | 19 | 20 | 22 | 24 |
| 128 | 0.04795 | 5 | 7 | 9 | 10 | 12 | 14 | 15 | 16 | 18 |

Other exact designs found (worst-case size over all paths): equal-level Pocock `0.01734` x4 -> 0.04937; K=2 `[0.005, 0.04795]` (looks 64/128) -> 0.04941; K=2 `[0.01, 0.04421]` -> 0.04972; K=4 `[0.001, 0.003, 0.01, 0.04476]` -> 0.04955; K=4 `[0.002, 0.005, 0.015, 0.04071]` -> 0.04897. The recommended list keeps the final test within 0.003 of the fixed-n level, so power is essentially unchanged; early looks catch only gross damage (11 losses and no gains by item 32).

### Verification method (what THRU-10's tests must do)
1. **DP for one path:** state = distribution of losses among discordants so far; at look k convolve with `Bin(e_k, 1/2)` (e_k = discordant increment), remove and sum the mass with `b >= crit_k(d_k)`.
2. **Exhaustive worst case:** depth-first over increments `e_k in [0, n_k - n_{k-1}]`, carrying the surviving vector; the last look is evaluated in closed form with the binomial tail table (`sum_S surv[S] * P(Bin(e_K) >= crit_K - S)`), so cost is ~1.2 M x O(d) ~ 10^8 flops (numpy took ~7 s; Rust release should be seconds; debug `[ASSUMED]` up to about a minute: measure in the plan, and keep the exhaustive test in the default `cargo test -p vox-eval --lib` only if debug time is acceptable, else `#[ignore]` plus a CI nextest slow profile).
3. **Independent cross-check of the DP:** brute-force every 2^n direction sequence x every discordance mask for a tiny schedule (e.g. looks 4/8/12 over n = 12) and require equality with the DP to 1e-12.
4. **Load-time guard:** the loader recomputes `worst_case_size(rule)` and refuses a YAML whose rule exceeds `alpha` (so an edited, invalid schedule cannot gate anything). Looks are derived as `ceil(fraction * SLICE_SIZE)` from a `looks_fraction` list; no counts in code.
5. **Mutation proofs (AGENTS.md):** (a) replace an interim level by `alpha` -> worst-case test must fail; (b) flip `<` to `<=` in the boundary; (c) drop the final look; (d) use `d_k` of the wrong look in `crit`; (e) report `stopped_at_look` off by one. Each with a before/after presence check.
6. **Unconditional table** (exact `(b,c)` DP over items, `scratchpad/seq4.py`) for the report/docs.

### Expected items saved and power (exact DP, base accuracy 0.5, pi_d = discordant rate; relative drop d)
Recommended rule vs fixed-n, P(fail) | E[items of 128]:

| pi_d | 0% (size) | 5% | 10% | 20% | 50% |
|---|---|---|---|---|---|
| 0.1 | .027 / 128 (fixed .027) | .147 / 127 (.151) | .454 / 125 (.460) | .997 / 98 (.997) | - |
| 0.2 | .032 / 128 (.034) | .110 / 128 (.116) | .278 / 126 (.288) | .775 / 117 (.783) | - |
| 0.3 | .034 / 128 (.037) | .094 / 128 (.101) | .212 / 127 (.224) | .602 / 121 (.618) | 1.0 / 66 |

- Power loss vs fixed-n is <= 0.012 everywhere; false-fail is lower, not higher.
- **Under H0 the expected saving is ~0 items; at a 20% regression 4-9% (pi_d 0.2-0.1: 117-98 items); only a ~50% regression saves ~48%.** Equal-level Pocock saves more on regressions (E = 101 at 20%/0.2) but costs power (.647 vs .775); not recommended.
- **Deterministic curtailment for the pass direction cannot fire before the last few items:** failing needs only 5 losses with 0 gains, so "even b + remaining losses cannot reject" is false until almost all items are in. D-11's curtailment therefore saves nothing at alpha 0.05 and should be recorded as analysed-and-rejected.
- **Futility (early pass) option, off by default:** stop for pass at look k < K when the conditional probability of a final rejection under a stated alternative (20% relative drop at pi_d 0.2, i.e. p_l 0.15, p_g 0.05) is < gamma. Exact results for the K=4 rule: gamma 0.05 gives P(early pass) 0.58-0.77 under H0 and E[items] 107-90 (16-30% saved) with per-slice power change <= 0.001 (e.g. .277 vs .278 at 10% / pi_d .2). Caveat that makes it a user decision: stopping a slice early shrinks the pooled test (n 384 -> as low as 192-320) and data-dependent truncation means the pooled exact binomial level is **not proven**; if enabled, the pooled test needs its own enumeration or must be defined on the generated items with a stated bound. Savings apply to the adapter side only when the base side is cached (see below), i.e. 8-15% of a first-time gate.
- **Where savings can apply at all:** the base side must be complete before pairing, so early stop saves *adapter-side* generation only (about half of a first-time gate, all of the cost of a re-gate). Fail-fast across slices already stops a failing size after the first failing slice.

### Wiring constraints that decide cost
- Looks must not become barriers. Simulation on the recorded 4B length distribution (continuous batching, F=227, A=43 ms): one stream 10.1 h per 384 generations at B=16 now; 12 barrier chunks of 32 cost +14% (B=16), +9% (B=32) and +30% (B=48, wider than the chunk) after the fixes. So deliver early stop as a **stream cancel**: the host keeps consuming `batch_next`, evaluates look k as soon as the first `n_k` items are all recorded (items finish out of order, so that is when the slowest of them finishes, ~the end of the first wave), and on a crossed boundary sends `{"batch_cancel":true}`. Waste on stop <= B in-flight sequences. The pass path then costs exactly the same as fixed-n.
- The report records `rule` (an id string distinct from `PAIRED_RULE`), the `(n_k, losses, gains, p, alpha_k)` of every look evaluated, and `stopped_at_look`/`items_used`; the stored-report re-verifier replays the looks from those counts (the checkpoint also allows a zero-GPU re-judge of any earlier run by prefix, which is how a 335-era fixed-n pass can be re-judged under the sequential rule for free).
- Think-initiation metrics use the same looks (d is usually ~0). Pooled test: fixed-n, computed only when all three slices completed (no slice stopped), identical to today.

## Architecture Patterns

### System architecture (data flow of one gated size)
```text
mens eval-collateral-damage --policy ladder.yaml
   |-- read YAML: alpha, slices pins, rule {fixed_n | sequential(looks,alpha_k)}, protocol env
   |-- DecodeSettings(protocol,max_tokens,..) -> checkpoint inputs (+ report.decode)
   v
 for side in [base (cached after 1st adapter), adapter]:
   with_backend(model_dir)  ---- ONE load per side, unload at end -------------+
      pending = all uncached items of all slices, pinned order, tagged by bench |
      plugin batch session: {"batch":[...], "free_bytes"|"batch_size"}          |
        admit: serial prefill -> preallocated KV                                 |
        step: batched project/norm/rope/mlp ; per-row {slice_set,QK,softmax,PV}  |
              ; head ; to_vec2 ; sampler ; tokenizer                             |
        batch_next -> {index,response}  --> ck.record (fsync, per-bench file)    |
      host: after each record, if look k's prefix complete -> boundary check ----+--> crossed? send batch_cancel, stop
   v
 pair per item by id -> PairedMetric per slice/metric (fixed-n or sequential replay) -> pooled -> report
```

### Recommended structure (additive)
```text
crates/vox-eval/src/decode_protocol.rs       # NEW pure: DecodeProtocol, DecodeSettings
crates/vox-eval/src/sequential_mcnemar.rs    # NEW pure: SequentialRule, replay, worst_case_size
crates/vox-plugin-mens-candle-metal/src/
  kv_cache.rs (new) or inside model.rs       # preallocated per-layer cache + GQA decode attention
  inference/batch.rs                         # batch_cancel, exact fit_width
crates/vox-ml-cli/src/commands/mens/         # item_checkpoint.rs, reasoning_eval.rs, eval_collateral.rs wiring (AFTER 335)
mens/config/eval-gates-ladder.yaml           # collateral_damage.sequential block
contracts/config/env-vars.v1.yaml            # VOX_MENS_DECODE_PROTOCOL
```
New modules go in new files because `reasoning_slice.rs` (983 LoC) and `model.rs` (2078 LoC) are near budget and 335 is editing the former.

### YAML pre-registration shape (proposal)
```yaml
collateral_damage:
  alpha: 0.05
  rule: fixed_n            # fixed_n | sequential_mcnemar_v1  (default stays fixed_n until the user flips it)
  sequential:
    looks_fraction: [0.25, 0.5, 0.75, 1.0]      # looks = ceil(fraction * SLICE_SIZE) on the pinned order
    look_alpha:     [0.0005, 0.002, 0.005, 0.04795]
    # worst-case type-I over every discordant-count path = 0.04884 <= alpha (enumerated; the loader re-checks)
    futility: null         # off; see 337-RESEARCH for the option and its pooled-test caveat
```

### Anti-patterns to avoid
- **Per-look barriers** (a chunked `score_slice` per look): +9-30% on the pass path with zero benefit there.
- **A new key field for f32** (orphans all caches).
- **Editing `REASONING_SCORER`** (orphans all caches).
- **Counting on futility or curtailment for the overnight budget.**
- **Using `Unquantized` for the dense inference linear** (different GEMM layout).
- **Re-deriving looks/widths from constants in code** (violates D-16).

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| Exact tails | new binomial code | `mcnemar_one_sided_p` (+ the tail table in the new module's tests only) | already log-space exact and mutation-proved |
| KV append | custom buffer writer | `Tensor::slice_set` (or `candle_nn::kv_cache::Cache` with fixed capacity) | tested in candle; contiguity rules known |
| GPU memory probe | new probe | `sysinfo` `available_memory()` already used in `decode_batched` | derived, no constant |
| Hashing | any new hash | `hash_json` / `vox-crypto` | crypto policy |
| Policy parsing | second YAML reader | `eval_gate::load_policy` + serde struct | one source of the policy |
| Timing spans | a profiler dependency | `std::time::Instant` + `Device::synchronize()` inside the `#[ignore]` bench only | production path must not gain sync points |

## Runtime State Inventory
Not a rename/refactor phase. One adjacent runtime-state fact: cached generations under `mens/runs/reasoning_compare/checkpoints/` are keyed by content hash; the plan must not alter `REASONING_SCORER`, the `inputs` key set, or `model_identity` for the default protocol, or ~20 files (hours of 4B-14B decode) are orphaned. None else (no stored data/service/OS state tied to a name).

## Common Pitfalls

### Pitfall 1: A "faster" change that is not byte-identical
**What goes wrong:** cached f32 text silently mixes with text from a different numeric path under one key.
**How to avoid:** every plugin change ships a bitwise test (synthetic Metal) and a real-model fixed-slice identity run at the widest width used; anything that fails is moved to a new tag (D-03).
**Warning signs:** a greedy token differs from batch=1 at a near-tie; `(a,b)` equal in CPU test but not on Metal.

### Pitfall 2: Trusting `VOX_REASONING_EVAL_BATCH_SIZE` docs ("token-identical at any size")
Identity holds for f32 because the f32 tiles at M >= 16 keep BK = 16; it is a property of `select_tile_config` and device class, not a law. bf16 breaks it (BK = 32 at M >= 16). Cap f32 width at the widest verified width; pin bf16 width in its protocol or prove invariance.

### Pitfall 3: Measurement under load
Dispatch-bound code is host-CPU-bound; the 335 numbers were taken at load average 18-44. Run THRU-01 on a quiet host and record `uptime` and swap in the artifact; compare before/after on the same load.

### Pitfall 4: Measuring while the GPU is busy
A 0.6B compare was running at research time. Every real-model step is `autonomous: false` with an explicit "no other Metal process" precondition (check `ps` for `vox-ml-cli`/`cargo run`).

### Pitfall 5: Sequential rule as a speed-up
It saves ~0 on passing runs. Do not put it on the critical path for the overnight claim.

### Pitfall 6: bf16 head
A bf16 LM head/logits makes near-tied logits exactly tied and changes greedy picks; keep head and logits f32.

### Pitfall 7: policy hash churn in 338
Any later edit of the `sequential:` block or `PAIRED_RULE` invalidates every 338 receipt. Pre-register once, before 338-02.

## Code Examples

```rust
// Source: candle-core-0.10.2 tensor_cat.rs:246 (slice_set); pattern for the preallocated cache.
// cache: [1, n_kv, cap, hd] contiguous, src: [1, n_kv, 1, hd] contiguous, distinct storage.
k_cache.slice_set(&k_new, 2, pos)?;                 // no cat, no realloc
let k = k_cache.narrow(2, 0, pos + 1)?;             // strided view; Metal gemm takes batch stride = cap*hd
// GQA: treat n_rep as the matmul M dimension; no repeat_kv, no K^T copy.
let q = q.reshape((1, n_kv, n_rep, hd))?;           // head = kv*n_rep + r, same order repeat_kv produced
let att = (q.matmul(&k.transpose(2, 3)?)? * scale)?;           // M = n_rep < 16 -> same 32x32x16 tile as M = 1
let att = candle_nn::ops::softmax(&att, candle_core::D::Minus1)?; // keep the 5-op softmax (numerics)
let y = att.matmul(&v)?.reshape((1, n_heads, 1, hd))?;
```

```rust
// Dense inference linear: same ops and layout as qlora's base path, minus the zero-LoRA branch.
// Source: patches/qlora-rs-1.0.5/src/qlora.rs:417-466 (base path), :468-496 (branch to drop).
let w_t = self.w.t()?;                              // keep the non-contiguous view: nt kernel, same as today
let (b, s, i) = x.dims3()?;
x.reshape((b * s, i))?.matmul(&w_t)?.reshape((b, s, w_t.dim(1)?))
```

```text
sequential replay, one path (see §Verification method): surv = [1.0]; for look k: surv = surv (*) Bin(e_k,1/2);
d += e_k; c = crit(d, alpha_k); rej += sum(surv[c..]); surv = surv[..c]
```

## State of the Art

| Old approach | Current approach | Impact |
|---|---|---|
| per-row `attend` with cat/repeat/contiguous | preallocated cache + GQA-as-M (standard in candle-transformers `KvCache`) | 5-9x less per-row cost |
| LoRA branch evaluated at inference after folding | fold once, dense linear | removes ~800 dispatches/step |
| fixed-n only | exact sequential with per-look alpha | cheap failure detection, no pass-path gain |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The per-row/fixed split F ~ 50, A ~ 19-22 ms and its scaling to 1.7B-14B (A by layers x GQA factor) | Diagnosis, Projections | s/item and the 4B gate hours are off by up to ~2x; THRU-01 replaces them |
| A2 | Op counts per row-layer (~55-70) and per-op encode cost 5-8 us | Diagnosis | dispatch share of A could be lower (traffic higher); the fixes still help both |
| A3 | Qwen3 head counts: 0.6B/1.7B 16 q / 8 kv, 4B/8B 32/8, 14B 40/8; layers 28/28/36/36/40 | Diagnosis, Projections | traffic factors shift; read each `config.json` in the plan |
| A4 | M5 Max classified as `Max`/`Medium` by `device_type()` so f32 nt tiles keep BK = 16 at M >= 16 | Byte-identity | wide-batch identity could fail; caught by the width-identity test |
| A5 | Effective memory bandwidth ~0.2-0.5 TB/s | Diagnosis | only affects the traffic-to-time conversion |
| A6 | bf16 speedups by size | bf16 | bf16 plan priority only; decode_dtype_bench decides |
| A7 | Metal gemm accepts the narrowed cache view and transposed K without a copy and gives identical bits | Pattern 1 | fallback keeps one `.contiguous()` on K^T (still removes cat + repeat); Metal bitwise test decides |
| A8 | Debug-build time of the 1.19 M-path enumeration | Sequential | may need `#[ignore]` + CI profile |
| A9 | Direction of discordant items independent of which items are discordant (the standard McNemar premise) | Sequential | same assumption as the existing fixed-n test |
| A10 | Load time per model (NF4 quantise on CPU) is minutes at 4B-14B | Projections | per-run overhead; THRU-01 records the existing `load ... at <s>s` lines |

## Open Questions

1. **Should sequential become the default rule, and at which registration?** (one-way for receipts)
   - Known: it costs nothing on a pass path (if wired as a stream cancel), loses <= 0.012 power, saves ~0 when the adapter is fine.
   - Unclear: whether the user wants early stop for catastrophic adapters at all, given it needs a plugin protocol addition.
   - Recommendation: build it behind `rule: fixed_n` default; flip only on the user's word. Registration of `look_alpha` is one-way once any size is gated under it or 338 receipts hash it.
2. **Futility (early pass):** enable? It changes the meaning of a pass and the pooled test's level is unproven. Recommendation: off; revisit only with a pooled enumeration.
3. **Decode `kernels` tag:** adopting fused softmax/rms-norm/rope (more speed) would be a new tag and a full re-baseline. Recommendation: not in 337.
4. **Observation outside 337's scope, worth the owner's attention (not verified as a bug):** adapters are evaluated as NF4(W + dW) (delta folded before quantisation, `inference.rs:369-375, 430-436`) but trained as NF4(W) + dW x. Quantisation noise relative to a small dW could distort or partially erase the delta; several recorded adapter-vs-base scores are identical (e.g. 4B gsm8k 0.625 vs 0.625). This affects what "f32" means and is not changed here; any fix is a new protocol tag.
   **Resolved 2026-11-06:** confirmed as a bug and fixed outside 337 (both plugins now serve NF4(W) plus the full-precision LoRA arm, proven equal to the training layer to <1e-5 while the old fold differed by >100x). Adapter checkpoint keys gained `adapter_serving: "nf4_base_plus_lora_arm v2"`, so pre-fix adapter generations are never reused. 337-06 is amended to keep the trained arm.
5. **338-02 text:** its `decode_blake3` description includes "batch size"; it should hash `DecodeSettings::to_report_object()` which excludes batch width and includes the protocol. Needs the orchestrator to amend 338-02 (or 337 summary to state it).
6. **Phase dependency:** 338-02/05/07 need only 337's decode object, protocol tag and YAML rule; they do not need the throughput work. Recommend ordering 337's protocol (THRU-07/09) and sequential-rule (THRU-10) plans first and letting 338 start after them while the throughput plans (GPU-bound checkpoints) proceed.
7. **Genuinely one-way decisions:** (a) protocol tag strings and "default omitted from inputs" (receipts hash it); (b) `look_alpha` and look fractions once used; (c) making bf16 the default ladder protocol (deferred); (d) changing `REASONING_SCORER`/key shape (orphans caches).
8. **Cap policy:** 39% of generations hit 2048 and are mostly graded wrong; a lower cap or loop-stop would save a lot but changes scores. Raise with the user separately; not in 337.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Metal GPU, Apple M5 Max, 128 GB | all benchmarks | yes, **busy** (0.6B `eval-reasoning-compare`, PID 28185, started 00:55) | - | schedule after it ends; CPU unit tests meanwhile |
| rust/cargo (via build broker) | builds, tests | yes | workspace pin | per-crate only |
| candle-core/-nn/-metal-kernels | decode | yes | 0.10.2 | - |
| HF snapshots Qwen3-0.6B/1.7B/4B/8B/14B | benchmarks | yes in `~/.cache/huggingface/hub` | - | - |
| uv + python | research scratch only | yes | - | not used by plans |
| Free memory for 14B f32 + B=32 KV (~84 GB) | 14B bench | borderline (128 GB total, swap used by other work) | - | smaller width; bf16 |

Missing with no fallback: none. Blocking until GPU free: all real-model steps.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (std harness; nextest in CI) |
| Config file | per-crate; `crates/vox-plugin-mens-candle-metal` Metal-gated tests need `--features metal` |
| Quick run command | `cargo test -p vox-eval --lib -- decode_protocol sequential_mcnemar` |
| Full suite command | `cargo test -p vox-eval --lib && cargo test -p vox-plugin-mens-candle-metal --lib && cargo test -p vox-ml-cli --features gpu --lib -- item_checkpoint reasoning_eval eval_collateral eval_gate` |

Background every build/test; per-crate only (AGENTS.md).

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| THRU-01 | step spans + decomposition bench produce F, A per B and size | ignored hardware test + CPU unit for the span plumbing | `cargo test --release -p vox-plugin-mens-candle-metal --features metal -- --ignored --nocapture decode_step_decomposition` | no (Wave 0; extend `batch_bench.rs`) |
| THRU-02 | new attention path bitwise equal to `attend_reference` (CPU synthetic, then Metal synthetic) | unit | `cargo test -p vox-plugin-mens-candle-metal --lib -- attention_fast_path` ; Metal: `... --features metal -- --ignored metal_attention_identity` | no (Wave 0) |
| THRU-02/03/04 | batched greedy decode token-identical to batch=1 after the changes | unit (existing synthetic engine) | `cargo test -p vox-plugin-mens-candle-metal --lib -- batch::tests` | yes (`inference/batch.rs` tests; extend) |
| THRU-03 | row-batched norm/rope/gate bitwise equal to per-row | unit | `... -- rows_prelude_identity` | no (Wave 0) |
| THRU-04 | dense linear bitwise equal to qlora base+zero-LoRA (CPU; Metal ignored) | unit | `... -- dense_linear_matches_qlora_zero_lora` | no (Wave 0) |
| THRU-05 | real-model byte identity at widest width (fixed 8-16 items, short cap) | ignored real-model, non-autonomous | `VOX_BATCH_BENCH_*=... cargo test --release -p vox-plugin-mens-candle-metal --features metal -- --ignored batched_decode_wall_clock_on_a_real_model` | yes (harness exists) |
| THRU-05 | width derivation uses exact KV size, no constants | unit | `... -- fit_width` | partly (`fit_width` exists) |
| THRU-06 | one load per side across slices; cancel stops consumption; failed batch falls back | unit with fake backend | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval::tests eval_collateral` | partly (`batch_backend` fake exists in `reasoning_eval.rs`) |
| THRU-07 | f32 inputs/keys unchanged (golden key), bf16 adds field, unknown tag errors, mixed-tag pair is Err | unit | `cargo test -p vox-eval --lib -- decode_protocol` ; `cargo test -p vox-ml-cli --features gpu --lib -- item_checkpoint` | no (Wave 0) |
| THRU-08 | bf16 plugin path selected by tag; head stays f32; tag echoed in load line | unit + ignored Metal | `... -- compute_dtype_for_protocol` | no |
| THRU-09 | base reuse across two adapters; collateral never removes base file; `--force-restart-base` only base | unit | `... -- item_checkpoint eval_collateral` | partly |
| THRU-10 | worst-case size over all paths <= alpha; DP equals brute force on a tiny schedule; loader refuses an invalid YAML | unit (exhaustive) | `cargo test -p vox-eval --lib -- sequential_mcnemar` | no (Wave 0) |
| THRU-11 | crossed boundary cancels stream, report records rule/look/items_used, replay from counts equals live | unit with fake backend | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral::sequential` | no |
| THRU-12 | measured table + 4B gate under budget | non-autonomous run | see Plan checkpoints | no |

### Mutation proofs (record before/after presence check, per AGENTS.md)
Loosen an interim `look_alpha` to `alpha` (THRU-10 test fails); `<` to `<=`; remove the K^T `contiguous` removal's test dependency by reintroducing `repeat_kv` index order wrong (THRU-02 test fails); swap head/row order in GQA reshape; remove `omit when f32` (golden-key test fails); change `false` to `force_restart` at `eval_collateral.rs:628` (base-never-restarted test fails); drop the pairing-protocol check (mixed-tag test fails).

### Sampling Rate
- **Per task commit:** the touched crate's quick command (CPU tests only; Metal-gated tests are `#[ignore]` and run in checkpoints).
- **Per wave merge:** full suite command above.
- **Phase gate:** full suite green plus the recorded THRU-01/05/12 artifacts before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `decode_step_decomposition` ignored test (extend `inference/batch_bench.rs`; records F, A, op counts, `uptime`)
- [ ] `attend_reference` kept `#[cfg(test)]` + synthetic bitwise tests (CPU and Metal-ignored)
- [ ] golden checkpoint-key test (f32 key bytes equal today's) in `item_checkpoint.rs` tests
- [ ] `crates/vox-eval/src/decode_protocol.rs` and `sequential_mcnemar.rs` with tests first
- [ ] env-var entry in `contracts/config/env-vars.v1.yaml`
- [ ] Wall-clock states for every proposed run (below)

### Proposed runs with expected wall-clock (all `autonomous: false`, GPU idle precondition: `ps aux | grep -c "[v]ox-ml-cli"` is 0)
| Run | Purpose | Expected wall-clock |
|---|---|---|
| R1 step-decomposition, before then after: 0.6B / 1.7B / 4B / 8B / 14B, B in {1,4,8,16,32}, 64 steps each, fixed 512-token context and a 2048-context point | THRU-01, before/after table | ~5 / 8 / 15 / 25 / 40 min per pass (load-dominated; two passes) |
| R2 byte identity on a real model: 0.6B, 8 items, cap 256, batch 1 vs {8,16,32}; 4B, 4 items, cap 128, batch 1 vs {4,8} | THRU-05 | ~5 min; ~10 min |
| R3 decode_dtype_bench (existing) | bf16 priority | ~5 min |
| R4 bf16 baseline + adapter, 0.6B 128/slice | THRU-08 | ~1.5-2 h |
| R5 4B f32 128/slice collateral gate (576 new generations) | THRU-12 | ~3.8 h expected, ~5.8 h conservative, hard budget 10 h |
| R6 optional 14B gate | THRU-12 stretch | ~8 h f32 / ~7 h bf16 (expected), 11 h conservative |

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication / V3 Session / V4 Access | no | local CLI, no new network surface |
| V5 Input Validation | yes | env `VOX_MENS_DECODE_PROTOCOL` and the YAML `sequential:` block are parsed strictly: unknown protocol, non-monotone/short `look_alpha`, fractions not ending at 1.0, or a worst-case size > alpha are hard errors; batch requests validated (`batch_cancel` with no session errors like `batch_next`) |
| V6 Cryptography | yes | hashes only through `vox-crypto` (`hash_json`); no new primitive |
| V12 Files | yes | checkpoint files stay under `REASONING_CHECKPOINT_ROOT`; keep fsync-per-line and torn-line semantics |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A stale/foreign-protocol generation reused under the wrong tag | Tampering | tag in key (non-default), header equality refusal, mixed-pair hard error |
| An edited YAML that silently raises the false-fail rate | Tampering | load-time exact worst-case guard + test |
| Memory exhaustion from a too-wide batch | DoS | width derived from free memory with headroom; refuse overflow of preallocated cache |
| Hidden global state (env) selecting numerics | Repudiation | protocol echoed in the plugin load line and recorded in the header/report |

## State of the work (what 337 must not redo)
Batched decode, per-item checkpoints keyed by inputs, prefix seeding for grown slices, base-never-restarted in collateral, slice pins and grader id, and the 14B load-path fix are landed and are not touched except where listed above.

## Sources

### Primary (HIGH confidence) - read this session
- `crates/vox-plugin-mens-candle-metal/src/{model.rs, inference.rs, inference/batch.rs, inference/batch_bench.rs, decode_dtype_bench.rs}`
- `crates/vox-ml-cli/src/commands/mens/{item_checkpoint.rs, reasoning_eval.rs, reasoning_compare.rs, eval_collateral.rs}`; `crates/vox-eval/src/reasoning_slice.rs`
- `patches/qlora-rs-1.0.5/src/qlora.rs`, `patches/candle-metal-kernels-0.10.2/src/{kernels/mlx_gemm.rs, metal/device.rs, metal/commands.rs}`
- `~/.cargo/registry/.../candle-core-0.10.2/src/{tensor_cat.rs, metal_backend/device.rs, metal_backend/mod.rs}`, `candle-nn-0.10.2/src/{kv_cache.rs, ops.rs, layer_norm.rs}`
- `mens/config/eval-gates-ladder.yaml`, `contracts/config/env-vars.v1.yaml`, `.planning/phases/{335,338}-*`, ROADMAP/STATE; `git show --stat 7ccb4adaa 31c692401`
- Data: all `mens/runs/reasoning_compare/checkpoints/*/*.jsonl` (759+37 generations, file birth/mtime)

### Secondary (MEDIUM) - own computation
- Exact enumeration and DP scripts (numpy): `scratchpad/{seq.py,seq4.py,seq5.py,seq7.py,sim.py,lens2.py,loops.py}` (results quoted above; planners re-derive them in Rust tests)

### Tertiary (LOW)
- Qwen3 architecture constants, bandwidth class of the M5 Max, per-op encode cost: training knowledge, tagged `[ASSUMED]`

## Metadata
**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies; all APIs read in source.
- Diagnosis: MEDIUM - cause identified from code and the four measured points; the F/A split and the dispatch-vs-traffic proportions are unmeasured.
- Statistics: HIGH - exact enumeration, reproducible; assumptions (A9) are the same as the existing test's.
- Projections: LOW - extrapolated from one measured size; replaced by THRU-01/12.
- Pitfalls: HIGH for identity/key hazards (read in code), MEDIUM for load effects.

**Research date:** 2026-11-06
**Valid until:** 2026-11-13 (335 is landing; re-read `reasoning_eval.rs`, `eval_collateral.rs`, `item_checkpoint.rs`, `reasoning_slice.rs` before each host-side task)

## Suggested plan decomposition (planner may refine)
1. 337-01 (plugin, no 335 dep): step spans + `decode_step_decomposition` ignored bench; checkpoint R1-before. Tests first.
2. 337-02 (plugin): dense inference linear (THRU-04) with reference tests.
3. 337-03 (plugin): preallocated KV + GQA decode attention + row-batched prelude (THRU-02/03) with `attend_reference` and synthetic bitwise tests; checkpoint R1-after + R2.
4. 337-04 (vox-eval, pure, may start now): `decode_protocol` + `sequential_mcnemar` modules with exhaustive tests (THRU-07 types, THRU-10 core). No 335 files touched.
5. 337-05 (after 335 SUMMARY): host wiring - protocol into checkpoint inputs/report/refusal, `--force-restart` scope, YAML block + loader guard (THRU-07, 09, 10 policy). Hand 338 the `decode` object here.
6. 337-06 (after 337-05): one load per side, continuous cross-slice stream, width derivation, `batch_cancel` (THRU-05, 06).
7. 337-07 (after 337-06): sequential early-stop wiring + report fields + replay (THRU-11).
8. 337-08 (after R3 numbers): bf16 plugin path + R4 (THRU-08).
9. 337-09 (checkpoint): R5 (and optional R6), before/after table, 337-VERIFICATION with mutation records (THRU-12).
