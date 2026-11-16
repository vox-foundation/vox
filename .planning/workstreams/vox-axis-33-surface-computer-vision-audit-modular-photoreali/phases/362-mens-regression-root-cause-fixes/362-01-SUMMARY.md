---
phase: 362-mens-regression-root-cause-fixes
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
requirements-completed: [MENS-REGFIX-01, MENS-REGFIX-02, MENS-REGFIX-03, MENS-REGFIX-04]
completed: 2026-11-08
---

# Phase 362 Plan 01 Summary: MENS Regression Root-Cause Fixes

See `362-01-PLAN.md` and unit tests in `crates/vox-corpus` and `crates/vox-eval` for full verification proofs.
