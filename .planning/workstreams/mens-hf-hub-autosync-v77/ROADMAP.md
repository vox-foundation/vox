# Roadmap: Milestone v7.7 (`mens-hf-hub-autosync-v77`) — Hugging Face Hub Auto-Sync, Model Cards v3, Free Inference Providers & Playground Space

**Workstream:** `.planning/workstreams/mens-hf-hub-autosync-v77/`
**Scope:** End-to-end Hugging Face Hub publishing excellence for the Nov 2026 multi-family MENS ladder (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) plus legacy `Qwen3-*` pins: research SSOT, multi-family `hub_repo` / `hub_sync` / `hub_cards_push` / Collections / auto-sync fixes, native BF16 causal-LM recombination & zero-alloc PEFT export, Model Cards v3 / SVGs / lints / reference docs, `vox-mens-playground` HF Space + Router provider bridge, and post-27B live Hub verification. Phases 453–457 execute in parallel while Milestone `v7.3` Phase 428 (`Qwen3.8-27B`) trains on Metal; Phase 458 gates on Phase 428/429/430 outputs.

## Phase Checklist

- [x] **Phase 453: HF Model Cards & Free Inference Providers Research SSOT + Milestone v7.7 Init** (`HF-RES-01..02`)
- [x] **Phase 454: Multi-Family `hub_repo`, `hub_sync`, `hub_cards_push`, `.eval_results`, Collections & Auto-Sync Fixes** (`HF-SYNC-01..05`)
- [x] **Phase 455: Native BF16 Causal-LM Recombine, Zero-Alloc PEFT Export, Tokenizer/Chat Template & Inference Seams** (`HF-PKG-01..04`)
- [x] **Phase 456: Model Cards v3, Prompts, `image-generator` Raster Banners, Lints, Goldens & `mens-models.md`** (`HF-CARD-01..04`)
- [x] **Phase 457: HF Space `vox-mens-playground` & Free Inference Provider Bridge** (`HF-SPACE-01..04`)
- [x] **Phase 457.1: Plain-English Hero Banners, What/Why/How Copy, Beginner Onboarding & Python/Vox Bindings (INSERTED)** (`HF-ONBOARD-01..04`)
- [x] **Phase 457.2: 0.8B Canary Model Remediation, Idiomatic Vox Corpus/Prompt Upgrade & Production-Only Hub Cards (INSERTED)** (`HF-CANARY-01..04`)
- [ ] **Phase 458: Post-27B Release Rebuild, Live Hub Sync, Space/Collection Push & Milestone Audit** (`HF-LIVE-01..03`)

---

### Phase 453: HF Model Cards & Free Inference Providers Research SSOT + Milestone v7.7 Init

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/453-hf-excellence-research-and-ws-init/`
- **Goal:** Establish the authoritative research SSOT for Hugging Face Model Cards, `.eval_results/*.yaml`, Collections, Spaces, and free HF Inference Providers, documenting verified codebase gaps (`G1`–`G10`) while Milestone `v7.3` Phase 428 runs on Metal.
- **Depends on:** Milestone `v7.7` initialization (runs in parallel with Milestone `v7.3` Phase 428 Metal training)
- **Requirements:** `HF-RES-01`, `HF-RES-02`
- **Success Criteria:**
  1. `docs/src/architecture/hf-model-cards-and-inference-providers-research-2026.md` documents 2025–2026 flagship model cards (`Qwen3.5`, `DeepSeek-R1/V3`, `SmolLM3`, `StarCoder2`), YAML frontmatter contracts, `.eval_results/*.yaml`, Collections, Spaces, and multi-runtime quickstarts.
  2. Verified gap matrix `G1`–`G10` maps technical requirements across `router.huggingface.co/v1`, Featherless AI, `transformers`, `peft`, `vLLM`, and HF Space WebGPU / Gradio / Router APIs directly to exact crate targets.
  3. Research doc passes `vox-doc-pipeline --lint-only` with compliant Starlight YAML frontmatter and `kb:summary`.

### Phase 454: Multi-Family `hub_repo`, `hub_sync`, `hub_cards_push`, `.eval_results`, Collections & Auto-Sync Fixes

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/454-multi-family-hub-repo-and-autosync/`
- **Goal:** Eliminate hardcoded `Qwen3-*` assumptions across `hub_repo.rs`, `hub_sync.rs`, `hub_cards_push.rs`, `eval-gate`, `train_arm.rs`, and `vox mens pipeline`, enabling zero-reupload card/eval collateral updates, Clavis token propagation, path sanitization, and idempotent HF Collections REST sync.
- **Depends on:** Phase 453
- **Requirements:** `HF-SYNC-01`, `HF-SYNC-02`, `HF-SYNC-03`, `HF-SYNC-04`, `HF-SYNC-05`
- **Success Criteria:**
  1. `crates/vox-populi/src/mens/hub_repo.rs` resolves all Nov 2026 multi-family rungs (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) while preserving backwards-compatible legacy `Qwen3-*` (`1.7B`, `8B`, `14B`, `32B`) pins.
  2. `crates/vox-populi/src/mens/hub_sync.rs` drops hardcoded `Qwen3-{size}` / `mens/runs/qwen3-{}` paths, propagates Clavis tokens on all read/tag requests, strips host/HF cache paths via `sanitize_json_paths`, emits `.eval_results/vox_mens_eval.yaml`, and stages `--spoke` / `external_serving_handoff_v1.json`.
  3. `crates/vox-populi/src/mens/hub_cards_push.rs` (`is_card_collateral` + `plan_card_push`) syncs Model Cards v3, `assets/*.jpg` raster banners, and `.eval_results/*.yaml` in-place when remote `.safetensors` weights match local, including when `gate_current == true`, without redundant weight re-uploads.
  4. `upload_ladder_index` runs idempotent `ensure_mens_collection` linking `<ns>/vox-mens-index`, all ladder model repos, and `<ns>/vox-mens-playground` Space; `eval-gate` policy-hash auto-publish mismatch (`eval-gates.yaml` vs `eval-gates-ladder.yaml`), `train_arm.rs` gated-`main` protection, and `vox mens pipeline` card/index wiring pass unit tests.

### Phase 455: Native BF16 Causal-LM Recombine, Zero-Alloc PEFT Export, Tokenizer/Chat Template & Inference Seams

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/455-inference-provider-peft-and-merged-packaging/`
- **Goal:** Package merged BF16 causal-LM checkpoints and standard Hugging Face PEFT adapters cleanly for Featherless AI, HF Router, `transformers`, `vLLM`, Ollama, and WebGPU without dtype bloat or root-directory PEFT collisions.
- **Depends on:** Phase 453, Phase 454 (coordinated handoff paths)
- **Requirements:** `HF-PKG-01`, `HF-PKG-02`, `HF-PKG-03`, `HF-PKG-04`
- **Success Criteria:**
  1. `crates/vox-quantize` provides `SafeTensorsSource::load_native` and `recombine_hf_causal_lm` preserving native BF16 dtype (no 2x F32 bloat) and promoting `Qwen3_5ForConditionalGeneration` (`model.language_model.*`) to standalone `Qwen3_5ForCausalLM` (`model_type: qwen3_5_text`, `model.*`, stripping `vision_config` and `mtp_num_hidden_layers`).
  2. `crates/vox-populi/src/mens/tensor/peft_export.rs` performs zero-tensor-allocation Candle-to-PEFT export (`adapter_config.json` with `use_rslora`, `r`, `lora_alpha`, `target_modules`, `base_model_name_or_path` + `adapter_model.safetensors`) placed in root for adapter-only pushes and in `peft/` alongside root `model.safetensors` when `distribute_merged == true`, while always retaining root `candle_qlora_adapter.safetensors`.
  3. `finish_recombined` bundles `special_tokens_map.json`, `chat_template.jinja`, and Ollama `Modelfile` (`SYSTEM` Vox grammar prompt); `eval_local_prompt.rs` decouples Qwen3.5/3.8 from literal `" /no_think"`; `gradient_checkpoint_segments` auto-scales for `>=48`-layer checkpoints.
  4. `crates/vox-code-audit/src/detectors/llm_provider_call.rs` allows `/api/collections` alongside `/api/models`, and `vox-quantize`, `vox-populi`, and `vox-code-audit` tests pass.

### Phase 456: Model Cards v3, Prompts, `image-generator` Raster Banners, Lints, Goldens & `mens-models.md`

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/456-model-cards-v3-prompts-and-lints/`
- **Goal:** Upgrade Model Cards v3 templates, metadata rules, `image-generator` raster hero banners (`assets/overview.jpg` and per-rung `assets/banner.jpg`, zero SVGs), `hub-cards.yaml`, goldens, and `mens-models.md` for the Nov 2026 multi-family ladder, free Inference Providers, and `128 GB` Apple Silicon ceiling while passing 100% of card and doc lints.
- **Depends on:** Phase 453, Phase 454
- **Requirements:** `HF-CARD-01`, `HF-CARD-02`, `HF-CARD-03`, `HF-CARD-04`
- **Success Criteria:**
  1. `crates/vox-populi/templates/hub/` (`partials/frontmatter.md`, `model.md`, `index.md`, `partials/use_it.md`) and `crates/vox-populi/tests/fixtures/hub_cards/hf-metadata-rules.v1.yaml` emit dynamic `library_name` (`transformers`/`peft`/`vox`), `conversational`, `spaces:`, `datasets:`, family-accurate tags/titles, Playground Space badges/links, and multi-runtime quickstarts.
  2. `crates/vox-populi/src/mens/hub_card/svg.rs` serves binary `image-generator` raster JPEG assets (`assets/overview.jpg` and per-rung `assets/banner.jpg` from `crates/vox-populi/assets/hub/*.jpg`, zero SVG output).
  3. `mens/config/hub-cards.yaml`, `config.rs`, `tests/index_card.rs`, `lints_tests.rs`, and `tests/golden/hub_cards/` pass 100% of card, sentence, frontmatter, and Vox-fence lints.
  4. `docs/src/reference/mens-models.md` aligns with the Nov 2026 multi-family ladder, HF Inference Providers, PEFT export layout, and card anchor contracts, passing `vox-doc-pipeline --lint-only`.

### Phase 457: HF Space `vox-mens-playground` & Free Inference Provider Bridge

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/457-space-playground-and-free-provider-bridge/`
- **Goal:** Upgrade the Hugging Face Space playground (`README.md.tmpl`, `index.html.tmpl`, `app.py.tmpl`) for dynamic Nov 2026 `ladder_index.json` discovery, canonical Vox system prompts, mathematically exact rsLoRA WebGPU merging, and dual-target HF Router / Featherless AI / local inference fallback.
- **Depends on:** Phase 453, Phase 454, Phase 455, Phase 456
- **Requirements:** `HF-SPACE-01`, `HF-SPACE-02`, `HF-SPACE-03`, `HF-SPACE-04`
- **Success Criteria:**
  1. `crates/vox-populi/templates/hub/space/{README.md.tmpl,index.html.tmpl,app.py.tmpl}` supports Nov 2026 multi-family rungs via dynamic `ladder_index.json` discovery, with retired `@table` and `@component` syntax removed from `app.py.tmpl`.
  2. Space renders and injects canonical `generate_training_system_prompt()` and applies ` /no_think` strictly when `usesNoThinkSwitch` is true.
  3. Space WebGPU LoRA merger computes `use_rslora` scaling as `alpha / sqrt(rank)`.
  4. HF Router mode (`router.huggingface.co/v1/chat/completions`) queries both `${baseModelId}:auto` and `${repoId}:auto` with `:featherless-ai` and recorded/local fallbacks verified.

### Phase 457.1: Plain-English Hero Banners, What/Why/How Copy, Beginner Onboarding & Python/Vox Bindings (INSERTED)

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/457.1-plain-english-cards-and-bindings/`
- **Goal:** Eliminate insider jargon across all `image-generator` banners and programmatic Hub card / Space sync sources, explaining **What** Vox & Vox MENS do, **Why** each rung exists, and **How** beginning developers start with step-by-step Vox and Python bindings.
- **Depends on:** Phases 456, 457
- **Requirements:** `HF-ONBOARD-01`, `HF-ONBOARD-02`, `HF-ONBOARD-03`, `HF-ONBOARD-04`
- **Success Criteria:**
  1. All 7 `image-generator` raster banners (`crates/vox-populi/assets/hub/{overview,banner-0.6b,banner-2b,banner-4b,banner-9b,banner-27b,banner-generic}.jpg`) use clear, plain-English descriptions with zero jargon, zero garbled text, and zero SVGs.
  2. Programmatic sync sources (`mens/config/hub-cards.yaml`, `crates/vox-populi/templates/hub/{model.md,index.md,partials/*}`, `crates/vox-populi/templates/hub/space/{README.md.tmpl,index.html.tmpl}`, `docs/src/reference/mens-models.md`) clearly explain What/Why/How without hand-editing generated Hub cards.
  3. Model cards include `## Vox and Python` (`partials/onboarding.md`) covering beginner `.vox` quickstart (`vox check`, `vox run --interp`), `vox_actor_runtime::llm` / `HF_CHAT_MODEL` / `VOX_POPULI_LOCAL_OLLAMA_URL`, `openai` + `huggingface_hub` / `transformers` / `peft` Python bindings, `[python]` `Vox.toml` + `vox py sync`/`bind`, and `vox emit openapi` + `openapi-python-client` — with zero retired `@py.import`/`@table`/`@component` syntax.
  4. All `vox-quantize`, `vox-code-audit`, `vox-populi` (without `VOX_UPDATE_GOLDEN`), `vox-ml-cli --lib`, `vox-doc-pipeline --lint-only`, `vox-arch-check`, and `lints_hold_on_rendered_cards_dir` checks pass cleanly.

### Phase 457.2: 0.8B Canary Model Remediation, Idiomatic Vox Corpus/Prompt Upgrade & Production-Only Hub Cards (INSERTED)

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/457.2-08b-canary-and-clean-hub-cards/`
- **Goal:** Fix `Vox Mens 0.8B` model quality and eliminate all unfinished/broken/internal-audit artifacts from published Hugging Face Model Cards and the Ladder Index before advancing to larger rungs (`2B`–`27B`).
- **Depends on:** Phase 457.1
- **Requirements:** `HF-CANARY-01`, `HF-CANARY-02`, `HF-CANARY-03`, `HF-CANARY-04`
- **Success Criteria:**
  1. `hub_cards_push.rs` blocks pushing ungated (`!gate_current`) model cards to Hugging Face; `index.md` and `index_facts.rs` surface only gate-passed published models with zero `planned` spoke vaporware columns, zero `not evaluated` rows, and zero `## What's next` missing-file TODO dumps.
  2. `model.md`, `partials/{examples.md,use_it.md}`, and `examples.rs` remove `### Still fails` broken-code sections on gate-passed cards, remove `"no serving memory figure has been measured"` / checkpoint-filename disclaimers, and rank 3 verified, semicolon-free, non-stub passing examples for `widget:` and `## Examples`.
  3. `builtin_system_prompt()` (`crates/vox-corpus/src/training/mod.rs`) documents canonical Vox `Option[T]` list indexing (`xs[i].unwrap()`), `str` iteration/slicing, tuple lists, loop/math idioms, and non-stub mutations/tests (`< 2,200` chars); `target/dogfood_v4/train.jsonl` pairs semicolon-free Vox examples + compiler-verified collection/string/full-stack completions + systemless `<think>...</think>` traces (`0` held-out leakage).
  4. Retrained 2-epoch `Vox Mens 0.8B` improves over the `46.2%` (`24/52`) baseline and reasoning baselines, passes all `5/5` release gates, passes `vox-populi` & `lints_hold_on_rendered_cards_dir`, and syncs live (`v2`) to `brbrainerd/vox-mens-qwen3.5-0.8b`, `brbrainerd/vox-mens-index`, and `brbrainerd/vox-mens-playground` before any larger rung is trained or evaluated.

### Phase 458: Post-27B Release Rebuild, Live Hub Sync, Space/Collection Push & Milestone Audit

- **Directory:** `.planning/workstreams/mens-hf-hub-autosync-v77/phases/458-post-train-live-hub-sync-and-audit/`
- **Goal:** Following Milestone `v7.3` Phase 428 (`Qwen3.8-27B`) completion, rebuild `target/release/vox-ml-cli`, run Phase 429/430 paired evaluations and BF16 causal-LM + PEFT exports, push all live Hub repos/cards/evals/Space/Collection, verify live Inference Providers, and close Milestone `v7.7`.
- **Depends on:** Phases 454, 455, 456, 457, 457.1, and Milestone `v7.3` Phase 428 (`Qwen3.8-27B`) + Phase 429/430 evaluations
- **Requirements:** `HF-LIVE-01`, `HF-LIVE-02`, `HF-LIVE-03`
- **Success Criteria:**
  1. `target/release/vox-ml-cli` (`mens-candle-metal`) is rebuilt with all Phase 454–457 changes, and all gate-passing Nov 2026 rungs complete Phase 429/430 evaluations plus BF16 causal-LM `merge-qlora` and PEFT adapter packaging.
  2. Live `vox mens hub sync --push` publishes `<ns>/vox-mens-index`, every ladder model repo (`model.safetensors`, `peft/`, `candle_qlora_adapter.safetensors`, tokenizer/chat artifacts, `.eval_results/vox_mens_eval.yaml`, Model Cards v3, `assets/*.jpg` raster banners), `<ns>/vox-mens-playground` Space, and the HF Collection cleanly.
  3. Live HF repos, rendered Model Cards, `.eval_results`, Space execution, Collection items, and HF Router / Featherless AI / WebGPU endpoints pass end-to-end checks; `vox-arch-check`, `vox-code-audit`, `vox-doc-pipeline --lint-only`, and `v7.7-MILESTONE-AUDIT.md` pass with `0` errors.
