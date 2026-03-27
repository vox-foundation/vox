---
title: "Mens native training SSOT (Candle QLoRA–first; Burn LoRA deprecated in dispatch)"
description: "Official documentation for Mens native fine-tuning: contract-first Candle QLoRA, legacy Burn paths, merge/serve matrix."
category: "reference"
last_updated: 2026-03-26
training_eligible: true
---
# Mens native training SSOT (Candle QLoRA–first)

> **Code SSOT:** `vox mens train` dispatches through `vox_populi::mens::tensor::run_mens_training` ([`lora_train.rs`](../../../crates/vox-populi/src/mens/tensor/lora_train.rs)). **`PopuliTrainBackend::BurnLora` is rejected at runtime** with an explicit error; the **supported** native trainer is **`CandleQlora`** (`--backend qlora`, `--tokenizer hf` for HF-shaped models). Docs below still describe Burn for **merge-weights / `vox mens serve`** and historical parity—treat **Burn training** as legacy/optional tooling, not an active `schola train` backend.

## Truth tables (train → merge → serve)

| Path | Train (CLI) | Merge | Serve in-tree |
|------|-------------|-------|----------------|
| **Candle QLoRA** | `vox mens train --backend qlora --tokenizer hf …` | `vox schola merge-qlora` (alias `merge-adapter`) → f32 subset shards | **No** — use vLLM/Ollama/HF (or external OpenAI-compatible stack); `vox mens serve` does not load QLoRA merge outputs |
| **Burn LoRA** | **Not** via `schola train` dispatch (use historical/legacy flows if you still maintain Burn checkpoints) | `vox mens merge-weights` → `model_merged.bin` | **Yes** — `vox mens serve` (`execution-api`) loads `*.bin` / merged Burn checkpoints |

## Why

- **One canonical CLI** for in-repo native fine-tuning: **`vox mens train`**.
- **Contract-first control plane** (in `vox-populi::mens::tensor`): **`FineTuneContract`** + **`ExecutionPlanner`** + **`preflight_train`** gate impossible combos before kernels run (`finetune_contract.rs`, `execution_planner.rs`, `preflight_train.rs`). **Preflight output schema (F04, extend alongside code):** [`contracts/mens/training-preflight.schema.json`](../../../contracts/mens/training-preflight.schema.json). After a successful `preflight_for_contract` inside `run_mens_training`, the trainer writes **`training-preflight.json`** next to run artifacts when an output directory is set (fields: `schema_version`, `contract_digest`, `execution_kernel`, optional `notes`). Capability table: [hf-finetune-capability-matrix.md](../architecture/hf-finetune-capability-matrix.md). Gap labels: [hf-finetune-gap-matrix.md](hf-finetune-gap-matrix.md).
- **Honest execution-kernel split**:
  - **Burn + wgpu LoRA** (`--backend lora`): default **`VoxTokenizer`** JSONL; optional **`--tokenizer hf`** for **GPT-2-shaped** HF configs + ChatML-supervised HF tokenization + optional **embed warm-start** (`burn_hf_load.rs`). **Not** NF4 QLoRA.
  - **Candle + qlora-rs** (`--backend qlora`, `--tokenizer hf`): **NF4-quantized full-graph training** over loaded decoder blocks with trainable LoRA adapters. Current trainer path is full graph only (LM-head-only/partial-depth flags are parsed for contract compatibility but rejected at runtime). **Context embeddings** stay **mmap `f32`** (`index_select`). Same **`--device`** story: CUDA / Metal with **`mens-candle-cuda`** / **`mens-candle-metal`**, else CPU; **`VOX_CANDLE_DEVICE=cpu`** forces CPU. Telemetry includes **`execution_kernel`**, **`telemetry_schema`**, and **`candle_compat_mode`** for cutover observability.
- **Remaining gaps (explicit)**: full **causal NF4** blocks in Candle (see [candle-full-graph-feasibility.md](../architecture/candle-full-graph-feasibility.md)); Burn **`LoraAttention::merge`** requires **`use_rope == false`** (GPT-2-style); RoPE stacks must stay **unmerged** or use native LoRA modules at serve time. **Double quant:** `QLoraConfig.quantization.double_quant` defaults **on**; CLI **`--qlora-no-double-quant`** disables for ablation. See [ADR 006 (full-graph)](../adr/006-mens-full-graph-qlora-qlora-rs.md) and [ADR 007 (API gate)](../adr/007-qlora-rs-multi-layer-training-api.md).
- **GPU visibility (Burn)**: stderr + **`burn_wgpu_device`** under **`vox_mens_gpu`**.
- **CI / CUDA**: When **`nvcc`** is on `PATH`, CI runs **`scripts/check_cuda_feature_builds.sh`**. See [`ci/runner-contract.md`](../ci/runner-contract.md#optional-cuda-compile-gate).

## Provenance and trajectory metadata (2026 update)

MENS run artifacts now treat lineage and trajectory policy as explicit metadata:

- **Provenance fields** (contract + manifest):
  - upstream family id,
  - upstream model id,
  - license class,
  - attribution-required flag.
- **Trajectory-weighting fields** (config + telemetry semantics):
  - optional weighting toggle for tool-trace style rows,
  - optional boost for failure/error categories,
  - optional quality floor and quality boost.
- **Experimental optimizer lane**:
  - `optimizer_experiment_mode` defaults to `off`,
  - non-default modes require `VOX_MENS_EXPERIMENTAL_OPTIMIZER=1`.

These defaults remain conservative and do not change baseline behavior unless enabled.
Context and source-strength notes for Composer/Kimi findings are documented in
[`../architecture/mens-composer-kimi-findings-2026.md`](../architecture/mens-composer-kimi-findings-2026.md).

## `finetune_contract_digest` scope

`finetune_contract_digest` is a reproducibility fingerprint for planner-relevant training semantics. Current scope includes:

- model/config/tokenizer file identity used by the contract,
- quantization and adapter method knobs,
- tokenizer mode and selected QLoRA behavior gates,
- provenance metadata fields (`base_family`, `upstream_model_id`, `license_class`, `attribution_required`).

It intentionally excludes runtime-only telemetry counters and post-hoc eval outcomes.

## What (surfaces)

| Piece | Role |
|--------|------|
| **`vox-cli`** `vox mens train` | **Compile:** `cargo build -p vox-cli --features gpu` (default features are **`mens-base` only**). **Operational default:** `--backend qlora --tokenizer hf` (Candle QLoRA). Legacy `--backend lora` is deprecated and retained only for compatibility context. **Mobile edge export:** **`--deployment-target mobile_edge`** or **`--preset mobile_edge`** → planner gates + **`--device cpu`** required; see [mobile-edge-ai.md](mobile-edge-ai.md). |
| **`vox-cli`** `vox mens serve` | **Requires `execution-api`** when building `vox-cli` (not in default features). Serves Burn **LoRA** checkpoints (`model_final.bin`, `checkpoint_*.bin`) and **merged** `model_merged.bin` from **`merge-weights`** (same HTTP surface). |
| **`vox-populi`** `PopuliTrainBackend` | Enum + `FromStr` / serde in `crates/vox-populi/src/mens/tensor/train_backend.rs`. |
| **`vox-populi`** `TrainingBackend` | Trait in `tensor/backend.rs`; Candle implementation in `tensor/backend_candle_qlora.rs` + `tensor/candle_qlora_train` modules. |
| **`vox-populi`** `run_mens_training` | Dispatch in `tensor/lora_train.rs` with contract/planner/preflight gates. |
| **`vox-populi`** `LoraTrainingConfig` | `tensor/training_config.rs` (`MensTokenizerMode`, provenance/trajectory knobs). |
| **`vox train`** | Legacy: **`--provider local`** bails with the canonical **`vox mens train --backend qlora …`** command (no shipped `train_qlora.vox`). **`--native`** uses the old Burn scratch trainer when built with **`mens-dei`**. Together remote unchanged. |
| **`vox mens train-uv`** | **Retired** — bails; use **`vox mens train --backend qlora`**. |

## Who / when

- **Implementers**: `vox-populi` (`mens::tensor`), `vox-cli` (`commands/schola/train/*`, `commands/mens/populi/*`), `vox-schola` (`src/train.rs`), corpus preflight (`vox-corpus::training`).
- **When to touch**: training knobs, telemetry keys, CLI flags, qlora-rs / Candle versions, or merge/export behavior.

## Where (files)

- `crates/vox-populi/src/mens/tensor/train_backend.rs` — CLI/backend enum (`PopuliTrainBackend`) + execution kernel
- `crates/vox-populi/src/mens/tensor/finetune_contract.rs` — `FineTuneContract`, provenance, digest
- `crates/vox-populi/src/mens/tensor/execution_planner.rs` — planner + hard gates
- `crates/vox-populi/src/mens/tensor/preflight_train.rs` — shared preflight entry
- `crates/vox-populi/src/mens/tensor/hf_keymap.rs` — shared HF weight key maps
- `crates/vox-populi/src/mens/tensor/training_text.rs` — prompt / ChatML text policy
- `crates/vox-populi/src/mens/tensor/telemetry_schema.rs` — stable telemetry keys
- `crates/vox-populi/src/mens/tensor/adapter_schema_v3.rs` — adapter manifest v3 + merge bridge
- `crates/vox-populi/src/mens/tensor/training_config.rs` — `LoraTrainingConfig`
- `crates/vox-populi/src/mens/tensor/backend.rs` — `TrainingBackend` trait
- `crates/vox-populi/src/mens/tensor/backend_candle_qlora.rs` — Candle qlora-rs entry
- `crates/vox-populi/src/mens/tensor/candle_qlora_train/*` — trainer graph, loop, checkpoints
- `crates/vox-populi/src/mens/tensor/train_log.rs` — `[mens-train]` stderr + fallback notes
- `crates/vox-populi/src/mens/tensor/qlora_preflight.rs` — HF safetensors + tokenizer checks
- `crates/vox-populi/src/mens/tensor/operator_messages.rs` — shared operator error strings
- `crates/vox-populi/src/mens/tensor/lora_train.rs` — `run_mens_training`
- `crates/vox-cli/src/commands/mens/mod.rs` — `--backend` CLI mapping
- `crates/vox-cli/src/commands/schola/train.rs` — `run_train` → `run_mens_training`
- `crates/vox-schola/src/train.rs` — standalone `vox-schola train` QLoRA path
- `crates/vox-cli/src/commands/mens/mod.rs` — `train-uv` **retired** (inline bail; use `vox mens train --backend qlora`)
- `AGENTS.md` § 2.2.3, `docs/src/ref-cli.md` (Mens), `docs/src/expl-ml-pipeline.md` (train matrix)
- Plans: `.cursor/plans/native_qlora_ssot_dea968e4.plan.md`, `.cursor/plans/qlora_ssot_grounded_plan_cc5501f2.plan.md`

## Full-graph QLoRA design (Phase 2c)

**Architecture gate (2026-03):** [ADR 007](../adr/007-qlora-rs-multi-layer-training-api.md) records the qlora-rs API surface audit used by the native trainer. Keep this ADR in sync with any future trainer graph changes.

**HF layout:** `vox_mens::tensor::hf_load::HfTransformerLayout` parses `config.json` (`model_type`, `architectures`, `hidden_size`, `num_attention_heads`, `num_hidden_layers`, `vocab_size`) for Llama/Mistral/Qwen-style and GPT-2-shaped configs. `qlora_preflight` checks **`hidden_size` matches** the embedding tensor width discovered in safetensors.

## How (contracts)

- **Build**: `cargo check -p vox-populi --features mens-train` (pulls qlora-rs + candle trainer path). Optional CUDA lane: `--features mens-train,mens-candle-qlora-cuda`.
  > [!IMPORTANT]
  > **Windows MSVC/NVCC constraint**: Building the CUDA `candle-kernels` completely fails if executed through a nested subshell (e.g. `cmd.exe /c "vcvars64.bat && cargo build"`). The inner `bindgen_cuda` executable natively drops nested path states, leading to an immediate `'cl.exe' is not recognized` failure. You **must** interactively open the VS Developer Command Prompt or physically run `vcvars64.bat` in your persistent PowerShell window before typing cargo commands for CUDA.
- **Workspace deps**: root `[workspace.dependencies]` **`qlora-rs`** pin must stay aligned with `vox-populi` optional deps. Keep notes in `VOX_PATCH.md` synchronized with whichever qlora-rs patches are active for trainer stability.
- **Input**: `train.jsonl` (and `mens/config/training_contract.yaml` / preflight overrides).
- **Telemetry**: `train_start` includes `train_backend: "burn_lora"` or `"candle_qlora"`. **Candle QLoRA** `train_start` also records **`epochs`**, **`planned_steps_per_epoch`**, **`planned_steps_total`** (upper bound if no vocab/hidden skips). Progress logs (**~5s**): **`ETA_smoothed≈…`** from an **interval throughput EMA** (after step **24**), plus **step/s** and **% of planned** — no duplicate `step 20/40/…` log lines (those are **`telemetry.jsonl` only**). **`step`** rows add **`steps_per_sec_ema`**, **`eta_seconds_remaining`** (EMA-based), **`progress_fraction`**. **`train_complete`**: **`wall_seconds`**, **`mean_steps_per_sec`**. See `telemetry_schema` keys.

## Training objective mismatch (Burn vs Candle)

- **Burn (`--backend lora`)**: full-graph **f32** causal LM on wgpu (or NdArray in tests). Objective = standard next-token CE over the whole decoder graph you enabled.
- **Candle (`--backend qlora`)**: **NF4** frozen bases via qlora-rs with a full-forward training graph over loaded decoder blocks; loss is masked next-token CE on supervised suffix positions (`--qlora-ce-last-k`).
- **Operator impact**: do **not** expect loss / perplexity curves to match Burn. Use `training_manifest.json` **`candle_qlora_graph_id`**, **`candle_qlora_ce_last_k`**, **`training_objective_note`**, telemetry, and tiered **parity tests** (`candle_burn_*`) for **shared f32 primitives** only — not end-to-end NF4-vs-Burn LM identity.

## Burn LoRA vs Candle QLoRA — which path, when (4080 Super and beyond)

### Is QLoRA “better” than Burn LoRA?

**Not universally.** They solve different problems:

| Goal | Prefer |
|------|--------|
| **Train a real Hugging Face base** (e.g. Qwen3.5-4B-Instruct) on **16G VRAM** with industry-style **NF4 + LoRA** | **Candle QLoRA** (`--backend qlora`, `--tokenizer hf`, `--model …`, CUDA build) |
| **Full in-tree f32 causal LM** on **VoxTokenizer JSONL** (docs/examples → pairs), **merge → `vox mens serve`** without an external runtime | **Burn LoRA** (`--backend lora`, legacy path) |
| **Apples-to-apples loss** with “full decoder” next-token CE on the **same** architecture | **Burn** is still the easiest controlled parity lane for the in-tree small model; Candle QLoRA is optimized for real HF checkpoints |

So: **QLoRA is “better” for large-model, VRAM-efficient fine-tuning on shipped HF weights.** **Burn LoRA is “better” for the closed Vox corpus loop and first-class serve/merge in this repo.** You may run **both** in a serious program: Burn for **syntax/docs/tooling-shaped** adapters on the native head; QLoRA for **Qwen-class** behavior on HF bases.

### Should a 4080 Super workstation use Candle CUDA QLoRA?

**Yes, when the target is a real Qwen (or similar) checkpoint** and you have built **`vox-cli` with `gpu,mens-candle-cuda`**. That is the **documented** 16G-class path (preset **`qwen_4080_16g`** / **`--preset 4080`**). Your **Vulkan/wgpu** logs still mean **Burn** is correctly using the GPU; that is **not** a substitute for **Candle CUDA** — different stacks.

### Strengths and weaknesses (persistent reference)

**Burn + wgpu LoRA (`PopuliTrainBackend::BurnLora`)**

| Strengths | Weaknesses |
|-----------|------------|
| **End-to-end Vox story**: corpus JSONL → train → **`merge-weights`** → **`vox mens serve`** (HTTP) on **`*.bin` / `model_merged.bin`**. | Does **not** load arbitrary **multi-billion** HF transformers in **f32** on a 16G card; use QLoRA for that. |
| **Full-graph f32** objective on the **in-repo** `LoraVoxTransformer` (honest CE over the graph you compiled). | **`LoraAttention::merge`** path requires **`use_rope == false`** (GPT-2-style); RoPE stacks stay **unmerged** or need native LoRA at serve time (see top-of-file gaps). |
| **Cross-platform GPU** via wgpu (Vulkan / DX12 / Metal); no NVIDIA CUDA toolchain required. | **Different model** than production Qwen: eval numbers vs HF chat models are **not** directly comparable. |
| Fewer external artifacts: no mandatory **`tokenizer.json` + safetensors`** for the default **`--tokenizer vox`** path. | Optional **`--tokenizer hf`** is **GPT-2-shaped** configs + embed warm-start — still not arbitrary Llama/Qwen **full** weight training in Burn. |

**Candle + qlora-rs QLoRA (`PopuliTrainBackend::CandleQlora`)**

| Strengths | Weaknesses |
|-----------|------------|
| **NF4 base + trainable LoRA** on **real** HF shards; **VRAM-efficient** vs full fine-tune; matches **operator expectations** for “train Qwen locally”. | Native qwen3_5 hybrid path is now enforced in Candle; keep eval-local quality checks in your promotion gate for each model tier. |
| **NVIDIA CUDA** (and Metal) **first-class** when built with **`mens-candle-cuda`** / **`mens-candle-metal`**. | **`vox mens serve`** does **not** load **`merge-qlora`** outputs; use **vLLM / Ollama / HF** (or export pipeline TBD) for merged **f32** shards. |
| Strong **preflight** (`qlora_preflight`) catches tokenizer / embedding width / shard key issues **before** long runs. | **`--qlora-require-full-proxy-stack`** is intentionally strict and can hard-fail when shard coverage is incomplete. |
| **Preset family** (`qwen_4080_16g`, `4080`, etc.) tuned for **16G** cards. | **Patch + contract** coupling: in-tree **`qlora-rs`** patch for stable deep stacks; upgrade pins need care (`VOX_PATCH.md`). |

### Last-minute flight check (before a “real” training push)

Use this as an ordered gate; skip steps that do not apply to your target backend.

1. **Compile**: `cargo check -p vox-cli --features gpu` (Burn + CPU QLoRA baseline). For **CUDA QLoRA on 4080**: `cargo check -p vox-cli --features gpu,mens-candle-cuda` (release build: ensure **`vox.exe`** is not locked by another process on Windows).
2. **CLI/registry drift**: `vox ci command-compliance` (or `cargo run -p vox-cli --features gpu -- ci command-compliance`).
3. **Training acceptance profile**: `cargo run -p vox-cli -- ci mens-gate --profile training` (see [mens-finetune-acceptance-runbook.md](../architecture/mens-finetune-acceptance-runbook.md)).
4. **Language/tooling confidence** (orthogonal to trainer): `cargo check --workspace`, `cargo test` for areas you touched; MCP **`vox-mcp`** and orchestrator paths assume a healthy **`vox`** binary and repo root — see [AGENTS.md](../../../AGENTS.md) § orchestration / capability registry.
5. **Data**: canonical **`train.jsonl`** under **`--data-dir`** (often **`target/dogfood`** after corpus mix). Operator mix (**`vox mens corpus mix --config mens/config/mix.yaml`**) is **strict by default**: every non-optional `mens/config/mix.yaml` source must exist and emit at least one row. Use **`--allow-missing-sources`** for the old warn-only behavior (automation / first-time trees). A JSON report is written next to the mix output (**`*.mix_report.json`**, same stem as the mixed JSONL) with per-source weights, line counts, and output share. Optional: **`VOX_TRAIN_SKIP_CORPUS_MIX=1`** when the JSONL is already final.
6. **Choose artifact + inference**: **Burn** → **`merge-weights`** → **`vox mens serve`**; **QLoRA** → **`merge-qlora`** → external **OpenAI-compatible** or HF runtime (not `serve` today).
7. **Long runs (detached)**: **`--log-dir`** always re-invokes the current binary with logs redirected and the parent exiting immediately. **`--background`** alone does the same using the default log directory (**`<repo>/mens/runs/logs`** when the workspace root is known, else **`mens/runs/logs`** relative to the process cwd). On Windows, spawns use **`CREATE_BREAKAWAY_FROM_JOB`** so IDE/agent job objects are less likely to tear down the trainer when the parent exits. **`vox mens train`** behaves the same (**`--background`** defaults logs to **`mens/runs/logs`**). Monitor with **`Get-Content …\train_*.log -Wait -Tail 25`** or **`tail -f`**. Gate wrappers: **`scripts/populi/release_training_gate.ps1`** (training profile), **`scripts/mens_release_gate.ps1`** (m1m4) — isolated `target` + temp **`vox.exe`** copy to avoid Windows file locks during nested **`cargo`**.

**“Full model build” in practice** means: (a) **data** corpus at quality gate, (b) **trainer** chosen and **manifest** recorded, (c) **merge/export** aligned with **where inference will run** (Vox HTTP vs external LLM), (d) **eval** (`vox mens corpus eval` / `eval-local` where applicable) before promoting artifacts.

## RTX 4080-class CUDA (16G) — canonical QLoRA (copy-paste)

- **Preset**: **`qwen_4080_16g`** (rank 16, seq 384, batch 1, grad_accum 8). CLI **`--preset 4080`** is an **alias** of the same profile (default **`DEFAULT_PRESET`** is **`4080`**).
- **Compile check (CUDA Candle stack)**: `cargo check -p vox-cli --features gpu,mens-candle-cuda` (or `cargo vox-cuda-release`).
- **Train (Qwen3.5-4B example)**:
  `vox mens train --backend qlora --tokenizer hf --preset qwen_4080_16g --model Qwen/Qwen3.5-4B --data-dir target/dogfood --output-dir mens/runs/qwen35_qlora --device cuda --qlora-require-full-proxy-stack`
- **Qwen3.5 ladder guidance (text native phase):**
  - `Qwen/Qwen3.5-0.8B`: use `--preset qwen_4080_16g` (or `--preset auto`), allow longer seq where VRAM permits.
  - `Qwen/Qwen3.5-2B`: same preset family; keep moderate sequence lengths for throughput.
  - `Qwen/Qwen3.5-4B`: canonical 4080 dogfood baseline in this repo.
  - `Qwen/Qwen3.5-9B`: use tighter sequence and higher grad accumulation on 16G; promote on 24G+ tiers.
  - Multimodal training/inference is an explicit next phase and is not included in current native text acceptance.
- **`--device cuda`** without **`mens-candle-cuda`** fails fast at CLI with rebuild instructions.
- **Local-first safety knobs**: `--require-gpu` fails if runtime resolves to CPU; `--allow-cpu-fallback=false` disables automatic fallback for `--device best`.
- **CPU smoke**: `VOX_CANDLE_DEVICE=cpu` forces Candle on CPU for debugging.
- **IDE / Cursor timeouts (long builds + train + gates)**: Hosted agent tools often cap wall time (~tens of seconds to a few minutes). Prefer **detach + log** instead of blocking a single tool invocation on **`mens-gate`** (training profile commonly **5–40+ minutes** depending on cold compile and disk):
  - **Mens gate**: from repo root, **`pwsh scripts/populi/release_training_gate.ps1 -Detach`** or **`pwsh scripts/populi/release_ci_full_gate.ps1 -Detach`** — returns immediately; watch **`target/mens-gate-logs/`**. Same pattern as [`mens_gate_safe.ps1`](../../../scripts/populi/mens_gate_safe.ps1). For quick local signal without the full gate, run a **single** targeted test (examples in **Regression tests** below).
  - **Train**: `vox mens train … --background` or `vox mens train … --log-dir mens/runs/logs` — parent exits immediately; monitor with `Get-Content mens/runs/logs/train_*.log -Wait -Tail 25` (or `tail -f`).
  - **CUDA `cargo` build**: normal terminal or `Tee-Object`; detached build: [`scripts/populi/cursor_background_cuda_build_detached.ps1`](../../../scripts/populi/cursor_background_cuda_build_detached.ps1) (and `scripts/mens/…` copies if present). Example train launcher: [`scripts/populi/cursor_background_train_example.ps1`](../../../scripts/populi/cursor_background_train_example.ps1).
  - **Skip corpus mix** (optional): `VOX_TRAIN_SKIP_CORPUS_MIX=1` skips the pre-train `mix` refresh when you already have the desired `train.jsonl` or need a shorter path under automation.
- **Benchmark telemetry (Codex)**: set **`VOX_BENCHMARK_TELEMETRY=1`** so select CLI paths append unified `benchmark_event` rows (`VoxDb::record_benchmark_event`, session `bench:<repository_id>`): `vox mens bench-completion`, **`vox mens eval-local` only when `vox-cli` is built with feature `gpu`** (CPU-only eval skips telemetry rows), `vox ci build-timings`, optional train gate (`VOX_BENCHMARK` eval-local subprocess), and the ignored `run_benchmark` integration test warm pass. Set **`VOX_REPOSITORY_ROOT`** so subprocess `repository_id` matches MCP when CWD differs. Query via MCP `vox_benchmark_list` when Codex is attached. Syntax-K runs can be routed independently with **`VOX_SYNTAX_K_TELEMETRY=1`** (`metric_type = syntax_k_event`, session `syntaxk:<repository_id>`), with fallback to `VOX_BENCHMARK_TELEMETRY` when unset.
- **JSONL rows**: `vox_tensor::data::TrainingPair` accepts **`instruction`** as alias for **`prompt`** and **`output`** for **`response`** so corpus rows are not silently dropped.
- **Full-graph forward (current implementation)**: one forward pass per row/micro-batch item over loaded decoder layers, then masked CE on supervised suffix positions.
- **Suffix CE (`--qlora-ce-last-k K`)**: default **`64`**. `K=0` uses all supervised assistant positions; `K>0` uses only the last `K` supervised positions from the trimmed sequence.
- **Depth ablation (CLI + digest)**: **`--qlora-proxy-max-layers N`** and **`--qlora-lm-head-only`** still feed **contract digest / planner / preflight** (`candle_qlora_proxy_stack_complete`, graph id). **Candle training rejects** LM-head-only, `proxy_max_layers=0`, and any cap **below** model depth; run without those flags (or set the cap **≥** `num_hidden_layers`) so the trainer runs the **full** proxy graph and the manifest matches execution.
- **Debug**: **`VOX_QLORA_DEBUG_NORMS=1`** prints mean-|activation| after each middle block (stderr; local ablation only).
- **Deferred flags**: `--qlora-lm-head-only` and partial-depth `--qlora-proxy-max-layers` are intentionally not implemented in the current full-graph trainer; keep them for contract/rollout compatibility only.

## Pre-push release gate (acceptance matrix)

- **Canonical (cross-platform)**: `cargo run -p vox-cli -- ci mens-gate --profile training` (add `--profile ci_full` for the wider matrix).  
  Steps live in [`scripts/populi/gates.yaml`](../../../scripts/populi/gates.yaml) (legacy fallback `scripts/mens/gates.yaml`). Nested `cargo` steps use `target/nested-ci` (see flight checklist above).
- **Thin shims**: `pwsh scripts/populi/release_training_gate.ps1`, `pwsh scripts/populi/release_ci_full_gate.ps1`, `pwsh scripts/mens_release_gate.ps1` (m1m4) — all forward to [`scripts/populi/mens_gate_safe.ps1`](../../../scripts/populi/mens_gate_safe.ps1). **Cursor / agent wall-clock limits:** run **`pwsh scripts/populi/release_training_gate.ps1 -Detach`** (or **`release_ci_full_gate.ps1 -Detach`**) so a **new** PowerShell process owns the multi-minute nested `cargo test` work; tail **`target/mens-gate-logs/mens_gate_*.log`**. Optional **`-LogFile C:\path\to\gate.log`** pins the tee path. Bash peers remain where present — mirrors [`mens-finetune-acceptance-runbook.md`](../architecture/mens-finetune-acceptance-runbook.md) rows 1–10 (planner, keymap, strict preflight, Burn smoke, parity tests, merge, `merge_v2`).

## Regression tests

- **Execution planner + hard gates**: `cargo test -p vox-populi execution_planner`
- **QLoRA strict proxy stack (missing middle keys)**: `cargo test -p vox-populi --features mens-train preflight_strict_rejects_missing_o_proj`
- **Fine-tune digest (`qlora_proxy_max_layers`)**: `cargo test -p vox-populi --features mens-train finetune_contract_digest_changes_with_proxy_max_layers`
- **Fine-tune digest (`qlora_ce_last_k`)**: `cargo test -p vox-populi --features mens-train finetune_contract_digest_changes_with_ce_last_k`
- Candle qlora trainer unit tests: `cargo test -p vox-populi --features mens-train`
- **Burn LoRA checkpoint parity tests**: use `vox-tensor` crate unit tests where applicable.
- **Legacy Burn merge parity tests**: kept for historical compatibility only.
- **Burn linear LR warmup** (Burn `LinearLrScheduler`): `cargo test -p vox-tensor --features gpu --lib linear_warmup_sequence_matches`
- **Candle vs Burn f32 parity touchpoints**: `cargo test -p vox-populi --features mens-train --test <parity_test_name>`
- **Tier B NF4 dequant reference parity**: `cargo test -p vox-populi --features mens-train --test candle_burn_nf4_dequant_lm_reference_parity`
- **Candle vs Burn cross-entropy parity**: `cargo test -p vox-populi --features mens-train --test candle_burn_cross_entropy_parity`
- **`merge-qlora` rejects Burn `*.bin`**: `cargo test -p vox-cli merge_qlora_rejects_burn_bin_adapter`
- **`merge-weights`** rejects `candle_qlora_adapter.safetensors` (Burn path only) and points to **`merge-qlora`**: `cargo test -p vox-cli merge_weights_rejects_candle_qlora_adapter_file`
- **`merge-qlora` CLI** synthetic roundtrip: `cargo test -p vox-cli merge_qlora_cli_roundtrip_lm_head_subset`
- Adapter **v2** merge math: `cargo test -p vox-populi --features mens-train merge_v2_applies_lm_head_delta`

## Evaluation protocol (trajectory and cost)

Use a small, repeatable local harness before promoting new training knobs:

- Build a mixed eval set with:
  - baseline code-completion prompts,
  - tool/terminal trajectory prompts,
  - explicit success and failure recovery prompts.
- Run two adjacent configurations:
  - control (`trajectory_weighting_enabled=false`),
  - candidate (trajectory weighting and/or provenance metadata enabled).
- Compare:
  - trajectory pass rate,
  - failure-recovery success rate,
  - mean tokens and wall-clock per successful solve (`cost-per-success` proxy).

Promotion criteria should require non-regressing baseline quality while improving trajectory metrics.

## Rollout gates and env toggles

- `VOX_QWEN35_NATIVE_CUTOVER`
  - `shadow`: allow qwen2 with warning, qwen3_5 preferred.
  - `default` (default): qwen3_5 preferred; qwen2 requires `VOX_ALLOW_QWEN2_NATIVE=1`.
  - `enforced`: reject qwen2 native training.

- `VOX_ORCHESTRATOR_MESH_TRAINING_ROUTING_EXPERIMENTAL`
  - Enables training-task specific route scoring (still local execution only).
- `VOX_ORCHESTRATOR_MESH_TRAINING_BUDGET_PRESSURE`
  - Soft scalar (0.0-1.0) that penalizes expensive training placements under budget pressure.
- `VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL`
  - Existing federation visibility signal; combine with training routing toggle for staged rollout.

Recommended rollout order: shadow (`routing_experimental`), then training scoring (`training_routing_experimental`), then budget pressure tuning.

## Acceptance criteria and rollout protocol

- **A/B baseline:** run control (`trajectory_weighting_enabled=false`) and candidate with the same data + seed envelope.
- **4080-first gate:** local RTX 4080 class run must remain non-regressed before enabling any distributed/cloud knobs.
- **Staged toggles:** enable `VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL` first, then `VOX_ORCHESTRATOR_MESH_TRAINING_ROUTING_EXPERIMENTAL`, then set `VOX_ORCHESTRATOR_MESH_TRAINING_BUDGET_PRESSURE`.
- **Promotion gate:** require non-regressing baseline quality plus improved trajectory/failure-recovery metrics.
- **Cost guardrail:** compare mean wall-seconds and tokens per successful trajectory solve (`cost-per-success` proxy) against baseline.

## Merge / export / inference

| Command / artifact | Status |
|--------------------|--------|
| **`vox mens merge-weights`** | Merges **Burn** LoRA checkpoints (`*.bin` from `--backend lora`) into `model_merged.bin`. Requires **`gpu`**. |
| **`candle_qlora_adapter.safetensors`** | **LoRA A/B per logical layer** (`mid0`…`lm_head`); sidecar **`candle_qlora_adapter_meta.json`** format **`vox_mens_qlora_lora_only_v2`** (`QloraAdapterMetaV2`). |
| **`vox schola merge-qlora`** (alias **`merge-adapter`**) | **Candle QLoRA path only:** merges v2 or **v3** adapter meta + LoRA tensors into **f32** base shards for keys in `base_key_map` (subset output safetensors). Distinct from **`merge-weights`** and from Burn **`*.bin`** checkpoints. There is **no** supported conversion from Burn **`*.bin`** LoRA checkpoints into Candle adapter safetensors for this command — use **`merge-weights`** for Burn → `model_merged.bin`. |
| **`vox mens serve`** (HTTP, `execution-api`) | Loads **Burn** checkpoints: LoRA `*.bin` **or** merged **`VoxTransformer`** (`model_merged.bin` from **`merge-weights`**). Does **not** load Candle **`merge-qlora`** output safetensors; use HF/Ollama/vLLM or another stack for merged QLoRA f32 shards. |
| **`populi_adapter_manifest_v3.json`** | Unified adapter manifest (method + quant + layer order + `base_key_map`); written beside v2 meta on Candle runs. |
| **Full causal NF4 + PEFT parity** | Open work — deeper block coverage beyond o_proj proxy stack. |

## Related

- **LLM / agent PR hygiene:** [`mens-llm-pr-checklist.md`](../architecture/mens-llm-pr-checklist.md) — LoRA duplication, layouts, merge, CI test names, parity tiers.
- **LoRA ownership boundary:** [`mens-lora-ownership.md`](mens-lora-ownership.md)
- **Speech / ASR** (Oratio): [`oratio-speech.md`](oratio-speech.md) — orthogonal to training; use top-level **`vox oratio`** / **`vox speech`**. CLI STT commands need **`vox-cli`** feature **`oratio`** (not default **`mens-base`**).

