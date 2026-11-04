---
phase: 320-train-the-sizes-size-03
plan: 07
status: complete
completed: "2026-10-28"
---

# Plan 320-07 Summary — S4 training runs

## Done

- Task 1: Full uncapped 0.6B → `mens/runs/qwen3-0.6b` (`trained`, 1108 steps, peak 15.3 GB, ~2.8 h). Task-1 verify PASS.
- Task 2: Full uncapped 1.7B → `mens/runs/qwen3-1.7b` (`trained`, 138 opt / 1108 micro, peak 27.0 GB, ~4.2 h). Loss quarters fall. Only fitting sizes after 0.6B.
- Task 3: `not_trained` records for 4B (measured S3 peak), 8B/14B/32B (D-06 not attempted); `320-S4-RUNS.md` ends in `Stage S4: PASS`.

## Key artifacts

- `.planning/phases/320-train-the-sizes-size-03/320-S4-RUNS.md`
- `evidence/s4-record-{0.6B,1.7B,4B,8B,14B,32B}.json`
- `evidence/s4-loss-{0.6B,1.7B}.json`

## Note

1.7B S4 peak (27 GB) is far below the S3 probe peak (54.6 GB) after the Metal pool fix — record the S4 peak for Plan 08 floor updates.
