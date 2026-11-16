# Phase 426 Summary: Parallel HF Text-Tower Prefetch & Multi-Spoke Corpus Regeneration

**Status:** Complete  
**Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)  
**Phase:** `426-prefetch-and-corpus`

## Delivered Scope

### Wave 2 — HF Text-Only Snapshot Prefetch (`426-N1`, `426-N2`, `426-N3`)
All 5 authorized pinned checkpoints cached cleanly under `~/.cache/huggingface/hub/` with `tokenizer.json` and `config.json` present and zero vision-only shards:
- `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca` -> `~/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca` (`model.safetensors` `1.4G`, `generation_config.json=true`, `eos_token_id: [151645, 151643]`, 0 vision-only shards)
- `Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` -> `~/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/15852e8c16360a2fea060d615a32b45270f8a8fc` (`model.safetensors-00001-of-00001.safetensors` `4.2G`, `generation_config.json=false` absent upstream, `config.json` `eos_token_id: 248044`, `tokenizer_config.json` `"248044": "<|endoftext|>"`, `"248046": "<|im_end|>"`, 0 vision-only shards)
- `Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` -> `~/.cache/huggingface/hub/models--Qwen--Qwen3.5-4B/snapshots/851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` (2 shards: `5.0G` + `3.7G`, `generation_config.json=false` absent upstream, `248044`/`248046`, 0 vision-only shards)
- `Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a` -> `~/.cache/huggingface/hub/models--Qwen--Qwen3.5-9B/snapshots/c202236235762e1c871ad0ccb60c8ee5ba337b9a` (4 shards: `4.9G`, `5.0G`, `5.0G`, `3.1G`, `generation_config.json=false` absent upstream, `eos_token: "<|im_end|>"` `248046`, `pad_token: "<|endoftext|>"` `248044`, 0 vision-only shards)
- `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` -> `~/.cache/huggingface/hub/models--Qwen--Qwen3.8-27B/snapshots/37561816e66b5b9153b1b64c10968c18c1e6f4a0` (18 shards, `55562855904` bytes / ~55.56 GB, `generation_config.json=true`, `eos_token_id: [248046, 248044]`, `num_hidden_layers: 64`, `vocab_size: 248320`, 0 vision-only shards)

### Wave 2A — Corpus Pipeline & Qwen3.5 EOS/Sidecar Fixes
- **`scripts/mens-corpus/build_rust_authoring_corpus.vox`:** Removed service-surface comment markers (`@query/@mutation/@component` -> retired prose) so `vox run` stays on the interpreter lane instead of `BuildMode::App`, and replaced O(N^2) `while` clone loops with O(N) `split`/`replace`; executes in ~3s over `25440` lines (`input_lines=25440`, `relabeled_lines=25440`, `output_lines=25440`).
- **`crates/vox-corpus/src/corpus/extract_rs.rs` (L478–488):** Switched `write_to_jsonl` from `.append(true)` to `.write(true).truncate(true)` so repeated runs emit `25440` clean lines without append bloat.
- **`crates/vox-corpus/src/training/mix_prepare.rs` (L672) & `crates/vox-ml-cli/src/commands/corpus/mod.rs` (L525–545):** Exposed `pub fn ensure_missing_mix_sources` and wired it into `CorpusAction::Mix` so missing mix components auto-synthesize before mixing.
- **`crates/vox-plugin-mens-candle-metal/src/{inference.rs,candle_qlora_train/finalize.rs}`, `crates/vox-plugin-mens-candle-cuda/src/{inference.rs,candle_qlora_train/finalize.rs}`, and `crates/vox-plugin-mens-candle-metal/tests/qwen38_vlm_e2e_smoke.rs`:** Added `_tokenizer.token_to_id("<|im_end|>")` EOS fallback (handling upstream `Qwen3.5-{2B,4B,9B}` repos without `generation_config.json`), added `generation_config.json` and `tokenizer_config.json` sidecar copying in `finalize_training_run`, added `qwen35_without_generation_config_includes_im_end_eos`, rebuilt `./target/release/vox-ml-cli` (`mens-candle-metal`), and installed `mens-candle-metal` v0.6.0 plugin.

### Wave 2B — Corpus Generation, Validation & All 7 Spoke Mixes
- `mens corpus generate --force-regen`: `6772` synthetic pairs (`mens/data/synthetic.jsonl` `6622` lines, organic `252/292`).
- `mens corpus extract`: `232` lines -> `mens/data/validated.jsonl`.
- `mens corpus extract-docs`: `6969` pairs -> `mens/data/mix_sources/docs.jsonl`.
- `mens corpus extract-gsd`: `38` pairs -> `mens/data/mix_sources/gsd_workflows.jsonl`.
- `mens corpus extract-rs`: `25440` pairs -> `mens/data/mix_sources/rust_source.jsonl`.
- `vox run scripts/mens-corpus/build_rust_authoring_corpus.vox`: `25440` lines -> `target/dogfood/rust_authoring.validated.jsonl`.
- `mens corpus validate-batch`: Input `232`, accepted pre-dedup `231`, accepted after dedup `229`, rejected JSON `0`, rejected compiler `1`, rejected reward `0` -> `mens/data/validated.clean.jsonl`.
- `mens corpus pairs mens/data/validated.clean.jsonl -o target/dogfood/train.jsonl --docs docs/src`: `1485` pairs (`1163` unique answers, `299` error-correction) -> `target/dogfood/train.jsonl`.
- `mens corpus eval target/dogfood/train.jsonl --print-summary`: `1485` samples, `59.1%` parse rate, `34.1%` coverage (`15/44` taxonomy constructs).
- All 7 mix configs generated non-empty datasets (`wc -l` total `39365` across `8` JSONL files):
  - `target/dogfood/train.jsonl`: `1485`
  - `target/dogfood/train_mixed.jsonl` (`mix.yaml`): `4125` (`1731` `vox_codegen`, `1894` `vox_docs_qa`, `500` `vox_research_expert`)
  - `mens/data/train_mixed_vox_lang.jsonl` (`mix-vox-lang.yaml`): `7089` (`120` `vox_codegen`, `6969` `vox_docs_qa`)
  - `mens/data/train_mixed_rust.jsonl` (`mix-rust.yaml`): `25440` (`25440` `vox_rust_authoring`)
  - `mens/data/train_mixed_tool_selection.jsonl` (`mix-tool-selection.yaml`): `252` (`252` `vox_tool_selection`)
  - `mens/data/train_mixed_argument_generation.jsonl` (`mix-argument-generation.yaml`): `260` (`260` `vox_argument_generation`)
  - `mens/data/train_mixed_agents.jsonl` (`mix-agents.yaml`): `164` (`158` `vox_tooling`, `4` `vox_argument_generation`, `2` `vox_tool_selection`)
  - `mens/data/train_mixed_harness.jsonl` (`mix-harness.yaml`): `550` (`264` `vox_argument_generation`, `254` `vox_tool_selection`, `32` `vox_tooling`)
