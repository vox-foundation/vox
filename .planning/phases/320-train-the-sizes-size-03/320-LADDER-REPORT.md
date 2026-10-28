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
| S3 probes | ~120 s cap, preset per rung |
| S4 caps | None for 0.6B/1.7B; 14B/32B would use 14400 s if probed fit (not reached) |

Presets: `qwen3_dev_cpu` (0.6B), `qwen3_6g` (1.7B), `qwen3_12g` (4B), `qwen3_16g` (8B), `qwen3_24g` (14B), `qwen3_96g` (32B).

## Ladder outcomes (S4 + floors)

| size | status | pin | preset | opt steps | final loss EMA | peak MB (S4 / probe) | floor before → after (measured?) | wall h | trunc / prompt-trim | adapter |
|------|--------|-----|--------|-----------|----------------|----------------------|----------------------------------|--------|---------------------|---------|
| 0.6B | trained | `Qwen/Qwen3-0.6B@c1899de…` | qwen3_dev_cpu | 1108 | 0.690 | 15,319 (S4) | 2000 → 23675 (yes) | 2.82 | 34 / 34 | `mens/runs/qwen3-0.6b/candle_qlora_adapter.safetensors` |
| 1.7B | trained | `Qwen/Qwen3-1.7B@70d244cc…` | qwen3_6g | 138 | 0.617 | 27,007 (S4); 54,583 (S3 probe) | 5000 → 54583 (yes) | 4.17 | 34 / 34 | `mens/runs/qwen3-1.7b/candle_qlora_adapter.safetensors` |
| 4B | not trained | `Qwen/Qwen3-4B@1cfa9a72…` | qwen3_12g | — | — | 84,541 (S3 probe, lower bound) | 9800 → 56000 (estimate) | — | — | none |
| 8B | not attempted | `Qwen/Qwen3-8B@b968826d…` | qwen3_16g | — | — | — | 12000 → 62000 (estimate) | — | — | none |
| 14B | not attempted | `Qwen/Qwen3-14B@40c06982…` | qwen3_24g | — | — | — | 20000 → 70000 (estimate) | — | — | none |
| 32B | not attempted | `Qwen/Qwen3-32B@9216db57…` | qwen3_96g | — | — | — | 60000 → 85000 (estimate) | — | — | none |

Run records: `mens/runs/qwen3-<size>/train-record.json`. Evidence copies: `evidence/s4-record-<size>.json`, `evidence/ladder.json`, `evidence/floor-changes.json`.

## SIZE-03 success criteria

1. **Trained or capped sizes have adapter + pin in manifest** — checked against real run dirs; see `evidence/size-03-check.txt` (0.6B and 1.7B **trained**, manifest snapshots match pins).
2. **Non-fitting / skipped sizes have not_trained records, measurements when probed, no adapter** — 4B `not_trained` with probe peak; 8B/14B/32B `not_trained` / not attempted with null adapters and no `.safetensors` under `mens/runs/qwen3-*`.

**Overall:** `SIZE-03: PASS` (six size lines + stages S0–S4 PASS in `evidence/size-03-check.txt`).

## Findings

1. **Phase 315 serve gate (D-05).** The serve gate requires `collateral_damage_report.json` and `gate_receipt.json` in the adapter directory. S2 settled this by the decision recorded in `320-S2-SMOKE.md` (collateral + receipt produced for smoke adapter).
2. **Serve path bypass (Phase 321 HARN-01).** `vox mens serve --model` pointing at the adapter **file** rather than its directory skips the gate check at `dispatch.rs:350-359` (looks for `training_manifest.json` or `candle_qlora_adapter.safetensors` inside the `model` path). Not exercised in this phase (P-320-09).
3. **`--max-runtime-secs` before Plan 02.** Local cap did not reach the Metal trainer (field missing in plugin config); Plan 02 wired it through. S4 full runs were uncapped.
4. **`gpu-specs.yaml` embed.** Floors measured in Plan 08 apply on the next release build of `vox` / `vox-ml-cli` (YAML compiled in at build time).
5. **Probe vs full-run peak (1.7B).** S3 probe peaked **54,583 MB** at grad_accum 8; S4 full epoch peaked **27,007 MB** on the same preset after commit `c56e13a33` (LoRA grads kept off the Metal private pool). S4 peak is the floor input for 1.7B; probe peak remains the ladder’s conservative probe record.
6. **4B does not fit** at preset accum 8 (S3 watchdog); ladder climbing stopped per D-06.
7. **`agentic_default` picks changed** (from `floor-changes.json` after raised QLoRA estimates): at live MB 13926, 20890, 27853, 31334 → **Qwen2.5-Coder-7B** (was Qwen3-8B / Qwen3-14B); at 83558 MB → **Qwen3-14B** qlora (was Qwen3-32B).

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
- Only **0.6B** and **1.7B** adapters are on disk under `mens/runs/qwen3-0.6b` and `mens/runs/qwen3-1.7b`.
