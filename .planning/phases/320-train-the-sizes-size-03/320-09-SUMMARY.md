---
phase: 320-train-the-sizes-size-03
plan: 09
subsystem: mens-training
tags: [size-ladder, SIZE-03, qlora, evidence]

requires:
  - phase: 320-08
    provides: floor-changes.json and measured/estimate floors in gpu-specs
provides:
  - SIZE-03 check against run directories (size-03-check.txt)
  - Final ladder.json with s4_status and floor fields
  - 320-LADDER-REPORT.md for Phase 321 and operators
affects: [321]

actuals:
  tokens: 28000
  tasks: 2
  commits: 2
plan_head_before: 9519a565980badfcf77df677be55392b7f6c7ce7
plan_head_after: abdf7d8e0

tech-stack:
  added: []
  patterns: [report-only closeout P-320-16, run-dir-backed SIZE-03 gate]

key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/evidence/size-03-check.txt
    - .planning/phases/320-train-the-sizes-size-03/320-LADDER-REPORT.md
  modified:
    - .planning/phases/320-train-the-sizes-size-03/evidence/ladder.json

key-decisions:
  - "SIZE-03 proven from mens/runs/qwen3-* and stage tail lines, not prose alone (T-320-32)"

requirements-completed: [SIZE-03]

duration: 20min
completed: 2026-10-28
status: complete
---

# Phase 320 Plan 09: SIZE-03 closeout Summary

**Single SIZE-03 check and ladder report: 0.6B/1.7B trained with pinned adapters; 4B–32B not_trained or not attempted; phase gates S0–S4 PASS.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2/2
- **Commits:** 2 (`6c254a889`, `abdf7d8e0`)

## SIZE-03

**Overall:** `SIZE-03: PASS` in `evidence/size-03-check.txt`.

| Size | Result |
|------|--------|
| 0.6B | PASS trained (adapter + pin) |
| 1.7B | PASS trained (adapter + pin) |
| 4B | PASS not_trained (peak_mb=84541) |
| 8B/14B/32B | PASS not_trained (not attempted) |
| Stages S0–S4 | PASS |

## Deviations from Plan

None — plan executed as written (report-only; P-320-16 satisfied).

## Self-Check: PASSED

- `evidence/size-03-check.txt` — FOUND
- `320-LADDER-REPORT.md` — FOUND
- `evidence/ladder.json` (s4 + floor fields) — FOUND
- Commits `6c254a889`, `abdf7d8e0` — FOUND on HEAD ancestry
