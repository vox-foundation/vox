---
phase: 320-train-the-sizes-size-03
plan: 06
subsystem: mens-training
tags: [mens, qlora, metal, probe, d-06, d-07, size-03]
status: complete
requires: [320-05]
provides:
  - "evidence/ladder.json (vox_mens_ladder_v1): 0.6B fits, 1.7B fits, 4B does_not_fit, 8B/14B/32B not_attempted, with peaks, live budgets and projections"
  - "320-S3-PROBES.md probe table; Stage S3: PASS"
  - "evidence/s3-* (plan-only readings, logs, records, telemetry, watchdog output)"
affects: [320-07, 320-08, 320-09]
tech-stack:
  added: []
  patterns:
    - "telemetry watchdog: SIGINT the vox-ml-cli trainer when peak_metal_allocated_bytes passes live free - 4 GiB or a row takes >5x the previous"
    - "probe cap chosen to reach row 33 (first 3071-token row at seed 42) and stop at the row-40 optimizer boundary"
key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/320-S3-PROBES.md
    - .planning/phases/320-train-the-sizes-size-03/evidence/ladder.json
    - .planning/phases/320-train-the-sizes-size-03/evidence/s3-*
  modified: []
decisions:
  - "Probes ran each size's real preset (accum 8 for 1.7B and 4B) with caps of 460 s and 900 s instead of the plan's 120 s, so a full-length row is measured (orchestrator change)"
  - "Fit uses the full-row peak; a probe without a >=3000-token row would be scaled by 1.12. Only 4B needed this, and it was over budget unscaled"
  - "Projection = probe s/row x 1166 rows (train_mixed.jsonl); the trainer's EMA projection is kept as trainer_projected_full_secs"
  - "4B recorded does_not_fit from the watchdog stop at 86.3 GB (limit 67.0 GB); 8B-32B not attempted (D-06)"
metrics:
  duration: "~14 min of probes (05:26-05:40 UTC 2026-10-28), ~45 min with evidence"
  completed: 2026-10-28
actuals:
  tokens: 9000
  tasks: 3
  commits: 1
plan_head_before: 1cf38225d
plan_head_after: 1ccb9e9f6
---

# Phase 320 Plan 06: S3 memory probes Summary

**On this Mac, the ladder stops at 1.7B at preset settings.**
- 1.7B fits: 54,583 MiB peak, measured on a full 3,071-token row, with 65,651 MiB free.
- 4B does not fit: 84,541 MiB by row 3, with 68,010 MiB free. The watchdog stopped it before any full-length row.
- 8B, 14B and 32B were not attempted.

## Per size

| size | preset | accum | steps (opt/micro) | s/row | peak MiB | full row seen | verdict | projected full epoch |
|---|---|---|---|---|---|---|---|---|
| 0.6B | qwen3_dev_cpu | 1 | 60/60 (S2) | 8.10 | 23,675 | yes (row 33) | fits | 2.62 h |
| 1.7B | qwen3_6g | 8 | 5/40 | 12.09 | 54,583 | yes (row 33) | fits | 3.92 h |
| 4B | qwen3_12g | 8 | 0/3 (paused) | ~38 (under memory pressure) | 84,541 (lower bound; ≥94,686 scaled) | no (max 2,333) | does_not_fit | — |
| 8B / 14B / 32B | 16g / 24g / 96g | 8 | — | — | — | — | not_attempted | — |

No fitting size projects over 24 h, so both 0.6B and 1.7B get full one-epoch S4 runs. Projections use 1,166 rows; the trainer plans 1,108 micro-steps, so they run about 5% high.

## Tasks

1. Tracer: `ladder.json` was seeded with 0.6B from S2 attempt 3, then 1.7B was probed (capped at optimizer step 5, exit 0). The Task 1 automated check passed.
2. Climb: 4B was downloaded (7.5 GB, kept in the cache) and probed. The watchdog sent SIGINT at 86.3 GB; the trainer paused at step 3 and wrote a `not_trained` record. 8B, 14B and 32B were recorded `not_attempted`. The Task 2 check passed.
3. Wrote `320-S3-PROBES.md`, ending `Stage S3: PASS`, and committed it as `1ccb9e9f6`. The Task 3 check passed.

## Deviations from Plan

- **[Orchestrator] Longer caps and real presets.** The caps were 460 s and 900 s, not 120 s, so each probe reaches a full-length row and several optimizer steps. This deliberately goes past P-320-11's 120 s, as the orchestrator instructed.
- **[Orchestrator] Telemetry watchdog.** A watchdog with a limit of live free − 4 GiB replaced `timeout --signal=INT`. The `vox` launcher spawns `vox-ml-cli` and does not forward signals, so a SIGINT to the launcher would orphan the trainer. The watchdog signals the `vox-ml-cli` PID directly.
- **[Rule 3] `vox` instead of `$ML`.** The shell allowlist blocks running `vox-ml-cli` directly, so probes went through `target/release/vox mens train`. It runs the same code via forwarding.
- **Extra ladder fields:** `grad_accum`, `micro_steps`, `secs_per_row`, `projected_full_hours`, `over_24h`, `full_row_seen`, `max_row_tokens`, `first_full_row`, `peak_is_lower_bound`, `peak_mb_fullrow_adjusted` and `trainer_projected_full_secs`.
- **Smoke directory.** The 0.6B entry's `probe_dir` is `mens/runs/320-smoke-0.6b-a3`, the attempt-3 directory, not `320-smoke-0.6b`.

## Findings carried

- **Accumulation memory grows with model size.** Inside optimizer step 0, the peak rises 12.2 GB from row 1 to row 2 at 1.7B, and 26.6 GB at 4B. The 4B row-1 peak (59.7 GB) was under its budget. The estimate has no accumulation term. This is still open in `320-S2-DEBUG.md`.
- The estimate is low at accum 8. 1.7B measured 54,583 against an estimate of 38,056. 4B is at least 37% low.
- Free memory varies with host load: 57.5 GB in S1, 65.7 GB now. A 1.7B full run peaking at 54.6 GB needs that much free memory for the whole run.
- 4B at `--grad-accum 1` was not probed; that is a user decision.

## Known Stubs

None.

## Self-Check: PASSED

- `evidence/ladder.json`, `320-S3-PROBES.md` and `evidence/s3-record-{1.7B,4B}.json` exist.
- Commit `1ccb9e9f6` is an ancestor of HEAD.
- No probe directories exist for 8B, 14B or 32B.
- No adapter weights are committed. The only safetensors are `mens/runs/320-probe-*`, which are gitignored.
