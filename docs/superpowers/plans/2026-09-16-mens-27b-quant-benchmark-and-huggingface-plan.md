# Qwen 3.5 27B Quantization Benchmarking, Hardware Sizing & Hugging Face Hosting Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Benchmark and quantify the efficacy, intelligence, responsiveness, and efficiency of all 5 Qwen 3.5 27B model tiers (`merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m`), implement the hardware-aware Auto model selector with 20% headroom in `vox-orchestrator` and `vox-gui`, and generate the comprehensive Hugging Face hosting repository suite (`vox-foundation/vox-mens-27b`).

**Architecture:** A release-profile benchmarking harness (`vox-ml-cli mens eval-local`) measuring both standard metrics (PPL, Pass@1) and quantization-specific degradation modes (symbol binding, repetition, constraint violations); a VRAM-aware selection engine in `vox-orchestrator`; and an automated Hugging Face staging and upload pipeline (`scripts/hf_model_pipeline.vox`).

**Tech Stack:** Rust (Candle, Metal, Axum), VoxScript (`vox run`), React/TypeScript (Tauri GUI), Hugging Face Hub API (`hf-hub 1.0.0`).

**Spec:** [`docs/superpowers/specs/2026-09-16-mens-27b-quant-benchmark-and-huggingface-spec.md`](../specs/2026-09-16-mens-27b-quant-benchmark-and-huggingface-spec.md)

## Global Constraints
- Single unchained terminal commands only (no `&&`, `|`, `;` in tool calls).
- Never run `cargo fmt --all` (format dirty files only via `vox run scripts/fmt.vox`).
- Preserve the model-agnostic LLM boundary (`vox_actor_runtime::llm`) and Clavis secrets SSOT.
- 20% default safety headroom buffer for VRAM calculation.
- All benchmark metrics stored in the SSOT report `contracts/reports/mens-27b-matrix.v1.json`.

---

### Task 1: High-Performance Release Harness & Metric Extraction Engine

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_local.rs:170-260`
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs:110-140`
- Create: `scripts/quant_benchmark_matrix.vox`

**Interfaces:**
- Consumes: `vox-plugin-mens-candle-metal` via C-ABI `run_inference`.
- Produces: `contracts/reports/mens-27b-matrix.v1.json` containing `ppl`, `pass_at_1_compile`, `pass_at_1_exec`, `symbol_binding_accuracy`, `distinct_4_ratio`, `constraint_adherence_rate`, `prefill_tok_per_sec`, `decode_tok_per_sec`, and `peak_vram_gb_2k_ctx`.

- [ ] **Step 1: Build `vox-ml-cli` with `--release` for high-throughput evaluation**

Run: `cargo build --release -p vox-ml-cli`  
Expected: Succeeded, producing `target/release/vox-ml-cli`.

- [ ] **Step 2: Add quantization degradation metrics to `eval_local.rs`**

In `crates/vox-ml-cli/src/commands/mens/eval_local.rs`:
1. Add Distinct-4 n-gram calculation ($D_4 = \frac{\text{unique 4-grams}}{\text{total 4-grams}}$) to detect repetition traps.
2. Add Unresolved Symbol Rate detection (flagging `E020` / undefined identifiers in multi-file context prompts).
3. Add Negative Constraint Violation check (flagging deprecated `@endpoint` or unwanted commentary).
4. Add execution pass verification (`pass_at_1_exec`) by running `vox_interp` or `vox test` on runnable `@test` functions.

- [ ] **Step 3: Write unit tests for degradation metrics**

Add test in `eval_local.rs`:
```rust
#[test]
fn test_distinct_4_ratio_catches_repetition() {
    let loop_text = " - - - - - - - - - - - - - - - -";
    assert!(calculate_distinct_4(loop_text) < 0.2);
    let healthy = "fn add(a: int, b: int) to int { return a + b; }";
    assert!(calculate_distinct_4(healthy) > 0.8);
}
```

- [ ] **Step 4: Run unit tests**

Run: `cargo test -p vox-ml-cli --lib -- test_distinct_4`  
Expected: PASS.

- [ ] **Step 5: Create multi-tier benchmark runner `scripts/quant_benchmark_matrix.vox`**

Create `scripts/quant_benchmark_matrix.vox` to iterate across:
- `mens/runs/qwen3_27b_metal_check` (`merged_bf16`)
- `mens/runs/qwen3_27b_metal_check/quant_q8_0`
- `mens/runs/qwen3_27b_metal_check/quant_q6_k`
- `mens/runs/qwen3_27b_metal_check/quant_q5_k_m`
- `mens/runs/qwen3_27b_metal_check/quant_q4_k_m`

And aggregate results into `contracts/reports/mens-27b-matrix.v1.json`.

- [ ] **Step 6: Commit Task 1**

```bash
git add crates/vox-ml-cli/src/commands/mens/eval_local.rs scripts/quant_benchmark_matrix.vox
git commit -m "feat(mens): add degradation stress metrics and multi-tier benchmark runner"
```

---

### Task 2: Hardware Sizing & Dynamic Auto Model Selector (`vox-orchestrator`)

**Files:**
- Create: `crates/vox-orchestrator/src/models/auto_select.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs`
- Modify: `crates/vox-orchestrator/src/models/select.rs`

**Interfaces:**
- Consumes: `vox_populi::mens::hardware::get_available_gpu_memory() -> u64`.
- Produces: `pub fn select_optimal_local_model(headroom_ratio: f64) -> AutoModelSelection`.

- [ ] **Step 1: Write failing unit test for `auto_select.rs`**

Create `crates/vox-orchestrator/src/models/auto_select.rs` with tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_selection_tiers() {
        assert_eq!(select_tier_for_vram(128.0, 0.20), "vox-mens-27b/merged_bf16");
        assert_eq!(select_tier_for_vram(48.0, 0.20), "vox-mens-27b/quant_q8_0");
        assert_eq!(select_tier_for_vram(24.0, 0.20), "vox-mens-27b/quant_q5_k_m");
        assert_eq!(select_tier_for_vram(16.0, 0.20), "vox-mens-27b/quant_q4_k_m");
        assert_eq!(select_tier_for_vram(12.0, 0.20), "vox-mens-8b-v0.6");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-orchestrator --lib -- test_auto_selection_tiers`  
Expected: FAIL (module or function missing).

- [ ] **Step 3: Implement `auto_select.rs` logic**

Implement `select_tier_for_vram(total_vram_gb: f64, headroom: f64) -> &'static str`:
- Calculate `usable_gb = total_vram_gb * (1.0 - headroom)`.
- Apply hierarchy:
  - $\ge 55.0$ GB $\to$ `vox-mens-27b/merged_bf16`
  - $\ge 30.0$ GB $\to$ `vox-mens-27b/quant_q8_0`
  - $\ge 22.0$ GB $\to$ `vox-mens-27b/quant_q6_k`
  - $\ge 18.0$ GB $\to$ `vox-mens-27b/quant_q5_k_m`
  - $\ge 15.0$ GB $\to$ `vox-mens-27b/quant_q4_k_m`
  - $< 15.0$ GB $\to$ `vox-mens-8b-v0.6`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p vox-orchestrator --lib -- test_auto_selection_tiers`  
Expected: PASS.

- [ ] **Step 5: Wire into `vox-orchestrator::models::select`**

Allow model string `"auto"` to resolve via `select_optimal_local_model(0.20)`.

- [ ] **Step 6: Commit Task 2**

```bash
git add crates/vox-orchestrator/src/models/auto_select.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/select.rs
git commit -m "feat(orchestrator): add hardware-aware auto model selector with 20% headroom"
```

---

### Task 3: GUI Model Picker & LLM Settings Integration (`vox-gui`)

**Files:**
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`
- Modify: `crates/vox-gui/src/commands/models.rs`

**Interfaces:**
- Consumes: Tauri IPC `get_auto_model_recommendation()`.
- Produces: UI option "Auto (Recommended: `quant_qX`)" with live hardware badge.

- [ ] **Step 1: Add Tauri IPC endpoint for auto selection recommendation**

In `crates/vox-gui/src/commands/models.rs`:
Add `get_auto_model_recommendation()` returning `{ selected_model_id, detected_vram_gb, tier_reason }`.

- [ ] **Step 2: Update `ChatModelPicker.tsx` to render the Auto recommendation**

In `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`:
Add "Auto" entry with recommended quant tag and detected VRAM tooltip.

- [ ] **Step 3: Verify TypeScript builds**

Run: `pnpm --filter @vox/gui build` (or `pnpm check`)  
Expected: Clean build with zero TypeScript errors.

- [ ] **Step 4: Commit Task 3**

```bash
git add crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx crates/vox-gui/src/commands/models.rs
git commit -m "feat(gui): integrate auto model recommendation and VRAM badge in model picker"
```

---

### Task 4: Hugging Face Model Card & Documentation Generator

**Files:**
- Create: `scripts/generate_hf_card.vox`
- Create: `crates/vox-ml-cli/tests/fixtures/hf_readme_template.md`

**Interfaces:**
- Consumes: `contracts/reports/mens-27b-matrix.v1.json`.
- Produces: Formatted `README.md` for Hugging Face repository.

- [ ] **Step 1: Create template with YAML frontmatter & markdown sections**

Include:
- YAML tags (`language: [en, vox]`, `license: apache-2.0`, `base_model: Qwen/Qwen3.5-27B`, `tags: [code, rust, candle, ggml, metal]`).
- Comparative benchmark table generated dynamically from JSON metrics.
- GPU recommendation matrix matching real hardware.
- Getting Started code snippets (`vox chat`, `vox mens serve`, Python Candle, llama.cpp / GGUF).

- [ ] **Step 2: Implement generator script `scripts/generate_hf_card.vox`**

Reads `contracts/reports/mens-27b-matrix.v1.json`, populates template variables, and writes `mens/staging/vox-mens-27b/README.md`.

- [ ] **Step 3: Test generation with sample matrix data**

Run: `vox run scripts/generate_hf_card.vox`  
Expected: Emits complete, valid Markdown `README.md`.

- [ ] **Step 4: Commit Task 4**

```bash
git add scripts/generate_hf_card.vox crates/vox-ml-cli/tests/fixtures/hf_readme_template.md
git commit -m "feat(mens): add dynamic Hugging Face model card and documentation generator"
```

---

### Task 5: Staging, Verification & Hugging Face Upload Pipeline

**Files:**
- Modify: `scripts/hf_model_pipeline.vox`
- Modify: `crates/vox-populi/src/mens/hub.rs`

**Interfaces:**
- Consumes: `mens/runs/qwen3_27b_metal_check/` quants & merged weights.
- Produces: Staged repository in `mens/staging/vox-mens-27b/` and verified uploads to `vox-foundation/vox-mens-27b`.

- [ ] **Step 1: Update `scripts/hf_model_pipeline.vox` staging command**

Stage files:
- `config.json`, `tokenizer.json`, `tokenizer_config.json`, `generation_config.json`
- `README.md` (generated from Task 4)
- Subdirectories `merged_bf16/`, `quant_q8_0/`, `quant_q6_k/`, `quant_q5_k_m/`, `quant_q4_k_m/`
- Hard-link or copy files into `mens/staging/vox-mens-27b/`

- [ ] **Step 2: Add verify command**

Verify each staged tier loads successfully and generates at least 1 token.

- [ ] **Step 3: Add upload command**

Call `vox mens hub upload --repo vox-foundation/vox-mens-27b --dir mens/staging/vox-mens-27b`.

- [ ] **Step 4: Commit Task 5**

```bash
git add scripts/hf_model_pipeline.vox crates/vox-populi/src/mens/hub.rs
git commit -m "feat(mens): add staging, verification, and chunked upload pipeline for 27B model suite"
```

---

### Task 6: Execution Gate & Full Matrix Evaluation

- [ ] **Step 1: Run release benchmark matrix across all 5 tiers**

Run: `vox run scripts/quant_benchmark_matrix.vox`  
Expected: All 5 tiers evaluated, `contracts/reports/mens-27b-matrix.v1.json` written with complete metrics.

- [ ] **Step 2: Generate the final Hugging Face Model Card**

Run: `vox run scripts/generate_hf_card.vox`  
Expected: `mens/staging/vox-mens-27b/README.md` created with verified numbers.

- [ ] **Step 3: Validate code snippets**

Run `vox check` on code snippets extracted from the README.  
Expected: All Vox snippets compile with 0 errors.

- [ ] **Step 4: Commit benchmark report and staged artifacts**

```bash
git add contracts/reports/mens-27b-matrix.v1.json
git commit -m "chore(mens): record empirical 27B 5-tier evaluation matrix report"
```
