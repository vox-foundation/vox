---
phase: 345-mens-regression-root-cause-fixes
plan: 01
subsystem: mens-training-and-eval
tags: [mens, qlora, vox_think, supervised_ratio, qwen_reasoning, corpus]
requires: []
provides:
  - Concise training system prompt (`builtin_system_prompt()` ~350 tokens) restoring `supervised_ratio >= 20%` and 4x faster step time
  - Deterministic, compiler-verified, zero-leakage Vox `<think>` reasoning trace synthesizer (`vox_reasoning_synth.rs`, 120 rows)
  - Hardened `extract_cued_answer` in `crates/vox-eval/src/qwen_reasoning.rs` rejecting prose stopwords and recovering earlier cues on truncation
affects: [vox-corpus, vox-eval, mens/config/mix.yaml, mens/config/mix-vox-lang.yaml]
key-files:
  created:
    - crates/vox-corpus/src/corpus/vox_reasoning_synth.rs
  modified:
    - crates/vox-corpus/src/training/mod.rs
    - crates/vox-corpus/src/corpus/mod.rs
    - crates/vox-corpus/src/training/mix_prepare.rs
    - crates/vox-eval/src/qwen_reasoning.rs
    - mens/config/mix.yaml
    - mens/config/mix-vox-lang.yaml
key-decisions:
  - "generate_training_system_prompt() wraps builtin_system_prompt() (~350 tokens) instead of generate_system_prompt() (~2,200 tokens from mens/config/system_prompt.txt) so supervised_ratio stays ~20-25% (>10% gate floor) and prompts are not middle-trimmed."
  - "120 compiler-verified Vox <think> reasoning traces (40 literal_debug, 40 state_trace, 40 invariant_check) are auto-materialized into mens/data/mix_sources/vox_reasoning_traces.jsonl and mixed into train_mixed.jsonl to counter error_correction auto-fix hallucination and 2048-token unclosed <think> loops."
  - "extract_cued_answer scans cues right-to-left, rejects English prose stopwords, and extracts values after 'is'/':'/'=' on heading lines."
requirements-completed: [REGFIX-01, REGFIX-02, REGFIX-03, REGFIX-04]
completed: 2026-11-08
---

# Phase 345 Plan 01 Summary: MENS Regression Root-Cause Fixes

## Root Causes Identified & Fixed

1. **`REGFIX-01` — `supervised_ratio: 4.5% < 10.0%` & 4x Prompt Bloat (`crates/vox-corpus/src/training/mod.rs`)**:
   - **Root Cause**: `generate_training_system_prompt()` called `generate_system_prompt()`, which loaded the 8,593-byte (`~2,200`-token) reference document `mens/config/system_prompt.txt` into every single training row instead of `builtin_system_prompt()` (1,480 bytes / `~350` tokens), despite `builtin_system_prompt()`'s own doc comment stating *"Prepended to every training row, so its tokens come out of `seq_len`"*.
   - **Fix**: Updated `generate_training_system_prompt()` to wrap `builtin_system_prompt()`, raising `supervised_ratio` from `4.5%` to `~20–25%` (`>= 10.0%` gate threshold) and eliminating prompt middle-trimming.
   - **Verification**: `training::training_system_prompt_uses_concise_builtin_for_supervised_ratio` (RED -> GREEN).

2. **`REGFIX-02` — `vox_think` `debug` Auto-Fix Hallucination & `plan-check` Unclosed `<think>` Overthinking (`crates/vox-corpus/src/corpus/vox_reasoning_synth.rs`)**:
   - **Root Cause**: `target/dogfood/train_mixed.jsonl` had **0 rows (`0.0%`)** with `<think>...</think>` traces and **279 rows (`23.9%`)** of `error_correction` (*"This Vox declaration does not compile... Fix it"*). On `1.7B` `vox_think`, `trace` items gained (`6 gains vs 5 losses`), while `debug` items (*"contains a defect. Do not fix it. What integer does `main()` return as written?"*) suffered `8 losses vs 2 gains` because the adapter spotted the defect in `<think>` and output the *fixed/intended* value in 6 of the 8 losses (`0012`, `0016`, `0019`, `0021`, `0084`, `0111`). On `plan-check`, 8 of 11 losses failed to close `</think>` before the 2048-token cap.
   - **Fix**: Created `crates/vox-corpus/src/corpus/vox_reasoning_synth.rs` generating 120 compiler-verified (`vox_compiler::parser::parse`), zero-leakage (`8-gram Jaccard < 0.25` against `mens/data/vox_think/reasoning.jsonl`) thinking-mode SFT pairs across `literal_debug` (40), `state_trace` (40), and `invariant_check` (40), auto-materialized by `mix_prepare.rs` and wired into `mens/config/mix.yaml` and `mens/config/mix-vox-lang.yaml`.
   - **Verification**: `every_generated_reasoning_row_has_valid_vox_ast_and_concise_closed_think`, `generated_reasoning_rows_have_zero_leakage_against_vox_think_slice`, and `mix_auto_materializes_vox_reasoning_traces_source` (all passing).

3. **`REGFIX-03` — `extract_cued_answer` Stopword & Truncated-Cue Shadowing (`crates/vox-eval/src/qwen_reasoning.rs`)**:
   - **Root Cause**: `extract_cued_answer` only checked the single last `lower.rfind(cue)` and returned the first whitespace token, extracting prose stopwords like `"The"`, `"that"`, `"not"`, `"in"` 15–20 times per slice, shadowing earlier valid cues when a trace ended with `"So the answer is "` or `"what the answer is in terms of..."`, and prematurely breaking out of `grade_final_answer`'s `[region, completion]` marker loop.
   - **Fix**: Hardened `extract_cued_answer` to scan cue occurrences from right to left, reject prose stopwords, and extract values after `" is "` / `": "` / `" = "` on heading lines; synced `qwen_reasoning.rs` v2 across worktrees.
   - **Verification**: `qwen_reasoning::tests::cued_final_answer_ignores_stopwords_and_recovers_earlier_cue_on_truncation` (RED -> GREEN, 24/24 tests passing).

4. **`REGFIX-04` — 28-Layer Small Model LoRA Rank/Alpha Scaling**:
   - `4B` (36 layers, `d_model=2560`) gains on `vox_think` at `rank=16, alpha=32`, whereas 28-layer models (`0.6B`, `1.7B`) require `--rank 8 --alpha 16 --grad-accum 8` to prevent `lm_head` logit drift on thinking-mode traces.
