# Populi native training SSOT (Burn LoRA, Candle qlora-rs QLoRA)

## Why

- **One canonical CLI** for in-repo native fine-tuning: **`vox populi train`**.
- **Contract-first control plane** (in `vox-populi`): **`FineTuneContract`** + **`ExecutionPlanner`** + **`preflight_train`** gate impossible combos before kernels run (`finetune_contract.rs`, `execution_planner.rs`, `preflight_train.rs`). Capability table: [hf-finetune-capability-matrix.md](hf-finetune-capability-matrix.md). Gap labels: [hf-finetune-gap-matrix-ssot.md](hf-finetune-gap-matrix-ssot.md).
- **Honest execution-kernel split**:
  - **Burn + wgpu LoRA** (`--backend lora`): default **`VoxTokenizer`** JSONL; optional **`--tokenizer hf`** for **GPT-2-shaped** HF configs + ChatML-supervised HF tokenization + optional **embed warm-start** (`burn_hf_load.rs`). **Not** NF4 QLoRA.
  - **Candle + qlora-rs** (`--backend qlora`, `--tokenizer hf`): **NF4-quantized** trainable stack: when every expected **block output projection** (`o_proj` / GPT-2 `h.{L}.attn.c_proj.weight`) is present in the HF shards, **`training_step_lm`** runs **sequentially** through those layers plus the **tied LM head**; otherwise **LM-head-only** (backward compatible). **Context embeddings** stay **mmap `f32`** (`index_select`). Same **`--device`** story: CUDA / Metal with **`populi-candle-cuda`** / **`populi-candle-metal`**, else CPU; **`VOX_CANDLE_DEVICE=cpu`** forces CPU. Telemetry includes **`execution_kernel`**, **`telemetry_schema`**, and **`candle_compat_mode`** for Candle transitional scope.
- **Remaining gaps (explicit)**: full **causal NF4** blocks in Candle (see [candle-full-graph-feasibility.md](candle-full-graph-feasibility.md)); Burn **`LoraAttention::merge`** requires **`use_rope == false`** (GPT-2-style); RoPE stacks must stay **unmerged** or use native LoRA modules at serve time. **Double quant:** `QLoraConfig.quantization.double_quant` defaults **on**; CLI **`--qlora-no-double-quant`** disables for ablation. See [ADR 006 (full-graph)](../adr/006-populi-full-graph-qlora-qlora-rs.md) and [ADR 007 (API gate)](../adr/007-qlora-rs-multi-layer-training-api.md).
- **GPU visibility (Burn)**: stderr + **`burn_wgpu_device`** under **`vox_populi_gpu`**.
- **CI / CUDA**: When **`nvcc`** is on `PATH`, CI runs **`scripts/check_cuda_feature_builds.sh`**. See [`ci/runner-contract.md`](../ci/runner-contract.md#optional-cuda-compile-gate).

## What (surfaces)

| Piece | Role |
|--------|------|
| **`vox-cli`** `vox populi train` | **Compile:** `cargo build -p vox-cli --features gpu` (default features are **`populi-base` only**). **Runtime defaults:** **`--backend lora`** + **`--tokenizer vox`**. **`--backend qlora`**: **`--tokenizer hf`**, **`--model`**, safetensors + tokenizer; optional GPU via **`populi-candle-cuda`** / **`populi-candle-metal`**. Alias **`populi-qlora`** = **`gpu`**. **Mobile edge export:** **`--deployment-target mobile_edge`** or **`--preset mobile_edge`** → planner gates + **`--device cpu`** required; see [mobile-edge-ai-ssot.md](mobile-edge-ai-ssot.md). |
| **`vox-cli`** `vox populi serve` | **Requires `execution-api`** when building `vox-cli` (not in default features). Serves Burn **LoRA** checkpoints (`model_final.bin`, `checkpoint_*.bin`) and **merged** `model_merged.bin` from **`merge-weights`** (same HTTP surface). |
| **`vox-populi`** `PopuliTrainBackend` | Enum + `FromStr` / serde in `crates/vox-populi/src/tensor/train_backend.rs`. |
| **`vox-populi`** `TrainingBackend` | Trait in `tensor/backend.rs`; Burn `tensor/backend_burn_lora.rs`; Candle `tensor/backend_candle_qlora.rs` + `tensor/candle_qlora_train.rs` (compiled with feature **`train`**, which pulls **`candle-qlora`** + **`qlora-rs`**; do not rely on **`candle-qlora`** alone for the trainer graph). |
| **`vox-populi`** `run_populi_training` | Dispatch in `tensor/lora_train.rs` → `.run()` on the matching backend unit struct. |
| **`vox-populi`** `LoraTrainingConfig` | `tensor/training_config.rs` (`PopuliTokenizerMode`, shared across backends). |
| **`vox train`** | Legacy: **`--provider local`** bails with the canonical **`vox populi train --backend qlora …`** command (no shipped `train_qlora.vox`). **`--native`** uses the old Burn scratch trainer when built with **`populi-dei`**. Together remote unchanged. |
| **`vox populi train-uv`** | **Retired** — bails; use **`vox populi train --backend qlora`**. |

## Who / when

- **Implementers**: `vox-populi` (tensor/train), `vox-cli` (`commands/populi/train.rs`, `mod.rs`), corpus preflight (`vox-corpus::training`).
- **When to touch**: training knobs, telemetry keys, CLI flags, qlora-rs / Candle versions, or merge/export behavior.

## Where (files)

- `crates/vox-populi/src/tensor/train_backend.rs` — CLI/backend enum (`PopuliTrainBackend`) + `ExecutionKernel` alias
- `crates/vox-populi/src/tensor/finetune_contract.rs` — `FineTuneContract` + sub-specs
- `crates/vox-populi/src/tensor/execution_planner.rs` — planner + hard gates
- `crates/vox-populi/src/tensor/preflight_train.rs` — shared preflight entry
- `crates/vox-populi/src/tensor/hf_keymap.rs` — shared HF weight key maps
- `crates/vox-populi/src/tensor/training_text.rs` — prompt / ChatML text policy
- `crates/vox-populi/src/tensor/telemetry_schema.rs` — stable telemetry keys
- `crates/vox-populi/src/tensor/adapter_schema_v3.rs` — adapter manifest v3 + merge bridge
- `crates/vox-populi/src/tensor/finetune_registry.rs` — method / quant / target registries
- `crates/vox-populi/src/tensor/burn_hf_load.rs` — HF → Burn warm-start (token embed + optional **GPT-2** decoder weights)
- `crates/vox-populi/src/tensor/training_config.rs` — `LoraTrainingConfig`
- `crates/vox-populi/src/tensor/backend.rs` — `TrainingBackend` trait
- `crates/vox-populi/src/tensor/backend_burn_lora.rs` — Burn + wgpu LoRA
- `crates/vox-populi/src/tensor/backend_candle_qlora.rs` — Candle qlora-rs entry (`train`)
- `crates/vox-populi/src/tensor/candle_qlora_train.rs` — qlora-rs trainer (`train`; mmap embed + optional **o_proj** proxy stack + tied LM head + v2 LoRA export)
- `crates/vox-populi/src/tensor/train_log.rs` — `[populi-train]` stderr + **`gpu_fallback`**
- `crates/vox-populi/src/tensor/qlora_preflight.rs` — HF safetensors + tokenizer checks (embedding table: `wte.weight` preferred over `model.embed_tokens.weight` when both exist across shards)
- `crates/vox-populi/src/tensor/operator_messages.rs` — shared operator error strings for planner + QLoRA preflight (avoid drift)
- `crates/vox-populi/src/tensor/burn_inference_load.rs` — load LoRA vs merged Burn checkpoints for serve/eval
- `crates/vox-populi/src/tensor/lora_train.rs` — `run_populi_training` / `run_lora_training`
- `crates/vox-cli/src/commands/populi/mod.rs` — `--backend` CLI mapping
- `crates/vox-cli/src/commands/populi/train.rs` — `run_train` → `run_populi_training`
- `crates/vox-cli/src/commands/populi/mod.rs` — `train-uv` **retired** (inline bail; use `vox populi train --backend qlora`)
- `AGENTS.md` § 2.2.3, `docs/src/ref-cli.md` (Populi), `docs/src/expl-ml-pipeline.md` (train matrix)
- Plans: `.cursor/plans/native_qlora_ssot_dea968e4.plan.md`, `.cursor/plans/qlora_ssot_grounded_plan_cc5501f2.plan.md`

## Full-graph QLoRA design (Phase 2c)

**Architecture gate (2026-03):** [ADR 007](../adr/007-qlora-rs-multi-layer-training-api.md) records audit of **qlora-rs 1.0.5**: `QLoraTrainer::training_step_lm` accepts **`&[&QuantizedLinear]`** and applies them **sequentially** in one forward/backward; `init_optimizer` trains **all** LoRA parameters registered via **`trainer.var_builder()`**. **Approach A** — expand in-tree graph with **public APIs only** (no fork) unless a future qlora-rs change breaks this contract.

**HF layout:** `vox_populi::tensor::hf_load::HfTransformerLayout` parses `config.json` (`model_type`, `architectures`, `hidden_size`, `num_attention_heads`, `num_hidden_layers`, `vocab_size`) for Llama/Mistral/Qwen-style and GPT-2-shaped configs. `qlora_preflight` checks **`hidden_size` matches** the embedding tensor width discovered in safetensors.

## How (contracts)

- **Build**: `cargo check -p vox-populi --features train` (**`train`** pulls **`candle-qlora`** + **`qlora-rs`**). Optional CUDA/Metal: **`populi-candle-cuda`** / **`candle-qlora-cuda`**, **`populi-candle-metal`** / **`candle-qlora-metal`**.
- **Workspace deps**: root `[workspace.dependencies]` **`qlora-rs`** pin must stay aligned with `vox-populi` optional dep. **`[patch.crates-io]`** (`patches/qlora-rs-1.0.5`, see `VOX_PATCH.md`) adds **RMSNorm (γ=1) between stacked projections** in `training_step_lm` and **gradient-accumulation scaling** for LM steps — upstream `qlora-rs` alone can report **~1e20 CE** on deep Vox `o_proj` proxy stacks (no residual / norm between layers).
- **Input**: `train.jsonl` (and `populi/config/training_contract.yaml` / preflight overrides).
- **Telemetry**: `train_start` includes `train_backend: "burn_lora"` or `"candle_qlora"`. **Candle QLoRA** `train_start` also records **`epochs`**, **`planned_steps_per_epoch`**, **`planned_steps_total`** (upper bound if no vocab/hidden skips). Progress logs (**~5s**): **`ETA_smoothed≈…`** from an **interval throughput EMA** (after step **24**), plus **step/s** and **% of planned** — no duplicate `step 20/40/…` log lines (those are **`telemetry.jsonl` only**). **`step`** rows add **`steps_per_sec_ema`**, **`eta_seconds_remaining`** (EMA-based), **`progress_fraction`**. **`train_complete`**: **`wall_seconds`**, **`mean_steps_per_sec`**. See `telemetry_schema` keys.

## Training objective mismatch (Burn vs Candle)

- **Burn (`--backend lora`)**: full-graph **f32** causal LM on wgpu (or NdArray in tests). Objective = standard next-token CE over the whole decoder graph you enabled.
- **Candle (`--backend qlora`)**: **NF4** frozen bases via qlora-rs; forward uses **`training_step_lm`** over the **supported bounded proxy graph** (see [candle-full-graph-feasibility.md](candle-full-graph-feasibility.md)): LM head always; optional middle `o_proj` / `c_proj` stack when shards are complete. **Not** a full causal NF4 decoder — **non-goal** until a future full-graph milestone; inference must not assume full-transformer logits.
- **Operator impact**: do **not** expect loss / perplexity curves to match Burn. Use `training_manifest.json` **`candle_qlora_graph_id`**, **`candle_qlora_ce_last_k`**, **`training_objective_note`**, telemetry, and tiered **parity tests** (`candle_burn_*`) for **shared f32 primitives** only — not end-to-end NF4-vs-Burn LM identity.

## Burn LoRA vs Candle QLoRA — which path, when (4080 Super and beyond)

### Is QLoRA “better” than Burn LoRA?

**Not universally.** They solve different problems:

| Goal | Prefer |
|------|--------|
| **Train a real Hugging Face base** (e.g. Qwen2.5-Coder) on **16G VRAM** with industry-style **NF4 + LoRA** | **Candle QLoRA** (`--backend qlora`, `--tokenizer hf`, `--model …`, CUDA build) |
| **Full in-tree f32 causal LM** on **VoxTokenizer JSONL** (docs/examples → pairs), **merge → `vox populi serve`** without an external runtime | **Burn LoRA** (`--backend lora`, default) |
| **Apples-to-apples loss** with “full decoder” next-token CE on the **same** architecture | **Burn** on the **small** Vox causal stack; Candle QLoRA is a **bounded proxy graph** (LM head + optional `o_proj`/`c_proj` stack), not a full NF4 transformer (see [candle-full-graph-feasibility.md](candle-full-graph-feasibility.md)) |

So: **QLoRA is “better” for large-model, VRAM-efficient fine-tuning on shipped HF weights.** **Burn LoRA is “better” for the closed Vox corpus loop and first-class serve/merge in this repo.** You may run **both** in a serious program: Burn for **syntax/docs/tooling-shaped** adapters on the native head; QLoRA for **Qwen-class** behavior on HF bases.

### Should a 4080 Super workstation use Candle CUDA QLoRA?

**Yes, when the target is a real Qwen (or similar) checkpoint** and you have built **`vox-cli` with `gpu,populi-candle-cuda`**. That is the **documented** 16G-class path (preset **`qwen_4080_16g`** / **`--preset 4080`**). Your **Vulkan/wgpu** logs still mean **Burn** is correctly using the GPU; that is **not** a substitute for **Candle CUDA** — different stacks.

### Strengths and weaknesses (persistent reference)

**Burn + wgpu LoRA (`PopuliTrainBackend::BurnLora`)**

| Strengths | Weaknesses |
|-----------|------------|
| **End-to-end Vox story**: corpus JSONL → train → **`merge-weights`** → **`vox populi serve`** (HTTP) on **`*.bin` / `model_merged.bin`**. | Does **not** load arbitrary **multi-billion** HF transformers in **f32** on a 16G card; use QLoRA for that. |
| **Full-graph f32** objective on the **in-repo** `LoraVoxTransformer` (honest CE over the graph you compiled). | **`LoraAttention::merge`** path requires **`use_rope == false`** (GPT-2-style); RoPE stacks stay **unmerged** or need native LoRA at serve time (see top-of-file gaps). |
| **Cross-platform GPU** via wgpu (Vulkan / DX12 / Metal); no NVIDIA CUDA toolchain required. | **Different model** than production Qwen: eval numbers vs HF chat models are **not** directly comparable. |
| Fewer external artifacts: no mandatory **`tokenizer.json` + safetensors`** for the default **`--tokenizer vox`** path. | Optional **`--tokenizer hf`** is **GPT-2-shaped** configs + embed warm-start — still not arbitrary Llama/Qwen **full** weight training in Burn. |

**Candle + qlora-rs QLoRA (`PopuliTrainBackend::CandleQlora`)**

| Strengths | Weaknesses |
|-----------|------------|
| **NF4 base + trainable LoRA** on **real** HF shards; **VRAM-efficient** vs full fine-tune; matches **operator expectations** for “train Qwen locally”. | Forward is **not** a full causal NF4 decoder; **proxy stack** + **`training_step_lm`** semantics — see objective mismatch above and ADR 006/007. |
| **NVIDIA CUDA** (and Metal) **first-class** when built with **`populi-candle-cuda`** / **`populi-candle-metal`**. | **`vox populi serve`** does **not** load **`merge-qlora`** outputs; use **vLLM / Ollama / HF** (or export pipeline TBD) for merged **f32** shards. |
| Strong **preflight** (`qlora_preflight`) catches tokenizer / embedding width / shard key issues **before** long runs. | **Shard key completeness** drives **full proxy** vs **LM-head-only**; **`--qlora-require-full-proxy-stack`** can hard-fail when keys are missing. |
| **Preset family** (`qwen_4080_16g`, `4080`, etc.) tuned for **16G** cards. | **Patch + contract** coupling: in-tree **`qlora-rs`** patch for stable deep stacks; upgrade pins need care (`VOX_PATCH.md`). |

### Last-minute flight check (before a “real” training push)

Use this as an ordered gate; skip steps that do not apply to your target backend.

1. **Compile**: `cargo check -p vox-cli --features gpu` (Burn + CPU QLoRA baseline). For **CUDA QLoRA on 4080**: `cargo check -p vox-cli --features gpu,populi-candle-cuda` (release build: ensure **`vox.exe`** is not locked by another process on Windows).
2. **CLI/registry drift**: `vox ci command-compliance` (or `cargo run -p vox-cli --features gpu -- ci command-compliance`).
3. **Training acceptance profile**: `cargo run -p vox-cli -- ci populi-gate --profile training` (see [populi-finetune-acceptance-runbook.md](populi-finetune-acceptance-runbook.md)).
4. **Language/tooling confidence** (orthogonal to trainer): `cargo check --workspace`, `cargo test` for areas you touched; MCP **`vox-mcp`** and orchestrator paths assume a healthy **`vox`** binary and repo root — see [AGENTS.md](../../../AGENTS.md) § orchestration / capability registry.
5. **Data**: canonical **`train.jsonl`** under **`--data-dir`** (often **`target/dogfood`** after corpus mix); optional **`VOX_TRAIN_SKIP_CORPUS_MIX=1`** when the JSONL is already final.
6. **Choose artifact + inference**: **Burn** → **`merge-weights`** → **`vox populi serve`**; **QLoRA** → **`merge-qlora`** → external **OpenAI-compatible** or HF runtime (not `serve` today).
7. **Long runs**: `vox populi train … --log-dir …` + **`--background`** as needed; see RTX 4080 section below.

**“Full model build” in practice** means: (a) **data** corpus at quality gate, (b) **trainer** chosen and **manifest** recorded, (c) **merge/export** aligned with **where inference will run** (Vox HTTP vs external LLM), (d) **eval** (`vox populi corpus eval` / `eval-local` where applicable) before promoting artifacts.

## RTX 4080-class CUDA (16G) — canonical QLoRA (copy-paste)

- **Preset**: **`qwen_4080_16g`** (rank 16, seq 384, batch 1, grad_accum 8). CLI **`--preset 4080`** is an **alias** of the same profile (default **`DEFAULT_PRESET`** is **`4080`**).
- **Compile check (CUDA Candle stack)**: `cargo check -p vox-cli --features gpu,populi-candle-cuda` (or `cargo vox-cuda-release`).
- **Train (Qwen2.5-Coder-3B example)**:
  `vox populi train --backend qlora --tokenizer hf --preset qwen_4080_16g --model Qwen/Qwen2.5-Coder-3B-Instruct --data-dir target/dogfood --output-dir populi/runs/qwen25_qlora --device cuda --qlora-require-full-proxy-stack`
- **`--device cuda`** without **`populi-candle-cuda`** fails fast at CLI with rebuild instructions.
- **CPU smoke**: `VOX_CANDLE_DEVICE=cpu` forces Candle on CPU for debugging.
- **IDE / Cursor timeouts (long builds + train)**: Agent tools often cap wall time. Prefer **logging + background** instead of blocking:
  - **Train**: `vox populi train … --log-dir populi/runs/logs` — parent spawns a detached child and exits immediately; monitor with `Get-Content populi/runs/logs/train_*.log -Wait -Tail 25` (or `tail -f`). Combine with `--background` for low priority + VRAM cap (see `vox populi train --help`).
  - **CUDA `cargo` build**: run in a normal terminal or `Tee-Object` to a file, e.g. [`scripts/populi/cursor_background_cuda_build.ps1`](../../../scripts/populi/cursor_background_cuda_build.ps1). To return immediately from an agent while the build continues, use [`scripts/populi/cursor_background_cuda_build_detached.ps1`](../../../scripts/populi/cursor_background_cuda_build_detached.ps1). Example train launcher: [`scripts/populi/cursor_background_train_example.ps1`](../../../scripts/populi/cursor_background_train_example.ps1).
  - **Skip corpus mix** (optional): `VOX_TRAIN_SKIP_CORPUS_MIX=1` skips the pre-train `mix` refresh when you already have the desired `train.jsonl` or need a shorter path under automation.
- **Benchmark telemetry (Codex)**: set **`VOX_BENCHMARK_TELEMETRY=1`** so select CLI paths append unified `benchmark_event` rows (`VoxDb::record_benchmark_event`, session `bench:<repository_id>`): `vox populi bench-completion`, **`vox populi eval-local` only when `vox-cli` is built with feature `gpu`** (CPU-only eval skips telemetry rows), `vox ci build-timings`, optional train gate (`VOX_BENCHMARK` eval-local subprocess), and the ignored `run_benchmark` integration test warm pass. Set **`VOX_REPOSITORY_ROOT`** so subprocess `repository_id` matches MCP when CWD differs. Query via MCP `vox_benchmark_list` when Codex is attached.
- **JSONL rows**: `vox_tensor::data::TrainingPair` accepts **`instruction`** as alias for **`prompt`** and **`output`** for **`response`** so corpus rows are not silently dropped.
- **Bounded proxy forward (supported; in-tree qlora-rs patch)**: `training_step_lm` uses **pre-norm residual** middle blocks `h ← h + (1/√n_mid)·F(RMSNorm(h))` and scales again by **`1/√n_mid`** before the LM head so deep `o_proj` stacks stay **finite** and trainable. Merge and serve paths must use the **same** graph as training (manifest records **`candle_qlora_graph_id`**). This is **not** a full transformer residual path inside attention/FFN — see feasibility doc.
- **Suffix CE (`--qlora-ce-last-k K`)**: default **`1`** = predict the **last** token from `E[t-1]` only. **`K > 1`** runs one `training_step_lm` per row for each target index `t` in **`max(1, L−K) .. L−1`**, i.e. next-token CE on the **last K positions** of the (trimmed) sequence — closer to standard LM on a suffix, at **~K×** optimizer micro-steps per JSONL row. Capped vs **`seq_len`** at CLI.
- **Depth ablation**: **`--qlora-proxy-max-layers N`** caps how many ordered middle projections are stacked (`0` = LM-head-only; omit = full stack when keys are complete). Conflicts with **`--qlora-lm-head-only`** when `N > 0`.
- **Debug**: **`VOX_QLORA_DEBUG_NORMS=1`** prints mean-|activation| after each middle block (stderr; local ablation only).
- **Stable dogfood (QLoRA)**: if CE is still pathological, keep **`--qlora-lm-head-only`** as the **operator escape hatch** (preferred over env; survives `--log-dir` re-spawn). Env **`VOX_QLORA_LM_HEAD_ONLY=1`** remains for ad-hoc runs.

## Pre-push release gate (acceptance matrix)

- **Canonical (cross-platform)**: `cargo run -p vox-cli -- ci populi-gate --profile training`  
  Steps live in [`scripts/populi/gates.yaml`](../../../scripts/populi/gates.yaml) (`training` profile).
- **Thin shims** (delegate to the same command): `bash scripts/populi/release_training_gate.sh`, `pwsh scripts/populi/release_training_gate.ps1`  
  Mirrors [`populi-finetune-acceptance-runbook.md`](populi-finetune-acceptance-runbook.md) rows 1–10 (planner, keymap, strict preflight, Burn smoke, parity tests, merge, `merge_v2`).

## Regression tests

- **Execution planner + hard gates**: `cargo test -p vox-populi --features train execution_planner`
- **QLoRA strict proxy stack (missing middle keys)**: `cargo test -p vox-populi --features train preflight_strict_rejects_missing_o_proj`
- **Fine-tune digest (`qlora_proxy_max_layers`)**: `cargo test -p vox-populi --features train finetune_contract_digest_changes_with_proxy_max_layers`
- **Fine-tune digest (`qlora_ce_last_k`)**: `cargo test -p vox-populi --features train finetune_contract_digest_changes_with_ce_last_k`
- Candle qlora trainer unit test in `candle_qlora_train.rs` (hidden state shape): `cargo test -p vox-populi --features train` (filter `last_hidden` / module tests as documented in crate).
- **Burn LoRA `Checkpoint` bin roundtrip** (NdArray): `cargo test -p vox-populi --features train lora_vox_transformer_checkpoint_roundtrip`
- **Burn `LoraVoxTransformer::merge` full forward parity** (NdArray): `cargo test -p vox-populi --features train merged_vox_transformer_matches_lora_full_forward`
- **Burn linear LR warmup** (Burn `LinearLrScheduler`): `cargo test -p vox-tensor --features gpu --lib linear_warmup_sequence_matches`
- **Candle vs Burn f32 matmul** (CPU numeric touchpoint, not full LM logits): `cargo test -p vox-populi --features train --test candle_burn_f32_matmul_parity`
- **Candle vs Burn f32 LM linear logits** (`x @ W + bias`, not NF4 / not full graphs): `cargo test -p vox-populi --features train --test candle_burn_f32_linear_lm_logits_parity`
- **Tier B — NF4 dequant reference linear** (qlora-rs quantize→dequant→shared f32 `W`, same layout as `QuantizedLinear` matmul): `cargo test -p vox-populi --features train --test candle_burn_nf4_dequant_lm_reference_parity`
- **Candle vs Burn cross-entropy** (same f32 logits + labels): `cargo test -p vox-populi --features train --test candle_burn_cross_entropy_parity`
- **`merge-qlora` rejects Burn `*.bin`**: `cargo test -p vox-cli merge_qlora_rejects_burn_bin_adapter`
- **`merge-weights`** rejects `candle_qlora_adapter.safetensors` (Burn path only) and points to **`merge-qlora`**: `cargo test -p vox-cli merge_weights_rejects_candle_qlora_adapter_file`
- **`merge-qlora` CLI** synthetic roundtrip: `cargo test -p vox-cli merge_qlora_cli_roundtrip_lm_head_subset`
- Adapter **v2** merge math: `cargo test -p vox-populi --features train merge_v2_applies_lm_head_delta`

## Merge / export / inference

| Command / artifact | Status |
|--------------------|--------|
| **`vox populi merge-weights`** | Merges **Burn** LoRA checkpoints (`*.bin` from `--backend lora`) into `model_merged.bin`. Requires **`gpu`**. |
| **`candle_qlora_adapter.safetensors`** | **LoRA A/B per logical layer** (`mid0`…`lm_head`); sidecar **`candle_qlora_adapter_meta.json`** format **`vox_populi_qlora_lora_only_v2`** (`QloraAdapterMetaV2`). |
| **`vox populi merge-qlora`** (alias **`merge-adapter`**) | **Candle QLoRA path only:** merges v2 or **v3** adapter meta + LoRA tensors into **f32** base shards for keys in `base_key_map` (subset output safetensors). Distinct from **`merge-weights`** and from Burn **`*.bin`** checkpoints. There is **no** supported conversion from Burn **`*.bin`** LoRA checkpoints into Candle adapter safetensors for this command — use **`merge-weights`** for Burn → `model_merged.bin`. |
| **`vox populi serve`** (HTTP, `execution-api`) | Loads **Burn** checkpoints: LoRA `*.bin` **or** merged **`VoxTransformer`** (`model_merged.bin` from **`merge-weights`**). Does **not** load Candle **`merge-qlora`** output safetensors; use HF/Ollama/vLLM or another stack for merged QLoRA f32 shards. |
| **`populi_adapter_manifest_v3.json`** | Unified adapter manifest (method + quant + layer order + `base_key_map`); written beside v2 meta on Candle runs. |
| **Full causal NF4 + PEFT parity** | Open work — deeper block coverage beyond o_proj proxy stack. |

## Related

- **LLM / agent PR hygiene:** [`populi-llm-pr-checklist.md`](populi-llm-pr-checklist.md) — LoRA duplication, layouts, merge, CI test names, parity tiers.
- **LoRA ownership boundary:** [`populi-lora-ownership-ssot.md`](populi-lora-ownership-ssot.md)
- **Speech / ASR** (Oratio): [`oratio-speech-ssot.md`](oratio-speech-ssot.md) — orthogonal to training; shares `vox populi` CLI namespace only. CLI STT commands need **`vox-cli`** feature **`populi-oratio`** (not default **`populi-base`**).

