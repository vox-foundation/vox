---
phase: "456"
slug: "456-model-cards-v3-prompts-and-lints"
title: "Model Cards v3, Prompts, `image-generator` Raster Banners, Lints, Goldens & `mens-models.md`"
status: "passed"
verified_at: "2026-11-15T06:52:00Z"
requirements:
  - "HF-CARD-01"
  - "HF-CARD-02"
  - "HF-CARD-03"
  - "HF-CARD-04"
---

# Phase 456 Verification

## Outcome: PASSED

Phase 456 deliverables (`HF-CARD-01` through `HF-CARD-04`) are complete and verified against Milestone `v7.7` criteria.

## Artifacts & Requirements Coverage

| Requirement | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `HF-CARD-01` | `crates/vox-populi/templates/hub/partials/frontmatter.md:1-99`; `crates/vox-populi/templates/hub/model.md:1-62`; `crates/vox-populi/templates/hub/index.md:1-136`; `crates/vox-populi/templates/hub/partials/use_it.md:1-139`; `crates/vox-populi/templates/hub/partials/onboarding.md:1-98`; `crates/vox-populi/tests/fixtures/hub_cards/hf-metadata-rules.v1.yaml:1-43`; `crates/vox-populi/src/mens/hub_card/card_facts.rs:372-482` | Verified | `frontmatter.md:8-38` emits dynamic `library_name` (`transformers` when `inference_provider_ready`, `peft` when `has_peft_adapter`, else `vox`), `datasets:` (`frontmatter.md:12-17`), `spaces: [{{ f.playground_space_id }}]` (`frontmatter.md:18-19`), family-accurate tags (`f.family_tag()`, `qwen3_5`, `gated-delta-networks`, `rslora`, `conversational`, `text-generation-inference`, `vllm`), and `base_model_relation: finetune` vs `adapter`. `model.md:6-21`, `use_it.md:16-104`, and `onboarding.md:1-98` link the Playground Space, `assets/banner.jpg`, multi-runtime quickstarts (`router.huggingface.co/v1`, `:auto` / `:featherless-ai`, `transformers`, `vLLM`, `Ollama`, `peft`), and Vox/Python onboarding. |
| `HF-CARD-02` | `crates/vox-populi/src/mens/hub_card/svg.rs:1-191`; `crates/vox-populi/assets/hub/{overview,banner-0.6b,banner-2b,banner-4b,banner-9b,banner-27b,banner-generic}.jpg` | Verified | `svg.rs:11-86` embeds all 7 binary 16:9 JPEG hero banners (`overview.jpg` 149K, `banner-0.6b.jpg` 195K, `banner-2b.jpg` 147K, `banner-4b.jpg` 147K, `banner-9b.jpg` 156K, `banner-27b.jpg` 142K, `banner-generic.jpg` 99K) and emits `assets/banner.jpg` and `assets/overview.jpg` with zero SVG output (`svg.rs:94-190`). |
| `HF-CARD-03` | `mens/config/hub-cards.yaml:1-97`; `crates/vox-populi/src/mens/hub_card/config.rs:1-220`; `crates/vox-populi/src/mens/hub_card/lints.rs:1-300`; `crates/vox-populi/src/mens/hub_card/lints_tests.rs:70-448`; `crates/vox-populi/tests/golden/hub_cards/{failed,index,missing_output,needs_re_gate,no_examples,not_evaluated,not_trained,passed,stale_serving}.md` | Verified | `mens/config/hub-cards.yaml:40-97` defines plain-English `why` blurbs and `regate_eta` projections for all 5 Nov 2026 ladder rungs (`0.6B`, `2B`, `4B`, `9B`, `27B`); all 9 golden markdown files under `crates/vox-populi/tests/golden/hub_cards/` and `lints_tests.rs` pass 100% of duplicate-sentence, doc-anchor, HF frontmatter, and Vox-fence compile lints with zero retired jargon or syntax. |
| `HF-CARD-04` | `docs/src/reference/mens-models.md:1-255` | Verified | `mens-models.md` documents the Nov 2026 multi-family ladder (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`, up to 128 GB Apple Silicon workstations, `mens-models.md:20-26`), `qwen3_non_thinking_codegen_v1` `/no_think` vs `enable_thinking=False` (`mens-models.md:158`), zero-alloc PEFT export under `peft/` and HF Inference Providers (`mens-models.md:170-175`), `assets/banner.jpg` / `assets/overview.jpg` card generation (`mens-models.md:178`), and `## Vox and Python bindings` (`mens-models.md:202-246`), matching all 16 required `doc_slugs(DOCS_MD)` anchors (`lints_tests.rs:129-177`). |

## Automated Checks Run

- Direct non-Cargo hygiene & lint checks run in this session:
  - `rg -ni "\.svg|<svg" crates/vox-populi/templates/ crates/vox-populi/tests/golden/hub_cards/` -> exit `1` (`0` matches).
  - `rg -ni "compiler-gated|qlora spoke|adapter ladder" mens/config/hub-cards.yaml crates/vox-populi/templates/hub/ crates/vox-populi/tests/golden/hub_cards/` -> exit `1` (`0` matches).
  - `rg -n "@py\.import|@table|@component" crates/vox-populi/templates/hub/ docs/src/reference/mens-models.md` -> exit `1` (`0` matches).
  - `./target/debug/vox-doc-pipeline --lint-only --paths architecture/hf-model-cards-and-inference-providers-research-2026.md reference/mens-models.md` -> exit `0` (`vox-doc-pipeline lint complete — no hard errors.`).
- Pre-turn verified Cargo baseline in `target/v77-verify`:
  - `cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib` -> exit `0` (`608 passed; 0 failed; 4 ignored`), verified after golden refresh without `VOX_UPDATE_GOLDEN`.
