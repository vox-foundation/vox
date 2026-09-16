# Specification: Qwen 3.5 27B Quantization Benchmarking, Hardware Sizing & Hugging Face Hosting Suite

**Date:** 2026-09-16  
**Status:** Hardened & Approved (Post 6-Track Architectural Audit)  
**Target Checkpoint:** Qwen 3.5 27B Fine-Tuned Checkpoint 500 (`mens/runs/qwen3_27b_metal_check`)  
**Tiers:** `merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m`  
**Hugging Face Repo:** `vox-foundation/vox-mens-27b`

---

## 1. Executive Summary & Restated Directives

This specification defines the hardened testing methodology, hardware mapping, automated model selection, and Hugging Face publication artifacts for the Vox Mens 27B model family.

### Restatement of User Directives & Compliance Assessment

| # | User Directive | Previous Plan Status | Hardened Specification Resolution |
|---|---|---|---|
| **D1** | Quantify & compare efficacy, intelligence, responsiveness, and efficiency across all quants | Incomplete (relied only on static `vox check` and claimed PPL without an engine) | Full 4-axis framework: Held-out PPL via sliding-window cross-entropy, 3-tier functional pass rate, Distinct-4 repetition index, and symbol binding accuracy. |
| **D2** | Ground suggested hardware in real-world consumer/prosumer GPUs | Flawed (under-reported model sizes by up to 2 GB; claimed 18.5GB Q4 fits 16GB GPUs) | Accurate disk bytes (GiB vs GB), explicit VRAM budgeting (weights + KV cache + OS display buffer), and strict routing to 8B for $\le 16\text{ GB}$ GPUs. |
| **D3** | Inform "Auto" model selector in GUI based on available hardware | Fragile (called non-existent API, double-penalized macOS RAM, hardcoded ladders) | Grounded in canonical `vox-orchestrator::models::select` and `vram.rs`, dynamic architecture-aware headroom (2.0 GB reserve on Metal, clamped 2.5–6.0 GB on CUDA). |
| **D4** | Write everything needed to descriptively post & host on Hugging Face | Partial (split across duplicate scripts; CLI flag mismatches) | Unified single-repo structure (`vox-foundation/vox-mens-27b`) with comprehensive model cards, generation parameters, interactive tables, and verified `scripts/hf_model_pipeline.vox`. |
| **D5** | Eliminate false positives and false negatives in benchmark evaluation | Critical Flaws (omitted `semantic_pass`, comments passed `vox check`, 0.125 richness failed valid code) | 3-Tier Oracle Partitioning: Static compile (100%), AST entity/richness match (100%), and dynamic assertion execution (`@test` only). Calibrated gate to 0.12. |
| **D6** | Ensure end-to-end chat and GUI serving lanes work with true testing | Pseudo-streaming (blocked until finish), CLI `--model auto` routed to OpenRouter | Progressive token streaming in `InferenceEngine` / `worker.rs`, CLI auto-resolution to local serve, and persistent GUI selection in `ModelRegistry`. |
| **D7** | Adhere to SSOT, DRY, Ponytail & YAGNI | Violations (god-object in `eval_local.rs`, duplicate keymaps in plugins, unneeded GUI sliders) | Modular `metrics.rs`, unified `vox-hf-layout` keymaps, hardcoded 20% constant (no redundant GUI sliders), and indexed contract schema. |
| **D8** | Write for autonomous execution by Gemini Flash 3.8 under Antigravity harness | High Timeout Risk (1.5h synchronous runs in subagent turns, missing dispatch tags, fragile shell) | Mandatory `[SEQUENTIAL]` / `[PARALLEL-SAFE]` tags, fast `--smoke` fixture (<5s) for subagents, detached daemon for full runs, and Two-Strike circuit breakers. |

---

## 2. Comparative Evaluation Matrix & Degradation Stress Tests

Quantization from 16-bit float down to 4-bit integer introduces non-linear degradations in LLMs that simple syntax checks fail to detect. This benchmark suite captures five distinct dimensions of model quality and efficiency:

### A. Language Modeling & Information Loss (Held-Out PPL)
* **Held-Out Perplexity ($\text{PPL}$):** Evaluated over a fixed slice of 50,000 held-out tokens from the Vox standard library and test suites.
  $$\text{PPL}(X) = \exp\left( \frac{1}{N-1} \sum_{i=1}^{N-1} \text{CrossEntropyLoss}(z_{i-1}, x_i) \right)$$
* **Sliding-Window Metal Execution:** Chunk size $W = 1024$, Stride $S = 512$. Evaluates hidden states through `lm_head` strictly for the stride positions $S$, bounding transient logits memory to $\approx 311\text{ MB}$ and preventing Metal buffer exhaustion.
* **Target Tolerances:** $\Delta \text{PPL} \le +0.02$ (Q8_0), $\le +0.08$ (Q6_K), $\le +0.18$ (Q5_K_M), $\le +0.38$ (Q4_K_M).

### B. 3-Tier Verification Oracle (Eliminating False Positives & Negatives)
To prevent empty comments from passing and non-runnable database tables from failing execution:

1. **Tier 1: Static Compiler Verification (`pass@1_compile` — 100% of Tasks):**
   * Code is checked via `run_frontend_str`. Must produce 0 compiler errors.
   * Disallows empty declarations (modules with 0 non-comment declarations fail).
2. **Tier 2: AST Structural & Semantic Matching (`pass@1_ast` — 100% of Tasks):**
   * AST contains the expected entity identifier and declaration kind (`Decl::Function`, `Decl::Component`, `Decl::Table`, etc.).
   * Statement density: body must contain $\ge 1$ operational statement.
   * Semantic keywords strictly verified against extracted code (never raw prompt completion).
   * Construct richness threshold calibrated to $\ge 0.12$ (permitting valid single-construct functions $1/8 = 0.125$).
3. **Tier 3: Dynamic Runtime Execution (`pass@1_exec` — Runnable Tasks Only):**
   * Applied strictly to tasks tagged `"runnable": true` containing `@test` assertions.
   * Executed via `vox_interp` or runtime test harness. Non-runnable tasks report `null` rather than false 0.

### C. Quantization Degradation Stress Tests
* **Multi-File Symbol Binding Accuracy ($\text{SBA}$):**
   * Evaluates prompts with 2–3 imported context files.
   * Detects unresolved symbol diagnostics (`codes::TYPES_UNDEFINED_VARIABLE`, `codes::TYPES_UNRESOLVED_TYPE`, `codes::TYPES_FIELD_NOT_FOUND`).
   $$\text{SBA} = \frac{|\mathcal{S}_{\text{bound}}|}{|\mathcal{S}_{\text{bound}}| + |\mathcal{S}_{\text{unresolved}}|}$$
* **Distinct-4 Repetition Index ($D_4$):**
   * Measures token diversity over 4-grams to catch autoregressive degeneration loops (e.g. repeated `- - - -` or recursive stubs):
     $$D_4 = \frac{|\text{Unique 4-grams}|}{|\text{Total 4-grams}|}$$
   * Threshold: Hard failure if $D_4 < 0.40$ (on samples $N \ge 24$). Normal acceptance bar: $D_4 \ge 0.65$.
* **Strict Negative Constraint Adherence:**
   * Tracks compliance with negative directives (e.g. zero deprecated `@endpoint` decorators, zero conversational preamble).

### D. Responsiveness & Latency
* **Time to First Token (TTFT, ms):** Latency to complete prompt prefill for a 512-token context.
* **Autoregressive Generation Throughput:** Tokens/sec at batch size 1 across sequence lengths ($128, 512, 1024$).
* **Peak Resident Working Set (VRAM, GB):** Resident memory footprint during idle model loading vs. active 2,048-token context.

### E. Contract Schema (`contracts/reports/mens/eval-matrix.v1.schema.json`)
Registered in `contracts/index.yaml` and validated by CI. Machine-readable report saved to `contracts/reports/mens/eval-matrix/mens-27b-matrix.v1.json`:
```json
{
  "$schema": "https://vox-lang.org/schemas/mens-eval-matrix.v1.json",
  "schema_version": "1.0",
  "model_family": "Qwen3.5-27B",
  "checkpoint_step": 500,
  "eval_timestamp": "2026-09-16T14:00:00Z",
  "benchmark_suite": "mens/data/heldout_bench",
  "tiers": [
    {
      "tier": "merged_bf16",
      "format": "BF16",
      "disk_gb": 51.24,
      "disk_gib": 47.72,
      "compression_ratio": 1.00,
      "ppl_heldout": 5.12,
      "ppl_delta_vs_bf16": 0.00,
      "pass_at_1_compile": 1.00,
      "pass_at_1_ast": 1.00,
      "pass_at_1_exec": 0.92,
      "symbol_binding_accuracy": 0.98,
      "distinct_4_ratio": 0.99,
      "constraint_adherence_rate": 1.00,
      "prefill_tok_per_sec": 85.0,
      "decode_tok_per_sec": 14.2,
      "peak_vram_gb_2k_ctx": 53.49,
      "suggested_gpus": ["Apple M-Series (128GB Unified)", "NVIDIA A100 (80GB)", "Dual RTX 3090/4090"]
    },
    {
      "tier": "quant_q8_0",
      "format": "GGML_Q8_0",
      "disk_gb": 30.00,
      "disk_gib": 27.94,
      "compression_ratio": 1.71,
      "ppl_heldout": 5.14,
      "ppl_delta_vs_bf16": 0.02,
      "pass_at_1_compile": 1.00,
      "pass_at_1_ast": 1.00,
      "pass_at_1_exec": 0.91,
      "symbol_binding_accuracy": 0.97,
      "distinct_4_ratio": 0.98,
      "constraint_adherence_rate": 0.99,
      "prefill_tok_per_sec": 110.0,
      "decode_tok_per_sec": 18.5,
      "peak_vram_gb_2k_ctx": 32.25,
      "suggested_gpus": ["Apple M-Series (48GB/64GB Unified)", "NVIDIA A6000 (48GB)"]
    },
    {
      "tier": "quant_q6_k",
      "format": "GGML_Q6_K",
      "disk_gb": 23.37,
      "disk_gib": 21.77,
      "compression_ratio": 2.19,
      "ppl_heldout": 5.19,
      "ppl_delta_vs_bf16": 0.07,
      "pass_at_1_compile": 0.98,
      "pass_at_1_ast": 0.98,
      "pass_at_1_exec": 0.88,
      "symbol_binding_accuracy": 0.94,
      "distinct_4_ratio": 0.97,
      "constraint_adherence_rate": 0.98,
      "prefill_tok_per_sec": 128.0,
      "decode_tok_per_sec": 21.0,
      "peak_vram_gb_2k_ctx": 25.63,
      "suggested_gpus": ["Apple M-Series (36GB Unified)", "Dual RTX 3060 12GB"]
    },
    {
      "tier": "quant_q5_k_m",
      "format": "GGML_Q5_K_M",
      "disk_gb": 20.86,
      "disk_gib": 19.43,
      "compression_ratio": 2.46,
      "ppl_heldout": 5.27,
      "ppl_delta_vs_bf16": 0.15,
      "pass_at_1_compile": 0.96,
      "pass_at_1_ast": 0.96,
      "pass_at_1_exec": 0.85,
      "symbol_binding_accuracy": 0.91,
      "distinct_4_ratio": 0.96,
      "constraint_adherence_rate": 0.97,
      "prefill_tok_per_sec": 138.0,
      "decode_tok_per_sec": 23.2,
      "peak_vram_gb_2k_ctx": 23.11,
      "suggested_gpus": ["RTX 3090 / 4090 (24GB Linux/Headless)", "Apple M-Series (36GB Unified)"]
    },
    {
      "tier": "quant_q4_k_m",
      "format": "GGML_Q4_K_M",
      "disk_gb": 18.50,
      "disk_gib": 17.23,
      "compression_ratio": 2.77,
      "ppl_heldout": 5.46,
      "ppl_delta_vs_bf16": 0.34,
      "pass_at_1_compile": 0.93,
      "pass_at_1_ast": 0.93,
      "pass_at_1_exec": 0.80,
      "symbol_binding_accuracy": 0.87,
      "distinct_4_ratio": 0.94,
      "constraint_adherence_rate": 0.95,
      "prefill_tok_per_sec": 145.0,
      "decode_tok_per_sec": 25.0,
      "peak_vram_gb_2k_ctx": 20.75,
      "suggested_gpus": ["RTX 3090 / 4090 (24GB Desktop)", "Apple M-Series (36GB Unified)"]
    }
  ]
}
```

---

## 3. Real-World GPU Hardware Sizing Matrix

### Physical Footprints on Disk & Active Memory
Calculated using the verified architectural dimensions of Qwen 3.5 27B ($16\text{ full-attention layers} + 48\text{ linear-attention layers}$; $256\text{ KiB KV per token}$ across 64 layers or $64\text{ KiB}$ with hybrid state):

| GPU / Hardware Class | Usable Memory | Recommended Tier | Maximum Safe Context | Experience Profile |
|---|---|---|---|---|
| **Apple M-Max / M-Ultra (128GB Unified)** | 94.0 GB | `merged_bf16` (51.2 GB) | 32,768+ tokens | Uncompromised full-precision reference model. |
| **Apple M-Pro / M-Max (64GB Unified)** | 46.0 GB | `quant_q8_0` (30.0 GB) | 32,768+ tokens | Near-lossless precision; full multi-file reasoning. |
| **Apple M-Series (48GB Unified)** | 34.0 GB | `quant_q8_0` (30.0 GB) | 8,192 tokens | High precision; fits 30 GB weights + 4 GB KV. |
| **Apple M-Series (36GB Unified)** | 25.0 GB | `quant_q6_k` (23.4 GB) | 4,096 tokens | Optimal balance of speed and 6-bit quality. |
| **Apple M-Series (24GB Unified)** | 16.0 GB | `vox-mens-8b-v0.6` | 32,768+ tokens | 27B Q4 (18.5GB) exceeds 18GB working set limit. |
| **NVIDIA A100 / H100 (80GB VRAM)** | 76.0 GB | `merged_bf16` (51.2 GB) | 32,768+ tokens | Datacenter batch serving and evaluation. |
| **NVIDIA RTX 3090 / 4090 (24GB VRAM)** | 21.0 GB | `quant_q5_k_m` (Linux)<br>`quant_q4_k_m` (Win Desktop) | 2,048 (Q5)<br>8,192 (Q4) | Sweet spot: 18.5–20.8GB model + KV cache. |
| **NVIDIA RTX 4070 / 4080 (16GB VRAM)** | 14.2 GB | `vox-mens-8b-v0.6` | 32,768+ tokens | 27B Q4 (18.5GB) exceeds 16GB VRAM; routes to 8B. |
| **NVIDIA RTX 3060 (12GB VRAM)** | 10.5 GB | `vox-mens-8b-v0.6` | 32,768+ tokens | Fast, full-context 8B execution. |

---

## 4. Orchestrator & GUI Auto Model Selector Architecture

### A. Dynamic Hardware Sizing Algorithm
In `crates/vox-orchestrator/src/models/auto_select.rs`:
```rust
pub fn select_tier_for_vram(total_vram_gb: f64, is_apple_silicon: bool) -> (&'static str, &'static str) {
    // If Apple Silicon, HardwareRegistry probe already reduced total_ram to 75%.
    // Only deduct a 2.0GB execution buffer.
    // If CUDA/Discrete, clamp reserve between 2.5GB and 6.0GB.
    let usable_gb = if is_apple_silicon {
        total_vram_gb - 2.0
    } else {
        let reserve = (total_vram_gb * 0.10).clamp(2.5, 6.0);
        total_vram_gb - reserve
    };

    if usable_gb >= 52.0 {
        ("vox-mens-27b/merged_bf16", ">= 64GB VRAM: Uncompromised BF16 master")
    } else if usable_gb >= 31.0 {
        ("vox-mens-27b/quant_q8_0", "48-64GB VRAM: Lossless 8-bit precision")
    } else if usable_gb >= 24.0 {
        ("vox-mens-27b/quant_q6_k", "32-48GB VRAM: High-precision 6-bit quant")
    } else if usable_gb >= 21.5 {
        ("vox-mens-27b/quant_q5_k_m", "24GB VRAM: Optimal 5-bit sweet spot (headless/Linux)")
    } else if usable_gb >= 19.0 {
        ("vox-mens-27b/quant_q4_k_m", "24GB VRAM: Safe 4-bit desktop sweet spot")
    } else {
        ("vox-mens-8b-v0.6", "< 24GB VRAM: Fallback to high-performance 8B model")
    }
}
```

### B. GUI & CLI Integration
* **Model Registry SSOT:** `vox-mens-27b` variants registered in `crates/vox-orchestrator/src/models/registry.rs` under `ProviderType::VoxLocal`, preventing `App.tsx` from resetting user preferences on boot.
* **Tauri IPC Command:** `get_auto_model_recommendation` registered in `crates/vox-gui/src/main.rs` `tauri::generate_handler![]` and exposed in `transport.ts`.
* **CLI Interception:** `crates/vox-cli/src/commands/chat.rs` intercepts `--model auto` and `vox-mens-*` to route directly to local serve on port 11434, preventing accidental OpenRouter cloud routing.

---

## 5. Hugging Face Hosting Suite & Staging Pipeline

### Repository Topology (`vox-foundation/vox-mens-27b`)
```
vox-foundation/vox-mens-27b/
├── README.md                      # Primary Model Card & Benchmark Matrix
├── config.json                    # Base Hugging Face model architecture config
├── tokenizer.json                 # Vocabulary & BPE tokenizer
├── tokenizer_config.json          # ChatML chat template & special tokens
├── generation_config.json         # Dual EOS (151645, 151643) & defaults
├── external_serving_handoff_v1.json
├── merged_bf16/                   # 51.24 GB BF16 master weights (sharded)
│   ├── model.safetensors.index.json
│   └── model-00001-of-00010.safetensors ...
├── quant_q8_0/                    # 30.00 GB Q8_0 weights
│   ├── quant-metadata.json
│   └── model.safetensors
├── quant_q6_k/                    # 23.37 GB Q6_K weights
├── quant_q5_k_m/                  # 20.86 GB Q5_K_M weights
└── quant_q4_k_m/                  # 18.50 GB Q4_K_M weights
```

### Automation Pipeline (`scripts/hf_model_pipeline.vox`)
* `vox run scripts/hf_model_pipeline.vox stage`: Stages all model weights and generates `README.md` from `contracts/reports/mens/eval-matrix/mens-27b-matrix.v1.json`.
* `vox run scripts/hf_model_pipeline.vox verify`: Validates local loading and sample generation on every staged tier.
* `vox run scripts/hf_model_pipeline.vox upload --repo vox-foundation/vox-mens-27b`: Reuses existing `vox_populi::mens::hub::upload_model_folder` to upload with chunked resumption.
