# Phase 320 — Stage S3: memory probes up the ladder (D-06)

**Run:** 2026-10-27 22:26-22:40 local (2026-10-28T05:26Z-05:40Z). One GPU job at a time.

**Host.** Train record `host` `Apple Silicon GPU`; `measured_on` `silicon-gpu-metal` (the plan's rule: lowercase, leading `apple ` removed, spaces to `-`, `-metal` appended). Metal recommended working set 110,101 MB.

**Binaries.** `target/release/vox --version` printed `vox 0.6.0+build.7442 (cdc8fdc3a)` (same build as S2 attempt 3). The installed Metal plugin and `target/release/libvox_plugin_mens_candle_metal.dylib` are identical (`diff -q`), sha256 `73e98f3dcd216e04c6117dae3aad3601bfbfbb421f44f2bed45ad3f7af5cfe6f`.

## Probe settings

- Command (from the repo root): `target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-<S> --output-dir mens/runs/320-probe-<S> --max-runtime-secs <cap> --force-restart`. `vox` forwards to `vox-ml-cli`; the shell allowlist blocks calling `vox-ml-cli` directly.
- **Each size's real preset** (as S4 will run it). Batch and accumulation are the trainer's own (`train_start` telemetry): 1.7B `qwen3_6g` batch 1 / accum 8, 4B `qwen3_12g` batch 1 / accum 8. No `--grad-accum` override.
- **Caps longer than the plan's 120 s, on purpose (orchestrator change).** The probe has to reach a full-length row: with seed 42 the corpus order is the same for every size, and row 33 is the first 3071-token row (rows 1-32 run 2,277-2,511 tokens). The cap stops only at an optimizer boundary, so at accum 8 a cap that expires during rows 33-40 stops at row 40, which is 5 optimizer steps. The caps were 460 s for 1.7B and 900 s for 4B, picked from the guessed s/row.
- **Fit rule.** A size fits when the record is `trained` or `capped`, `peak_mb > 0`, and the full-row peak is at most the live `free_mb`. The live `free_mb` comes from a fresh `--plan-only --json` run just before launch (`evidence/s3-plan-<S>.json`, source `Metal working set`). When no row of 3,000 tokens or more was seen, the peak is a lower bound and is scaled by 1.12 (the S2 full-row factor) for the decision.
- **Watchdog (orchestrator safety rule).** A shell loop read the probe's `telemetry.jsonl` every 3 s. It would SIGINT the `vox-ml-cli` trainer for any of these:
  - the largest `peak_metal_allocated_bytes` (or `observed_bytes`) exceeded (live free MB − 4,096 MiB)
  - a row took more than 5× the previous row
  - no row for more than 5× the last interval (and over 60 s) before `run cap reached`
  - a deadline passed

  It would SIGKILL 120 s after a SIGINT if the trainer was still alive. A probe the watchdog stops is recorded as not fitting, with its measurement. The output is in `evidence/s3-watchdog-<S>.txt`.
- `VOX_MENS_FORCE_TRAIN` was **not** set for any probe. Neither the floor check nor the run-time estimate refused 1.7B or 4B, so no forced probe was needed.
- Disk before the 4B download: `df -g` showed 230 GB free, above the 29 GB needed (9 + 20). The 4B base (7.5 GB on disk) is left in the cache for later use.

## Results

| size | preset | accum | verdict | forced | free MB (source) | peak MB (source) | full row seen | full-row peak MB | lower bound | record status | opt / micro steps | s/row | projected full epoch (h) | reason |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0.6B | qwen3_dev_cpu | 1 | fits | no | 58,547 (Metal working set, S1) | 23,675 (run_summary) | yes, row 33 (3,071 tok) | 23,675 | — | capped | 60 / 60 | 8.10 | 2.62 | S2 attempt 3 smoke (480 s cap) |
| 1.7B | qwen3_6g | 8 | fits | no | 65,651 (Metal working set) | 54,583 (run_summary) | yes, row 33 (3,071 tok) | 54,583 | — | capped | 5 / 40 | 12.09 | 3.92 | — |
| 4B | qwen3_12g | 8 | does_not_fit | no | 68,010 (Metal working set) | 84,541 (telemetry) | no (max 2,333 tok) | ≥ 94,686 (×1.12) | peak is a lower bound | not_trained | 0 / 3 | ~38 (under memory pressure) | — | watchdog SIGINT at row 2: 86,329,229,312 B > limit 67,018,686,464 B; paused after row 3 |
| 8B | qwen3_16g | 8 | not_attempted | — | — | — | — | — | — | — | — | — | — | above non-fitting 4B (D-06) |
| 14B | qwen3_24g | 8 | not_attempted | — | — | — | — | — | — | — | — | — | — | above non-fitting 4B (D-06) |
| 32B | qwen3_96g | 8 | not_attempted | — | — | — | — | — | — | — | — | — | — | above non-fitting 4B (D-06) |

**First size that does not fit: 4B.** 8B, 14B and 32B were not attempted, and no probe directory exists for them.

### 1.7B (fits)

- Exit 0. `run cap reached after 483 s at optimizer step 5; finalizing`. `Training started: first backward pass done after 15.1s (139 optimizer steps planned, grad_accum=8)`. The estimate line read `VRAM: est. ~38056 MB`.
- Peak per row (`evidence/s3-telemetry-1.7B.jsonl`):

  | rows | peak |
  |---|---|
  | 1 | 35.53 GB |
  | 2 | 47.77 GB |
  | 3 | 50.32 GB |
  | 4-19 | 50.70 GB |
  | 20-28 | 50.75-50.86 GB |
  | 29-32 | 51.07 GB |
  | 33-40 (row 33 is 3,071 tokens) | **57.23 GB** |

  The full row added 6.2 GB. The record's `peak_mb` is 54,583 (`run_summary`), 83% of live free and 11,068 MiB under it.
- Record (`evidence/s3-record-1.7B.json`): `capped`, pin `Qwen/Qwen3-1.7B@70d244cc…`, 5 optimizer / 40 micro steps, `train_wall_secs` 483.7. That gives 12.09 s per row. `final_loss_ema` 1.820, truncated / prompt-trimmed rows 1 / 1. The watchdog never fired (last peak 57,233,555,456 B against a limit of 64,545,095,680 B).

### 4B (does not fit)

- The base downloaded at pin `1cfa9a72…`. The estimate did not refuse (`VRAM: est. ~61637 MB`; live free 68,010). `Training started: first backward pass done after 36.4s`.
- Peak per row:

  | row | tokens | peak |
  |---|---|---|
  | 1 | 2,333 | 59.70 GB |
  | 2 | 2,277 | **86.33 GB** |
  | 3 | 2,280 | 88.65 GB |

  The watchdog sent SIGINT after row 2 (86.3 GB against a 67.0 GB limit). The trainer logged `Training paused at step 3` and wrote its record. Exit 0.
- Record (`evidence/s3-record-4B.json`): `not_trained`, `no run_summary.json from this run (paused or interrupted)`, `peak_mb` 84,541 (`telemetry`). That is 124% of live free memory, before any full-length row. Scaled for a full row it is ≥ 94,686 MiB. Rows 2-3 took 37.0 s and 38.9 s, but the peak was above free memory then, so that rate is not a projection.
- The estimate (61,637 MiB, every row at 3,072 tokens) is at least 37% low for 4B at the preset's accum 8.

## Projections for S4 (`evidence/ladder.json`)

- Method: projected full epoch = this probe's `train_wall_secs / micro_steps` × **1,166 rows**, which is `wc -l target/dogfood/train_mixed.jsonl`. The trainer itself plans 1,108 micro-steps for those rows, so these projections run about 5% high.
- The plan's step-7 figure (`planned_optimizer_steps / steps_per_sec_ema`) is kept as `trainer_projected_full_secs`: 0.6B 9,111 s and 1.7B 7,736 s. It is lower because the early-run EMA lags the real rate.
- 0.6B: 8.10 s/row gives **2.62 h**. 1.7B: 12.09 s/row gives **3.92 h**. Neither is over 24 h, so both get full one-epoch runs under D-07. No fitting size needs a time cap.

## Findings (not fixed here)

- **Accumulation memory grows with model size.** Going from row 1 to row 2 (still inside optimizer step 0) adds 12.2 GB at 1.7B and 26.6 GB at 4B. That is about 2.2×, close to the parameter ratio. Something sized like the base weights stays resident across micro-steps. The estimate has no accumulation term (still open in `320-S2-DEBUG.md`). This is why 4B misses: its row-1 peak (59.7 GB) was under the live budget.
- **The 1.7B margin depends on host load.** Live free memory was 57,521 MiB in S1 and 65,651 MiB here. A 3.9 h run that peaks at 54.6 GB needs that much free memory for the whole run.
- The estimate under-predicts: 1.7B at accum 8 measured 54,583 against the 38,056 estimate, and 4B is ≥ 37% low (above).
- 4B with `--grad-accum 1` was not probed. The orchestrator's rule is real preset settings, and D-06 stops climbing at the first non-fit. Whether 4B could run at accum 1 (row-1 peak 59.7 GB plus the full-row increase) is a user decision, not part of this stage.

Stage S3: PASS
