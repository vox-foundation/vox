---
phase: 320-train-the-sizes-size-03
plan: 07
subsystem: mens-training
tags: [mens, qlora, metal, s4, size-03]
status: complete
requires: [320-06]
provides:
  - "mens/runs/qwen3-0.6b and qwen3-1.7b full-epoch trained adapters"
  - "not_trained records for 4B/8B/14B/32B"
  - "320-S4-RUNS.md; Stage S4: PASS"
  - "evidence/s4-record-* and s4-loss-{0.6B,1.7B}.json"
affects: [320-08, 320-09, 321]
tech-stack:
  added: []
  patterns: []
key-files:
  created:
    - mens/runs/qwen3-0.6b/
    - mens/runs/qwen3-1.7b/
    - .planning/phases/320-train-the-sizes-size-03/320-S4-RUNS.md
    - .planning/phases/320-train-the-sizes-size-03/evidence/s4-*
  modified: []
decisions:
  - "Ran uncapped one-epoch S4 on both fitting sizes after Metal pool fix c56e13a33"
metrics:
  duration: "~7h wall (0.6B ~2.8h + 1.7B ~4.2h)"
  completed: 2026-10-28
---

# Phase 320 Plan 07: S4 training runs Summary

**Two sizes fully trained on this Mac:** Qwen3-0.6B and Qwen3-1.7B. 4B–32B remain `not_trained`.

| size | status | steps | final_loss_ema | peak MB | wall |
|---|---|---|---|---|---|
| 0.6B | trained | 1108 | 0.690 | 15,319 | 2.82 h |
| 1.7B | trained | 138 opt / 1108 micro | 0.617 | 27,007 | 4.17 h |

Stage S4: PASS. Next: Plan 08 measured floors in `gpu-specs.yaml`.
