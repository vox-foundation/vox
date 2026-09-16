# Qwen 3.5 27B Quantization Benchmarking, Hardware Sizing & Hugging Face Hosting Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Benchmark and quantify the efficacy, intelligence, responsiveness, and efficiency of all 5 Qwen 3.5 27B model tiers (`merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m`), implement the hardware-aware Auto model selector in `vox-orchestrator` and `vox-gui`, and generate the comprehensive Hugging Face hosting repository suite (`vox-foundation/vox-mens-27b`).

**Architecture:** A release-profile benchmarking harness (`vox-ml-cli mens eval-local`) measuring both standard metrics (PPL, Pass@1) and quantization degradation modes (symbol binding, repetition, constraint violations); a VRAM-aware selection engine in `vox-orchestrator`; native `QMatMul` quantized inference in `vox-plugin-mens-candle-metal`; and an automated Hugging Face staging and upload pipeline (`scripts/hf_model_pipeline.vox`).

**Tech Stack:** Rust (Candle, Metal, Axum), VoxScript (`vox run`), React/TypeScript (Tauri GUI), Hugging Face Hub API (`hf-hub 1.0.0`).

**Spec:** [`docs/superpowers/specs/2026-09-16-mens-27b-quant-benchmark-and-huggingface-spec.md`](../specs/2026-09-16-mens-27b-quant-benchmark-and-huggingface-spec.md)

## Global Constraints & Circuit Breakers
- **Single unchained terminal commands only** (never combine commands with `&&`, `|`, or `;`).
- **Never run `cargo fmt --all`** (format dirty files only via `vox run scripts/fmt.vox`).
- **Preserve model-agnostic LLM boundary** (`vox_actor_runtime::llm`) and Clavis secrets SSOT.
- **Strict Two-Strike Circuit Breaker:** If a verification command fails twice, the subagent MUST halt immediately, revert uncommitted edits, and write `contracts/reports/handoff-notes/<task-id>-strike2.md`.
- **Fast-Path Verification:** Subagents in interactive turns MUST verify code using `--smoke` (<5 seconds). The 1.5-hour full 27B evaluation is run exclusively via background daemons (`scripts/quant_benchmark_matrix.vox --background`).
- **Contracts Index SSOT:** All new contract files must be registered in `contracts/index.yaml` and verified with `vox ci contracts-index`.
- **God Object Protection:** `eval_local.rs` must not exceed 500 lines; new metric algorithms belong in `crates/vox-ml-cli/src/commands/mens/metrics.rs`.

---

### Task 1: Native Quantized Tensor Inference & Metric Extraction Engine [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-plugin-mens-candle-metal/src/inference.rs:290-320, 375-410`
- Create: `crates/vox-ml-cli/src/commands/mens/metrics.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/eval_local.rs:165-225, 480-580`
- Modify: `crates/vox-ml-cli/src/commands/mens/mod.rs`
- Create: `contracts/reports/mens/eval-matrix.v1.schema.json`
- Modify: `contracts/index.yaml`

**Interfaces:**
- Consumes: Candle native `candle_core::quantized::QMatMul` for GGML inference; `run_frontend_str` for syntax checks.
- Produces: `crates/vox-ml-cli/src/commands/mens/metrics.rs` with `calculate_distinct_4`, `verify_symbol_binding`, `check_negative_constraints`, and `run_test_assertions`.

- [ ] **Step 1: Write failing unit tests for degradation metrics in `metrics.rs` (TDD Red)**

Create `crates/vox-ml-cli/src/commands/mens/metrics.rs` with tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distinct_4_ratio_catches_repetition() {
        let loop_text = " - - - - - - - - - - - - - - - - - - - -";
        assert!(calculate_distinct_4(loop_text) < 0.25);
        let healthy = "fn add(a: int, b: int) to int { return a + b; }";
        assert!(calculate_distinct_4(healthy) >= 0.65);
    }

    #[test]
    fn test_negative_constraint_checking() {
        let bad_code = "@endpoint fn old_api() {}";
        assert!(!check_negative_constraints(bad_code));
        let good_code = "fn new_api() {}";
        assert!(check_negative_constraints(good_code));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-ml-cli --lib -- test_distinct_4_ratio`  
Expected: FAIL with compilation error (functions undefined).

- [ ] **Step 3: Implement metric calculations in `metrics.rs` (TDD Green)**

Implement:
1. `calculate_distinct_4(code: &str) -> f64`: Tokenizes into lexemes, computes unique 4-grams over total 4-grams (length-guarded: returns 1.0 for $N < 4$).
2. `check_negative_constraints(code: &str) -> bool`: Flags deprecated decorators (`@endpoint`, `@mutation`) and chatty commentary.
3. `verify_symbol_binding(result: &FrontendResult) -> f64`: Counts undefined symbol diagnostics matching `codes::TYPES_UNDEFINED_VARIABLE` and `codes::TYPES_UNRESOLVED_TYPE`.

- [ ] **Step 4: Run unit tests to verify they pass**

Run: `cargo test -p vox-ml-cli --lib -- test_distinct_4_ratio`  
Expected: PASS (`ok. 1 passed`).

- [ ] **Step 5: Fix double-quantization memory hazard in `vox-plugin-mens-candle-metal`**

In `crates/vox-plugin-mens-candle-metal/src/inference.rs`:
1. Stop calling `qt.dequantize(&_device)?` and `QuantizedLinear::from_weight`.
2. Load quantized weights directly into Candle's native `candle_core::quantized::QMatMul`, avoiding 108 GB F32 buffer allocations.
3. Add support in `InferenceEngine::load` for unquantized sharded safetensors when `model.safetensors.index.json` is present (for `merged_bf16`).

- [ ] **Step 6: Register `contracts/reports/mens/eval-matrix.v1.schema.json` in `contracts/index.yaml`**

Add contract entry to `contracts/index.yaml` and verify schema.

- [ ] **Step 7: Commit Task 1**

Run: `git add crates/vox-ml-cli/src/commands/mens/metrics.rs crates/vox-ml-cli/src/commands/mens/eval_local.rs crates/vox-plugin-mens-candle-metal/src/inference.rs contracts/reports/mens/eval-matrix.v1.schema.json contracts/index.yaml`  
Run: `git commit -m "feat(mens): add degradation stress metrics and native QMatMul quantized inference"`

---

### Task 2: Hardware Sizing & Dynamic Auto Model Selector (`vox-orchestrator`) [SEQUENTIAL]

**Files:**
- Create: `crates/vox-orchestrator/src/models/auto_select.rs`
- Modify: `crates/vox-orchestrator/src/models/mod.rs`
- Modify: `crates/vox-orchestrator/src/models/select.rs`
- Modify: `crates/vox-orchestrator/src/models/registry.rs`
- Modify: `crates/vox-cli/src/commands/chat.rs`

**Interfaces:**
- Consumes: `vox_orchestrator::models::vram::free_vram_mb_hint()` and `macos_metal::probe_metal()`.
- Produces: `pub fn select_optimal_local_model(is_apple_silicon: bool) -> AutoModelSelection`.

- [ ] **Step 1: Write failing unit test for `auto_select.rs` (TDD Red)**

Create `crates/vox-orchestrator/src/models/auto_select.rs` with test:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_selection_tiers() {
        // Apple Silicon tests (probe already reduces RAM to 75%, 2.0GB execution reserve deducted)
        assert_eq!(select_tier_for_vram(96.0, true).0, "mens/runs/qwen3_27b_metal_check/merged_bf16");
        assert_eq!(select_tier_for_vram(36.0, true).0, "mens/runs/qwen3_27b_metal_check/quant_q8_0");
        assert_eq!(select_tier_for_vram(27.0, true).0, "mens/runs/qwen3_27b_metal_check/quant_q6_k");
        assert_eq!(select_tier_for_vram(18.0, true).0, "vox-mens-8b-v0.6");

        // CUDA / Discrete tests (10% clamped reserve between 2.5GB and 6.0GB)
        assert_eq!(select_tier_for_vram(80.0, false).0, "mens/runs/qwen3_27b_metal_check/merged_bf16");
        assert_eq!(select_tier_for_vram(24.0, false).0, "mens/runs/qwen3_27b_metal_check/quant_q4_k_m");
        assert_eq!(select_tier_for_vram(16.0, false).0, "vox-mens-8b-v0.6");
        assert_eq!(select_tier_for_vram(12.0, false).0, "vox-mens-8b-v0.6");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p vox-orchestrator --lib -- test_auto_selection_tiers`  
Expected: FAIL with compilation error (module or function missing).

- [ ] **Step 3: Implement `auto_select.rs` logic (TDD Green)**

Implement `select_tier_for_vram(total_vram_gb: f64, is_apple_silicon: bool) -> (&'static str, &'static str)` matching the audited specification thresholds.

- [ ] **Step 4: Register `vox-mens-27b` variants in `ModelRegistry`**

In `crates/vox-orchestrator/src/models/registry.rs`:
Register candidate entries for `merged_bf16`, `quant_q8_0`, `quant_q6_k`, `quant_q5_k_m`, `quant_q4_k_m` under `ProviderType::VoxLocal`.

- [ ] **Step 5: Intercept `--model auto` and `vox-mens-*` in `vox chat`**

In `crates/vox-cli/src/commands/chat.rs`:
Update `resolve_chat_config` so `model == "auto"` resolves via `select_optimal_local_model` and routes to local inference server (`http://127.0.0.1:11434`), preventing OpenRouter cloud egress.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p vox-orchestrator --lib -- test_auto_selection_tiers`  
Expected: PASS.

- [ ] **Step 7: Commit Task 2**

Run: `git add crates/vox-orchestrator/src/models/auto_select.rs crates/vox-orchestrator/src/models/mod.rs crates/vox-orchestrator/src/models/select.rs crates/vox-orchestrator/src/models/registry.rs crates/vox-cli/src/commands/chat.rs`  
Run: `git commit -m "feat(orchestrator): add hardware-aware auto model selector and local chat routing"`

---

### Task 3: GUI Model Picker & Tauri IPC Integration (`vox-gui`) [SEQUENTIAL]

**Files:**
- Modify: `crates/vox-gui/src/commands/models.rs`
- Modify: `crates/vox-gui/src/main.rs:195-240`
- Modify: `crates/vox-gui/ui/src/transport.ts`
- Modify: `crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx`
- Modify: `crates/vox-gui/ui/e2e/lib/tauriMock.ts`

**Interfaces:**
- Consumes: Tauri command `get_auto_model_recommendation`.
- Produces: UI component `ChatModelPicker.tsx` rendering "Auto (Recommended: `quant_qX`)" badge.

- [ ] **Step 1: Implement `get_auto_model_recommendation` in `commands/models.rs`**

Add Tauri command returning `{ selected_model_id: String, detected_vram_gb: f64, tier_reason: String }`.

- [ ] **Step 2: Register command in `crates/vox-gui/src/main.rs`**

Add `commands::models::get_auto_model_recommendation` into `tauri::generate_handler![]`.

- [ ] **Step 3: Update `transport.ts` and `tauriMock.ts`**

Expose `getAutoModelRecommendation` and add mock return value in testing mocks.

- [ ] **Step 4: Update `ChatModelPicker.tsx` to render the Auto badge**

Render Auto option with live VRAM indicator and tooltip.

- [ ] **Step 5: Verify TypeScript build**

Run: `pnpm --filter vox-gui-ui build`  
Expected: Clean build with exit code 0.

- [ ] **Step 6: Commit Task 3**

Run: `git add crates/vox-gui/src/commands/models.rs crates/vox-gui/src/main.rs crates/vox-gui/ui/src/transport.ts crates/vox-gui/ui/src/components/surfaces/Chat/ChatModelPicker.tsx crates/vox-gui/ui/e2e/lib/tauriMock.ts`  
Run: `git commit -m "feat(gui): integrate auto model recommendation and VRAM indicator in model picker"`

---

### Task 4: Unified Staging, Hugging Face Documentation & Upload Pipeline [PARALLEL-SAFE]

**Files:**
- Modify: `scripts/hf_model_pipeline.vox`
- Create: `mens/config/hf_readme_template.md`

**Interfaces:**
- Consumes: `contracts/reports/mens/eval-matrix/mens-27b-matrix.v1.json`.
- Produces: Staged repository in `mens/staging/vox-mens-27b/` with generated `README.md`.

- [ ] **Step 1: Create `mens/config/hf_readme_template.md`**

Template with YAML frontmatter, badges, interactive benchmark comparison table, real-world GPU sizing guide, and copy-paste code snippets.

- [ ] **Step 2: Enhance `scripts/hf_model_pipeline.vox`**

Implement subcommands:
- `stage`: Reads benchmark report, renders `README.md`, stages configs, tokenizer, and hard-links model directories into `mens/staging/vox-mens-27b/`.
- `verify`: Checks that each staged tier directory contains valid metadata and loadable weights.
- `upload`: Calls `vox mens hub upload --repo vox-foundation/vox-mens-27b --model-dir mens/staging/vox-mens-27b`.

- [ ] **Step 3: Test template rendering with dry run**

Run: `vox run scripts/hf_model_pipeline.vox -- stage --dry-run`  
Expected: Stage plan validated with exit code 0.

- [ ] **Step 4: Commit Task 4**

Run: `git add scripts/hf_model_pipeline.vox mens/config/hf_readme_template.md`  
Run: `git commit -m "feat(mens): add unified staging, model card generator, and upload pipeline"`

---

### Task 5: Fast-Path Benchmark Runner & Detached Background Daemon [PARALLEL-SAFE]

**Files:**
- Create: `scripts/quant_benchmark_matrix.vox`

**Interfaces:**
- Consumes: `target/release/vox-ml-cli mens eval-local`.
- Produces: `contracts/reports/mens/eval-matrix/mens-27b-matrix.v1.json`.

- [ ] **Step 1: Implement `scripts/quant_benchmark_matrix.vox`**

Features:
- `--smoke`: Fast synthetic check on 1 sample completing in < 5 seconds for subagent verification.
- `--background`: Launches detached background evaluation with logging to `mens/logs/benchmark_27b.log` and status tracking in `contracts/reports/mens/eval-matrix/status.json`.
- `--tier <name>`: Evaluates a single tier and saves incremental progress.
- Calculates PPL via sliding window ($W=1024, S=512$).

- [ ] **Step 2: Verify `--smoke` execution**

Run: `vox run scripts/quant_benchmark_matrix.vox -- --smoke`  
Expected: Passes in < 5 seconds, validating harness invocation.

- [ ] **Step 3: Commit Task 5**

Run: `git add scripts/quant_benchmark_matrix.vox`  
Run: `git commit -m "feat(mens): add multi-tier benchmark runner with smoke verification and background daemon"`

---

### Task 6: Release Build, Background Evaluation & Verification Gate [SEQUENTIAL]

- [ ] **Step 1: Build release binary for high-speed evaluation**

Run: `cargo build --release -p vox-ml-cli`  
Expected: Produces `target/release/vox-ml-cli` in release profile.

- [ ] **Step 2: Launch production benchmark run in background daemon**

Run: `vox run scripts/quant_benchmark_matrix.vox -- --background`  
Expected: Daemon spawned; logs available at `mens/logs/benchmark_27b.log`.

- [ ] **Step 3: Verify contracts index compliance**

Run: `cargo run -p vox-cli -- ci contracts-index`  
Expected: PASS (all contracts and schemas registered).

- [ ] **Step 4: Stage Hugging Face artifacts upon benchmark completion**

Run: `vox run scripts/hf_model_pipeline.vox -- stage`  
Expected: `mens/staging/vox-mens-27b/README.md` populated with live metrics.

- [ ] **Step 5: Verify all code snippets in README**

Run: `vox check mens/staging/vox-mens-27b/README.md` (or extract fenced Vox snippets and run `vox check`)  
Expected: All Vox code snippets compile with 0 errors.

- [ ] **Step 6: Commit final benchmark report and staging metadata**

Run: `git add contracts/reports/mens/eval-matrix/mens-27b-matrix.v1.json contracts/index.yaml`  
Run: `git commit -m "chore(mens): record empirical 27B 5-tier evaluation matrix report"`
