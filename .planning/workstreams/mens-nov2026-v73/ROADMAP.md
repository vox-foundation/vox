# Roadmap: Milestone v7.3 (`mens-nov2026-v73`) — Nov 2026 Dense MENS QLoRA Ladder Calibration, Multi-Spoke Corpus, Training, Evaluation & Promotion

**Workstream:** `.planning/workstreams/mens-nov2026-v73/`
**Scope:** Authorized Nov 2026 Dense Qwen3.5/Qwen3.8 QLoRA ladder (`2B`, `4B`, `9B`, `27B`) on Apple M5 Max (`128 GiB` Metal unified memory) + `Qwen/Qwen3-0.6B` sub-1B CPU/WebGPU rung. Excludes `crates/vox-gui/ui/**`, root `.planning/**`, and unapproved weights (`MiMo-V2.6-Distill-Qwen-9B`, `FrogNano-4B-2609`, `Qwen3.6-35B-A3B`).

## Phase Checklist

- [x] **Phase 425: Stage D Pre-Flight Code Seams & Zero-Copy Cache Prefetch CLI** (`MENS-NOV26-PREFLIGHT-01..04`)
- [x] **Phase 426: Parallel HF Text-Tower Prefetch & Multi-Spoke Corpus Regeneration** (`MENS-NOV26-PREP-01..03`)
- [x] **Phase 427: Metal Memory Model Slope Calibration & Ladder Floor Measurement** (`MENS-NOV26-CAL-01..03`)
- [x] **Phase 428: Packed QLoRA Ladder Fine-Tuning & Adapter Recombine Verification** (`MENS-NOV26-TRAIN-01..04`)
- [ ] **Phase 429: Paired Held-Out, Spoke & Collateral Reasoning Gate Evaluation** (`MENS-NOV26-EVAL-01..03`)
- [ ] **Phase 430: Post-Gate Ladder Promotion, Hub Cards v2 & Milestone v7.3 Audit** (`MENS-NOV26-PROMOTE-01..03`)

---

### Phase 425: Stage D Pre-Flight Code Seams & Zero-Copy Cache Prefetch CLI

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/425-preflight-seams/`
- **Goal:** Fix all six Stage D pre-flight code seams (`generation_config.json`/`special_tokens_map.json` download, `detect::snapshot_complete` vision/MTP shard exclusion, `compare_json_path` Qwen3.5/Qwen3.8 naming, `size_train` multi-family ladder support, `HubAction::Download --cache-only`, and `size_select` `qwen3_5_code` support), pass crate unit tests, and build one shared `target/release/vox-ml-cli` (`mens-candle-metal`) binary.
- **Depends on:** Prior MENS Nov 2026 Stages A–C & synthetic Stage E
- **Requirements:** `MENS-NOV26-PREFLIGHT-01`, `MENS-NOV26-PREFLIGHT-02`, `MENS-NOV26-PREFLIGHT-03`, `MENS-NOV26-PREFLIGHT-04`
- **Success Criteria:**
  1. `vox_populi::mens::hub::download_model` downloads `generation_config.json` and `special_tokens_map.json` when present in HF siblings and exposes them on `DownloadedModelFiles`.
  2. `detect::snapshot_complete` excludes `model.visual.*` and `mtp.*`-only shards via `vox_populi::mens::hub::is_text_tower_weight_key`.
  3. `compare_json_path`, `size_train`, and `size_select` support `Qwen3.5-{2B,4B,9B}` and `Qwen3.8-27B` while preserving legacy `qwen3_code` tests.
  4. `vox-ml-cli mens hub download --cache-only` populates and verifies snapshots without duplicating shards.

### Phase 426: Parallel HF Text-Tower Prefetch & Multi-Spoke Corpus Regeneration

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/426-prefetch-and-corpus/`
- **Goal:** Prefetch text-only HF snapshots (`Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) concurrently across network I/O workers while a CPU corpus worker regenerates compiler-verified `mens/data/` and spoke mixes.
- **Depends on:** Phase 425
- **Requirements:** `MENS-NOV26-PREP-01`, `MENS-NOV26-PREP-02`, `MENS-NOV26-PREP-03`
- **Success Criteria:**
  1. All authorized pinned checkpoints (`Qwen3-0.6B`, `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, `Qwen3.8-27B`) pass `snapshot_complete` with `generation_config.json` present and zero vision/MTP-only shards downloaded.
  2. Fresh compiler-verified corpus and spoke mixes (`vox-lang`, `rust`, `tool-selection`, `argument-generation`, `agents`, `harness`) are generated with zero held-out benchmark overlap.

### Phase 427: Metal Memory Model Slope Calibration & Ladder Floor Measurement

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/427-metal-calibration/`
- **Goal:** Fit real `candle-metal` `act_bytes_per_lht` slope from two `Fits` observations on `Qwen3.5-2B`, measure real Metal QLoRA peak floors across `4B`, `9B`, and `27B`, write relative-path measurement records, and update `contracts/mens/memory-model.v1.yaml` and `mens/config/gpu-specs.yaml` (`floor_measured: true`).
- **Depends on:** Phase 426
- **Requirements:** `MENS-NOV26-CAL-01`, `MENS-NOV26-CAL-02`, `MENS-NOV26-CAL-03`
- **Success Criteria:**
  1. Two distinct `tokens_per_step` Metal runs on `Qwen3.5-2B` determine a real `act_bytes_per_lht` slope for `candle-metal`.
  2. Measured `peak_mb`, safe `floor_mb`, `floor_measured: true`, and `measured_on: silicon-gpu-metal` are recorded in `mens/config/gpu-specs.yaml` and `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.json`.
  3. `vox-ml-cli mens probe --detailed --model Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` renders a `Fits` verdict.

### Phase 428: Packed QLoRA Ladder Fine-Tuning & Adapter Recombine Verification

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/428-qlora-ladder-train/`
- **Goal:** Sequentially train packed QLoRA adapters (`--use-rslora`, `--sequence-packing`, `--token-normalized-grad-accum`, `--lora-exclude-lm-head` on tied `2B`/`4B`) for `Qwen3.5-2B`, `Qwen3.5-4B`, `Qwen3.5-9B`, and `Qwen3.8-27B` on Metal, and verify `merge-qlora` recombine outputs.
- **Depends on:** Phase 427
- **Requirements:** `MENS-NOV26-TRAIN-01`, `MENS-NOV26-TRAIN-02`, `MENS-NOV26-TRAIN-03`, `MENS-NOV26-TRAIN-04`
- **Success Criteria:**
  1. Each rung completes training into `mens/runs/qwen3.5-{2b,4b,9b}` and `mens/runs/qwen3.8-27b` with real steps, EMA/val loss, runtime, and `peak_mb` recorded in `run_summary.json`.
  2. `merge-qlora` succeeds cleanly for every trained rung.

### Phase 429: Paired Held-Out, Spoke & Collateral Reasoning Gate Evaluation

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/429-paired-eval-gates/`
- **Goal:** Execute paired `eval-local`, hash-pinned `eval-reasoning-compare` (`gsm8k_think`, `math500_think`, `vox_think`), `eval-collateral-damage` McNemar check, and `eval-gate` across the trained ladder rungs, producing the `gate_receipt.json` + `reasoning_compare/` inputs consumed dynamically by `hub_card::facts`. (`size-train` was retired in commit `9a429892f`.)
- **Depends on:** Phase 428
- **Requirements:** `MENS-NOV26-EVAL-01`, `MENS-NOV26-EVAL-02`, `MENS-NOV26-EVAL-03`
- **Success Criteria:**
  1. Every trained rung has paired base vs candidate `eval-local`, reasoning compare JSON under `mens/runs/reasoning_compare/`, and `gate_receipt.json`.
  2. `hub_card::facts` dynamically ingests the rung `gate_receipt.json` and `mens/runs/reasoning_compare/` outputs for Hub Cards v2 without manual tables or retired `size_train` receipts.

### Phase 430: Post-Gate Ladder Promotion, Hub Cards v2 & Milestone v7.3 Audit

- **Directory:** `.planning/workstreams/mens-nov2026-v73/phases/430-hub-promotion-audit/`
- **Goal:** Promote gate-passing rungs into `hub_repo.rs`, `hub-cards.yaml`, and reference docs, pass all architectural, doc-pipeline, card-lint, and hardcoded-path audits, and close Milestone `v7.3`.
- **Depends on:** Phase 429
- **Requirements:** `MENS-NOV26-PROMOTE-01`, `MENS-NOV26-PROMOTE-02`, `MENS-NOV26-PROMOTE-03`
- **Success Criteria:**
  1. Gate-passing rungs are reflected in `hub_repo.rs`, `mens/config/hub-cards.yaml` (strict 2–3 sentences, zero ASCII digits in `why`/`purpose`), and `docs/src/reference/mens-models.md` + `docs/src/architecture/mens-model-selection-nov-2026-research-2026-11-13.md`.
  2. `vox-populi` (including rendered card lints), `vox-ml-cli`, `vox-arch-check`, `vox-doc-pipeline --lint-only`, and hardcoded-path scans all pass with `0` errors.
  3. `v7.3-MILESTONE-AUDIT.md` and Phase summaries are complete.
