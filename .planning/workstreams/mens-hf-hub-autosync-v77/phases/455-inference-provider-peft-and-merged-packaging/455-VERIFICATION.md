---
phase: "455"
slug: "455-inference-provider-peft-and-merged-packaging"
title: "Native BF16 Causal-LM Recombine, Zero-Alloc PEFT Export, Tokenizer/Chat Template & Inference Seams"
status: "passed"
verified_at: "2026-11-15T06:52:00Z"
requirements:
  - "HF-PKG-01"
  - "HF-PKG-02"
  - "HF-PKG-03"
  - "HF-PKG-04"
---

# Phase 455 Verification

## Outcome: PASSED

Phase 455 deliverables (`HF-PKG-01` through `HF-PKG-04`) are complete and verified against Milestone `v7.7` criteria.

## Artifacts & Requirements Coverage

| Requirement | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `HF-PKG-01` | `crates/vox-quantize/src/read.rs:250-272`, `:607-644`; `crates/vox-quantize/src/recombine.rs:13-36`, `:73-172`, `:190-370` | Verified | `SafeTensorsSource::load_native` (`read.rs:252-272`) loads tensors on CPU in their native storage dtype (`BF16`/`F16`/`F32`) without upcasting to `F32`. `recombine_hf_causal_lm` (`recombine.rs:77-101,334-346`) preserves native base tensor dtypes and flattens multimodal `Qwen3_5ForConditionalGeneration` configs/weights (`model.language_model.*` -> `model.*`, setting `architectures: ["Qwen3_5ForCausalLM"]` and `model_type: "qwen3_5_text"`, stripping `vision_config`, `mtp_num_hidden_layers`, and vision/video token IDs in `flatten_multimodal_qwen35_config`, `recombine.rs:23-30,138-172,292-294`). |
| `HF-PKG-02` | `crates/vox-populi/src/mens/peft_export.rs:185-335`; `crates/vox-populi/src/mens/hub_sync.rs:870-909` | Verified | `export_peft_from_candle` (`peft_export.rs:188-335`) rewrites the SafeTensors JSON header in-place (`base_model.model.*`, `.lora_A.weight` / `.lora_B.weight`, `__metadata__.format = "pt"`) and stream-copies raw tensor bytes via `std::io::copy` (`peft_export.rs:305-306`) with zero float buffer allocations, emitting `adapter_model.safetensors` and `adapter_config.json` (`use_rslora`, `r`, `lora_alpha`, `target_modules`, normalized `base_model_name_or_path`). `stage_publish_bundle` (`hub_sync.rs:870-909`) stages PEFT into `peft/` when `distribute_merged == true` (stripping `model.language_model.` when `is_qwen3_5_causal_lm_config`) and at repo root when adapter-only, while always retaining `candle_qlora_adapter.safetensors` in root. |
| `HF-PKG-03` | `crates/vox-ml-cli/src/commands/schola/merge_qlora.rs:245-274`, `:392-512`; `crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs:51-151`, `:321-347`; `crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/forward.rs:311-332`, `:616-628` | Verified | `finish_recombined` (`merge_qlora.rs:471-512`) copies `tokenizer.json`, `tokenizer_config.json`, `special_tokens_map.json`, `generation_config.json`, and `chat_template.jinja`, and writes an Ollama `Modelfile` with `OLLAMA_CANONICAL_VOX_SYSTEM` (`render_ollama_modelfile`, `merge_qlora.rs:392-443`). `prepare_bench_item_for_model` (`eval_local_prompt.rs:51-151`) uses `model_uses_no_think_user_switch(model_hint)` so `Qwen3.5` / `Qwen3.8` omit literal `" /no_think"` while retaining the empty `<think>` prefill. `gradient_checkpoint_segments_with` (`forward.rs:311-324`) auto-scales segment count to `(num_layers / 2).clamp(16, num_layers)` when `num_layers >= 48` (e.g., `24` for 48 layers, `32` for 64 layers) and honors `VOX_MENS_GC_SEGMENTS`. |
| `HF-PKG-04` | `crates/vox-code-audit/src/detectors/llm_provider_call.rs:215-225`, `:428-440` | Verified | `llm_provider_call.rs:218-222` allowlists read-only `/api/collections` catalog endpoints alongside `/api/models`, `/v1/models"`, and `/v1beta/models?` when `!line.contains("generateContent")`. |

## Automated Checks Run

- Pre-turn verified Cargo baseline in `target/v77-verify`:
  - `cargo test -p vox-code-audit --lib llm_provider_call` -> exit `0` (`11 passed; 0 failed; 0 ignored; 477 filtered out`).
  - `cargo test -p vox-quantize --features engine --lib` -> exit `0` (`70 passed; 0 failed`).
  - `cargo test -p vox-plugin-mens-candle-core --lib gradient_checkpoint` -> exit `0` (`2 passed; 0 failed; 0 ignored; 89 filtered out`).
  - `cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib` -> exit `0` (`608 passed; 0 failed; 4 ignored`), covering `peft_export::tests` and `hub_sync::tests`.
