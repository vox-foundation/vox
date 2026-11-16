---
title: "MENS & Workspace Model Selection Deep Research (November 2026)"
description: "Adversarially audited November 2026 evaluation of Vox MENS dense QLoRA bases, +32B Qwen MoE paths, non-Qwen authors and tradeoffs, dedicated harness-control models, Qwen3.0 lane decoupling, retrieval embedders, speech STT models, and eight blocking Candle/quantizer parity defects."
category: "Architecture SSOTs"
status: "research"
topic: "mens-training"
domain: "ml"
authorship: "llm-generated"
authorship_model: "google-deepmind-agent"
training_eligible: false
---

# MENS & Workspace Model Selection Deep Research (November 2026)

<!-- kb:summary -->
This adversarially audited report evaluates every model family trained, served, or orchestrated across Vox as of November 2026, covering dense QLoRA upgrades, `+32B` MoE execution paths, non-Qwen alternatives with explicit capability-vs-cost tradeoffs, permissive harness-control specialists, and Qwen3.0 lane-migration requirements. It documents live Hugging Face commit pins and license traps, details eight blocking Candle and `vox-quantize` parity defects across `qwen3_5` and `qwen3_8`, and separates Stage A–C fixes from Stage D hardware gates before modifying live hub cards.
<!-- /kb:summary -->

> Scope constraint: This report documents verified research and codebase findings only. Do not update `mens/config/gpu-specs.yaml`, `mens/config/hub-cards.yaml`, `crates/vox-populi/src/mens/hub_card/svg.rs`, or `platformSsot` generated outputs until Stage D cached-base smoke checks and paired `vox-eval` gates succeed.

---

## 1. Confidence Rubric & Evaluation Discipline

- **`★★★` Primary Verified:** Observed directly on `2026-11-13` via `git ls-remote`, raw `config.json` / official Hugging Face `README.md` cards, or exact workspace source reads.
- **`★★` Engineering Analysis / Synthesized Finding:** Derived from verified upstream formulas, model geometry, and inspected Rust/TypeScript code paths across multiple crates.
- **`★` Unmeasured Estimate:** Footprint or throughput projection where real accelerator execution on target hardware has not yet completed.

All candidate local training weights are gated on:
1. Permissive licensing (**Apache-2.0** verified on the HF repo — custom community licenses and gated weights are excluded from automated MENS training).
2. Revision pinning to full 40-character Hugging Face commit SHAs verified on `2026-11-13`.
3. Passing Candle forward/backward/inference numerical parity, cached base smoke checks, full spoke training, and `vox-eval` paired collateral gates before touching `mens/config/gpu-specs.yaml` or `mens/config/hub-cards.yaml`.

---

## 2. Executive November 2026 Recommended Matrix (`★★★`)

### 2.1 Dense QLoRA Ladder (`mens/config/gpu-specs.yaml` Future Targets)

| Tier / Role | Current Pinned Base | Recommended Nov 2026 Target | Live HF `refs/heads/main` SHA (`2026-11-13`) | License | Architecture (`model_type`) | Action Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| CPU Smoke & WebGPU Browser | `Qwen/Qwen3-0.6B` (`c1899de2...`) | **KEEP `Qwen/Qwen3-0.6B`** | `c1899de289a04d12100db370d81485cdf75e47ca` | `apache-2.0` | `qwen3` (28L, `151,936` vocab, tied) | Keep; `Qwen3.5-0.8B` starves & breaks v3 WebGPU ONNX |
| 6 GB Class | `Qwen/Qwen3-1.7B` (`70d244cc...`) | **UPGRADE -> `Qwen/Qwen3.5-2B`** | `15852e8c16360a2fea060d615a32b45270f8a8fc` | `apache-2.0` | `qwen3_5` VLM (`24L`, `2048` hid, `248,320` vocab, tied) | Gated on Section 6 Candle fixes |
| 10-12 GB Class | `Qwen/Qwen3-4B` (`1cfa9a72...`) | **UPGRADE -> `Qwen/Qwen3.5-4B`** | `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` | `apache-2.0` | `qwen3_5` VLM (`248,320` vocab, `attn_output_gate`) | Gated on Section 6 Candle fixes |
| 16 GB Class (Default) | `Qwen/Qwen3-8B` (`b968826d...`) | **UPGRADE -> `Qwen/Qwen3.5-9B`** | `c202236235762e1c871ad0ccb60c8ee5ba337b9a` | `apache-2.0` | `qwen3_5` VLM (`32L`, `4096` hid, `tie_word_embeddings: false`) | Gated on Section 6 Candle fixes |
| 24-48 GB Class (Interim) | `Qwen/Qwen3-14B` (`40c06982...`) | **Retain temporarily / retire after 27B measurement** | `40c069824f4251a91eefaf281ebe4c544efd3e18` (`gpu-specs.yaml` pin) | `apache-2.0` | `qwen3` dense | `Qwen3.5` has no 14B; `3.5-9B` surpasses `Qwen3-30BA3B` on GPQA/MMLU-Pro |
| Primary Workstation Hub (`27B-32B`) | `Qwen/Qwen3.8-27B@1d4bf0f2...` (missing `license`) & `Qwen/Qwen3-32B` (`9216db57...`) | **CONSOLIDATE -> `Qwen/Qwen3.8-27B`** | `37561816e66b5b9153b1b64c10968c18c1e6f4a0` | `apache-2.0` | `qwen3_5` VLM (`64L`, `5120` hid, `248,320` vocab, untied) | Replace phantom SHA, add `license: apache-2.0`, fix inverted `floor_mb: 34000`, fix Section 6 bugs |

### 2.2 `+32B` Qwen Sparse MoE Candidates (`Requires Separate Execution Path`)

| Candidate Model | Live HF `refs/heads/main` SHA (`2026-11-13`) | Total / Active Params | Experts | License | Architecture (`model_type`) | Feasibility Assessment |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`Qwen/Qwen3.6-35B-A3B`** | `345d467b117d1fd5eda9b879a5988de9ae5f47a7` | **35B / 3B active** | 256 (`8` routed + `1` shared), `512` inter | `apache-2.0` | `qwen3_5_moe` (`40L`, `2048` hid, `248,320` vocab, untied) | **Primary +32B target**: 4-bit weights ~20 GiB, highest agentic & code scores (`SWE-bench Verified 73.4`) |
| **`Qwen/Qwen3-Coder-Next`** | `a7fbcb5c0e12d62a448eaa0e260346bf5dcc0feb` | **~80B / ~3B active** | 512 (`10` routed + `1` shared), `512` inter | `apache-2.0` | `qwen3_next` text-only (`48L`, `2048` hid, `151,936` vocab, untied) | **Alternate coding specialist**: no vision tower, larger 4-bit footprint (~44 GiB) |
| **`Qwen/Qwen3.5-122B-A10B`** | `dc4d348443bc740c68e2d77492492c11606384d5` | **122B / 10B active** | 256 (`8` routed + `1` shared), `1024` inter | `apache-2.0` | `qwen3_5_moe` (`48L`, `3072` hid) | Exceeds comfortable single-128 GB QLoRA budget with optimizer state; API/cloud tier only |
| `Qwen/Qwen3.8-Flash-Next` | — | ~180B (`125B` `6B`-act + `51B` n-gram + `4B` MTP) | MoE + QSA + 20M n-gram | **`qwen-community-1.0`** | Custom non-standard | **Disqualified**: non-Apache license (`other`) and 180B memory footprint |
| `NousResearch/Hermes-3-Llama-3.1-70B` | — | 70B dense | Dense Llama-3.1 | `llama3.1` | `llama` | **Cancelled & deleted** (`107 GiB` purged from disk) |

### 2.3 Retrieval Embedders, Speech/STT, WebGPU & Cloud Pins

| Subsystem | Current Repo Pin / Implementation | Recommended Nov 2026 Target | Live HF SHA (`2026-11-13`) & License | Rationale |
| :--- | :--- | :--- | :--- | :--- |
| MENS B3 Retrieval & Search Embedder | `Qwen/Qwen3-0.6B@c1899de...` (causal LM!) in `domain-profiles.yaml:12`, `mens_spoke_routing.rs:326`, `MensTrainingView.tsx:843,1639-1646`, `svg.rs:744-750`; `Alibaba-NLP/gte-modernbert-base` (`e7f32e3c...`, `apache-2.0`) default in `embedding_env.rs:103` (runs FNV-1a hash in `embed.rs:28-75` and `vox-search/src/library/mod.rs:390-408`) | **`Qwen/Qwen3-Embedding-0.6B`** (+ **`4B` / `8B`** server tiers; evaluate **`google/embeddinggemma-2`** for multimodal) | `97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3` (`apache-2.0`); `gte-modernbert-base`: `e7f32e3c00f91d699e8c43b53106206bcc72bb22` (`apache-2.0`); `embeddinggemma-2`: `f9972ac2...` | Real bi-encoder (`1024d` MRL `32..1024`, `32K` ctx, `75.41` MTEB Code v1) vs causal LM or hash stub; `embeddinggemma-2` (`78.68` MTEB Code) is alternate if audio/image search is required |
| Edge Speech / STT (Sherpa ONNX) | `csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8` (`main`, unpinned) & dead 404 `k2-fsa/sherpa-onnx-whisper-tiny.en` (`main`) | **Pin `parakeet-tdt-0.6b-v3-int8` SHA** & replace dead repo with **`csukuangfj/sherpa-onnx-whisper-tiny.en`** | Parakeet: `ed5d745623472f97cfc2e6c2916ee248798fc5a8`; Whisper-tiny.en: `d026532c022fa99fd789d6b32446a1df7b6bfc43` | Fixes 404 on Whisper fallback & pins native `nemo_transducer` revision (`vox-speech/src/backends/sherpa_model_config.rs`) |
| Batch / Multilingual Speech / STT | `candle_whisper.rs` (`refs/pr/15..36`); `vox-tauri-stt` unimplemented desktop stub | **`Qwen/Qwen3-ASR-0.6B`** / **`1.7B`** (server / Candle upgrade path) | `0.6B`: `5eb144179a02acc5e5ba31e748d22b0cf3e303b0`; `1.7B`: `7278e1e70fe206f11671096ffdd38061171dd6e5` (`apache-2.0`) | `5.59` / `6.31` mean WER across Open ASR Leaderboard, 52 languages/dialects, native LID |
| WebGPU Playground | `@huggingface/transformers@3.5.0` patching `onnx-community/Qwen3-*-ONNX` `model_q4f16.onnx` | Keep `Qwen3-0.6B` ONNX on v3 until **Transformers.js v4+** port | `onnx-community/Qwen3.5-0.8B-ONNX` (`8fe9dbe3...`) requires v4 | Hybrid `linear_attention` ONNX ops lack `.self_attn.q_proj` MatMul Q4 layout expected by `index.html.tmpl:1060-1160` |
| Cloud Catalog & Reference Eval Panel | `model-catalog.bootstrap.v1.json` (1000x price bug, `qwen/qwen-3.6-27b`, stale Opus/Sonnet 3.7); `platformSsot.generated.ts` (auto-generated) maps `qwen-3-coder` -> DeepSeek; `llm-panel.v1.yaml` pinned `2026-05-15` | Normalize `cost_per_1k` units in bootstrap catalog; update `qwen/qwen-3.8-27b` & `qwen/qwen-3.6-35b-a3b` and re-run `sync-platform-ssot.mjs` in Stage D | `contracts/orchestration/model-defaults.v1.yaml` (`~aliases` ok) | `model-defaults.v1.yaml` unversioned `~latest` aliases stay healthy; generated SSOT twins must be regenerated via `sync-platform-ssot.mjs`, never hand-edited |

---

## 3. Full Workspace Model Inventory & Adversarial Audit (`★★★`)

Direct workspace inspection and adversarial cross-verification found eight active configuration, routing, pricing, and test-coupling bugs that must be understood alongside any model upgrade:

1. **Phantom `Qwen3.8-27B` Pin & 4-Band Ladder Inversion (`mens/config/gpu-specs.yaml:324-372`, Gate-Protected Until Stage D):**
   - Pinned to stale revision `Qwen/Qwen3.8-27B@1d4bf0f2ff6012fd82039f2fa52739d0dd7c60c0` while HF `refs/heads/main` is `37561816e66b5b9153b1b64c10968c18c1e6f4a0`.
   - Deliberately omits `license:` because a stale September comment assumed Qwen3.8-27B was unconfirmed (`"does not match any Qwen3 dense size Alibaba has publicly released"`), causing `resolve_license_class` (`crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:240-275`) to **hard-error** unless an operator passes an explicit `--license-class apache-2.0` override.
   - **Adversarial Correction on Ladder Inversion:** Unlike `qwen3_code` (which contains `Qwen/Qwen3-4B` at `floor_mb: 34460`), `strong_code_default` (`324-359`) and `agentic_default` (`360-372`) do not include `Qwen3-4B`. Their actual 4-band `max_by_key(floor_mb)` inversion works as follows:
     - `< 34,000 MB` (16–32 GB GPUs) **fails closed** with no selectable rung;
     - `44,000..99,999 MB` selects **`Qwen3-14B` LoRA (`44000`)** over **`Qwen3.8-27B` QLoRA (`34000`)**;
     - `100,000..122,875 MB` (e.g., 128 GB Mac unified memory = `118,784 MB`) selects **`Qwen3-32B` BF16 LoRA (`100000`)** — the exact config that peaked at **353,743 MiB**;
     - `122,876..195,381 MB` (H200/B200) **downgrades** via `max_by_key(floor_mb)` to **`Qwen3-8B` QLoRA (`122876`)**.
   - **Stage C vs Stage D Resolution:** Keep `mens/config/gpu-specs.yaml` and `mens/config/hub-cards.yaml` untouched through Stage C so live `strong_code_default` / `agentic_default` profiles and fail-closed `spoke_base_resolver` unit tests remain intact; validate `Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0 --license-class apache-2.0` in Stage D before touching `gpu-specs.yaml` or `hub-cards.yaml`.
2. **Causal LM Pinned as Retrieval Embedder (`mens/config/domain-profiles.yaml:12`, `crates/vox-orchestrator/src/models/registry/mens_spoke_routing.rs:324-326`, `MensTrainingView.tsx:843,1639-1646`, `svg.rs:744-751`):**
   - `hub.embedder`, `B3_RETRIEVAL_EMBEDDER_PIN`, and `MensTrainingView.tsx` pin causal LM `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca` rather than the dedicated bi-encoder `Qwen/Qwen3-Embedding-0.6B@97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3` (`apache-2.0`). Notice `crates/vox-gui/src/commands/mens_run_reports.rs:1126` is inside a `#[test]` fixture checking `"Qwen3-Embedding-0.6B"`, whereas production defaults stay split across Rust, YAML, TSX, and `svg.rs` (the `svg.rs` hub card surface waits for Stage D).
3. **Fake Local Embedder & Direct Library Bypass (`crates/vox-search/src/embedding_env.rs:103-141`, `crates/vox-search/src/library/mod.rs:390-408`, `crates/vox-search/src/embeddings.rs:82-143`, `crates/vox-actor-runtime/src/llm/embed.rs:14-91`):**
   - Without API keys (`HF_TOKEN` -> `sentence-transformers/all-MiniLM-L6-v2`; OpenAI/OpenRouter -> `text-embedding-3-small`), `vox-search` returns `DEFAULT_LOCAL_EMBED_MODEL = "Alibaba-NLP/gte-modernbert-base"` (live HF SHA `e7f32e3c00f91d699e8c43b53106206bcc72bb22`, previously unpinned on `"refs/pr/1"`), which `llm_embed` routes into `local_embed_text()` — a deterministic **256-dimensional FNV-1a** unigram/bigram/trigram hash rather than neural weights. Furthermore, `crates/vox-search/src/library/mod.rs:390-408` invokes `local_embed_text` directly (bypassing `llm_embed`), while `VectorSpaceKey` in `embeddings.rs:82-143` enforces model + revision + dimension parity across indexed chunks.
4. **32B OOM History & 48-File Hardcoded `Qwen3-{0.6B..32B}` Coupling (`mens/config/hub-cards.yaml:97`, `crates/vox-ml-cli/src/commands/mens/size_train.rs:76-79`, `crates/vox-gui/src/commands/mens_run_reports.rs:678-715`, `crates/vox-orchestrator-mcp/src/gui_ops/mens_corpus.rs:409,494`):**
   - Operator-reported `Qwen3-32B` BF16 LoRA run (`floor_mb: 100000` in `mens/config/gpu-specs.yaml:359`) hit **353,743 MiB** peak unified memory over `168` optimizer steps (`1,185,861` tokens), confirming dense `32B` BF16 LoRA cannot share the `40B`-threshold `cache_dequantized = true` path safely.
   - `size_train.rs:76-79` writes `label.report_file_name()` (`Qwen3-{size}-{rev12}.json`) whereas `mens_run_reports.rs:713-715` reads `format!("Qwen_Qwen3-{size}.json")`, which is broken today even on `Qwen3-8B` and fails whenever non-`Qwen3-<size>` bases (`3.5-9B`, `3.8-27B`) are used.
   - Static ladder assertions (`Qwen3-{0.6B,1.7B,4B,8B,14B,32B}`) and hardcoded Qwen3 SHAs are embedded in `preset_schema.rs:68-100`, `size_select.rs:20-27,535-690`, `spoke_base_resolver.rs:32-40,344-430,788-823`, `reasoning_bench.rs:751-946`, `crates/vox-eval/tests/fixtures/`, `crates/vox-populi/tests/golden/hub_cards/`, and `contracts/mens/training-presets.v1.yaml:11`.
5. **WebGPU Browser Playground Locked to Transformers.js v3 & Dense `Qwen3` (`crates/vox-populi/templates/hub/space/index.html.tmpl:389-402,1060-1160,1338-1415`):**
   - Imports `@huggingface/transformers@3.5.0` and rewrites runtime Q4 MatMul initializers (`${onnxPrefix}_Q4`, `.self_attn.` -> `.attn.`) on `onnx-community/Qwen3-{0.6B,1.7B,4B,8B}-ONNX` (`model_q4f16.onnx`). `Qwen3.5` (`qwen3_5`) requires `@huggingface/transformers` v4+ and hybrid Gated DeltaNet ONNX graphs.
6. **Speech / STT Dead Repo (`404`), Unpinned Revisions & Desktop Tauri Stub (`crates/vox-speech/src/backends/sherpa_model_config.rs:31,75,185,246`, `candle_whisper.rs:53-74`, `crates/vox-tauri-stt/src/plugin.rs:24-31`):**
   - Live HF verification confirmed `k2-fsa/sherpa-onnx-whisper-tiny.en` returns `404` (`{"error":"Invalid username or password."}`); the live upstream repo is `csukuangfj/sherpa-onnx-whisper-tiny.en` (`d026532c022fa99fd789d6b32446a1df7b6bfc43`). Both `DEFAULT_SHERPA_HF_MODEL` and `DEFAULT_SHERPA_TRANSDUCER_HF_MODEL` (`csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8`, live SHA `ed5d745623472f97cfc2e6c2916ee248798fc5a8`) previously downloaded unpinned `"main"`. Candle Whisper relies on legacy `refs/pr/*`, and desktop Tauri `transcribe` returns `Err("on-device transcription is not wired...")`.
7. **Cloud Bootstrap Catalog & Generated SSOT Drift (`contracts/orchestration/model-catalog.bootstrap.v1.json:152-665`, `crates/vox-gui/ui/src/lib/platformSsot.generated.ts`, `crates/vox-orchestrator/src/platform_ssot_generated.rs`, `contracts/eval/llm-panel.v1.yaml:21,46-73`):**
   - `model-catalog.bootstrap.v1.json` mixes per-1k and per-1M pricing (`cost_per_1k: 15.0`..`50.0` on Claude/OpenAI rows alongside `0.00137` on others) and references stale `qwen/qwen-3.6-27b`. Both `platformSsot.generated.ts` and `platform_ssot_generated.rs` carry `DO NOT EDIT MANUALLY` banners from `sync-platform-ssot.mjs` (reserved for Stage D regeneration alongside catalog changes). Meanwhile `contracts/eval/llm-panel.v1.yaml` remains pinned to May 2026 (`claude-sonnet-4-6`, `gpt-5.4` / `gpt-5.5-pro`).
8. **Hardware Classifier & `vram_autodetect` Obsolescence (`crates/vox-hf-layout/src/lib.rs:386-412`, `crates/vox-populi/src/mens/tensor/memory_budget.rs:9-13`):**
   - `classify_hf_architecture` collapses all `qwen`/`llama`/`mistral` subtypes into `HfArchitecture::Qwen35` and drops `gemma4` and `granite` into `Gpt2`, while `uses_zero_centered_rmsnorm()` and `effective_rope_dim()` did not exist to separate `Qwen3.5`/`Qwen3.8` from dense `Qwen3`/`Granite`. Note that `vram_autodetect::auto_preset_for` has already been removed per `memory_budget.rs:9-13`.

---

## 4. Dense Ladder & Competitor Evaluation (`★★★` / `★★`)

### 4.1 Primary Benchmarks Across Candidate Dense Families

All scores below come directly from official Hugging Face model cards retrieved on `2026-11-13`:

#### Sub-1B through 9B Ladder (`Qwen3` vs `Qwen3.5` vs `Gemma 4`)

| Benchmark | `Qwen3-1.7B` | `Qwen3.5-0.8B` | `Qwen3.5-2B` | `Qwen3-4B-2507` (Think) | `Qwen3.5-4B` | `Qwen3.5-9B` | `Gemma 4 E2B` | `Gemma 4 E4B` | `Gemma 4 12B` |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **MMLU-Pro** | `56.5` | `42.3` | **`66.5`** | `74.0` | **`79.1`** | **`82.5`** | `60.0%` | `69.4%` | `77.2%` |
| **GPQA Diamond** | `40.1` | `11.9` | **`51.6`** | `65.8` | **`76.2`** | **`81.7`** | `43.4%` | `58.6%` | `78.8%` |
| **LiveCodeBench v6** | — | — | — | — | `55.8` | `65.6` | `44.0%` | `52.0%` | `72.0%` |
| **IFEval** | `72.5` | `44.0` | **`78.6`** | `87.4` | **`89.8`** | **`91.5`** | — | — | — |
| **IFBench** | `26.7` | `21.0` | **`41.3`** | `50.4` | **`59.2`** | **`64.5`** | — | — | — |
| **BFCL-V4 (Tools)** | — | `25.3` | **`43.6`** | `39.9` | **`50.3`** | **`66.1`** | — | — | — |
| **TAU2-Bench** | — | `11.6` | **`48.8`** | `43.2` | **`79.9`** | **`79.1`** | `24.5%` | `42.2%` | `69.0%` |
| **AA-LCR (Long Ctx)** | `6.7` | `4.7` | **`25.6`** | `32.0` | **`57.0`** | **`63.0`** | — | — | — |

#### Workstation Flagship Hub (`27B-32B` Class)

| Benchmark | `Qwen/Qwen3.8-27B` | `Qwen/Qwen3.6-27B` | `google/gemma-4-31B-it` | `Opus 4.6 Max` (Qwen Ref) |
| :--- | :---: | :---: | :---: | :---: |
| **SWE-bench Pro** | **`61.7`** | `53.5` | `35.7` | `53.4` |
| **Terminal Bench 2.1 (Terminus)** | **`73.0`** | `63.4` | — | `78.2` |
| **QwenSWEBench** | **`79.0`** | `49.3` | — | `63.8` |
| **DeepSWE 1.1** | **`42.2`** | `13.3` | — | — |
| **LiveCodeBench v6** | **`90.3`** | `83.9` | `80.0%` | `88.8` |
| **IFBench** | **`79.5`** | `69.1` | — | `62.5` |
| **GPQA Diamond** | **`89.2`** | — | `84.3%` | — |
| **MMLU-Pro** | — | — | `85.2%` | — |
| **OSWorld-Verified / WebArena** | **`84.3` / `64.8`** | — | — | — |

### 4.2 Why `Qwen3.5-0.8B` Is Rejected as the Bottom Rung

Despite matching the sub-1B parameter bucket, **`Qwen/Qwen3.5-0.8B` is unsuitable as Vox's smallest spoke**:
1. **Severe reasoning & tool collapse:** `GPQA Diamond 11.9` (`vs 51.6` at `2B`), `TAU2-Bench 11.6` (`vs 48.8`), and `IFEval 44.0` (`vs 78.6`).
2. **248,320 vocab footprint penalty:** Its tied `[248320, 1024]` embedding matrix consumes `254.3M` params (`~30%` of the entire model) and inflates cross-entropy logits buffers (`1.63x` wider than `Qwen3`'s `151,936`), making its training memory overhead barely smaller than `Qwen3.5-2B` while yielding dramatically worse accuracy.
3. **WebGPU incompatibility:** Current Space inference uses `@huggingface/transformers@3.5.0` Q4 ONNX patching (`index.html.tmpl`), which supports `Qwen/Qwen3-0.6B` natively and rejects `qwen3_5`. Keeping `Qwen/Qwen3-0.6B` for CPU smoke + WebGPU while upgrading active spokes to `2B`/`4B`/`9B`/`27B` preserves browser parity with zero quality regression.

### 4.3 Why `Qwen3.5/3.8` Wins Over `Gemma 4` (`31B / 12B / E4B / E2B`)

Even though **Gemma 4** switched to permissive **`apache-2.0`** and posts competitive scores on `Gemma 4 12B` (`LiveCodeBench v6 72.0%`), **`Qwen3.5`/`Qwen3.8` remains superior for MENS today** for two engineering reasons verified directly in `google/gemma-4-31B-it/config.json`:
1. **Large coding & agentic lead:** `Qwen/Qwen3.8-27B` scores **`61.7`** vs **`35.7`** (`Gemma 4 31B`) on SWE-bench Pro and **`90.3`** vs **`80.0%`** on LiveCodeBench v6; `Qwen3.5-9B` beats `Gemma 4 E4B` (`+13.1` MMLU-Pro, `+23.1` GPQA, `+36.9` Tau2) while also trailing `Gemma 4 12B` only on pure LiveCodeBench (`65.6` vs `72.0%`).
2. **Unsupported `gemma4_unified` / `gemma4_text` runtime primitives:** `google/gemma-4-12B-it` / `31B-it` use `vocab_size: 262144`, `vocab_size_per_layer_input: 262144` (PLE embeddings on E2B/E4B), `attention_k_eq_v: true` (unified K=V), `final_logit_softcapping: 30.0`, `sliding_attention` + `full_attention` (`global_head_dim: 512`), `hidden_activation: "gelu_pytorch_tanh"`, `<start_of_turn>` chat templates, and dual `rope_parameters` (`proportional` vs `default`). Adding Gemma 4 to native Candle would require a new transformer graph, whereas `Qwen3.5` and `Qwen3.8` already share the `qwen3_5` codebase once Section 6 bugs are resolved.

### 4.4 Non-Qwen Authors & Tradeoff Matrix (`★★★` / `★★`)

Evaluating non-Qwen checkpoints shows two distinct adoption tiers: **(a) drop-in `qwen3_5` fine-tunes from Microsoft & Xiaomi** that inherit Stage A–B Candle fixes with zero new kernel graph work, and **(b) distinct non-Qwen architectures** (IBM Granite 4.2, Google Gemma 4, Mistral Devstral Small 2, Ai2 SERA, DeepSeek-V4.1-Flash, GLM-5.3-Flash) with explicit porting or runtime tradeoffs.

| Model Candidate | HF Commit SHA (`2026-11-13`) & License | Architecture & Runtime Cost | Measured Benchmark Highlights vs `Qwen3.5/3.8` | Explicit Tradeoffs & Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **`XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B`** | `1f09ddcc1a0ea45751ae0dfe2af20d533168c46f` (`mit`, `attribution_required: true`) | `qwen3_5` (`Qwen3.5-9B` base, 77.4B agentic distillation tokens) — **zero extra Candle kernel cost** | **`SWE-bench Pro 44.6`** vs `Qwen3.5-9B` `32.0` (**`+12.6`**); **`Terminal-Bench 2.1 37.1`** vs `27.0` (`+10.1`); **`Toolathlon-Verified 35.2`** vs `25.9`; **`AutomationBench 30.3`** vs `5.0`; **`SWE Verified 61.1`** vs `60.0` | **Strongest 9B agentic spoke candidate**: MIT requires attribution header; uses compact `<think></think>` & XML `<tool_call><function=NAME><parameter=K>V</parameter></function></tool_call>`. |
| **`microsoft/FrogNano-4B-2609`** | `1844d06083df631f634e417938b3b8feda5f9f48` (`apache-2.0`) | `qwen3_5` (`Qwen3.5-4B` base, TaskPilot 5-iter executable SWE RL) — **zero extra Candle kernel cost** | **`SWE-bench Verified 61.5%`** Avg@3 (`71.0%` Pass@3 vs `39.4%` base, `+22.1 pp`); **`SWE-bench Pro 37.6%`** Avg@3 (`47.6%` Pass@3 — beats `Qwen3.5-9B` `32.0%`, `Gemma 4 31B` `35.7%`, `Granite-4.2-30B` `33.29%` at 4B!); `Terminal-Bench 2.0 31.1%`; `PatchEval-Verified 47.3%` | Narrow English/Python 5-tool (`read/write/edit/glob/bash`) RL without general-knowledge retention evals — best 4B SWE/CLI specialist, higher forgetting risk than `Qwen3.5-4B` as sole VoxLang syntax base. |
| **`ibm-granite/granite-4.2-{3b,8b,30b}`** | `3b`: `4e9c275e232f10211ec88644d106a1f6a1b60e06`; `8b`: `a7042144125ce5ae64f89610e8289647fc000b9a`; `30b`: `9e668ce1c538387ef24d3644e9b0606647762636` (`apache-2.0`) | Pure dense GQA `GraniteForCausalLM` (no GDN linear-attention state!) + identical ChatML `<think>` + XML `<tool_call>` | **`8B` beats `Qwen3.5-9B`** on **`LiveCodeBench v6` (`73.24` vs `65.6`)** and **`IFBench` (`79.33` vs `64.5`)** | Weaker `GPQA` (`64.14` vs `81.7`), `BFCL v4` (`52.39` vs `66.1`), and `30B` `SWE Pro` (`33.29` vs `Qwen3.8-27B` `61.7`). Worth evaluating as fallback if any GDN linear-attention workload regresses. |
| **`google/gemma-4-{E2B,E4B,12B,26B-A4B,31B}-it`** | `E4B`: `4ecc2ba63bcac23ba4a767fc87025b1450f84c9c`; `12B`: `43c9d014b231474fa7212dbb48a762566c2b255d` (`apache-2.0`) | Custom `gemma4_unified` (`K=V`, p-RoPE, logit softcap, `<start_of_turn>`, `262K` vocab, audio/vision encoder-free) | `12B` hits `72.0%` LiveCodeBench v6 (`26B-A4B` hits `77.1%` at 4B active) and native encoder-free audio+vision | High porting cost (`gemma4_unified` Candle graph + tokenizer template) and `31B` trails `Qwen3.8-27B` on `SWE Pro` (`35.7` vs `61.7`). Use `gemma-4` via API/sidecar when multimodal audio+vision is needed. |
| **`mistralai/Devstral-Small-2-24B-Instruct-2512`** | `55c5b41e98c2dbd21b0c8afffc540dcfc9eb5128` (`apache-2.0`) | `mistral3` FP8 `compressed-tensors` + Tekken `[TOOL_CALLS]` | `~68%` SWE-bench Verified (`50.0%` at 32K in Ai2 SERA eval) | Older Nov 2025 base, FP8 weight unarchiving + Tekken `[TOOL_CALLS]` parser work; **`Devstral-2-123B`** is **disqualified** (non-OSI Modified MIT license). |
| **`allenai/SERA-{8B,32B}`** | `8B`: `359cef1e06dc791ac775bee5dd88073502c93434`; `32B`: `b4f538362b11ec5274817c8f047c10f460c21063` (`apache-2.0`) | Dense `qwen3` (zero new runtime cost; matches existing Qwen3 Candle graph) | `SERA-32B` `49.5%`, `SERA-8B` `31.7%` SWE-bench Verified via Soft-Verified Generation (SVG) | Trails `Qwen3.8-27B` as a flagship base, but its **SVG trajectory-generation recipe (26x cheaper than RL)** is directly reusable to specialize MENS agents to Vox crates. |
| **`deepseek-ai/DeepSeek-V4.1-Flash-1017`** & **`zai-org/GLM-5.3-Flash`** | `DS`: `08c8657f02291e11b6f6b63e0e16014f8e748518` (`mit`); `GLM`: `46a050bec13002ad29ee795c16e61cf4fa3c4a9c` (`mit`) | Cloud/cluster frontier MoE (`384` experts + `384M` engrams / `320B/18B` active mHC+DSA) | `DeepSeek-V4.1-Flash-1017`: **`DeepSWE v1.1 77.0`**, `Terminal-Bench 4.0 42.3`, `NL2Repo-Bench 68.2` | **Unfit for single-node local QLoRA**, but top **MIT-licensed cloud teacher & routing targets** for synthetic corpus distillation and `model-catalog.bootstrap.v1.json`. |

### 4.5 Dedicated Harness-Control Models, License Traps & Vox Orchestrator Seams (`★★★` / `★★`)

Evaluating specialized harness-control models revealed that several widely cited tool-calling and orchestration models carry **non-commercial or gated licenses**, making them **unusable** for automated Vox MENS distribution:

- **Disqualified License / Access Traps:**
  - `Salesforce/xLAM-2-{1b,3b,8b,32b,70b}-fc-r`: **`cc-by-nc-4.0`** (non-commercial).
  - `MadeAgents/Hammer2.1-{0.5b,1.5b,3b,7b}`: **`cc-by-nc-4.0` / `other`** (non-commercial).
  - `nvidia/Nemotron-Orchestrator-8B`: **NVIDIA proprietary license ("research and development only")** — adopt *only* its published **ToolOrchestra GRPO cost/latency/accuracy reward design** on Vox's own `tool-selection` spoke, never its weights.
  - `katanemo/Plano-Orchestrator-{4B,30B-A3B}` & `Arch-Agent-*`: **`license: other`**.
  - `google/functiongemma-270m-it`: **gated `license: gemma`**; `osmosis-ai/Osmosis-Apply-1.7B` & `agentica-org/DeepSWE-32B-Preview`: **gated / auth-blocked on HF**.

- **Permissive (`apache-2.0`) Harness Specialists & Direct Vox Seams:**
  1. **Safety & Stream Policy Guardrail -> `Qwen/Qwen3Guard-Gen-0.6B` (`fada3b2f655b89601929198343c94cd2f64d93cc`) / `4B` / `8B` & `Qwen3Guard-Stream-*` (`apache-2.0`, dense `qwen3`):** Zero Candle porting cost; drop-in pre-tool and stream-token safety/policy classifier.
  2. **Multi-Intrinsic Token-Span Adapter Switching -> `ibm-granite/granite-switch-4.1-8b-preview` (`8a22410af7cbbfa2cbbd39a3af281ba98c2df846`, `apache-2.0`):** Uses Activated-LoRA (`aLoRA`) to invoke 12 harness intrinsics (Requirement Check, Uncertainty, Answerability, Hallucination Detection, Guardian, Policy Guardrails) over short spans while **preserving base KV cache** — directly serving as the architectural blueprint for upgrading Vox's `DomainRouter` (`crates/vox-orchestrator/src/domain_router.rs`) and `try_agent_turn.rs` beyond lexical substring scoring (`score_conversational_turn_across_spokes` in `conversational_routing.rs`).
  3. **CLI/SWE Loop & Repo Trajectory Synthesis -> `microsoft/FrogNano-4B-2609` (`apache-2.0`) + `allenai/SERA-{8B,32B}` (`apache-2.0`):** High-signal `read/write/edit/glob/bash` patch-verify loop control + 26x-cheaper Soft-Verified Generation trajectories for Vox crate adaptation.
  4. **Fast Next-Edit / Patch Apply -> `zed-industries/zeta` (`80f73ce5ee059377ce0662ec5c45b592c3025ae5`, `apache-2.0`, `qwen2` 7B):** Sub-second deterministic edit-region prediction and hunk application.
  5. **Stuck-Loop & Effort Judges -> `HarnessIssueJudge` (`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/harness_issue_judge.rs`) & `TaskCategory::CodeEffortJudge` (`crates/vox-cli/src/commands/audit_effort.rs`):** Target small verified judges (or Granite `aLoRA` intrinsics / `Qwen3.5-2B`) instead of ad-hoc heuristics for loop termination and wasted-turn scoring.
  6. **Hybrid GUI + `bash` Computer Use -> `microsoft/Mage-VL-Cowork` (`6fbe7f91da477c1c73790dbb1f08d001b8626be1`, `apache-2.0`, 4B, `66.0%` OSWorld-Verified) & `ByteDance-Seed/UI-TARS-1.5-7B` (`683d002dd99d8f95104d31e70391a39348857f4e`, `apache-2.0`):** Candidate visual+terminal harnesses when browser/GUI automation moves beyond DOM scripting.
  7. **Domain-Routing & Salvage Envelopes -> `conversational_routing.rs` & `salvage_and_stream.rs`:** Replace substring heuristics and JSON-only `<tool_call>` salvage with unified XML + JSON extraction and semantic/aLoRA routing.

---

## 5. `+32B` Qwen Sparse MoE Deep-Dive & Execution Paths (`★★★` / `★★`)

The `+32B` Qwen tier **cannot share the dense QLoRA trainer**. Inspecting `Qwen/Qwen3.6-35B-A3B` (`config.json`, `model_type: "qwen3_5_moe"`) and `Qwen/Qwen3-Coder-Next` (`config.json`, `model_type: "qwen3_next"`) against `crates/vox-plugin-mens-candle-*` shows why native Candle cannot train or serve MoE today:
- **Classifier Blindness:** `crates/vox-hf-layout/src/lib.rs:386-412` returns `HfArchitecture::Qwen35` for all `"qwen3"` substrings without parsing `num_experts`, `num_experts_per_tok`, `moe_intermediate_size`, `shared_expert_intermediate_size`, or `mlp_only_layers`.
- **Linker Stubs Panic on CUDA:** `crates/vox-plugin-mens-candle-cuda/src/moe_stubs.rs:23-80` deliberately provides panicking stubs for `moe_gemm_wmma`, `moe_gemm_gguf`, and `moe_gemm_gguf_prefill` so pure-Rust `candle-kernels` links without `nvcc`.
- **No Sparse Router or 3D Expert Linear (`[E, out, in]`):** Both Metal and CUDA model graphs assume dense `gate_proj` / `up_proj` / `down_proj` per layer, with zero top-k router gate, shared-expert residual, auxiliary load-balancing loss, or NF4 quantization over 3D grouped expert tensors (`num_experts = 256` or `512`).

### 5.1 `+32B` Qwen Candidate Comparison (`★★★`)

| Metric / Property | **`Qwen/Qwen3.6-35B-A3B`** | **`Qwen/Qwen3-Coder-Next`** | **`Qwen/Qwen3.5-122B-A10B`** |
| :--- | :--- | :--- | :--- |
| **Params (Total / Active)** | **35B / 3B** | **~80B / ~3B** | **122B / 10B** |
| **Layers / Hidden** | `40` layers (`3:1` DeltaNet:Attn), `2048` hid | `48` layers (`3:1` DeltaNet:Attn), `2048` hid | `48` layers (`3:1`), `3072` hid |
| **Experts / Routing** | `256` experts (`8` routed + `1` shared), `inter = 512` | `512` experts (`10` routed + `1` shared), `inter = 512` | `256` experts (`8` routed + `1` shared), `inter = 1024` |
| **Vocab / Vision** | `248,320` (`Qwen3_5MoeForConditionalGeneration`, vision) | **`151,936` (`Qwen3NextForCausalLM`, text-only)** | `248,320` (VLM) |
| **SWE-bench Verified** | **`73.4`** | `70.6` | `72.0` |
| **SWE-bench Pro** | **`49.5`** | `44.3` | — |
| **Terminal-Bench 2.0** | **`51.5`** | — | `49.4` |
| **LiveCodeBench v6** | **`80.4`** | — | `78.9` |
| **GPQA Diamond** | **`86.0`** | — | **`86.6`** |
| **Approx 4-bit Weights (`★`)** | **~20 GiB** (fits comfortably in 128 GB Mac + 32-48 GB GPU) | **~44 GiB** (fits 128 GB Mac Unified Memory with tight headroom) | **~65+ GiB** weights alone (tight/unfeasible with full-batch states on 128 GB desktop) |

**Winner (`★★`):** **`Qwen/Qwen3.6-35B-A3B`** outperforms both `Qwen3-Coder-Next` (on SWE Verified `+2.8` and SWE Pro `+5.2`) and `Qwen3.5-122B-A10B` (on SWE Verified & Terminal-Bench 2.0) while needing under half the 4-bit weight RAM of `Coder-Next`. However, observe that dense **`Qwen/Qwen3.8-27B`** scores materially higher than `35B-A3B` on coding & tool agents (**`SWE-bench Pro 61.7` vs `49.5`**, **`LiveCodeBench v6 90.3` vs `80.4`**); the value of `Qwen3.6-35B-A3B` is **3B-active inference latency** rather than beating `3.8-27B` accuracy.

### 5.2 Execution Path Options for `+32B` Qwen MoE (`★★`)

| Option | Mechanism | Timeline & Risk | Hardware Coverage | Tradeoff |
| :--- | :--- | :--- | :--- | :--- |
| **Path A: Sidecar Bridge (`mlx-lm` on Apple Silicon / Python PEFT `vox run` worker)** | Dispatch `+32B` MoE lanes via `.vox` script (`vox run`) invoking `mlx_lm.lora` (4-bit MoE on macOS Unified Memory) or PyTorch/PEFT + `bitsandbytes` (`qwen3_5_moe` / `qwen3_next` native Transformers >= `4.57.1`), emitting standard `candle_qlora_adapter.safetensors`-compatible LoRA adapters + run telemetry back to `vox-ml-cli` / `vox-eval`. | **Short (days)**; inherits upstream HF kernel parity (`RMSNormZeroCentered`, `attn_output_gate`, `partial_rotary_factor`, grouped expert routing) with zero custom Metal/CUDA kernel bugs. | `mlx-lm` targets Apple Silicon (128 GB Studio/Max); PEFT targets CUDA (`48-96+ GB`). | Requires external runtime dependency (`mlx-lm` / Python env) rather than single-binary pure-Rust execution (`crates/vox-python-bridge/worker/` exemption in `AGENTS.md`). |
| **Path B: Native Candle MoE (`vox-plugin-mens-candle-*`)** | Extend `vox-hf-layout` for `qwen3_5_moe` / `qwen3_next`, implement `SparseMoeBlock` + `moe_gemm_*` CUDA kernels (replacing `moe_stubs.rs`) + Metal batched expert matmul kernels + NF4 3D `[E, out, in]` storage and `vox-quantize` 3D expert k-quants. | **Long (multi-week)**; high kernel & autograd complexity (top-k scatter/gather backward gradients, NF4 per-expert group quantization, router auxiliary loss). | Single-binary pure Rust across Metal + CUDA once built. | Blocks `+32B` MoE experiments until kernel work finishes; low ROI given dense `Qwen3.8-27B` already outperforms `35B-A3B` on `SWE-bench Pro` and `LiveCodeBench`. |

---

## 6. Critical Candle `Qwen3.5` / `Qwen3.8` & `vox-quantize` Parity Audit (`★★★`)

**Do NOT switch live `gpu-specs.yaml` or HF bases to `Qwen3.5-2B/4B/9B` or `Qwen3.8-27B` before resolving all eight defects (`A1`–`A8`)** verified directly against upstream Hugging Face `transformers` (`modeling_qwen3_5.py`) and Vox's Metal, CUDA, layout, and quantizer crates:

| ID | Defect Summary & Upstream Spec (`modeling_qwen3_5.py`) | Broken Metal / Core Sites | Broken CUDA / Quantize / Layout Sites | Numerical / Runtime Impact |
| :--- | :--- | :--- | :--- | :--- |
| **A1** | **`Qwen35LinearAttention::l2norm_last` computes RMSNorm instead of L2Norm:** Divides squared sum `sq` by `d = 128` before `rsqrt`, making $\\|k_t\\|_2^2 = 128$ instead of $1.0$. | `candle-metal/src/model.rs:1154-1161` | `candle-cuda/src/model.rs:341-348` | Turns $(I - \beta_t k_t k_t^\top)$ into $(I - 128\beta_t u_t u_t^\top)$, exploding the recurrent state whenever $\beta_t > 0.0156$. |
| **A2** | **Missing $q$ scaling, overflowing softplus, and arbitrary clamps:** Upstream multiplies `query` by $1/\sqrt{\text{head\_k\_dim}}$ (`repeat_interleave` `48/16 = 3` on `27B`), uses overflow-safe softplus `F.softplus`, and applies no logits clamp. | `candle-metal/src/model.rs:1284-1343`, `1824` (`[-64,64]`) | `candle-cuda/src/model.rs:195,199,447,451,453-460,723` | Unscaled $q$, `g_pre.exp() + 1` overflow above `88.7`, and spurious clamps distort GDN decay $\exp(-e^{A} \text{softplus}(\cdot))$ and LM logits. |
| **A3** | **Time-reversed `conv1d` kernel taps on CUDA:** Inner loop pairs `x[t - j]` with `conv_weight[j]` instead of `conv_weight[k - 1 - j]`. | Fixed (`metal/src/model.rs:1192-1193`) | `candle-cuda/src/model.rs:367` | Every CUDA `qwen3_5` GDN layer runs backwards short causal convolution, breaking Metal/CUDA parity. |
| **A4** | **`q_proj` truncation & wrong `attn_output_gate`:** Upstream reshapes `[b, s, n_heads, 2*head_dim]`, chunks `(q, gate)` per head, and gates with `sigmoid(gate)`. | `metal/.../mod.rs:822-832,848-857`; `metal/model.rs:498-505,621-624,675-682,864-867,898-901` (flat-half + `silu`) | `cuda/.../mod.rs:741-751,768-776`; `cuda/model.rs:145-209` (no split); `candle-core/src/merge.rs:306-315` | Trainers chop off half of `q_proj`, Metal uses wrong head split + `silu` instead of `sigmoid`, CUDA panics on `[b,s,2*q_dim]`, and `merge.rs` assumes legacy top-half deltas. |
| **A5** | **Missing `zero_centered` RMSNorm (`1.0 + weight`) & hardcoded `1e-6` eps:** `ln1`, `ln2`, `q_norm`, `k_norm`, `final_norm` are `1.0 + w` on `qwen3_5*`/`qwen3_8*`/`qwen3_next`, while **`linear_attn.norm` (`Qwen3_5RMSNormGated`) stays `1.0`-centered** (and `qwen3`/`qwen2`/`llama`/`mistral`/`granite` stay `1.0`-centered). | `candle-core/src/qk_norm.rs:17-23`; `metal/.../mod.rs:674-683,867-870,1043-1053`; `metal/inference.rs:496-503,594-599,655-659` | `cuda/.../mod.rs:595-602,...`; `cuda/inference.rs:224-231,297-300,340-345,401-405` (hardcoded `1e-6`) | Initial weights near `0.0` collapse layer-0 activations near zero, corrupting base logits. |
| **A6** | **`partial_rotary_factor = 0.25` (`64` of `256` dims) ignored:** Inference & some fallback `inv_freq` paths pass `head_dim = 256` instead of `rope_dim = 64`. Note: text-only interleaved MRoPE (`mrope_section = [11, 11, 10]`, $t = h = w$) selects identical rotary frequencies per dim and reduces bit-identically to 1D RoPE over `0..64`. | `metal/inference.rs:638-644`; `metal/.../mod.rs:633-639` | `cuda/inference.rs:389-391`; `cuda/.../mod.rs:556-557`; `vox-hf-layout` missing helper | Rotates the non-rotary `192` head channels during inference and fallback training runs. |
| **A7** | **Quantized `in_proj_a`/`in_proj_b` & CUDA training parity gaps:** Small `[16..48, d_model]` scalar gate matrices are NF4-quantized despite `KeepF32`, plus CUDA lacks `lm_head_key_candidates`, `use_rslora`, `lora_exclude_lm_head`, and `vb_mmap_cpu`. | `metal/inference.rs:524-531`; `metal/.../mod.rs` | `cuda/inference.rs:256-267,406-414`; `cuda/.../mod.rs` | NF4 noise corrupts scalar decay rates $\beta_t, g_t$; missing flags silently diverge CUDA LoRA runs from Metal. |
| **A8** | **`vox-quantize` shard `ENOENT`, overlay, tied-head & sharded Metal gaps + Stage B memory traps:** `SafeTensorsSource::open` fails on missing `model.visual.*`/`mtp.*` shards and single-file overlay bases; `recombine.rs` rejects synthesized `lm_head.weight`; Metal quantized loader assumes single `model.safetensors`. Stage B adds `auto_cache` `40B` BF16 blowup (54 GB on 27B / 353 GiB on 32B), unchunked `[seq, 248320]` FP32 CE (`3.05 GiB`), disabled linear-attention packing, and disabled cached hybrid decode (`1727-1737`). | `metal/inference.rs:215-225,847-905`; `metal/model.rs:1638-1642,1727-1737`; `metal/.../training_loop/{mod,forward}.rs` | `vox-quantize/src/{policy,read,engine,recombine}.rs`; `cuda/.../{mod,training_loop/forward}.rs`; `cuda/inference.rs:540-586`; `vox-hf-layout/src/lib.rs` | Crashes VLM quantization on text-only shard downloads, rejects untied adapter export, OOMs 27B/32B, and forces $O(T^2)$ slow generation. |

### 6.2 Existing Qwen3.0 Lane Coupling & Migration Matrix (`★★★`)

Auditing `vox-eval`, `vox-corpus`, `vox-populi`, and `vox-orchestrator-mcp` shows six areas where Vox's existing training and evaluation lanes hardcode **Qwen3.0** conventions:

1. **`" /no_think"` User-Turn Suffix (`qwen_reasoning.rs`, `training_text.rs`, `inference.rs`):** Appending `" /no_think"` works only for `Qwen3` (`Qwen3-0.6B..32B`); `Qwen3.5`, `Qwen3.8-27B`, `MiMo-V2.6-Distill`, `FrogNano`, and `Granite-4.2` tokenize `" /no_think"` as literal user text rather than a reasoning switch while still honoring assistant `<think>\n\n</think>\n\n` prefill. Lane/chat assembly must gate `" /no_think"` strictly to dense `qwen3` (`model_uses_no_think_user_switch`) and skip it on `qwen3_5`/`qwen3_6`/`qwen3_8`/`qwen3_next`/`qwen3guard`/non-Qwen bases.
2. **XML vs JSON `<tool_call>` Serialization (`vox-eval/episode.rs`, `salvage_and_stream.rs`, `vox-corpus`):** Live `Qwen/Qwen3.8-27B/tokenizer_config.json`, `XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B`, and `ibm-granite/granite-4.2` emit XML `<tool_call><function=NAME><parameter=KEY>VALUE</parameter></function></tool_call>` rather than `Qwen3.0`'s JSON `{"name": ..., "arguments": ...}`. Both `parse_tool_call` (`vox-eval`) and `salvage_tool_calls_from_text` (`vox-orchestrator-mcp`) must parse both JSON and XML formats cleanly. Separately (corpus cleanup follow-up), `tool_selection_synth.rs` and `argument_generation_synth.rs` are not yet wired into `mix_prepare.rs`, and `agentic_synth.rs` emits `"result"`/`"success"` keys that diverge from `<tool_call>` envelopes.
3. **`248,320` Vocab, EOS IDs & Pad Token ID (`Qwen3.5` / `Qwen3.8` / `MiMo` / `FrogNano`):** Token IDs shifted from `151,936` (`<|endoftext|>=151643`, `<|im_start|>=151644`, `<|im_end|>=151645`) to `248,320` (`<|endoftext|>=248044`, `<|im_start|>=248045`, `<|im_end|>=248046`, `<tool_call>=248058`, `<think>=248068`). Both inference engines must union `generation_config.json` `eos_token_id` arrays (`[248046, 248044]`) with `config.json` rather than assuming a single scalar EOS ID or hardcoded `151645`.
4. **`preset_schema.rs` `seq_len <= 768` System-Prompt Truncation & Name Coupling:** Classifiers and presets hardcoded `Qwen3-{0.6B..32B}` and capped `S32` at `seq_len = 768`, which left-truncates Vox's `~1.1k`-token `system_prompt.txt` (`S27` for 48+ GB GPUs requires `>= 2048` `seq_len`, and `detect_qwen_size_class` must classify `2B`/`9B`/`27B` and non-Qwen names while ignoring 40-char HF revision SHAs).
5. **Report & Preset Slug Coupling:** `mens_run_reports.rs` looked up `Qwen_Qwen3-{size}.json` while `size_train.rs` wrote `BaseLabel::report_file_name()`. New presets (`qwen35_2b`, `qwen35_4b`, `qwen35_9b`, `qwen38_27b`) and dynamic `BaseLabel` report paths remove this lock-in.
6. **Non-Qwen Chat Templates & Tokenizers:** When piloting non-Qwen families with non-ChatML templates (`Gemma 4` `<start_of_turn>`, `Devstral` Tekken `[TOOL_CALLS]`), LCP prompt masking must never assume `<|im_start|>assistant\n`. Keeping `ChatML` (`Qwen3.5/3.8`, `MiMo`, `FrogNano`, `Granite 4.2`) decouples syntax first; distinct templates require template-aware tokenizer formatting in future Stage E pilots.

---

## 7. Staged Gate-Protected Rollout Plan & Primary Sources

### 7.1 Execution Sequence (Do Not Skip Gates)
1. **Stage A — Numerical Parity & Quantizer Fixes (Completed in Stage A Code Pass, No HF Downloads):**
   - Resolve `A1`–`A8` parity bugs across `vox-hf-layout`, `vox-quantize`, `vox-plugin-mens-candle-core` (`attn_math` + `build_rms_norm`), `vox-plugin-mens-candle-metal`, and `vox-plugin-mens-candle-cuda` (`sameness_gate` clean via shared `core` primitives), backed by deterministic unit tests.
2. **Stage B — Memory & Throughput Fixes:**
   - Replace `40B` `auto_cache` heuristic with param+budget bounds so 27B/32B stays safely in NF4 (`<= 10B` auto-cache threshold + 35% working-set bound); implement 256-row chunked CE (`training_loop/forward.rs`); implement `doc_ids` cross-document reset in Metal `Qwen35LinearAttention::forward_packed_ctx` and re-enable `--sequence-packing`; enable cached incremental hybrid decode (`forward_cached`) on both Metal and CUDA.
3. **Stage C — Qwen3.0 Decoupling, XML Tool Parser, Embedder & Catalog Cleanup (Pre-Gate Safe):**
   - Gate `" /no_think"` via `model_uses_no_think_user_switch` + `ChatmlConfig::user_no_think_switch`; add XML `<function=NAME><parameter=K>V</parameter></function>` parsing to `vox-eval` and `vox-orchestrator-mcp`; add `qwen35_{2b,4b,9b}` & `qwen38_27b` presets, `S2`/`S9`/`S27` size classes (`seq_len >= 2048` for `S27`), and `BaseLabel` report lookup in `mens_run_reports.rs`; upgrade embedder pins (`Qwen/Qwen3-Embedding-0.6B@97b0c614be4d77ee51c0cef4e5f07c00f9eb65b3`), pin `gte-modernbert-base` SHA (`e7f32e3c00f91d699e8c43b53106206bcc72bb22`) and `sherpa-onnx` Whisper/Parakeet SHAs (`csukuangfj/sherpa-onnx-whisper-tiny.en@d026532c022fa99fd789d6b32446a1df7b6bfc43`, `ed5d745623472f97cfc2e6c2916ee248798fc5a8`), and normalize `model-catalog.bootstrap.v1.json` `cost_per_1k` units. **Keep `mens/config/gpu-specs.yaml`, `mens/config/hub-cards.yaml`, `svg.rs`, and `platformSsot.generated.*` untouched until Stage D.**
4. **Stage D — Cached Base Smoke, Training & Eval Gates:**
   - Validate `Qwen3.5-2B`, `4B`, `9B`, `MiMo-V2.6-Distill-Qwen-9B` (or `Granite-4.2-8B`), and `Qwen3.8-27B` (`--model Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0 --license-class apache-2.0`) sequentially with paired `vox-eval` collateral checks.
   - **Only after Stage D passes:** update `mens/config/gpu-specs.yaml` (resolving `Qwen3.8-27B` SHA, `license: apache-2.0`, and inverted 4-band `floor_mb`), `mens/config/hub-cards.yaml`, `svg.rs`, regenerate `platformSsot.generated.*` via `sync-platform-ssot.mjs`, and publish updated HF cards/Space.
5. **Stage E — `+32B` Qwen MoE Pilot (`Qwen/Qwen3.6-35B-A3B`) & Corpus Cleanup:**
   - Wire `tool_selection_synth.rs` + `argument_generation_synth.rs` into `mix_prepare.rs` and fix `agentic_synth.rs` tool keys; prototype Path A (`.vox` runner via `mlx-lm` / PEFT) for `Qwen3.6-35B-A3B` and compare against `Qwen/Qwen3.8-27B`.

### 7.2 Primary External Sources (`2026-11-13`)
- Qwen3.8 / Qwen3.6 / Qwen3.5 / Qwen3: https://huggingface.co/Qwen/Qwen3.8-27B, https://huggingface.co/Qwen/Qwen3.6-35B-A3B, https://huggingface.co/Qwen/Qwen3-Coder-Next, https://huggingface.co/Qwen/Qwen3.5-122B-A10B, https://huggingface.co/Qwen/Qwen3.5-9B, https://huggingface.co/Qwen/Qwen3.5-4B, https://huggingface.co/Qwen/Qwen3.5-2B, https://huggingface.co/Qwen/Qwen3-0.6B, https://huggingface.co/Qwen/Qwen3-32B
- Non-Qwen & Permissive Coding / Agent Alternatives: https://huggingface.co/XiaomiMiMo/MiMo-V2.6-Distill-Qwen-9B, https://huggingface.co/microsoft/FrogNano-4B-2609, https://huggingface.co/ibm-granite/granite-4.2-8b, https://huggingface.co/ibm-granite/granite-4.2-3b, https://huggingface.co/ibm-granite/granite-4.2-30b, https://huggingface.co/google/gemma-4-12B-it, https://huggingface.co/google/gemma-4-31B-it, https://huggingface.co/mistralai/Devstral-Small-2-24B-Instruct-2512, https://huggingface.co/allenai/SERA-8B, https://huggingface.co/allenai/SERA-32B, https://huggingface.co/deepseek-ai/DeepSeek-V4.1-Flash-1017, https://huggingface.co/zai-org/GLM-5.3-Flash
- Harness Control & Guardrails: https://huggingface.co/Qwen/Qwen3Guard-Gen-0.6B, https://huggingface.co/ibm-granite/granite-switch-4.1-8b-preview, https://huggingface.co/zed-industries/zeta, https://huggingface.co/microsoft/Mage-VL-Cowork, https://huggingface.co/ByteDance-Seed/UI-TARS-1.5-7B
- Embedders, Speech & WebGPU: https://huggingface.co/Qwen/Qwen3-Embedding-0.6B, https://huggingface.co/Alibaba-NLP/gte-modernbert-base, https://huggingface.co/google/embeddinggemma-2, https://huggingface.co/csukuangfj/sherpa-onnx-whisper-tiny.en, https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8, https://huggingface.co/Qwen/Qwen3-ASR-0.6B, https://huggingface.co/Qwen/Qwen3-ASR-1.7B, https://huggingface.co/onnx-community/Qwen3.5-0.8B-ONNX
- Upstream `transformers` reference: https://github.com/huggingface/transformers/blob/main/src/transformers/models/qwen3_5/modeling_qwen3_5.py
