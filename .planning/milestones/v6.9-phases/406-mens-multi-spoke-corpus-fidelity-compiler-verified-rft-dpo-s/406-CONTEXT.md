---
phase: "406"
title: "MENS Multi-Spoke Corpus Fidelity, Compiler-Verified RFT/DPO & Trainer Optimizations"
status: "discussed"
track: "infrastructure-tooling"
requirements:
  - MENS-CORPUS-406-01
  - MENS-CORPUS-406-02
  - MENS-CORPUS-406-03
  - MENS-CORPUS-406-04
  - MENS-CORPUS-406-05
  - MENS-CORPUS-406-06
---

# Phase 406 Context

## Scope & Rationale

Pure technical infrastructure phase implementing the CPU-verifiable findings from [`docs/src/architecture/mens-multispoke-corpus-and-tuning-research-2026.md`](../../../docs/src/architecture/mens-multispoke-corpus-and-tuning-research-2026.md):

1. **Docs & AST Extraction Fidelity (`MENS-CORPUS-406-01`, `MENS-CORPUS-406-02`):**
   - `extract_docs.rs` currently strips ````vox` code blocks cleanly (`docs_code` lane, `response_mode: "code_only"`), but `extract_qa_sections` still needs clean filtering against non-editorial / malformed prose, and `DocTrainingPair` rating/lane/task_family routing must make prose replay explicit without polluting pure codegen spokes.
   - `extract_vox/part_walk.rs` must record AST-verified decl metadata or avoid degenerate 1-line fragments while emitting multi-turn / golden-preserving provenance.
   - `dpo.rs` must prefer compiler-verified AST mutations and keep `chosen` passing `vox_compiler::pipeline::run_frontend_str` before emitting syntax-preference pairs.

2. **All-Spoke Mix Coverage (`MENS-CORPUS-406-03`):**
   - `mens/config/domain-profiles.yaml` and spoke mixes (`mix-research-expert.yaml`, `mix-research.yaml`, `mix-populi-meta.yaml`, `mix-rocks.yaml`, `mix-agents.yaml`) must declare explicit `include_lanes:` filters (matching sibling `mix-vox-lang.yaml`, `mix-rust.yaml`, `mix-argument-generation.yaml`, `mix-harness.yaml`, `mix-tool-selection.yaml`) and keep `spoke_validate.rs` green.

3. **Trainer Correctness & Efficiency (`MENS-CORPUS-406-04`, `MENS-CORPUS-406-05`, `MENS-CORPUS-406-06`):**
   - Relative path resolution in `check_corpus_threshold` (`run_train.rs`) allowing `--data-dir` or `--train-path` (`<path>.jsonl`) without host-specific paths.
   - `LoraTrainingConfig` + CLI flags + `candle_qlora_train`:
     - `lora_exclude_lm_head` (already partially modeled or defaulted for tied-embedding models; make explicit and configurable end-to-end).
     - `use_rslora` (`alpha / sqrt(rank)` scaling in `peft-rs` / `candle_qlora_train`).
     - `token_normalized_grad_accum` (scale backward loss by valid supervised tokens per micro-batch relative to accumulation window rather than fixed micro-batch count).
     - `sequence_packing` + `forward_packed` (`block_diagonal_causal_mask` + per-segment RoPE position reset on `Device::Cpu` unit tests and Metal runtime).

## Boundaries & Sibling Isolation

- Concurrent `Phase 405` session owns `.planning/phases/405-*`, `crates/vox-corpus/{Cargo.toml,src/training/contract.rs}`, `crates/vox-populi/Cargo.toml`, `crates/vox-{compiler,db,cli,cli-ci,code-audit}`, `.cargo/config.toml`, `lefthook.yml`, `layers.toml`, `contracts/ci/fan-in-snapshot.v1.json`.
- Phase 406 MUST NOT touch any `Phase 405` files and MUST use 100% workspace-relative paths.
- While Metal GPU ladder evaluations run (`Qwen3-14B`, `Qwen3-8B`), Phase 406 tests MUST target `Device::Cpu` unit tests only (`cargo test -p <crate> --lib <filter>`) and MUST NOT rebuild `target/release/vox-ml-cli` or `.dylib` plugins.
