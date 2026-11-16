---
phase: 458-post-train-live-hub-sync-and-audit
plan: 01
type: execute
wave: 7
depends_on: [429-01, 430-01, 454, 455, 456, 457, 457.1]
files_modified:
  - target/release/vox-ml-cli
  - mens/runs/qwen3-0.6b/
  - mens/runs/qwen3.5-2b/
  - mens/runs/qwen3.5-4b/
  - mens/runs/qwen3.5-9b/
  - mens/runs/qwen3.8-27b/
  - mens/runs/hub_cards/
  - .planning/workstreams/mens-hf-hub-autosync-v77/v7.7-MILESTONE-AUDIT.md
autonomous: true
requirements:
  - HF-LIVE-01
  - HF-LIVE-02
  - HF-LIVE-03
---

# Phase 458-01: Post-27B Release Rebuild, BF16 Causal-LM + PEFT Packaging, Live Hub Sync & Milestone v7.7 Audit

## Objective
After Milestone `v7.3` Phase 428 (`Qwen3.8-27B`, `task-22621`) and Phase 429/430 evaluations pass, rebuild `target/release/vox-ml-cli` (`mens-candle-metal`) with all Phase 454–457.1 changes, re-merge and package all gate-passing Nov 2026 rungs (`0.6B`, `2B`, `4B`, `9B`, `27B`) using Phase 455 native BF16 `recombine_hf_causal_lm` (`Qwen3_5ForCausalLM` promotion) + `peft/` export, execute live `vox mens hub sync --push`, verify live HF repos, Space, Collection, `.eval_results/vox_mens_eval.yaml`, and Router/Featherless/WebGPU inference endpoints, and close Milestone `v7.7` with `v7.7-MILESTONE-AUDIT.md`.

## Execution Invariants & Hardware Guardrails
- **Release Build Command (exact):**
  ```sh
  cargo build --release -p vox-ml-cli --features gpu,mens-candle-metal,mens-hf-hub
  ```
- **Single-GPU / Heavy-Memory Serial Execution:** `cargo build --release` and every `merge-qlora --keep-merged` invocation are `[SEQUENTIAL]`. Never run concurrent `merge-qlora` or Metal workloads on the 128 GiB host (`27B` BF16 recombine uses ~54 GiB disk/RAM).
- **Why `2B`/`4B`/`9B` Must Re-Merge:** `mens/runs/qwen3.5-{2b,4b,9b}` finished training on Nov 14 with pre-Phase-455 `merged_model.safetensors` (`Qwen3_5ForConditionalGeneration` layout, no `peft/` dir). Re-running `merge-qlora --keep-merged --license-class apache-2.0` with the rebuilt Phase-455 binary promotes `recombined_full/` to native BF16 `Qwen3_5ForCausalLM`, strips `vision_config`/`mtp_num_hidden_layers`, bundles `chat_template.jinja`/`special_tokens_map.json`/`Modelfile`, and exports `peft/{adapter_config.json,adapter_model.safetensors}`.
- **Secret Guard (Clavis):** Never read `.env*` or echo `HF_TOKEN`. Validate token availability exclusively via `vox secrets doctor` before `--push`.

---

## Tasks

### Task 458.1 — Rebuild Release `vox-ml-cli` & Verify Clavis `HF_TOKEN` `[SEQUENTIAL]`
- **Precondition:** `task-22621` (`Qwen3.8-27B` QLoRA) has finished cleanly (`mens/runs/qwen3.8-27b/run_summary.json` present).
- **Actions:**
  1. Build the release binary with Metal + HF Hub features:
     ```sh
     cargo build --release -p vox-ml-cli --features gpu,mens-candle-metal,mens-hf-hub
     ```
  2. Verify Clavis secret readiness (aborts if `HF_TOKEN` missing/expired; never prints token):
     ```sh
     vox secrets doctor
     ```
  3. Verify `VOX_MENS_HF_NAMESPACE` (or pass `--namespace brbrainerd` explicitly on `hub` commands).
- **Abort Trigger:** Non-zero `cargo build --release` or missing `HF_TOKEN` in `vox secrets doctor` — **STOP** before any `--push`.

### Task 458.2 — Native BF16 Causal-LM `merge-qlora` & `peft/` Packaging (`0.6B`, `2B`, `4B`, `9B`, `27B`) `[SEQUENTIAL]`
- **Precondition:** Task 458.1 complete and Phase 429 `gate_receipt.json` (`overall_passed: true`) present for each rung to be packaged.
- **Commands (run strictly one rung at a time):**
  1. `0.6B` (`mens/runs/qwen3-0.6b`):
     ```sh
     ./target/release/vox-ml-cli mens merge-qlora --base-shard ~/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca/model.safetensors --adapter mens/runs/qwen3-0.6b/candle_qlora_adapter.safetensors --meta mens/runs/qwen3-0.6b/adapter_manifest.json --output mens/runs/qwen3-0.6b/merged_model.safetensors --keep-merged --license-class apache-2.0
     ```
  2. `2B` (`mens/runs/qwen3.5-2b`):
     ```sh
     ./target/release/vox-ml-cli mens merge-qlora --base-shard ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-2B/snapshots/15852e8c16360a2fea060d615a32b45270f8a8fc/model.safetensors-00001-of-00001.safetensors --adapter mens/runs/qwen3.5-2b/candle_qlora_adapter.safetensors --meta mens/runs/qwen3.5-2b/adapter_manifest.json --output mens/runs/qwen3.5-2b/merged_model.safetensors --keep-merged --license-class apache-2.0
     ```
  3. `4B` (`mens/runs/qwen3.5-4b`):
     ```sh
     ./target/release/vox-ml-cli mens merge-qlora --base-shard ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-4B/snapshots/851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a/model.safetensors-00001-of-00002.safetensors --adapter mens/runs/qwen3.5-4b/candle_qlora_adapter.safetensors --meta mens/runs/qwen3.5-4b/adapter_manifest.json --output mens/runs/qwen3.5-4b/merged_model.safetensors --keep-merged --license-class apache-2.0
     ```
  4. `9B` (`mens/runs/qwen3.5-9b`):
     ```sh
     ./target/release/vox-ml-cli mens merge-qlora --base-shard ~/.cache/huggingface/hub/models--Qwen--Qwen3.5-9B/snapshots/c202236235762e1c871ad0ccb60c8ee5ba337b9a/model.safetensors-00001-of-00004.safetensors --adapter mens/runs/qwen3.5-9b/candle_qlora_adapter.safetensors --meta mens/runs/qwen3.5-9b/adapter_manifest.json --output mens/runs/qwen3.5-9b/merged_model.safetensors --keep-merged --license-class apache-2.0
     ```
  5. `27B` (`mens/runs/qwen3.8-27b`):
     ```sh
     ./target/release/vox-ml-cli mens merge-qlora --base-shard ~/.cache/huggingface/hub/models--Qwen--Qwen3.8-27B/snapshots/37561816e66b5b9153b1b64c10968c18c1e6f4a0/model-00001-of-00018.safetensors --adapter mens/runs/qwen3.8-27b/candle_qlora_adapter.safetensors --meta mens/runs/qwen3.8-27b/adapter_manifest.json --output mens/runs/qwen3.8-27b/merged_model.safetensors --keep-merged --license-class apache-2.0
     ```
- **Verification per Rung:**
  - `mens/runs/<rung>/recombined_full/` contains BF16 `model.safetensors` (or sharded `model-*.safetensors` + `model.safetensors.index.json`), `config.json` (`model_type: "qwen3_5_text"` and `architectures: ["Qwen3_5ForCausalLM"]` for `2B`/`4B`/`9B`, zero `vision_config`/`mtp_num_hidden_layers`), `tokenizer.json`, `tokenizer_config.json`, `chat_template.jinja`, `Modelfile`.
  - `mens/runs/<rung>/peft/` contains `adapter_config.json` (`use_rslora`, `r`, `lora_alpha`, `target_modules`, `base_model_name_or_path`) and `adapter_model.safetensors`.
  - Root `mens/runs/<rung>/candle_qlora_adapter.safetensors` remains intact.

### Task 458.3 — Pre-Push Dry-Run & Rendered Card Lints `[SEQUENTIAL]`
- **Actions:**
  1. Dry-run Hub sync (no `--push`):
     ```sh
     ./target/release/vox-ml-cli mens hub sync --namespace brbrainerd
     ```
  2. Run rendered card lints on `mens/runs/hub_cards/`:
     ```sh
     VOX_HUB_CARDS_DIR=/Users/brbrainerd/dev/vox/mens/runs/hub_cards cargo test -p vox-populi --features mens-train,mens-hf-hub,mens-cloud --lib lints_hold_on_rendered_cards_dir -- --ignored
     ```
  3. Confirm zero SVGs in Nov 2026 card directories:
     ```sh
     rg -ni "\.svg|<svg" mens/runs/hub_cards/vox-mens-index mens/runs/hub_cards/vox-mens-playground mens/runs/hub_cards/vox-mens-qwen3-0.6b mens/runs/hub_cards/vox-mens-qwen3.5-2b mens/runs/hub_cards/vox-mens-qwen3.5-4b mens/runs/hub_cards/vox-mens-qwen3.5-9b mens/runs/hub_cards/vox-mens-qwen3.8-27b
     ```
     (Must exit `1` with `0` matches.)

### Task 458.4 — Live Hugging Face Hub Push (`hub sync --push`) `[SEQUENTIAL]`
- **Precondition:** Task 458.3 green and `vox secrets doctor` confirms `HF_TOKEN`.
- **Actions:**
  1. Execute live push for all gate-passing rungs, `<ns>/vox-mens-index`, `<ns>/vox-mens-playground` Space, and HF Collection:
     ```sh
     ./target/release/vox-ml-cli mens hub sync --namespace brbrainerd --push
     ```
  2. Verify Hub vs local parity:
     ```sh
     ./target/release/vox-ml-cli mens hub verify --namespace brbrainerd --json
     ```
- **Expected Artifacts on HF Hub:**
  - `<ns>/vox-mens-index` (`README.md`, `ladder_index.json`, `assets/overview.jpg`)
  - `<ns>/vox-mens-qwen3-0.6b`, `<ns>/vox-mens-qwen3.5-2b`, `<ns>/vox-mens-qwen3.5-4b`, `<ns>/vox-mens-qwen3.5-9b`, `<ns>/vox-mens-qwen3.8-27b` (root `model.safetensors` / shards, `peft/`, `candle_qlora_adapter.safetensors`, tokenizer/chat artifacts, `.eval_results/vox_mens_eval.yaml`, Model Card v3 `README.md`, `assets/banner.jpg`)
  - `<ns>/vox-mens-playground` Static Space (`README.md`, `index.html`, `app.py`)
  - HF Collection linking index, all published ladder repos, and Playground Space.

### Task 458.5 — Live Endpoint, Space & Quality Gate Verification `[PARALLEL-SAFE]`
- **Actions (run independently):**
  1. Verify HF API / Hub metadata for `<ns>/vox-mens-index`, `<ns>/vox-mens-qwen3.5-4b`, `<ns>/vox-mens-playground`, and the HF Collection (`README.md` frontmatter, `.eval_results/vox_mens_eval.yaml`, zero leaked local `/Users/` paths).
  2. Smoke-test live Inference Providers (`router.huggingface.co/v1/chat/completions` for `${baseModelId}:auto` and `${repoId}:auto` / `:featherless-ai` fallback) and confirm `<ns>/vox-mens-playground` Space loads `ladder_index.json` + WebGPU rsLoRA merger cleanly.
  3. Run final workspace verification gates:
     - `cargo run -p vox-arch-check`
     - `cargo test -p vox-code-audit --lib`
     - `cargo run -p vox-doc-pipeline -- --lint-only`

### Task 458.6 — Milestone `v7.7` Audit & Closure `[SEQUENTIAL]`
- **Precondition:** Tasks 458.1–458.5 all green.
- **Actions:**
  1. Write `.planning/workstreams/mens-hf-hub-autosync-v77/phases/458-post-train-live-hub-sync-and-audit/458-SUMMARY.md`.
  2. Write `.planning/workstreams/mens-hf-hub-autosync-v77/v7.7-MILESTONE-AUDIT.md` covering `HF-RES-01..02`, `HF-SYNC-01..05`, `HF-PKG-01..04`, `HF-CARD-01..04`, `HF-SPACE-01..04`, `HF-ONBOARD-01..04`, and `HF-LIVE-01..03`.
  3. Update `.planning/workstreams/mens-hf-hub-autosync-v77/ROADMAP.md` to mark Phase 458 complete.

---

## Abort & Rollback Triggers
- **Missing `HF_TOKEN` or Auth 401/403:** Hard stop before `--push`; fix via `vox secrets` (Clavis) and re-run `vox secrets doctor`.
- **Card Lint or SVG Check Failure:** Abort before `--push`; fix template/asset source and re-run `hub sync` dry-run.
- **Partial Upload / Diverged Repo State:** Do **not** force-overwrite gated `main` with ungated weights; inspect `vox mens hub verify --json` and re-run `vox mens hub sync --namespace brbrainerd --push --size <S>` for the affected rung only.
- **Router / Featherless Config Rejection:** Verify `recombined_full/config.json` has `model_type: "qwen3_5_text"` and `architectures: ["Qwen3_5ForCausalLM"]` and that `peft/` is isolated from root `model.safetensors`.
