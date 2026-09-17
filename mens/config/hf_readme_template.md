---
language:
- en
- vox
license: apache-2.0
base_model: Qwen/Qwen3-27B
tags:
- vox
- code
- fine-tuned
- quantized
- candle-metal
pipeline_tag: text-generation
library_name: candle
---

# Vox Mens 27B (`vox-foundation/vox-mens-27b`)

[![Hugging Face](https://img.shields.io/badge/%F0%9F%A4%97%20Hugging%20Face-vox--foundation%2Fvox--mens--27b-yellow.svg)](https://huggingface.co/vox-foundation/vox-mens-27b)
[![Candle](https://img.shields.io/badge/Candle-Metal%20Native-blue.svg)](https://github.com/huggingface/candle)
[![Metal](https://img.shields.io/badge/Apple%20Silicon-Metal-black.svg)](#)
[![Vox](https://img.shields.io/badge/Vox-Native%20Lang-purple.svg)](https://vox-lang.org)
[![License](https://img.shields.io/badge/License-Apache%202.0-green.svg)](https://opensource.org/licenses/Apache-2.0)

**Vox Mens 27B** is a high-performance open causal language model fine-tuned for native **Vox programming language** synthesis, compilation adherence, and semantic symbol binding.

Based on `Qwen/Qwen3.5-27B` (16 full-attention layers + 48 linear-attention layers), Vox Mens 27B is adapted on Apple Silicon Metal via NF4 QLoRA and published as both full-precision BF16 weights and four optimized quantization tiers (`Q8_0`, `Q6_K`, `Q5_K_M`, `Q4_K_M`) accelerated by Candle native Metal GGML kernels.

---

## Model Details

- **Model Family:** {{MODEL_FAMILY}}
- **Base Architecture:** Qwen 3.5 27B (Hybrid Full/Linear Attention)
- **Checkpoint Step:** {{CHECKPOINT_STEP}}
- **Evaluation Date:** {{EVAL_TIMESTAMP}}
- **Supported Quantizations:** `merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m`
- **Context Length:** 32,768 tokens (up to 8K-32K safe context depending on hardware and quant tier)
- **Primary Languages:** Vox, English

---

## Evaluation & Benchmark Degradation Matrix

Quantization degrades LLM capabilities non-linearly. To verify code generation reliability, all tiers are evaluated across a 4-axis framework:
1. **Held-Out Perplexity ($\text{PPL}$):** Evaluated over 50,000 tokens of held-out Vox standard library and runtime code.
2. **3-Tier Verification Oracle:**
   - **`Pass@1 Compile`:** Zero compiler errors via `vox check` (100% of tasks).
   - **`Pass@1 AST`:** Entity, keyword, and construct richness matching (100% of tasks).
   - **`Pass@1 Exec`:** Dynamic unit test assertion passing on runnable modules (`@test`).
3. **Quantization Degradation Stress:**
   - **Distinct-4 ($D_4$):** Repetition index guarding against degenerative loops (repetition failure if $D_4 < 0.40$).
   - **Symbol Binding Accuracy ($\text{SBA}$):** Resolution of multi-file module imports and variables.
   - **Constraint Adherence:** Enforcement of negative constraints (no deprecated decorators, no chatty preambles).
4. **Throughput:** Prefill and decode tokens/sec measured on Apple Silicon M-Series Metal.

| Tier | Format | Size | Held-Out PPL | Δ PPL | Pass@1 Compile | Pass@1 AST | Pass@1 Exec | Distinct-4 ($D_4$) | Symbol Binding | Prefill (tok/s) | Decode (tok/s) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| **`merged_bf16`** | BF16 | 51.24 GB | {{BF16_PPL}} | {{BF16_PPL_DELTA}} | {{BF16_PASS_COMPILE}} | {{BF16_PASS_AST}} | {{BF16_PASS_EXEC}} | {{BF16_DISTINCT_4}} | {{BF16_SYMBOL_BINDING}} | {{BF16_PREFILL}} | {{BF16_DECODE}} |
| **`quant_q8_0`** | GGML_Q8_0 | 30.00 GB | {{Q8_0_PPL}} | {{Q8_0_PPL_DELTA}} | {{Q8_0_PASS_COMPILE}} | {{Q8_0_PASS_AST}} | {{Q8_0_PASS_EXEC}} | {{Q8_0_DISTINCT_4}} | {{Q8_0_SYMBOL_BINDING}} | {{Q8_0_PREFILL}} | {{Q8_0_DECODE}} |
| **`quant_q6_k`** | GGML_Q6_K | 23.37 GB | {{Q6_K_PPL}} | {{Q6_K_PPL_DELTA}} | {{Q6_K_PASS_COMPILE}} | {{Q6_K_PASS_AST}} | {{Q6_K_PASS_EXEC}} | {{Q6_K_DISTINCT_4}} | {{Q6_K_SYMBOL_BINDING}} | {{Q6_K_PREFILL}} | {{Q6_K_DECODE}} |
| **`quant_q5_k_m`** | GGML_Q5_K_M | 20.86 GB | {{Q5_K_M_PPL}} | {{Q5_K_M_PPL_DELTA}} | {{Q5_K_M_PASS_COMPILE}} | {{Q5_K_M_PASS_AST}} | {{Q5_K_M_PASS_EXEC}} | {{Q5_K_M_DISTINCT_4}} | {{Q5_K_M_SYMBOL_BINDING}} | {{Q5_K_M_PREFILL}} | {{Q5_K_M_DECODE}} |
| **`quant_q4_k_m`** | GGML_Q4_K_M | 18.50 GB | {{Q4_K_M_PPL}} | {{Q4_K_M_PPL_DELTA}} | {{Q4_K_M_PASS_COMPILE}} | {{Q4_K_M_PASS_AST}} | {{Q4_K_M_PASS_EXEC}} | {{Q4_K_M_DISTINCT_4}} | {{Q4_K_M_SYMBOL_BINDING}} | {{Q4_K_M_PREFILL}} | {{Q4_K_M_DECODE}} |

---

## Real-World GPU Hardware Sizing Guide

Because 27B models require significant memory for weights plus KV cache, choosing the right quantization tier is critical for stability and throughput:

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

## Getting Started

### 1. Automatic Hardware Selection in Vox CLI

The easiest way to use Vox Mens 27B is via the automatic model selector. It detects your platform (Apple Silicon Unified Memory vs. CUDA VRAM) and automatically mounts the highest-quality tier that fits comfortably:

```bash
# Automatically select and load the best local model
vox chat --model auto
```

### 2. Local OpenAI-Compatible Server

Launch local HTTP serving with Candle acceleration:

```bash
# Serve 6-bit quant tier on local port 11434
vox ai serve --model-dir mens/staging/vox-mens-27b/quant_q6_k --port 11434
```

### 3. Programmatic Usage in Vox

Invoke local completions using native `std.http` and `std.json`:

```vox
import std.http;
import std.json;

fn generate_vox_code(prompt: str) to str {
    let payload = "{\"model\": \"vox-mens-27b\", \"prompt\": \"" + prompt + "\", \"max_tokens\": 512}";
    let resp = std.http.post("http://127.0.0.1:11434/v1/completions", payload);
    if resp.is_err() {
        return "// Error contacting local model server";
    }
    return resp.unwrap().body;
}

fn main() {
    let code = generate_vox_code("fn fibonacci(n: int) to int");
    print(code);
}
```

---

## Repository Topology

```
vox-foundation/vox-mens-27b/
├── README.md                      # Primary Model Card & Benchmark Matrix
├── config.json                    # Architecture configuration
├── tokenizer.json                 # Vocabulary & BPE tokenizer
├── tokenizer_config.json          # ChatML chat template & special tokens
├── generation_config.json         # Dual EOS & sampling parameters
├── external_serving_handoff_v1.json
├── merged_bf16/                   # 51.24 GB BF16 master weights
│   ├── config.json
│   ├── tokenizer.json
│   └── model.safetensors
├── quant_q8_0/                    # 30.00 GB Q8_0 weights
│   ├── config.json
│   ├── tokenizer.json
│   ├── quant-metadata.json
│   └── model.safetensors
├── quant_q6_k/                    # 23.37 GB Q6_K weights
├── quant_q5_k_m/                  # 20.86 GB Q5_K_M weights
└── quant_q4_k_m/                  # 18.50 GB Q4_K_M weights
```

---

## Citation & License

This model is licensed under the [Apache 2.0 License](https://opensource.org/licenses/Apache-2.0).

```bibtex
@misc{vox2026mens27b,
  author = {Vox Foundation},
  title = {Vox Mens 27B: High-Efficacy Code Synthesis and Verification for the Vox Language},
  year = {2026},
  publisher = {Hugging Face},
  howpublished = {\url{https://huggingface.co/vox-foundation/vox-mens-27b}}
}
```
