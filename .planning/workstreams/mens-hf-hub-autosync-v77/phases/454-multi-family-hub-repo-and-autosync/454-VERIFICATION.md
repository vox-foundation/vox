---
phase: "454"
slug: "454-multi-family-hub-repo-and-autosync"
title: "Multi-Family `hub_repo`, `hub_sync`, `hub_cards_push`, `.eval_results`, Collections & Auto-Sync Fixes"
status: "passed"
verified_at: "2026-11-15T06:52:00Z"
requirements:
  - "HF-SYNC-01"
  - "HF-SYNC-02"
  - "HF-SYNC-03"
  - "HF-SYNC-04"
  - "HF-SYNC-05"
---

# Phase 454 Verification

## Outcome: PASSED

Phase 454 deliverables (`HF-SYNC-01` through `HF-SYNC-05`) are complete and verified against Milestone `v7.7` criteria.

## Artifacts & Requirements Coverage

| Requirement | Artifact | Status | Notes |
| --- | --- | --- | --- |
| `HF-SYNC-01` | `crates/vox-populi/src/mens/hub_repo.rs:15-210`, `:216-300` | Verified | `SIZES` (`["0.6B", "2B", "4B", "9B", "27B"]`), `LEGACY_SIZES` (`["1.7B", "8B", "14B", "32B"]`), `family_of_model`, `size_of_pin`, `model_label_for`, `repo_slug_for_model`, `local_run_dir_for_size`, `repo_id_for_size`, and `space_repo_id` resolve Nov 2026 multi-family rungs (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) and legacy `Qwen3-*` pins (`Qwen3-4B`, `1.7B`, `8B`, `14B`, `32B`). |
| `HF-SYNC-02` | `crates/vox-populi/src/mens/hub_sync.rs:332-362`, `:498-635`, `:1036-1060`, `:1127-1621`, `:1865`, `:1973`; `crates/vox-populi/src/mens/hub.rs:186-205`; `crates/vox-populi/src/mens/tensor/external_serving_handoff.rs:7-115` | Verified | Eliminated hardcoded `Qwen3-{size}` / `mens/runs/qwen3-{}` assumptions via `hub_repo::repo_slug_for_model` and `local_run_dir_for_size`; serialized Clavis token env propagation under `HF_ENV_LOCK` (`normalize_hf_token_env` across `hub_sync.rs:650,1215,1234,1250,1621`); normalized HF cache snapshot paths and redacted host paths in `sanitize_json_paths` (`hub_sync.rs:332-362`); generated `.eval_results/vox_mens_eval.yaml` (`build_eval_results_yaml`, `hub_sync.rs:498-635,1036-1044`); staged `external_serving_handoff_v1.json` (`hub_sync.rs:1046-1059`) and `--spoke` subdirectory targets (`hub_sync.rs:1127,1562,1973`). |
| `HF-SYNC-03` | `crates/vox-populi/src/mens/hub_cards_push.rs:35-74`, `:124-156`, `:275-310`; `crates/vox-populi/src/mens/hub_cards_push_tests.rs:202-265`, `:328-366` | Verified | `is_card_collateral` (`hub_cards_push.rs:35-44`) admits `README.md`, `MODEL_CARD.md`, `external_serving_handoff_v1.json`, `assets/*.{jpg,jpeg,png,webp}`, and `.eval_results/*.{yaml,yml}`; `remote_weights_match` (`hub_cards_push.rs:59-74`) and `plan_card_push` (`hub_cards_push.rs:124-156`) return `CardPushPlan::CardOnly` whenever remote `.safetensors` hashes match local even when `gate_current == true`, avoiding redundant weight re-uploads. |
| `HF-SYNC-04` | `crates/vox-populi/src/mens/hub.rs:576-745`; `crates/vox-populi/src/mens/hub_sync.rs:2150-2166` | Verified | `upload_ladder_index` (`hub_sync.rs:2150-2166`) builds `collection_items` across all ladder model repos, `<ns>/vox-mens-index` (`model`), and `<ns>/vox-mens-playground` (`space`), invoking idempotent `ensure_mens_collection` (`hub.rs:576-745`) authenticated via Clavis `SecretId::HuggingFaceToken` with HTTP `409 Conflict` handling. |
| `HF-SYNC-05` | `crates/vox-ml-cli/src/commands/mens/eval_gate/run_gate.rs:9-332`; `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:675-783`; `crates/vox-ml-cli/src/commands/mens/pipeline.rs:659-739` | Verified | `run_eval_gate` (`run_gate.rs:9-55,205-251`) resolves `mens/config/eval-gates-ladder.yaml` as the primary ladder policy while emitting `gate_receipt.vox-lang.json` against `mens/config/eval-gates.yaml`; `train_arm.rs:688-724,778-783` checks `fetch_remote_sync_manifest` and `should_skip_ungated_overwrite` so ungated runs never overwrite a `passed` remote checkpoint on `main`; `pipeline.rs:659,687-739` wires `sync_local_hf_cards_offline` into `vox mens pipeline`. |

## Automated Checks Run

- Pre-turn verified Cargo baseline in `target/v77-verify`:
  - `cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib` -> exit `0` (`608 passed; 0 failed; 4 ignored`), covering `hub_repo::tests`, `hub_sync::tests`, `hub_cards_push_tests`, and `hub::tests`.
- Direct non-Cargo hygiene checks run in this session:
  - `rg -ni "\.svg|<svg" crates/vox-populi/templates/ crates/vox-populi/tests/golden/hub_cards/` -> exit `1` (`0` matches).
  - `./target/debug/vox-doc-pipeline --lint-only --paths architecture/hf-model-cards-and-inference-providers-research-2026.md reference/mens-models.md` -> exit `0` (`vox-doc-pipeline lint complete — no hard errors.`).
