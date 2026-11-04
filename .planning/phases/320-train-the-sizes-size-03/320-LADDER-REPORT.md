# Phase 320 — Qwen3 size ladder (SIZE-03 closeout)

**Host:** Apple Silicon GPU (`measured_on`: `silicon-gpu-metal`)  
**Branch / binaries:** `gsd/mens-compare-316`; release `vox` + mens-candle-metal at `c56e13a33` (Metal private-pool LoRA-grad fix for S4).

## Settings (all stages)

| Setting | Value |
|---------|--------|
| Backend | QLoRA on Metal (`--require-gpu`) |
| Sequence length | 3072 (`--seq-len 3072`) |
| Epochs | 1 |
| Corpus | `target/dogfood` (1166 rows after mix) |
| S3 probes | preset per rung; caps sized to pass the first 3071-token row (460 s 1.7B, 900 s 4B pre-fix, 1400 s 4B post-fix re-probe) |
| S4 caps | None for 0.6B/1.7B; 86400 s for 4B (not reached); 14B/32B would use 14400 s if probed fit (not reached) |

Presets: `qwen3_dev_cpu` (0.6B), `qwen3_6g` (1.7B), `qwen3_12g` (4B), `qwen3_16g` (8B), `qwen3_24g` (14B), `qwen3_96g` (32B).

## Ladder outcomes (S4 + floors)

| size | status | pin | preset | opt steps | final loss EMA | peak MB (S4 / probe) | floor before → after (measured?) | wall h | trunc / prompt-trim | adapter |
|------|--------|-----|--------|-----------|----------------|----------------------|----------------------------------|--------|---------------------|---------|
| 0.6B | trained | `Qwen/Qwen3-0.6B@c1899de…` | qwen3_dev_cpu | 1108 | 0.690 | 15,319 (S4) | 2000 → 23675 (320-08) → **15319** (yes, S4 peak) | 2.82 | 34 / 34 | `mens/runs/qwen3-0.6b/candle_qlora_adapter.safetensors` |
| 1.7B | trained | `Qwen/Qwen3-1.7B@70d244cc…` | qwen3_6g | 138 | 0.617 | 27,007 (S4); 54,583 (pre-fix S3 probe) | 5000 → 54583 (320-08) → **27007** (yes, S4 peak) | 4.17 | 34 / 34 | `mens/runs/qwen3-1.7b/candle_qlora_adapter.safetensors` |
| 4B | trained (2026-10-30; forced estimate, one resume) | `Qwen/Qwen3-4B@1cfa9a72…` | qwen3_12g | 138 | 0.543 (EMA of the 28 resumed rows; 0.600 before the resume) | 34,460 (S4, resumed segment); 33,071 (S4 rows 1-1108); 33,037 (post-fix re-probe); pre-fix probe ≥ 84,541 | 9800 → 56000 (320-08) → 33037 (re-probe) → **34460** (yes, S4 peak) | 10.96 (11.31 elapsed) | 0 / 0 (resumed rows only) | `mens/runs/qwen3-4b/candle_qlora_adapter.safetensors` |
| 8B | not attempted | `Qwen/Qwen3-8B@b968826d…` | qwen3_16g | — | — | — | 12000 → 62000 (320-08) → **122876** (estimate) | — | — | none |
| 14B | not attempted | `Qwen/Qwen3-14B@40c06982…` | qwen3_24g | — | — | — | 20000 → 70000 (320-08) → **195382** (estimate) | — | — | none |
| 32B | not attempted | `Qwen/Qwen3-32B@9216db57…` | qwen3_96g | — | — | — | 60000 → 85000 (320-08) → **295822** (estimate) | — | — | none |

Run records: `mens/runs/qwen3-<size>/train-record.json`. Evidence copies: `evidence/s4-record-<size>.json`, `evidence/ladder.json` (current floors, with the pre-fix 4B probe kept under `history`), `evidence/floor-changes.json` (320-08's snapshot). 4B re-probe: `320-S3-4B-REPROBE.md`, `evidence/s3r-*`.

The 8B/14B/32B estimates are `estimate_training_vram_mb_qlora` (`crates/vox-populi/src/mens/tensor/device.rs`) at seq 3072 with checkpointing. That is the trainer's own Metal estimate, so the floor check and the train-time estimate agree. 320-08's 62000/70000/85000 had been set above the pre-fix 1.7B probe peak (54,583), which the accumulation fix (c56e13a33) made obsolete. The estimate over-predicts the post-fix measured peaks, more so with size: measured/estimate is 0.74 (0.6B), 0.71 (1.7B) and 0.54 (4B re-probe; 0.56 for the 4B S4 peak). These floors are therefore conservative, and all three sit above this host's live free memory.

## SIZE-03 success criteria

1. **Trained or capped sizes have adapter + pin in manifest** — checked against real run dirs; see `evidence/size-03-check.txt` (0.6B and 1.7B **trained**, manifest snapshots match pins). 4B **trained** 2026-10-30: `training_manifest.json` `base_model` `Qwen/Qwen3-4B@1cfa9a72…` matches the pin.
2. **Non-fitting / skipped sizes have not_trained records, measurements when probed, no adapter** — 8B/14B/32B `not_trained` / not attempted with null adapters and no `.safetensors` under `mens/runs/qwen3-*`.

**Overall:** `SIZE-03: PASS` (six size lines + stages S0–S4 PASS in `evidence/size-03-check.txt`).

## Findings

1. **Phase 315 serve gate (D-05).** The serve gate requires `collateral_damage_report.json` and `gate_receipt.json` in the adapter directory. The S2 smoke adapter had neither, so `vox mens serve` refused at the gate (`evidence/s2-serve.txt`). By the D-05 decision `accept-eval-local` (2026-10-27, `320-S2-SMOKE.md`), S2 used `vox mens eval-local` as the load-and-answer proof. The serve check (W1) was carried to Phase 321. No collateral report or gate receipt was produced in this phase.
2. **Serve path bypass (Phase 321 HARN-01).** `vox mens serve --model` pointing at the adapter **file** rather than its directory skips the gate check at `dispatch.rs:350-359` (looks for `training_manifest.json` or `candle_qlora_adapter.safetensors` inside the `model` path). Not exercised in this phase (P-320-09).
3. **`--max-runtime-secs` before Plan 02.** Local cap did not reach the Metal trainer (field missing in plugin config); Plan 02 wired it through. S4 full runs were uncapped.
4. **`gpu-specs.yaml` embed.** Floors from Plan 08 and the 2026-10-29 follow-up (post-fix peaks, Qwen2.5 removal) apply on the next release build of `vox` / `vox-ml-cli` (YAML compiled in at build time).
5. **Floors come from post-fix peaks (D-08).** Before commit `c56e13a33`, LoRA grads were pinned in the Metal private pool across micro-steps (`320-S3-DEBUG.md`, resolved). The pre-fix 1.7B probe peaked at 54,583 MB, and the post-fix S4 full epoch on the same preset at 27,007 MB. The measured floors are now the post-fix peaks: 0.6B 15,319 and 1.7B 27,007 (S4 `run_summary`), and 4B 34,460 (S4 `run_summary`; the re-probe had 33,037). Each is `floor_mb = peak_mb`, `floor_measured: true`, `measured_on: silicon-gpu-metal`.
6. **4B fits on the fixed trainer, and was trained.** The pre-fix S3 probe was stopped by the watchdog at row 2 (86.3 GB), and ladder climbing stopped there per D-06. The post-fix re-probe (`320-S3-4B-REPROBE.md`) peaked at 33,037 MiB through the first 3071-token row. The user then approved a full run ("Yes, train 4B"), which ran on 2026-10-30 (`320-S4-RUNS.md`, 4B section): 35.11 s/row, 10.96 h of training (11.31 h elapsed), peak 34,460 MiB. Two things differ from 0.6B and 1.7B. First, `VOX_MENS_FORCE_TRAIN=1` was set because the trainer's estimate (61,637 MB) was above live free memory; the watchdog's measured-peak limit was the guard. Second, one resume from `checkpoint_step_1080` was needed because the watchdog misread post-epoch validation as a stall; memory was not involved. 8B and above were not probed: their estimate floors exceed live free memory.

7. **Selector never picks a retired Qwen2.5 base.** The Qwen2.5-Coder rungs were removed from `small_code_default`, `strong_code_default` and `agentic_default` (AGENTS.md Retired Surfaces: Qwen 3+), and a test now forbids a Qwen2.5 rung in any tag. `agentic_default` picks before → after:
   - 13926, 20890, 27853, 31334 MB: Qwen2.5-Coder-7B → **refuses** (`no 'agentic_default' base fits <N>MB VRAM`). The cheapest agentic_default Qwen3 rung is Qwen3.8-27B at 34000.
   - 83558 MB: Qwen3-14B QLoRA → **Qwen3-14B LoRA**. The 14B QLoRA estimate is now 195382, so the older 44000 LoRA estimate is the largest that fits.

   The size ladder (`qwen3_code`, used by `mens train` auto-pick) never held Qwen2.5 and is unchanged in shape.

## Stage gate (D-01)

| Stage | Evidence file | Last line |
|-------|---------------|-----------|
| S0 | `320-S0-EVIDENCE.md` | Stage S0: PASS |
| S1 | `320-S1-CORPUS.md` | Stage S1: PASS |
| S2 | `320-S2-SMOKE.md` | Stage S2: PASS |
| S3 | `320-S3-PROBES.md` | Stage S3: PASS |
| S4 | `320-S4-RUNS.md` | Stage S4: PASS |

## Phase 321 handoff

- Use this report + `evidence/ladder.json` for mesh/HARN work and any capped 14B/32B policy.
- **0.6B**, **1.7B** and **4B** adapters are on disk under `mens/runs/qwen3-{0.6b,1.7b,4b}`.
