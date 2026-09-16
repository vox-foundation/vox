# Specification: Qwen 3.5 27B Quantization Benchmarking, Hardware Recommendation Matrix & Hugging Face Hosting Suite

**Date:** 2026-09-16  
**Status:** In Review (Brainstorming Path: Architectural)  
**Target Checkpoint:** Qwen 3.5 27B Fine-Tuned Checkpoint 500 (`mens/runs/qwen3_27b_metal_check`)  
**Tiers:** `merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m`  
**Hugging Face Repo:** `vox-foundation/vox-mens-27b`

---

## 1. Executive Summary & Goals

This specification defines the testing methodology, reporting standards, automated hardware selection heuristics, and Hugging Face publication artifacts for the Vox Mens 27B fine-tuned code model.

### Core Objectives
1. **Empirical Quantization Quantification:** Establish an objective, multi-axis benchmark suite that measures standard language modeling metrics (Perplexity, Functional Pass@1) alongside degradation modes typical of low-bit quantization (symbol resolution drift, autoregressive repetition, and constraint violations).
2. **Hardware Sizing & Guidance:** Provide clear, empirical recommendations matching each quantization tier to consumer and prosumer GPUs across Apple Silicon (Unified Memory) and NVIDIA (CUDA VRAM).
3. **Auto Model Selector:** Implement an intelligent, hardware-aware model picker in `vox-orchestrator` and `vox-gui` that dynamically selects the optimal model tier based on detected GPU memory with a 20% safety headroom buffer.
4. **Hugging Face Hosting Suite:** Deliver a unified repository structure (`vox-foundation/vox-mens-27b`) with comprehensive model cards, generation parameters, interactive benchmark comparisons, and an automated publishing pipeline.

---

## 2. Comparative Evaluation Matrix & Degradation Stress Tests

Quantization from 16-bit float down to 4-bit integer introduces non-linear degradations in LLMs that simple syntax checks fail to detect. This benchmark suite captures five distinct dimensions of model quality and efficiency:

### A. Standard Language Modeling & Information Loss
* **Held-Out Perplexity ($\text{PPL}$):** Evaluated over a fixed slice of 50,000 held-out tokens from the Vox standard library and test suites.
  $$\Delta \text{PPL} = \text{PPL}_{\text{quant}} - \text{PPL}_{\text{bf16}}$$
  * *Target Tolerances:* $\Delta \text{PPL} \le +0.02$ (Q8_0), $\le +0.08$ (Q6_K), $\le +0.18$ (Q5_K_M), $\le +0.38$ (Q4_K_M).

### B. Functional Correctness (Static & Runtime Execution)
* **Static Verification Rate ($\text{pass@1}_{\text{compile}}$):** Code compiles with 0 errors via the Vox compiler frontend (`vox check`).
* **Runtime Execution Pass Rate ($\text{pass@1}_{\text{exec}}$):** The generated code is executed against unit test assertions (`vox test` / runtime interpreter) testing algorithmic correctness, boundary values, and state transitions.

### C. Quantization Degradation Stress Tests
* **Long-Context Symbol Binding Accuracy:**
  * Tests multi-file prompts with 3+ imported context files containing multiple types and function signatures.
  * *Metric:* **Unresolved Identifier Rate** (detects whether reduced-precision KV cache and RoPE cause the model to hallucinate or misbind variable names defined earlier in the prompt).
* **Autoregressive Degeneration & Repetition Trap Detection:**
  * *Metric:* **Distinct-4 N-Gram Ratio** ($D_4 = \frac{\text{unique 4-grams}}{\text{total 4-grams}}$).
  * Measures whether logit clipping in 4-bit / 5-bit layers triggers repetitive token loops or infinite punctuation (`- - - -`, `////`, recursive stubs).
* **Strict Negative Constraint Adherence:**
  * Tests prompt instructions with explicit negative constraints (e.g. "Do NOT use deprecated decorators like `@endpoint`", "Only return bare `table` declarations", "Respond with pure code and no conversational commentary").
  * *Metric:* **Constraint Violation Rate** ($\%$ of outputs containing forbidden tokens or commentary).

### D. Responsiveness & Latency
* **Time to First Token (TTFT, ms):** Latency to complete prompt prefill for a 512-token context.
* **Autoregressive Decode Throughput ($\text{tok/s}$):** Steady-state token generation rate at batch size 1.
* **Peak Resident Working Set (VRAM, GB):** Resident memory footprint during idle model loading vs. active 2,048-token context.

### E. Machine-Readable Schema (`contracts/reports/mens-27b-matrix.v1.json`)
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
      "disk_gb": 51.2,
      "compression_ratio": 1.00,
      "ppl_heldout": 5.12,
      "ppl_delta_vs_bf16": 0.00,
      "pass_at_1_compile": 1.00,
      "pass_at_1_exec": 0.92,
      "symbol_binding_accuracy": 0.98,
      "distinct_4_ratio": 0.99,
      "constraint_adherence_rate": 1.00,
      "prefill_tok_per_sec": 85.0,
      "decode_tok_per_sec": 14.2,
      "peak_vram_gb_2k_ctx": 55.4,
      "suggested_gpus": ["Apple M-Series (96GB/128GB)", "NVIDIA A100 (80GB)", "Dual RTX 3090/4090"]
    },
    {
      "tier": "quant_q8_0",
      "format": "GGML_Q8_0",
      "disk_gb": 28.0,
      "compression_ratio": 3.70,
      "ppl_heldout": 5.14,
      "ppl_delta_vs_bf16": 0.02,
      "pass_at_1_compile": 1.00,
      "pass_at_1_exec": 0.91,
      "symbol_binding_accuracy": 0.97,
      "distinct_4_ratio": 0.98,
      "constraint_adherence_rate": 0.99,
      "prefill_tok_per_sec": 110.0,
      "decode_tok_per_sec": 18.5,
      "peak_vram_gb_2k_ctx": 30.5,
      "suggested_gpus": ["Apple M-Series (48GB/64GB)", "NVIDIA A6000 (48GB)", "Dual RTX 3060 12GB"]
    },
    {
      "tier": "quant_q6_k",
      "format": "GGML_Q6_K",
      "disk_gb": 22.0,
      "compression_ratio": 4.75,
      "ppl_heldout": 5.19,
      "ppl_delta_vs_bf16": 0.07,
      "pass_at_1_compile": 0.98,
      "pass_at_1_exec": 0.88,
      "symbol_binding_accuracy": 0.94,
      "distinct_4_ratio": 0.97,
      "constraint_adherence_rate": 0.98,
      "prefill_tok_per_sec": 128.0,
      "decode_tok_per_sec": 21.0,
      "peak_vram_gb_2k_ctx": 24.2,
      "suggested_gpus": ["RTX 3090 (24GB)", "RTX 4090 (24GB)", "Apple M-Series (36GB)"]
    },
    {
      "tier": "quant_q5_k_m",
      "format": "GGML_Q5_K_M",
      "disk_gb": 19.0,
      "compression_ratio": 5.33,
      "ppl_heldout": 5.27,
      "ppl_delta_vs_bf16": 0.15,
      "pass_at_1_compile": 0.96,
      "pass_at_1_exec": 0.85,
      "symbol_binding_accuracy": 0.91,
      "distinct_4_ratio": 0.96,
      "constraint_adherence_rate": 0.97,
      "prefill_tok_per_sec": 138.0,
      "decode_tok_per_sec": 23.2,
      "peak_vram_gb_2k_ctx": 21.0,
      "suggested_gpus": ["RTX 3090 (24GB)", "RTX 4090 (24GB)", "Apple M-Series (36GB)"]
    },
    {
      "tier": "quant_q4_k_m",
      "format": "GGML_Q4_K_M",
      "disk_gb": 17.0,
      "compression_ratio": 6.01,
      "ppl_heldout": 5.46,
      "ppl_delta_vs_bf16": 0.34,
      "pass_at_1_compile": 0.93,
      "pass_at_1_exec": 0.80,
      "symbol_binding_accuracy": 0.87,
      "distinct_4_ratio": 0.94,
      "constraint_adherence_rate": 0.95,
      "prefill_tok_per_sec": 145.0,
      "decode_tok_per_sec": 25.0,
      "peak_vram_gb_2k_ctx": 18.5,
      "suggested_gpus": ["RTX 4070 Ti (16GB)", "RTX 4080 (16GB)", "Apple M-Series (24GB/36GB)"]
    }
  ]
}
```

---

## 3. Hardware Recommendation Matrix (Real-World GPUs)

To guide users downloading models from Hugging Face or selecting tiers in the Vox GUI, the matrix maps real-world GPUs to the recommended tier:

| GPU / System Configuration | Usable VRAM | Recommended Tier | Fallback Tier | Experience Profile |
|---|---|---|---|---|
| **Apple M-Max / M-Ultra (128GB RAM)** | 96–115 GB | `merged_bf16` | `quant_q8_0` | Uncompromised full-precision reference generation. |
| **Apple M-Pro / M-Max (64GB RAM)** | 48–56 GB | `quant_q8_0` | `quant_q6_k` | Near-lossless precision; full multi-file reasoning. |
| **Apple M-Series (36GB RAM)** | 26–28 GB | `quant_q6_k` | `quant_q5_k_m` | Optimal balance of speed and 6-bit quality. |
| **Apple M-Series (24GB RAM)** | 18–19 GB | `quant_q4_k_m` | `vox-mens-8b` | Efficient execution within tight unified memory limits. |
| **NVIDIA A100 / H100 (80GB VRAM)** | 80 GB | `merged_bf16` | `quant_q8_0` | High-throughput batch serving and evaluation. |
| **NVIDIA RTX 3090 / 4090 (24GB VRAM)** | 24 GB | `quant_q5_k_m` | `quant_q4_k_m` | Sweet spot: 19 GB model + 3.5 GB for 4k KV cache. |
| **NVIDIA RTX 4070 Ti / 4080 (16GB VRAM)** | 16 GB | `quant_q4_k_m` | `vox-mens-8b` | Tight fit: 14 GB resident weight mapping + 1.5 GB KV cache. |
| **NVIDIA RTX 3060 (12GB VRAM)** | 12 GB | `vox-mens-8b` | N/A | 27B exceeds 12GB VRAM; routes to fine-tuned 8B baseline. |

---

## 4. Orchestrator & GUI "Auto" Model Selector Architecture

### A. Dynamic Discovery & Sizing Heuristic
In `crates/vox-orchestrator/src/models/auto_select.rs`:
```rust
pub struct AutoModelSelection {
    pub selected_model_id: String,
    pub detected_vram_gb: f64,
    pub usable_headroom_gb: f64,
    pub tier_reason: String,
}

pub fn select_optimal_local_model() -> AutoModelSelection {
    // 1. Query hardware discovery via vox-populi
    let vram_bytes = vox_populi::mens::hardware::get_available_gpu_memory();
    let total_vram_gb = vram_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    
    // 2. Reserve 20% safety headroom for OS, display, and KV cache expansion
    let usable_gb = total_vram_gb * 0.80;

    let (tier, reason) = if usable_gb >= 55.0 {
        ("vox-mens-27b/merged_bf16", ">= 64GB VRAM: Uncompromised BF16 master")
    } else if usable_gb >= 30.0 {
        ("vox-mens-27b/quant_q8_0", "32-64GB VRAM: Lossless 8-bit precision")
    } else if usable_gb >= 22.0 {
        ("vox-mens-27b/quant_q6_k", "24-32GB VRAM: High-precision 6-bit quant")
    } else if usable_gb >= 18.0 {
        ("vox-mens-27b/quant_q5_k_m", "24GB VRAM: Optimal 5-bit sweet spot")
    } else if usable_gb >= 15.0 {
        ("vox-mens-27b/quant_q4_k_m", "16-20GB VRAM: Compact 4-bit quant")
    } else {
        ("vox-mens-8b-v0.6", "< 16GB VRAM: Fallback to high-performance 8B model")
    };

    AutoModelSelection {
        selected_model_id: tier.to_string(),
        detected_vram_gb: total_vram_gb,
        usable_headroom_gb: usable_gb,
        tier_reason: reason.to_string(),
    }
}
```

### B. GUI Integration
* In `ChatModelPicker.tsx`:
  * Displays **"Auto (Recommended: `quant_q5_k_m`)"** with a green badge showing detected VRAM (`24 GB VRAM detected`).
  * Allows user to manually expand the dropdown and pin a specific quant or remote model.
* In LLM Settings:
  * A slider for **Headroom Safety Buffer** (default: `20%`, range: `10%`–`40%`).

---

## 5. Hugging Face Repository & Publishing Suite

### A. Repository Topology (`vox-foundation/vox-mens-27b`)
A single unified repository with directory-based variants:
```
vox-foundation/vox-mens-27b/
├── README.md                      # Primary Model Card & Benchmark Matrix
├── config.json                    # Base Hugging Face model architecture config
├── tokenizer.json                 # Vocabulary & BPE tokenizer
├── tokenizer_config.json          # ChatML chat template & special tokens
├── generation_config.json         # Dual EOS (151645, 151643) & defaults
├── external_serving_handoff_v1.json
├── merged_bf16/                   # 51.2 GB BF16 master weights (sharded)
│   ├── model.safetensors.index.json
│   └── model-00001-of-00010.safetensors ...
├── quant_q8_0/                    # 28.0 GB Q8_0 weights
│   ├── quant-metadata.json
│   └── model.safetensors
├── quant_q6_k/                    # 22.0 GB Q6_K weights
├── quant_q5_k_m/                  # 19.0 GB Q5_K_M weights
└── quant_q4_k_m/                  # 17.0 GB Q4_K_M weights
```

### B. YAML Frontmatter Specification for `README.md`
```yaml
---
language:
- en
- vox
license: apache-2.0
base_model: Qwen/Qwen3.5-27B
tags:
- code
- rust
- vox-language
- qwen3
- candle
- ggml
- metal
- qlora
pipeline_tag: text-generation
inference: false
extra_gated_prompt: "Vox Mens is an open-weights model for the Vox programming language."
model_creator: Vox Foundation
model_name: Vox Mens 27B
---
```

### C. Automation Script (`scripts/hf_model_pipeline.vox`)
A dedicated VoxScript pipeline providing:
1. `vox run scripts/hf_model_pipeline.vox stage`: Verifies all required artifacts and generates `README.md` from the live benchmark JSON.
2. `vox run scripts/hf_model_pipeline.vox verify`: Tests local load and sample generation on every staged quant.
3. `vox run scripts/hf_model_pipeline.vox upload --repo vox-foundation/vox-mens-27b`: Uploads staged files using `vox_populi::mens::hub::upload_model_folder` with resume capability and chunked transfers.

---

## 6. Implementation & Verification Plan

### Step 1: Benchmark Matrix Generation
* Run `vox-ml-cli mens eval-local` across all 5 directories on Metal / CUDA.
* Emit `contracts/reports/mens-27b-matrix.v1.json`.

### Step 2: Auto Selector Module
* Implement `crates/vox-orchestrator/src/models/auto_select.rs`.
* Add unit tests verifying correct tier selection across simulated VRAM sizes (8GB, 16GB, 24GB, 36GB, 64GB, 128GB).

### Step 3: Hugging Face Artifact Assembly
* Generate `README.md` with interactive tables, hardware guide, and code snippets.
* Stage model files in `mens/staging/vox-mens-27b/`.

### Step 4: Verification Gate
* Validate that `vox check` passes on all code snippets in the README.
* Verify that `vox chat --model auto` automatically selects the expected tier based on active system memory.
