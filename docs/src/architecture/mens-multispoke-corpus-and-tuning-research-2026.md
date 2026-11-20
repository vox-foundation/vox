---
title: "MENS Multi-Spoke Corpus Fidelity & QLoRA Pipeline Audit (2026)"
description: "Forensic audit of golden example extraction, documentation Q&A/code ingestion, multi-spoke mix configurations, and QLoRA optimization upgrades across all MENS spokes and lanes."
category: "Architecture SSOTs"
status: "current"
topic: "training-corpus"
domain: "ml"
authorship: "llm-assisted"
authorship_model: "gemini-3.1-pro"
---

# MENS Multi-Spoke Corpus Fidelity & QLoRA Pipeline Audit (2026)

<!-- kb:summary -->
Forensic audit of the MENS training corpus and QLoRA pipeline across all spokes and lanes. While golden `.vox` extraction succeeds on compiling examples (`87/87` in `examples/golden/`), signal is degraded by 49 compile-failing workspace `.vox` files, tiny AST-fragmented snippets, uncontextualized `"Explain the Vox concept: <H2>"` prompts in `docs.jsonl`, silent `min_rating` and `include_lanes` filter drops across domain profiles, unmaterialized Wave 2–4 synthetic/RFT/DPO sources, and per-micro-step loss normalization in Metal QLoRA training.
<!-- /kb:summary -->

## 1. Golden `.vox` Examples (`examples/golden/`, `target/dogfood/train.jsonl`)

Source inventory (`contracts/reports/mens-vox-source-inventory.v1.json`) and mix config (`mens/config/vox-source-pool.yaml`) confirm:

- **What works:** `examples/golden` (`87/87` compile_ok & included) and `examples/golden-ts` (`75/75` compile_ok & included) are actively extracted (`271` files included, `206` unique content -> `1,422` AST pairs in `target/dogfood/train.jsonl` stamped `rating: 5`).
- **Leak 1 — 49 compile-failing workspace `.vox` files:** `compile_failed: 49`, including `22/51` files in `scripts/` and `3/7` in `scripts/ci/`, dropping real VoxScript CLI and automation idioms from training.
- **Leak 2 — Declaration-level AST fragmentation:** `target/dogfood/train.jsonl` decomposes `.vox` files into tiny `difficulty: 2` functions and trivial `@test` snippets without paired whole-file / multi-construct cohesive synthesis (e.g., `type` ADT + `@table` + `query`/`mutation` + `@component` + `@test`), keeping `construct_richness_mean` near the `0.125` gate floor.
- **Leak 3 — Split-brain mix configs & missing Wave 2–4 sources:** Default CLI training uses `mens/config/mix.yaml` (`target/dogfood/train_mixed.jsonl`, `seq_len: 3072`), whereas `mens/config/domain-profiles.yaml` routes `--domain vox-lang` to `mens/config/mix-vox-lang.yaml` (`mens/data/train_mixed_vox_lang.jsonl`, `seq_len: 1024`). Meanwhile, `train_mixed.mix_report.json` shows all Wave 2–4 compiler-verified sources (`target/dogfood/mutated_vox.jsonl`, `mens/data/mix_sources/back_translated.jsonl`, `mens/data/mix_sources/rft_vox.jsonl`, `mens/data/mix_sources/vox_reasoning_traces.jsonl`, `target/dogfood/preference_pairs.jsonl`) marked `optional: true` and currently `missing` (`0` rows).

## 2. Documentation Ingestion (`docs/src/` -> `extract_docs.rs` -> `mens/data/mix_sources/docs.jsonl`)

Inspecting `crates/vox-corpus/src/corpus/extract_docs.rs` and `mens/data/mix_sources/docs.jsonl` (`7,137` chunks, sampled via `sample_rate: 0.25, max_lines: 1500` in `mens/config/mix.yaml`) identified three high-impact defects:

- **Defect D1 — Ambiguous H2 Heading Prompts:** `extract_qa_sections` (`crates/vox-corpus/src/corpus/extract_docs.rs:357, 412`) emits `format!("Explain the Vox concept: {}", current_heading)` without including `frontmatter.title` or parent H1/H2 breadcrumbs. Generic section titles (`"Context"`, `"Decision"`, `"Status"`, `"Consequences"`, `"Overview"`, `"Returns"`) collide across dozens of ADRs and reference docs with conflicting target responses.
- **Defect D2 — Silent 100% Filter Drop in Domain Profiles:** `extract_qa_sections` assigns `rating: 3` (`rating: 3u8.saturating_sub(staleness_penalty).max(1)`) and `category: "documentation"` (`extract_docs.rs:380-383`), whereas `mens/config/domain-profiles.yaml` sets `min_rating: 4` for `vox-compiler`, `populi-meta`, `research`, and `research-expert`, and omits `"documentation"` from `vox-lang.context_filter.categories`. Domain-profile runs therefore silently filter out 100% of documentation Q&A rows.
- **Defect D3 — Unverified Doc Code Fences & `response_mode` Mismatch:** `extract_code_blocks` (`extract_docs.rs:243-289`) clears `preceding_context` on headings without prepending document titles, truncates preceding prose at 200 characters, and does not run `vox check` on extracted fenced blocks. Additionally, `extract_qa_sections` stamps `response_mode: "prose_only"` even when relational chunking injects ` ```vox ` files (`extract_docs.rs:360-375`).

## 3. Non-Vox Spokes & Lanes (`rust`, `tool-selection`, `argument-generation`, `harness`, `research-expert`, `populi-meta`)

- **Default `{vox_codegen}` `include_lanes` Trap:** `mens/config/mix-research-expert.yaml`, `mens/config/mix-research.yaml`, and `mens/config/mix-populi-meta.yaml` omit `include_lanes:`, causing the mixer to default to `{vox_codegen}` and silently drop domain-specific rows.
- **`rust_source.jsonl` & Empty Feedback Files:** `mens/data/mix_sources/rust_source.jsonl` (`55 MB`, `44,618` rows) carries `lane: "rust_context"`, yielding `0/44,618` rows passing `mix.yaml` filters (`no_lines_passed_filters`); `mens/config/mix-rust.yaml` documents that `scripts/mens-corpus/build_rust_authoring_corpus.vox` must materialize `target/dogfood/rust_authoring.validated.jsonl` as `optional: false`. Meanwhile `mens/data/mix_sources/{autofeedback,review_findings,rust_review_dpo}.jsonl` are `0B` empty files.
- **Tooling Spokes Unmaterialized:** `mens/data/mix_sources/tool_selection_synth.jsonl` and `mens/data/tool_traces.jsonl` (`weight: 0.0` in `mens/config/mix.yaml`) are missing on disk, and all synthetic/optional sources in `mix-tool-selection.yaml`, `mix-argument-generation.yaml`, and `mix-harness.yaml` remain `optional: true`.

## 4. QLoRA Training & Data Optimization Roadmap (`Phase 406`)

1. **Token-Normalized Gradient Accumulation (`forward.rs`):** `forward_masked_ce` (`crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/forward.rs:130`) currently divides each `batch_size=1` micro-step loss by its own `mask.sum_all()`, overweighting 15-token snippets relative to 900-token multi-construct `.vox` modules and `<think>` chains. Normalizing accumulated NLL by total supervised tokens across the `grad_accum` window fixes length bias.
2. **Document-Masked Sequence Packing + RoPE Reset:** Packing multiple variable-length examples into `seq_len=3072` bins with block-diagonal attention masks and position-ID resets at `<|im_start|>` turns yields `3-5x` Metal/CUDA throughput gains without cross-example leakage.
3. **Tied-Embedding `lm_head` Guard, `rsLoRA`, and LoRA+:** Empirical `Qwen3-0.6B` runs proved that full-rank LoRA updates to tied `151,936 x d_model` embeddings without English/prose replay degrade `gsm8k` (`0.617 -> 0.453`), whereas prose replay + lower rank (`train_mixed.jsonl`, `rank=8, alpha=16, lr=8.5e-5`) raised `eval-local` pass rate to `32.69%` (`17/52`). Adding `--lora-exclude-lm-head` on tied embeddings alongside `rsLoRA` (`alpha / sqrt(r)`) and LoRA+ differential learning rates protects reasoning at `<=4B` scales.
4. **Compiler/Validator-in-the-Loop RFT & Diagnostic-Repair DPO:** Materialize `mutated_vox`, `back_translated`, `rft_vox`, and diagnostic-repair DPO pairs (`vox check` for `vox-compiler`, `cargo check` for `rust`, JSON Schema + capability catalog validation for `tool-selection`/`argument-generation`, and `research_synthesis_eval` for `research-expert`).
5. **Unified Spoke Preflight & Portable Paths:** Retire `mix.yaml` vs `mix-vox-lang.yaml` drift, resolve relative `train_path` directly in `check_corpus_threshold` (`crates/vox-ml-cli/src/commands/schola/train/run_train.rs:13`), emit strictly workspace-relative paths in run manifests, and enforce non-empty, low-collision spoke gates via `vox mens corpus verify-spokes`.
