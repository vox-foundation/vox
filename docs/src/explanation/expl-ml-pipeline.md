---
title: "Native ML Training Pipeline"
description: "Official documentation for Native ML Training Pipeline for the Vox language. Detailed technical reference, architecture guides, and imple"
category: "explanation"
last_updated: 2026-03-24
training_eligible: true
---

# Native ML Training Pipeline

Vox "dogfoods" itself: the language, compiler, and documentation all feed a native machine learning loop that trains the **Populi** code assistant model. The **default** native path is [Burn](https://burn.dev) + **wgpu** (Vulkan / DX12 / Metal) — **no Python** and **no CUDA required** for that path.

**Two first-class trainers** share **`vox populi train`**: **Burn LoRA** (above) and optional **Candle + qlora-rs QLoRA** on **Hugging Face** weights (**CUDA/Metal optional**, **`--backend qlora`**). They are **not** interchangeable objectives or artifacts — see [Populi training SSOT — Burn vs QLoRA](../reference/populi-training.md#burn-lora-vs-candle-qlora--which-path-when-4080-super-and-beyond).

**Default GPU acceleration** for **`--backend lora`** uses **wgpu**, not NVIDIA CUDA. For **QLoRA on an RTX-class workstation**, build **`populi-candle-cuda`** and use **`--device cuda`**. Use CPU-only training when drivers or CI forbid GPU.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  DATA SOURCES                                               │
│  examples/*.vox ──────────┐                                 │
│  docs/src/*.md (code  ────┤──► vox populi corpus extract   │
│    blocks with frontmatter)│         │                       │
│  vox-cli generate-data ───┘         │                       │
└─────────────────────────────────────│───────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────┐
│  CORPUS PIPELINE                                            │
│  populi/data/validated.jsonl   (raw Vox → instruction pairs)│
│        │                                                    │
│        ▼                                                    │
│  vox populi corpus validate    (filter malformed pairs)     │
│        │                                                    │
│        ▼                                                    │
│  populi/data/train.jsonl       (rated + filtered pairs)     │
└─────────────────────────────────────│───────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────┐
│  TRAINING                                                  │
│                                                             │
│  Default: Native Burn LoRA (Rust) — GPU, no CUDA, no Python │
│  **`vox populi train`** (canonical CLI; `train.jsonl`)       │
│  → `LoraVoxTransformer` + wgpu (Vulkan/DX12/Metal)          │
│                                                             │
│  **`--backend qlora`**: Candle + **qlora-rs** (NF4 LM head + │
│  LoRA; mmap `f32` HF embeds). Optional CUDA/Metal features.  │
│  SSOT: `reference/populi-training.md`.                │
│                                                             │
│  Legacy: `vox train` (when `populi-dei` + `gpu`) — local     │
│  bails to **`vox populi train --backend qlora`**; Together  │
│  remote; **`--native`** Burn scratch (not Candle QLoRA).      │
└─────────────────────────────────────────────────────────────┘
                                      ▼
┌─────────────────────────────────────────────────────────────┐
│  EVAL + BENCHMARK GATES                                     │
│  vox populi corpus eval … → eval_results.json               │
│  VOX_BENCHMARK=1 → spawns vox populi eval-local (held-out)  │
│  Targets: vox_parse_rate ≥70%, coverage ≥50% (CI); VOX_EVAL_STRICT=1 fails promotion │
│  Held-out: VOX_BENCHMARK=1, VOX_BENCHMARK_MIN_PASS_RATE (default 0) │
└─────────────────────────────────────────────────────────────┘
```

---

## Data Schema

All training pairs follow this JSONL schema (must match across all tools):

```json
{
  "prompt": "Write a Vox actor that tracks a counter",
  "response": "actor Counter:\n    state count: int = 0\n    on increment() to int:\n        count = count + 1\n        count",
  "category": "actor",
  "rating": 5,
  "schema_version": "vox_dogfood_v1"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `prompt` | string | ✅ | The instruction/question (serde also accepts **`instruction`**) |
| `response` | string | ✅ | Valid Vox code (serde also accepts **`output`**) |
| `category` | string | recommended | Construct type (function, actor, etc.) |
| `rating` | u8 1-5 | recommended | Quality rating; 5=ground truth docs |
| `schema_version` | string | optional | Version for migration tracking |

---

## Tokenizer

`vox-tensor` includes a **deterministic, dependency-free character-level tokenizer** (`VoxTokenizer`):

- **95 printable ASCII characters** (IDs 3-97)
- **35 Vox compound tokens** (workflow, actor, fn , @component, etc.)
- **3 control tokens**: `[PAD]=0`, `[UNK]=1`, `[EOS]=2`
- **Total vocab**: 133 tokens

```vox
// Vox example — tokenized natively using VoxTokenizer
fn greet(name: str) to str:
    "Hello, " + name
```

Encoding uses greedy longest-match on compound tokens before falling back to single chars.

---

## VoxTransformer Architecture

The native Burn-backed model (`crates/vox-tensor/src/vox_nn.rs`, `gpu` feature):

| Parameter | Value | Notes |
|-----------|-------|-------|
| Layers | 12 | Transformer encoder blocks |
| Attention heads | 8 | Multi-head self-attention |
| Model dimension | 512 | Embedding size |
| FFN dimension | 2048 | Feed-forward inner size |
| Dropout | 0.1 | Applied in attention + FFN |
| Max sequence length | 512 | Tokens per training example |
| Vocab size | 133 | VoxTokenizer vocabulary |

---

## Running the Pipeline

### 1. Generate synthetic training data

```bash
vox generate-data --limit 500 --output populi/data/train.jsonl
```

### 2. Extract corpus from real Vox files (canonical flow, PowerShell)

```powershell
.\target\release\vox.exe corpus extract examples/ -o populi/data/validated.jsonl
.\target\release\vox.exe corpus extract docs/ -o populi/data/validated.jsonl 2>$null
.\target\release\vox.exe corpus validate populi/data/validated.jsonl --no-recheck -o populi/data/validated.jsonl
.\target\release\vox.exe corpus pairs populi/data/validated.jsonl -o target/dogfood/train.jsonl --docs docs/src/ --docs docs/src/research/ --docs docs/src/adr/
# Rustdoc merge skipped: response is Rust prose, not Vox code
```

### 3. Start local training (native Rust — GPU by default, no CUDA/Python)

```powershell
# Uses wgpu (Vulkan/DX12/Metal); no CUDA or Python required
.\target\release\vox.exe train --data-dir target/dogfood --output-dir populi/runs/v1
# For CI or CPU-only:
$env:VOX_BACKEND="cpu"; .\target\release\vox.exe train --data-dir target/dogfood --output-dir populi/runs/v1
```

### 4. Check eval gate

```powershell
.\target\release\vox.exe populi corpus eval target/dogfood/train.jsonl -o populi/runs/v1/eval_results.json
```

---

## Documentation → Training Pair Loop

Every documentation page with `training_eligible: true` in its frontmatter and a ` ```vox ` code block automatically contributes training pairs via `vox populi corpus pairs --docs docs/src/`.

This creates a **closed feedback loop**: better docs → more training data → better model → better completions → easier to write docs.

**Frontmatter format for training-eligible docs**:

```yaml
---
title: "My Guide"
category: how-to
constructs: [function, workflow]
training_eligible: true
difficulty: intermediate
---
```

---

## CI Integration

The ML pipeline runs automatically via `.github/workflows/ml_data_extraction.yml`:

- **Nightly**: Full corpus re-extraction at 4 AM UTC
- **On push**: Triggered when `*.vox`, compiler crates, or `docs/src/**` change
- **Manual**: `workflow_dispatch` with `force_train` or `native_train` option
- **Grammar drift**: Fingerprint check forces full re-extraction when syntax changes

### CI training job (GPU runner)

The **train** job runs on a self-hosted GPU runner when corpus changes or when manually triggered:

- **Native path (default)**: Prefer **`vox populi train`** with `VOX_BACKEND=cpu` for CI compatibility. Older workflows may still invoke **`vox train`**; **`--provider local`** now **bails** with the canonical Candle QLoRA command (no Python `train_qlora` script).
- **Workflow_dispatch `native_train: false`**: If still wired to **`vox train --provider local`**, expect the **bail** message directing operators to **`vox populi train --backend qlora`**. Use **`vox populi train`** directly in updated automation.
- **Eval strict mode**: `VOX_EVAL_STRICT=1` — training fails when eval gate thresholds are not met.
- **Benchmark gate**: `VOX_BENCHMARK=1` — runs held-out benchmark from `populi/data/heldout_bench/`; `VOX_BENCHMARK_MIN_PASS_RATE` (e.g. 0.80) fails promotion when pass rate is below threshold.
- **Artifact retention**: LoRA adapter `target/dogfood/run/` uploaded as `lora-adapter-$VCS_SHA`, retained 90 days. Eval results `eval_results.json` / `eval_gate_failed.json` retained 30 days.
- **Logging**: Training pair count and eval gate result (parse rate, coverage) are printed; eval gate failure writes `eval_gate_failed.json` and emits a warning.

### Runbook: Native training in CI

```bash
# CI uses VOX_BACKEND=cpu by default (no GPU drivers required)
VOX_BACKEND=cpu vox populi train --data-dir target/dogfood --output-dir target/dogfood/run
```

### Runbook: Evol-Instruct (optional, gated)

**Not wired** on the current slim `vox` binary. Use external tooling or scripts until a `corpus evol` subcommand lands.

```bash
# Intended future shape (not implemented):
# EVOL_GATE=1 vox populi corpus evol …
```

### Runbook: Optional extra corpus merge

Use **`vox populi corpus mix`** with `populi/config/mix.yaml`, or merge JSONL with your own tooling. There is no `vox corpus merge` subcommand today.

### Train matrix (canonical)

| Mode | Command | When to use |
|------|---------|-------------|
| Native Populi (Burn) | `vox populi train …` (`--backend lora`; `--tokenizer vox` default or `--tokenizer hf` for GPT-2-shaped HF) | Burn LoRA + wgpu; Vox ChatML or HF tokenizer + optional embed warm-start |
| Native Populi (Candle QLoRA) | `vox populi train --backend qlora --tokenizer hf --model <hf_repo> …` | Candle + **qlora-rs NF4** proxy stack + mmap `f32` embeds; CUDA/Metal optional |
| Qwen2.5-Coder (4080 16GB) | `cargo build -p vox-cli --release --features gpu,populi-candle-cuda` then `vox populi train --backend qlora --tokenizer hf --preset qwen_4080_16g --model Qwen/Qwen2.5-Coder-3B-Instruct --device cuda …` | Production-oriented QLoRA preset; strict proxy stack optional via `--qlora-require-full-proxy-stack` |
| Legacy `vox train` | `vox train …` (build `--features populi-dei`) | **`--provider local`** → bail + **`vox populi train --backend qlora`** copy-paste; Together remote; **`--native`** Burn scratch |
| CI strict | `VOX_EVAL_STRICT=1` | Fail promotion on eval gate failure |
| CI benchmark | `VOX_BENCHMARK=1` | Run held-out benchmark before promotion |

Artifact layout: `target/dogfood/train.jsonl` (canonical input), `target/dogfood/run/` (output). Version naming: `lora-adapter-$VCS_SHA`, `eval-gate-$VCS_SHA`.

---

## Next Steps

- [Actors & Workflows](expl-actors-workflows.md) — Build durable constructs for the training pipeline
- [CLI Reference](../reference/cli.md) — Implemented `vox` subcommands (`populi`, optional `train`)
- [Architecture Overview](expl-architecture.md) — How the compiler pipeline works
