---
title: "Hugging Face Model Cards, Auto-Sync & Free Inference Providers Research (November 2026)"
description: "Architecture and ecosystem research analyzing Hugging Face top open-weight model cards, algorithmic metadata, .eval_results, Collections, free Inference Provider requirements (HF Router, Featherless AI, PEFT, Static Spaces), and the Vox MENS programmatic sync upgrade plan."
category: "Architecture SSOTs"
status: "research"
topic: "mens-training"
domain: "ml"
authorship: "llm-generated"
authorship_model: "google-deepmind-agent"
training_eligible: false
---

# Hugging Face Model Cards, Auto-Sync & Free Inference Providers Research (November 2026)

<!-- kb:summary -->
This research defines how Vox MENS can align Hugging Face model releases, programmatic sync, and free inference routing with current open-weight standards. It evaluates high-adoption card metadata, decentralized evaluation YAML results, Collections, and zero-cost serving via Hugging Face Inference Router, Featherless AI, PEFT adapters, and static Spaces. Finally, it maps eleven audited codebase gaps, including `library_name` filtering, Qwen3.5 and Qwen3.8 CausalLM recombination, PEFT export collisions, and card-sync gating, into a Milestone `v7.7` remediation plan.
<!-- /kb:summary -->

> [!IMPORTANT]
> **Execution & Gate Boundary:** Milestone `v7.7` (`mens-hf-hub-autosync-v77`, Phases 453–458) implements these findings concurrently on CPU while Milestone `v7.3` completes `Qwen/Qwen3.8-27B` QLoRA training on Metal. All live Hugging Face Hub pushes remain strictly gated behind Phase 429/430 evaluations enforced by `mens/config/eval-gates-ladder.yaml` and `crates/vox-populi/src/mens/gate_policy.rs`.

## 1. What Makes Hugging Face's Highest-Adoption Model Releases Succeed (2025–2026 Patterns)

Across flagship 2025–2026 open-weight releases — specifically `Qwen/Qwen3.5-*`, `deepseek-ai/DeepSeek-R1`, `deepseek-ai/DeepSeek-V3`, `HuggingFaceTB/SmolLM3-3B`, and `bigcode/starcoder2` — developer adoption correlates directly with three engineering properties: machine-readable Hub discoverability, zero-friction interactive verification above the fold, and honest, reproducible evaluation telemetry.

### 1.1 Above-the-Fold Hero, Visual Strip & TL;DR Matrix

High-adoption releases treat the first viewport of `README.md` as a technical datasheet rather than marketing prose:

1. **Identity & Status Strip:** Compact shields (`Apache-2.0`, base lineage, architecture family, evaluation gate pass/pending status) paired with a crisp visual strip (`assets/scorecard.svg` and `assets/overview.svg`). Vox already generates dark-theme `1200x320` scorecards and `1200x360` overview banners in `crates/vox-populi/src/mens/hub_card/svg.rs`.
2. **TL;DR Parameter & Deployment Table:** Placed immediately beneath the visual strip so engineers can answer sizing questions in five seconds:
   - Active/total parameters and base architecture (for example, `qwen3_5_text` hybrid Gated DeltaNet + full attention)
   - Native context length (`seq_len` trained vs base window)
   - License (`apache-2.0`) and lineage (`finetune` vs `adapter`)
   - Thinking-mode default (`enable_thinking=False` or `<think>\n\n</think>\n\n` prefill for direct codegen)
   - Recommended precision/quantization tiers (`BF16` serverless/API, `Q4_K_M` / `Q8_0` local GGUF, browser `Q4F16` WebGPU for sub-1B rungs)
3. **Above-the-Fold Interactive Playground Link:** A direct call-to-action linking to the namespace Space (`<ns>/vox-mens-playground`) above the fold, letting visitors test prompts before cloning multi-gigabyte checkpoints.

### 1.2 Algorithmic YAML Frontmatter & Discovery Mechanics

Hugging Face Hub search ranking, library filters, widget activation, and Inference Provider onboarding are driven entirely by YAML frontmatter at the top of `README.md`.

| Frontmatter Field | Ecosystem Standard (`2025–2026`) | Why It Matters | Current Vox MENS Behavior |
|---|---|---|---|
| `license` | `"apache-2.0"` | Required by permissive enterprise filters and Featherless AI zero-cost hosting | Emitted (`crates/vox-populi/templates/hub/partials/frontmatter.md`) |
| `library_name` | `transformers` (merged causal LM) or `peft` (adapter-only) | Powers Hub "Use this model" snippets, download attribution, and Inference Providers; custom names such as `vox` hide models from ecosystem filters and disable serverless providers | **Hardcoded `library_name: vox`** in `frontmatter.md` and `index.md` |
| `pipeline_tag` | `text-generation` | Enables chat/completion routing and provider selection | Emitted in `frontmatter.md` |
| `language` | `["en"]` (ISO-639-1 codes) | Required for language search facets | Emitted |
| `base_model`, `base_model_relation`, `base_model_revision` | Exact HF repo ID, `finetune` vs `adapter`, pinned SHA | Builds the Hub lineage tree ("Finetunes / Adapters of X") | Emitted (`f.is_complete_model()` toggle) |
| `datasets` | Array of HF dataset repo IDs | Links training corpora bi-directionally to dataset cards | **Dead code:** computed in `CardFacts.datasets` (`cite_datasets` in `crates/vox-populi/src/mens/hub_card/card_facts.rs`), never rendered |
| `spaces` | `["<ns>/vox-mens-playground"]` | Pins the interactive Space badge on the right sidebar of the model card | Omitted |
| `tags` | `conversational`, `text-generation-inference`, `vllm`, `qwen3_5`, `gated-delta-networks`, `code`, `qlora`, `rs-lora` | Triggers chat-template UI, vLLM/TGI compatibility hints, and architecture search facets | Only emits `vox`, `vox-mens`, `qwen3`, `qlora`, `candle`, `code` |
| `widget` | Sample `{text, example_title}` inputs | Populates one-click Hub inference box examples | Omitted (`widget_prompt` exists in `crates/vox-populi/src/mens/hub_card/examples.rs` but unused in frontmatter) |
| `model-index` | Structured tasks, datasets, metrics | Populates Hub leaderboard badges and Papers-style metric pills | Omitted in frontmatter |

Target YAML frontmatter structure for a **merged CausalLM release** (`distribute_merged == true`):

```yaml
---
license: apache-2.0
library_name: transformers
pipeline_tag: text-generation
language:
  - en
base_model: Qwen/Qwen3.5-9B
base_model_relation: finetune
base_model_revision: <pinned_base_commit_sha>
datasets:
  - <ns>/vox-corpus-organic
spaces:
  - <ns>/vox-mens-playground
tags:
  - conversational
  - text-generation-inference
  - vllm
  - qwen3_5
  - gated-delta-networks
  - code
  - qlora
  - rs-lora
  - vox
  - vox-mens
widget:
  - text: "Write an idiomatic Vox actor with state persistence and HTTP routing."
    example_title: "Vox Actor Codegen"
model-index:
  - name: vox-mens-qwen35-9b
    results:
      - task:
          type: text-generation
        dataset:
          type: <ns>/vox-eval-heldout
          name: Vox Held-Out Codegen Suite
        metrics:
          - type: pass@1
            value: 0.0
---
```

When publishing an **adapter-only release** (`distribute_merged == false`), swap `library_name: peft` and `base_model_relation: adapter`, and ensure `adapter_config.json` resides at the repository root.

### 1.3 Decentralized `.eval_results/*.yaml` Community Evals

Alongside `model-index` in `README.md`, Hugging Face Hub now ingests decentralized benchmark records committed directly under `.eval_results/<suite>.yaml` inside the model repository. Storing structured metrics as standalone YAML artifacts decouples automated benchmark updates from hand-edited Markdown tables and lets CI/evaluation runners (`vox mens eval-gate` / Phase 429–430 runners) update metrics without rewriting prose or re-uploading weights.

Illustrative `.eval_results/vox-heldout.yaml` schema:

```yaml
- dataset:
    id: <ns>/vox-eval-heldout
    task_id: vox-codegen
    revision: <eval_dataset_sha>
  value: 0.0
  verify:
    source: vox-eval
    gate_policy_blake3: <policy_blake3>
    adapter_sha256: <adapter_sha256>
    verified_on_revision: <model_commit_sha>
```

For Vox, these `.eval_results/*.yaml` files must be treated as **card-collateral files** (alongside `README.md` and `assets/*.svg`) so metadata-only syncs preserve or update them cleanly.

### 1.4 Hugging Face Collections (`POST /api/collections`)

Top multi-size families (`Qwen3.5`, `SmolLM3`, `StarCoder2`) unify their model ladder, evaluation datasets, and demo Space into a single **Hugging Face Collection** pinned at the top of the organization/user profile. Collections (`https://huggingface.co/collections/<ns>/<slug>`) eliminate fragmented links by grouping:

1. The ladder index repository (`<ns>/vox-mens`)
2. Every model rung (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`)
3. The interactive playground Space (`<ns>/vox-mens-playground`)
4. Corpus and held-out benchmark dataset repositories (`crates/vox-populi/src/mens/hub_dataset.rs`)

Programmatic sync calls `POST /api/collections` (to create or fetch the canonical namespace collection) and `POST /api/collections/{namespace}/{slug}/items` (idempotently upserting each `model`, `space`, and `dataset` item) at the conclusion of a multi-rung publish.

### 1.5 Multi-Runtime Copy-Paste Quickstarts

Successful model cards remove guesswork by providing verified snippets across all five runtimes an engineer may reach for:

1. **Hugging Face Inference Router (`router.huggingface.co/v1` / `InferenceClient`):**
   ```python
   from huggingface_hub import InferenceClient

   client = InferenceClient(api_key="hf_...")
   resp = client.chat.completions.create(
       model="<ns>/vox-mens-qwen35-9b:auto",
       messages=[
           {"role": "system", "content": "<canonical Vox system prompt>"},
           {"role": "user", "content": "Implement a Vox component with local reactive state."},
       ],
       temperature=0.2,
       max_tokens=1024,
   )
   print(resp.choices[0].message.content)
   ```
2. **Python `transformers` (with non-thinking `enable_thinking=False` guidance):**
   Qwen3.5 and Qwen3.8 default to extended thinking unless disabled in the chat template; unlike Qwen3.0, appending `" /no_think"` to the user prompt is treated as literal user text (`crates/vox-eval/src/qwen_reasoning.rs`). Quickstarts must pass `enable_thinking=False` to `apply_chat_template`:
   ```python
   from transformers import AutoModelForCausalLM, AutoTokenizer

   model_id = "<ns>/vox-mens-qwen35-9b"
   tok = AutoTokenizer.from_pretrained(model_id)
   model = AutoModelForCausalLM.from_pretrained(model_id, torch_dtype="bfloat16", device_map="auto")
   prompt = tok.apply_chat_template(
       messages, tokenize=False, add_generation_prompt=True, enable_thinking=False
   )
   ```
3. **vLLM Multi-LoRA (`vllm serve --enable-lora`):**
   ```bash
   vllm serve Qwen/Qwen3.5-9B \
     --enable-lora \
     --lora-modules vox-code=<ns>/vox-mens-qwen35-9b
   ```
   When serving an adapter-only repository (`library_name: peft`), vLLM reads `adapter_config.json` and `adapter_model.safetensors` from the root; when serving a merged repository (`library_name: transformers`), run `vllm serve <ns>/vox-mens-qwen35-9b` directly without `--enable-lora` (or pull the `peft/` subfolder for adapter-only hot-swapping).
4. **Ollama `Modelfile`:**
   ```dockerfile
   FROM ./vox-mens-qwen35-9b-Q4_K_M.gguf
   SYSTEM """<canonical output of generate_training_system_prompt()>"""
   PARAMETER temperature 0.2
   PARAMETER top_p 0.9
   ```
5. **Native Vox Runtime (`vox mens serve` & `vox mens hub pull`):**
   ```bash
   vox mens hub pull <ns>/vox-mens-qwen35-9b
   vox mens serve --model <ns>/vox-mens-qwen35-9b
   ```

### 1.6 Honest Evaluation Evidence, Paired Deltas & Memory Measurements

Where `DeepSeek-R1` and `SmolLM3-3B` differentiate themselves is methodological transparency: reporting exact base-vs-tuned deltas, contamination checks, and concrete failure cases. Vox's existing `crates/vox-populi/src/mens/hub_card/` subsystem already outperforms most open fine-tune repos on honesty:

- **Paired Outputs ("Fixed vs Still Fails"):** `crates/vox-populi/src/mens/hub_card/examples.rs` and `crates/vox-populi/templates/hub/partials/examples.md` classify deterministic held-out examples into `Verdict` buckets (`Fixed`, `Regressed`, `StillFails`, `StillPasses`) with base-vs-tuned completions and `vox check` syntax diagnostics.
- **Cryptographic Gate Linkage:** `crates/vox-populi/src/mens/gate_policy.rs` hashes `mens/config/eval-gates-ladder.yaml` with BLAKE3 (`policy_blake3`) and pairs `gate_receipt.json` with `adapter_card.json` (`adapter_sha256`).
- **Hardware Telemetry:** `TrainingManifest` (`crates/vox-populi/src/mens/tensor/manifest/mod.rs`) records executed optimizer steps, skip rates, architecture split (`candle_qlora_linear_layers`, `candle_qlora_full_layers`), and exact `launch_argv` for reproducibility.

Fixing metadata discovery (`library_name`, `.eval_results`, multi-runtime snippets) ensures Hugging Face developers actually discover and trust this telemetry.

---

## 2. Free Hugging Face Inference Providers & Zero-Cost Serving Matrix

Deploying Apache-2.0 fine-tuned Vox MENS checkpoints without dedicated GPU hosting requires choosing the right distribution format for each zero-cost tier.

| Tier / Provider | Infrastructure Cost | Required Artifact Layout | Cold-Start & Availability | Best-Fit Vox Rungs | Current Vox Readiness |
|---|---|---|---|---|---|
| **HF Inference Router + Featherless AI (`:featherless-ai` / `:auto`)** | Free tier / serverless quota | Public `apache-2.0` repo; merged `BF16`/`FP16` CausalLM `safetensors` at root; `library_name: transformers`; `conversational`; complete tokenizer + `chat_template.jinja`; **no** root `adapter_config.json` | Warm serverless pool when eligible & popular; falls back if cold | `2B`, `4B`, `9B`, `27B` | **Blocked** (`G1`, `G2`, `G3`, `G4`) |
| **Standard HF PEFT + `vllm serve --enable-lora`** | Self-hosted / user Colab / free Kaggle | `adapter_model.safetensors` (`base_model.model.*.lora_{A,B}.weight`) + `adapter_config.json` (`use_rslora`, `r`, `lora_alpha`, `target_modules`); `library_name: peft` (if adapter-only) or under `peft/` (if merged) | Instant hot-swap onto base vLLM worker | All rungs (`0.6B`–`27B`) | **Missing** (`G5` — no `peft_export.rs`) |
| **Static HF Space (`sdk: static`) + WebGPU + Router** | **Always Free** (0 CPU billing) | `README.md` (`sdk: static`), `index.html`, Q4F16 ONNX + browser LoRA merge (small rungs) + Router `${repoId}:auto` / `:featherless-ai` + pre-recorded held-out fallback | Instant static HTML load; browser GPU warmup for ONNX | All rungs (WebGPU `<=2B`; Router `2B`–`27B`) | **Broken** (`G8` — `alpha/rank`, Qwen3-only maps, base-only Router) |
| **Local Zero-Cost Runtimes (`vox mens serve`, Ollama, `llama.cpp`)** | Free (operator hardware) | Root `candle_qlora_adapter.safetensors` + `adapter_manifest.json` for `vox`; or merged `BF16` -> GGUF `Q4_K_M` for Ollama/`llama.cpp` | Offline, deterministic, zero rate limits | All rungs (`0.6B`–`27B`) | **Partial** (`G4` drops Candle adapter when merged; prompt switch drift) |

### 2.1 HF Inference Router (`router.huggingface.co/v1/chat/completions`) & Featherless AI

Hugging Face consolidated serverless and third-party inference routing under `https://router.huggingface.co/v1/chat/completions` (`model: "<repo>:<provider>"` or `":auto"`). Among third-party providers integrated with the Router, **Featherless AI** (`:featherless-ai`) dynamically loads public, permissive-license open-weight models directly from Hub storage at zero upfront hosting fee for the model author.

To pass automated Hub + Featherless AI ingestion, a Vox repository MUST satisfy **all six conditions**:

1. **Public permissive license:** `license: apache-2.0` in YAML frontmatter.
2. **Complete merged weights in `BF16` or `FP16`:** `model.safetensors` (or sharded `model-00001-of-NNNNN.safetensors` plus `model.safetensors.index.json`). **Unmerged LoRA adapters are rejected by Featherless AI**, and `F32` recombined checkpoints bloat storage/VRAM 2x and frequently exceed serverless shard limits.
3. **Standard HF library tag:** `library_name: transformers` and `pipeline_tag: text-generation`.
4. **Chat capability tags & templates:** `conversational` tag plus `tokenizer.json`, `tokenizer_config.json`, `generation_config.json`, `special_tokens_map.json`, and `chat_template.jinja`.
5. **Standalone CausalLM `config.json`:** Must specify a CausalLM architecture (`Qwen3_5ForCausalLM` / `Qwen3ForCausalLM`) directly resolvable by `AutoModelForCausalLM`.
6. **Clean repository root:** **No `adapter_config.json` in the repo root** when merged weights are published.

#### Critical Qwen3.5 / Qwen3.8 Multimodal Tower Pitfall

Official `Qwen/Qwen3.5-{2B,4B,9B}` and `Qwen/Qwen3.8-27B` base checkpoints ship on Hugging Face as multimodal vision-language packages (`architectures: ["Qwen3_5ForConditionalGeneration"]`) whose `config.json` wraps language hyperparameters inside a nested `text_config` alongside `vision_config` and `mtp_num_hidden_layers`, while storing language tensors under `model.language_model.*` prefixes.

During Vox QLoRA training and quantization, `crates/vox-hf-layout/src/lib.rs` (`is_vision_or_mtp_key`) and `crates/vox-quantize/src/read.rs` intentionally skip `model.visual.*` and `mtp.*` tensors because Vox fine-tunes text-only code generation. However, the existing `recombine_with_shard_budget` implementation in `crates/vox-quantize/src/recombine.rs` introduces four breaking defects for HF Inference Providers:

1. **2x `F32` Bloat:** It calls `base.load_f32(name)` and upcasts merged overrides to `DType::F32` instead of keeping native `BF16` (`SafeTensorsSource::tensor_dtypes` already inspects safetensors headers in `crates/vox-quantize/src/read.rs` without loading full payloads).
2. **Stale `model.language_model.*` Tensor Prefix:** Base keys stay prefixed `model.language_model.*` while merged text-only CausalLM tooling expects `model.*`.
3. **Unflattened VLM `config.json`:** Copying the base `config.json` leaves `Qwen3_5ForConditionalGeneration`, `vision_config`, `mtp_num_hidden_layers`, and nested `text_config` in place, causing `AutoModelForCausalLM` and serverless loaders to fail when visual weights are absent.
4. **Missing Tokenizer & Chat Artifacts:** Only `config.json` is written; `tokenizer_config.json`, `generation_config.json`, `special_tokens_map.json`, and `chat_template.jinja` are not copied into the recombined directory.

**Required `v7.7` Fix (`recombine_hf_causal_lm` in `crates/vox-quantize/src/recombine.rs`):**
- Preserve native `BF16` dtype across untouched base tensors and merged delta tensors (`recombine_with_shard_budget` remains intact if legacy F32 tests depend on it, while `recombine_hf_causal_lm` targets HF distribution).
- Remap `model.language_model.*` tensor keys to `model.*` (dropping any residual visual/MTP metadata).
- Flatten `config.json`: promote `text_config` fields to top-level, set `"architectures": ["Qwen3_5ForCausalLM"]` and `"model_type": "qwen3_5_text"`, and strip `vision_config` and `mtp_*` keys.
- Preserve existing `tie_word_embeddings: false` flipping logic whenever `lm_head.weight` was synthesized/un-tied during merge.
- Copy `tokenizer.json`, `tokenizer_config.json`, `generation_config.json`, `special_tokens_map.json`, and `chat_template.jinja` from the cached base snapshot into the recombined output directory.

#### Root `adapter_config.json` Collision Rule

If a repository contains both `model.safetensors` and a root `adapter_config.json`, Hugging Face Hub loaders (`transformers`, TGI, vLLM, Featherless AI) treat the repo ambiguously or attempt adapter-only loading, breaking serverless inference. Conversely, Vox's own `vox mens hub pull` (`verify_pulled_adapter_dir` in `crates/vox-populi/src/mens/hub_sync.rs`) and browser WebGPU LoRA merging require `candle_qlora_adapter.safetensors` and `adapter_manifest.json` at the repo root.

Milestone `v7.7` enforces a strict **dual-layout invariant**:

| Publish Mode (`distribute_merged`) | Frontmatter `library_name` | Repo Root Files | Subdirectory `peft/` |
|---|---|---|---|
| `true` (gate passed + merged shards present) | `transformers` | `model.safetensors` (or shards + index), CausalLM `config.json`, all 5 tokenizer/chat files, `candle_qlora_adapter.safetensors`, `adapter_manifest.json`, `README.md`, `.eval_results/*.yaml`, `assets/*.svg` | `peft/adapter_config.json`, `peft/adapter_model.safetensors` |
| `false` (adapter-only / pre-merge) | `peft` | `adapter_config.json`, `adapter_model.safetensors`, `candle_qlora_adapter.safetensors`, `adapter_manifest.json`, `README.md`, `.eval_results/*.yaml`, `assets/*.svg` | *(omitted)* |

> [!NOTE]
> **Egress Boundary Check:** In `crates/vox-code-audit/src/detectors/llm_provider_call.rs`, `LlmProviderCallDetector::HOSTNAMES` currently monitors `"huggingface.co/api"` (among external LLM hosts) to protect the `vox_actor_runtime::llm` facade boundary. Browser-side Static Space calls to `router.huggingface.co` execute client-side using the user's HF token; any future Rust workspace proxy hitting `router.huggingface.co` must route through the `vox_actor_runtime::llm` facade and register the hostname in `LlmProviderCallDetector::HOSTNAMES`.

### 2.2 Standard HF PEFT + `vllm serve --enable-lora`

Vox trains LoRA adapters natively via Candle (`candle_qlora_adapter.safetensors`), storing projections with `.lora_a.weight` / `.lora_b.weight` keys rather than PEFT's `base_model.model.<module>.lora_A.weight` / `lora_B.weight` format. Historically, no standard HF `adapter_model.safetensors` or `adapter_config.json` existed, making Vox adapters unusable in Python `peft` or `vllm serve --enable-lora`.

Milestone `v7.7` introduces **`crates/vox-populi/src/mens/peft_export.rs`**:
1. **Zero-Copy Safetensors Header Rewrite:** Reads only the JSON header length and header map of `candle_qlora_adapter.safetensors`, rewrites tensor names from Candle layout (`<logical>.lora_a.weight` -> `base_model.model.<norm_module>.lora_A.weight` and `.lora_b.weight` -> `.lora_B.weight`, normalizing any `model.language_model.*` prefix to `model.*`), updates byte offsets for the new JSON header length, and streams the untouched binary tensor buffer directly to `adapter_model.safetensors` with zero float deserialization or memory spike.
2. **PEFT `adapter_config.json` Emission:** Synthesizes standard fields from `TrainingManifest` / `LoraTrainingConfig` (`crates/vox-populi/src/mens/tensor/training_config.rs`):
   - `peft_type: "LORA"`
   - `task_type: "CAUSAL_LM"`
   - `base_model_name_or_path`: exact upstream HF ID (e.g., `Qwen/Qwen3.5-9B`)
   - `r`: adapter rank, `lora_alpha`: alpha
   - `use_rslora`: boolean matching `LoraTrainingConfig::use_rslora` (`alpha / sqrt(rank)` scaling)
   - `target_modules`: deduced cleanly from rewritten tensor keys (never hardcoded to `q_proj`/`v_proj` alone, capturing Qwen3.5 linear-attention + MLP projections)
   - `bias: "none"`

This directly unlocks `vllm serve --enable-lora`, integrating cleanly with `VllmLoraClient` (`crates/vox-populi/src/mens/serving/vllm_lora.rs`) once HTTP adapter load/unload dispatch is wired.

### 2.3 Always-Free Interactive Tier — Static Space (`<ns>/vox-mens-playground`) + WebGPU + Router Bridge

Hugging Face Static Spaces (`sdk: static` in `crates/vox-populi/templates/hub/space/README.md.tmpl`) run purely in the visitor's browser with zero CPU charges and never sleep. Currently, `crates/vox-populi/templates/hub/space/index.html.tmpl` ships three defects that prevent interactive testing of `v7.3` and `v7.7` models:

1. **Broken rsLoRA Scaling:** Line 1047 computes `const scaleFactor = alpha / rank;`, corrupting inference outputs for any model trained with `use_rslora: true` (`alpha / Math.sqrt(rank)`).
2. **Router Skips Fine-Tuned Model:** Line 1257 sets ``model: mode === "local-serve" ? repoId : `${baseModelId}:auto` ``, querying only the un-tuned base model on HF Inference Router rather than contrasting `${baseModelId}:auto` against `${repoId}:auto` and `${repoId}:featherless-ai`.
3. **Qwen3-Only Maps & Drift Prompt:** `ONNX_REPO_BY_SIZE` (`0.6B`..`8B`) and `HF_BASE_MODEL_BY_SIZE` (`0.6B`..`32B`) omit `Qwen3.5-{2B,4B,9B}` and `Qwen3.8-27B`, while line 355 embeds a hardcoded 1-sentence system prompt instead of the canonical Vox training system prompt.

The upgraded Static Space enforces a **three-stage fallback cascade**:
1. **Live Serverless / Browser Inference:** Query HF Inference Router (`${repoId}:auto` with `:featherless-ai` hint + optional side-by-side `${baseModelId}:auto` comparison) for `2B–27B`, or WebGPU Q4F16 ONNX with rsLoRA-aware `scaleFactor = useRslora ? alpha / Math.sqrt(rank) : alpha / rank` for sub-1B/browser-sized checkpoints.
2. **Pre-Recorded Held-Out Checkpoints:** When HF Router rate-limits an anonymous visitor or Featherless AI has cold capacity on a niche rung, display verified base-vs-tuned outputs from the repo's `.eval_results` / held-out artifact JSON.
3. **One-Command Local Escape Hatch:** Offer pre-filled `vox mens hub pull <repo>` and `vox mens serve` commands with the canonical system prompt pre-configured.

### 2.4 Local Zero-Cost Runtimes (`vox mens serve`, Ollama `Modelfile`, `llama.cpp` GGUF)

Prompt protocol drift silently degrades codegen accuracy across runtimes. Two invariants keep local inference aligned with training:

1. **Single Canonical System Prompt (`generate_training_system_prompt()`):** Export the generated string from `vox-corpus` into every runtime template — Ollama `Modelfile` (`SYSTEM """..."""`), `chat_template.jinja` fallback system text, `crates/vox-populi/templates/hub/space/index.html.tmpl`, and `vox mens serve`.
2. **Thinking-Switch Alignment (`model_uses_no_think_user_switch`):** `crates/vox-eval/src/qwen_reasoning.rs` defines `model_uses_no_think_user_switch(model_or_type_hint)` with 17 non-switch markers (`qwen3.5`, `qwen3_5`, `qwen3.8`, `qwen3_8`, `smollm`, `granite`, `gemma`, etc.), and `ChatmlConfig::user_no_think_switch` (`crates/vox-populi/src/mens/tensor/training_config.rs`) records whether `" /no_think"` should be appended to direct user turns. All serving prompts, `chat_template.jinja` defaults, and Space JS request builders MUST mirror `model_uses_no_think_user_switch`: **never inject literal `" /no_think"` into Qwen3.5 or Qwen3.8 user turns**, using empty-think prefill (`<think>\n\n</think>\n\n` / `enable_thinking=False`) instead.

---

## 3. Audited Codebase Gap Matrix (`G1–G11`) & Architectural Remediation Blueprint (Milestone `v7.7`)

An end-to-end audit of `vox-populi`, `vox-quantize`, `vox-hf-layout`, `vox-eval`, `vox-ml-cli`, `vox-code-audit`, and `mens/config/` identified **eleven concrete gaps (`G1–G11`)** blocking full Hugging Face adoption and zero-cost Inference Provider activation.

### 3.1 Gap Matrix (`G1–G11`)

| Gap ID | Audited File(s) & Symbol(s) | Current Behavior (Verified) | Ecosystem / Operational Impact | Milestone `v7.7` Remediation |
|---|---|---|---|---|
| **G1** | `crates/vox-populi/templates/hub/partials/frontmatter.md`, `index.md` | Hardcodes `library_name: vox` | Hides models from HF `transformers`/`peft` ecosystem filters and disables Inference Providers | Emit `transformers` when `f.is_complete_model()` (merged), `peft` when adapter-only; fix `index.md` metadata |
| **G2** | `frontmatter.md`, `crates/vox-populi/src/mens/hub_card/card_facts.rs`, `examples.rs` | Omits `datasets`, `spaces`, `conversational`, `text-generation-inference`, `vllm`, `qwen3_5`, `gated-delta-networks`, `rs-lora`, `model-index`, `widget`; `CardFacts.{datasets,train_lanes,spokes}` marked `#[allow(dead_code)]` | No dataset/Space cross-links, missing chat-UI activation, unused widget/eval frontmatter despite computed facts | Wire `CardFacts` dead fields into Askama `frontmatter.md`; populate `datasets`, `spaces`, dynamic arch/LoRA `tags`, `widget`, and `model-index` |
| **G3** | `crates/vox-quantize/src/recombine.rs` (`recombine_with_shard_budget`), `crates/vox-ml-cli/src/commands/mens/` | Forces `DType::F32` (2x bloat); retains `model.language_model.*` tensor names and VLM `Qwen3_5ForConditionalGeneration` / `text_config` / `vision_config` / `mtp_num_hidden_layers` in `config.json`; omits tokenizer/chat files | Fails Featherless AI / `AutoModelForCausalLM` loading for `Qwen3.5-*` & `Qwen3.8-27B`; 2x storage/transfer overhead | Add `recombine_hf_causal_lm` using `SafeTensorsSource::tensor_dtypes` (`read.rs`) to preserve `BF16`, strip `model.language_model.`, flatten `text_config` -> `Qwen3_5ForCausalLM` (`qwen3_5_text`), strip `vision_config`/`mtp_*`, copy all tokenizer/chat files, wire `merge-qlora` CLI |
| **G4** | `crates/vox-populi/src/mens/hub_sync.rs` (`stage_publish_bundle`, `verify_pulled_adapter_dir`) | Uses `if distribute_merged { ... } else if adapter_src`, dropping `candle_qlora_adapter.safetensors` when merged; no `peft/` vs root collision separation | Merged pushes break `vox mens hub pull` and WebGPU; placing root `adapter_config.json` with merged weights breaks HF providers | Keep `candle_qlora_adapter.safetensors` + `adapter_manifest.json` in root; put PEFT adapter in `peft/` when `distribute_merged == true` and at root when `false` |
| **G5** | `crates/vox-populi/src/mens/peft_export.rs` *(planned; does not exist yet)* | No Candle-to-PEFT converter or `adapter_config.json` emitter exists | Blocks Python `peft` and `vllm serve --enable-lora` (`VllmLoraClient` in `serving/vllm_lora.rs`) | Create `peft_export.rs` with zero-copy Safetensors header rewrite + `adapter_config.json` (`use_rslora`, `r`, `lora_alpha`, `target_modules`, `base_model_name_or_path`) |
| **G6** | `hub_sync.rs` (`OPTIONAL_BUNDLE_FILES`), `crates/vox-populi/src/mens/hub_card/facts.rs` (`CONFIG_AND_TOKENIZER`) | Omits `tokenizer_config.json`, `generation_config.json`, `special_tokens_map.json`, `chat_template.jinja`, `.eval_results/*.yaml` | Inference Router & `transformers` fail chat formatting and default sampling params without these files | Expand `OPTIONAL_BUNDLE_FILES` and `CONFIG_AND_TOKENIZER` to stage full tokenizer/generation/chat configs and `.eval_results/*.yaml` |
| **G7** | `crates/vox-populi/src/mens/hub_repo.rs` (`SIZES`, `size_of_pin`, `repo_id_for_size`), `hub_card/config.rs`, `mens/config/hub-cards.yaml` | Locked to 6 `Qwen3-*` rungs (`0.6B`, `1.7B`, `4B`, `8B`, `14B`, `32B`); `size_of_pin` only strips `Qwen3-`; `config.rs` rejects unknown sizes | Cannot publish or render cards for `Qwen3.5-{2B,4B,9B}` or `Qwen3.8-27B` defined in `mens/config/gpu-specs.yaml` (`qwen3_5_code`) | Generalize `hub_repo.rs` + `hub_card/config.rs` + `hub-cards.yaml` to multi-family ladder (`Qwen3-0.6B`, `Qwen3.5-{2B,4B,9B}`, `Qwen3.8-27B`) with family-aware repo naming |
| **G8** | `crates/vox-populi/templates/hub/space/index.html.tmpl` | `alpha / rank` ignores rsLoRA (`alpha / sqrt(rank)`); HF Router calls `${baseModelId}:auto` only; Qwen3-only size maps; non-canonical 1-line system prompt | Corrupts rsLoRA browser inference; users cannot test fine-tuned model in Space; `v7.3` `27B` / `Qwen3.5` rungs missing | Add `useRslora` branch (`alpha / Math.sqrt(rank)`), query `${repoId}:auto` / `:featherless-ai` + `${baseModelId}:auto`, multi-family map, `generate_training_system_prompt()`, held-out fallback |
| **G9** | `crates/vox-populi/templates/hub/partials/use_it.md`, `hub_sync.rs` | Missing `.eval_results/*.yaml` generator; quickstarts lack `InferenceClient`/Router, `transformers` `enable_thinking=False`, `vllm`, Ollama `Modelfile` | Visitors lack working copy-paste snippets and automated `.eval_results` Hub ingestion | Emit `.eval_results/*.yaml` from eval/gate artifacts; expand `use_it.md` with all 5 verified runtimes + `model_uses_no_think_user_switch` alignment |
| **G10** | `crates/vox-populi/src/mens/hub.rs`, `crates/vox-code-audit/src/detectors/llm_provider_call.rs` | `hub.rs` has `upload_model_folder` & `upload_space_folder` but no HF Collections REST client; `LlmProviderCallDetector` tracks `huggingface.co/api` | Model ladder + Space + datasets remain ungrouped on HF profile; any future Rust Router call needs egress governance | Add `POST /api/collections` & `/items` helpers in `hub.rs`; keep any Rust-side HF Router egress strictly behind `vox_actor_runtime::llm` / audit policy |
| **G11** | `crates/vox-populi/src/mens/hub_cards_push.rs` (`plan_card_push`, `is_card_collateral`), `hub_cards_push_tests.rs` | `plan_card_push` returns `CardPushPlan::GatedWeights` whenever `gate_current == true` **even when `remote_weights_match(remote, local)` is true**; `is_card_collateral` misses `.eval_results/*.yaml` | Blocks in-place `--cards-only --push` updates to cards, SVGs, and `.eval_results` when weights on Hub are already current or unchanged | Update `plan_card_push` to return `CardOnly` when `remote_weights_match(remote, local)` holds (and whitelist `.eval_results/*.yaml` in `is_card_collateral`) |

### 3.2 Milestone `v7.7` Remediation Blueprint (`mens-hf-hub-autosync-v77`, Phases 453–458)

The eleven gaps decompose into six CPU-executable phases that require no Metal/CUDA GPU and can execute immediately in parallel with `v7.3` `Qwen/Qwen3.8-27B` training:

1. **Phase 453 — Multi-Family Ladder & Safe Card-Only Push (`G7`, `G11`):**
   - Generalize `crates/vox-populi/src/mens/hub_repo.rs`, `crates/vox-populi/src/mens/hub_card/config.rs`, and `mens/config/hub-cards.yaml` to support `Qwen3-0.6B`, `Qwen3.5-{2B,4B,9B}`, and `Qwen3.8-27B` alongside legacy rungs.
   - Fix `plan_card_push` and `is_card_collateral` in `crates/vox-populi/src/mens/hub_cards_push.rs` so matching remote weights allow `CardOnly` pushes for `README.md`, `assets/*.svg`, and `.eval_results/*.yaml`.
2. **Phase 454 — CausalLM `BF16` Recombine & Tokenizer Bundle (`G3`, `G6`):**
   - Implement `recombine_hf_causal_lm` in `crates/vox-quantize/src/recombine.rs` (native `BF16` via `read.rs::tensor_dtypes`, `model.language_model.*` -> `model.*`, top-level `Qwen3_5ForCausalLM` / `qwen3_5_text`, strip `vision_config` / `mtp_*`, copy all 5 tokenizer/chat files, preserve `tie_word_embeddings: false` when `lm_head.weight` is synthesized) and wire it into `crates/vox-ml-cli/src/commands/mens/`.
3. **Phase 455 — Zero-Copy PEFT Export & Dual-Layout Bundle Staging (`G4`, `G5`, `G6`):**
   - Implement `crates/vox-populi/src/mens/peft_export.rs` (header-only safetensors rewrite + `adapter_config.json` with `use_rslora`, `r`, `lora_alpha`, `target_modules`, `base_model_name_or_path`).
   - Update `stage_publish_bundle` and `OPTIONAL_BUNDLE_FILES` in `crates/vox-populi/src/mens/hub_sync.rs` to keep root `candle_qlora_adapter.safetensors` + `adapter_manifest.json` intact while placing PEFT artifacts in `peft/` when merged and at root when adapter-only.
4. **Phase 456 — Algorithmic Frontmatter, `.eval_results` & Multi-Runtime Quickstarts (`G1`, `G2`, `G9`):**
   - Replace `library_name: vox` with conditional `transformers` / `peft` in `crates/vox-populi/templates/hub/partials/frontmatter.md` and `index.md`.
   - Wire `CardFacts.{datasets,train_lanes,spokes}` and `ExampleFacts::widget_prompt` into frontmatter, emit `.eval_results/*.yaml` beside `model-index`, inject `generate_training_system_prompt()` and `model_uses_no_think_user_switch` alignment into `partials/use_it.md` (`InferenceClient`, `transformers` `enable_thinking=False`, `vllm`, Ollama `Modelfile`, `vox mens serve`).
5. **Phase 457 — Static Space WebGPU rsLoRA + Router Upgrade (`G8`):**
   - Update `crates/vox-populi/templates/hub/space/index.html.tmpl` to support `useRslora` (`alpha / Math.sqrt(rank)`), Router queries against both `${repoId}:auto`/`:featherless-ai` and `${baseModelId}:auto`, multi-family model metadata, canonical `generate_training_system_prompt()`, and pre-recorded held-out fallback rendering.
6. **Phase 458 — HF Collections Client, Egress Audit & Post-429/430 Publish Readiness (`G10`):**
   - Add `POST /api/collections` and `/items` synchronization in `crates/vox-populi/src/mens/hub.rs`.
   - Verify `vox-code-audit` (`crates/vox-code-audit/src/detectors/llm_provider_call.rs`) and `vox mens hub push --dry-run` against staged `v7.7` bundles, holding live Hub mutation until Phase 429/430 evaluation gates pass.
