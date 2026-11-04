# Phase 320 — S3 follow-up: Qwen3-4B re-probe on the fixed trainer

**Run:** 2026-10-29 18:46-19:13 local (2026-10-30T01:46:30Z-02:13:21Z). User decision 2026-10-29: "Yes, re-probe 4B". Nothing else was training (no `mens train` / `mens eval` process before launch).

**Verdict: 4B fits.** Peak 33,037 MiB at the preset's accum 8, including the first full-length row, against 57,754 MiB allowed (live free 61,850 − 4,096). The projected full epoch is **12.40 h**, under 24 h. Per the user's instruction, 4B was **not trained**. That decision is the user's.

## Binaries

- `target/release/vox --version`: `vox 0.6.0+build.7447 (c56e13a33)`. That is the accumulation fix (`320-S3-DEBUG.md`).
- Installed Metal plugin `~/Library/Application Support/vox/plugins/mens-candle-metal/0.6.0/libvox_plugin_mens_candle_metal.dylib` and `target/release/libvox_plugin_mens_candle_metal.dylib`: `diff -q` identical, sha256 `cdaf90fabe585607713e856a458ec228a427739b16552b01e20e7e0b6015bb17` for both. That is the c56e13a33 plugin recorded in `320-S3-DEBUG.md`. No rebuild was needed.
- `vox` forwarded to `target/release/vox-ml-cli` built at `3ec31a00f` (321-02). The launcher warned that the build does not match `vox` (`evidence/s3r-probe-4B-log.txt`). The training loop runs in the plugin, which is c56e13a33. The ml-cli build is later than c56e13a33 and carries 320-08's 4B estimate floor of 56,000.

## Command and settings

```text
target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu \
  --model "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c" --data-dir target/dogfood \
  --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --force-restart \
  --output-dir mens/runs/320-probe-4B-postfix --max-runtime-secs 1400
```

- Preset `qwen3_12g`: batch 1, accum 8 (`train_start` telemetry). There was no `--grad-accum` override.
- **Live free memory:** a fresh `--plan-only --json` just before launch read 61,850 MiB from the Metal working set (`evidence/s3r-plan-4B.json`). It passed the 56,000 estimate floor. `VOX_MENS_FORCE_TRAIN` was **not** set, and neither the floor check nor the estimate refused.
- **Cap.** The run cap stops at the first optimizer boundary after it expires. With seed 42, row 33 is the first 3,071-token row. A cap that expires during rows 33-40 stops at row 40. 1,400 s was sized from the old ~38 s/row, with margin on the slow side: any rate up to ~43 s/row still reaches row 33. A faster rate would only stop at a later boundary.
- **Watchdog.** A bash loop read `telemetry.jsonl` every 3 s and would send SIGINT to the `vox-ml-cli` PID directly (the launcher does not forward signals), then SIGKILL 120 s later, on any of:
  - the largest `peak_metal_allocated_bytes` above 60,559,458,304 B ((61,850 − 4,096) MiB)
  - a row taking more than 5× the previous row
  - no row for more than 5× the last interval (and over 60 s) before `run cap reached`
  - a 2,400 s deadline

  Before launch, its kill path was tested on the old 4B telemetry against a dummy process. It never fired (`evidence/s3r-watchdog-4B.txt`: `killed_at=0; last peak=34640986112 n=40`).

## Result

- Exit 0 (`evidence/s3r-probe-4B.exit`). `Training started: first backward pass done after 50.2s (139 optimizer steps planned, grad_accum=8)`. `run cap reached after 1531 s at optimizer step 5; finalizing`. The estimate line read `VRAM: est. ~61637 MB`.
- Record (`evidence/s3r-record-4B.json`):
  - status `capped`, pin `Qwen/Qwen3-4B@1cfa9a72…`, preset `qwen3_12g`
  - 5 optimizer / 40 micro steps, `train_wall_secs` 1,531.5
  - `peak_mb` **33,037** (`run_summary`, `peak_metal_allocated_bytes` 34,640,986,112)
  - `final_loss_ema` 1.776, truncated / prompt-trimmed rows 1 / 1
- **Per-row peaks** (`evidence/s3r-rows-4B.txt`, from `evidence/s3r-telemetry-4B.jsonl`):

  | rows | tokens | peak |
  |---|---|---|
  | 1-4 | 2,277-2,333 | 33.77 GB (32,207 MiB) |
  | 5-8 | 2,295-2,499 | 33.94 GB (32,369 MiB) |
  | 9-12 | 2,280-2,455 | 34.25 GB (32,664 MiB) |
  | 13-32 | 2,275-2,508 | 34.38 GB (32,791 MiB) |
  | **33** | **3,071** (first full-length row) | **34.64 GB (33,037 MiB)** |
  | 34-40 | 2,324-2,730 | 34.64 GB (33,037 MiB) |

  Row 2 does not jump. The pre-fix probe went 59.70 → 86.33 GB at row 2 and was killed. The peak is flat inside each optimizer step, and the full row adds 0.26 GB.
- **Full row seen:** yes, row 33 (3,071 tokens). The peak is not a lower bound, so no ×1.12 scaling applies.
- **s/row:** 1,531.5 s / 40 = **38.29 s**. Rows ran 28.6-54.9 s (row 1: 50.2 s, including warm-up).
- **Projected full epoch:** 38.29 × 1,166 rows = 44,644 s = **12.40 h**. The trainer plans 1,108 micro-steps for those rows, which gives 11.8 h. Both are under 24 h, so a full run would need no time cap under D-07.
- **Estimate vs measured (post-fix).**

  | size | estimate MiB | measured MiB | measured / estimate |
  |---|---|---|---|
  | 0.6B | 20,725 | 15,319 | 0.74 |
  | 1.7B | 38,056 | 27,007 | 0.71 |
  | 4B | 61,637 | 33,037 | 0.54 |

  The estimate comes from `estimate_training_vram_mb_qlora`, checkpointed, seq 3072. Its over-prediction grows with size.

| size | preset | accum | verdict | forced | free MB (source) | peak MB (source) | full row seen | s/row | projected full epoch (h) |
|---|---|---|---|---|---|---|---|---|---|
| 4B | qwen3_12g | 8 | fits | no | 61,850 (Metal working set) | 33,037 (run_summary) | yes, row 33 (3,071 tok) | 38.29 | 12.40 |

## Not done (user decision)

- 4B was **not** trained. The probe run dir `mens/runs/320-probe-4B-postfix` holds the capped 40-row adapter. It is gitignored and is not a ladder adapter. `mens/runs/qwen3-4b` keeps its `not_trained` record.
- 8B and above were not probed. Their floors are the Metal QLoRA estimate (122,876 / 195,382 / 295,822 MiB), all above this host's live free memory.

## Floor written back (D-08)

The 4B QLoRA rung in `mens/config/gpu-specs.yaml` is now `floor_mb: 33037, floor_measured: true, measured_on: silicon-gpu-metal, peak_mb: 33037`. A full-length row was seen, so the measurement covers the worst case the run can hit at seq 3072.
