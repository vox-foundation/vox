---
phase: "457"
slug: "457-space-playground-and-free-provider-bridge"
title: "HF Space `vox-mens-playground` & Free Inference Provider Bridge"
status: "passed"
verified_at: "2026-11-15T06:52:00Z"
requirements:
  - "HF-SPACE-01"
  - "HF-SPACE-02"
  - "HF-SPACE-03"
  - "HF-SPACE-04"
---

# Phase 457 Verification

## Outcome: PASSED

Phase 457 deliverables (`HF-SPACE-01` through `HF-SPACE-04`) are complete and verified against Milestone `v7.7` criteria.

## Artifacts & Requirements Coverage

| Requirement | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `HF-SPACE-01` | `crates/vox-populi/templates/hub/space/README.md.tmpl:1-18`; `crates/vox-populi/templates/hub/space/index.html.tmpl:404-513`, `:675-720`; `crates/vox-populi/templates/hub/space/app.py.tmpl:31-103`, `:364-380` | Verified | `README.md.tmpl:10-17` links `{{MODELS_YAML}}` and describes the Nov 2026 ladder (`0.6B`, `2B`, `4B`, `9B`, `27B`); `index.html.tmpl:404-457,675-720` (`RUNG_DEFAULTS`, `LEGACY_RUNG_DEFAULTS`, `loadLadderIndex`) and `app.py.tmpl:39-103,364-380` (`ACTIVE_SIZES`, `LADDER_DEFAULTS`, `load_live_ladder_index`, `resolve_ladder_meta`) dynamically discover rungs from `<namespace>/vox-mens-index/raw/main/ladder_index.json`; `app.py.tmpl:29-37` uses bare Vox declarations (`table Name`, `component Name()`) with retired `@table` and `@component` syntax removed. |
| `HF-SPACE-02` | `crates/vox-populi/templates/hub/space/index.html.tmpl:351-370`, `:509-518`, `:1504`, `:1558-1560`, `:1675`; `crates/vox-populi/templates/hub/space/app.py.tmpl:20-37`, `:101`, `:260`, `:265-275` | Verified | `index.html.tmpl:351-370` and `app.py.tmpl:20-37` embed the canonical `vox_corpus::training::generate_training_system_prompt()` system prompt (`VOX_SYSTEM_PROMPT`). `formatUserPromptForModel` (`index.html.tmpl:509-518`) and `format_qwen_nothink_prompt` (`app.py.tmpl:260,265-275`) append literal ` /no_think` strictly when `noThinkSwitch` / `uses_no_think_switch` is true (`family === "qwen3"`), passing `chat_template_kwargs: { enable_thinking: false }` for `Qwen3.5` / `Qwen3.8` (`index.html.tmpl:1558-1560`). |
| `HF-SPACE-03` | `crates/vox-populi/templates/hub/space/index.html.tmpl:520-525`, `:1278-1281`, `:1427-1430`; `crates/vox-populi/templates/hub/space/app.py.tmpl:135-144`, `:160-162` | Verified | `resolveUseRslora` (`index.html.tmpl:520-525`) and `resolve_use_rslora` (`app.py.tmpl:135-144`) inspect `peftConfig`, `adapterManifest`, `trainingManifest`, and `adapterMetaV2`; `applyLoraToOnnxQ4F16` (`index.html.tmpl:1281`), `fetchAdapterArtifacts` (`index.html.tmpl:1430`), and `CandleLoRAArm` (`app.py.tmpl:162`) compute `scaleFactor = useRslora ? (alpha / Math.sqrt(rank)) : (alpha / rank)` (`alpha / math.sqrt(rank)` in Python) while supporting both `candle_qlora_adapter.safetensors` and `peft/adapter_model.safetensors` (`index.html.tmpl:1215-1264,1410-1414`; `app.py.tmpl:115-132,165-185,250`). |
| `HF-SPACE-04` | `crates/vox-populi/templates/hub/space/index.html.tmpl:1468-1623` | Verified | `runProviderRouterSideBySide` (`index.html.tmpl:1495-1623`) queries `https://router.huggingface.co/v1/chat/completions` (`index.html.tmpl:1534`) for base `${baseModelId}:auto` (`index.html.tmpl:1565`) and adapter candidates `[`${repoId}:auto`, `${repoId}:featherless-ai`]` (`index.html.tmpl:1593`), falling back gracefully to recorded held-out checkpoint outputs or local `vox mens serve` instructions (`index.html.tmpl:1604-1622`). |

## Automated Checks Run

- Direct non-Cargo hygiene checks run in this session:
  - `rg -n "@py\.import|@table|@component" crates/vox-populi/templates/hub/ docs/src/reference/mens-models.md` -> exit `1` (`0` matches).
  - `rg -ni "\.svg|<svg" crates/vox-populi/templates/ crates/vox-populi/tests/golden/hub_cards/` -> exit `1` (`0` matches).
  - `rg -ni "compiler-gated|qlora spoke|adapter ladder" mens/config/hub-cards.yaml crates/vox-populi/templates/hub/ crates/vox-populi/tests/golden/hub_cards/` -> exit `1` (`0` matches).
- Pre-turn verified Cargo baseline in `target/v77-verify`:
  - `cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib` -> exit `0` (`608 passed; 0 failed; 4 ignored`), covering `render_playground_space_files` and Space template tests in `hub_sync::tests`.
