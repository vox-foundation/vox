---
phase: 321-harness-before-after-harn-01
plan: 02
subsystem: mens-benchmarking
tags: [metal, reasoning-compare, eval-local, checkpoint, baseline-from-compare]

requires:
  - phase: 321-harness-before-after-harn-01
    provides: ladder trained adapters and plan context
provides:
  - Compared reasoning JSON for Qwen3-0.6B and Qwen3-1.7B
  - Dual heldout_bench eval-local JSON per size (52 results each)
  - collateral_baseline.json per size (six reasoning keys)
  - checkpoint-scoring.json durable leg metadata
affects: [321-03 collateral wave, harness before/after report]

actuals:
  tokens: 88000
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Sequential Metal legs with checkpoint-scoring.json skip-if-complete"
    - "baseline-from-compare CLI for collateral pre-score"

key-files:
  created:
    - crates/vox-ml-cli/src/commands/mens/baseline_from_compare.rs
    - .planning/phases/321-harness-before-after-harn-01/evidence/checkpoint-scoring.json
  modified:
    - scripts/mens-ladder-before-after.vox
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
    - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs

key-decisions:
  - "Heldout pass@1 uses results length 52; top-level n is samples-per-item (1)"
  - "Checkpoint schema 321_checkpoint_scoring.v1 merged with script leg records"

patterns-established:
  - "One vox mens GPU subprocess at a time; progress logs every ~180s in leg logs"

requirements-completed: [BENCH-04]

coverage:
  - id: D1
    description: "0.6B and 1.7B reasoning before/after compared on three 32-item slices"
    requirement: BENCH-04
    verification:
      - kind: other
        ref: "jq .status on mens/runs/reasoning_compare/Qwen3-{0.6B,1.7B}-*.json"
        status: pass
    human_judgment: false
  - id: D2
    description: "Per-size heldout_bench eval-local base and adapter with full bench coverage"
    requirement: BENCH-04
    verification:
      - kind: other
        ref: "jq '.results|length' on mens/runs/qwen3-*/evidence/heldout_*.json"
        status: pass
    human_judgment: false
  - id: D3
    description: "Collateral baseline JSON with six reasoning keys from compare base side"
    requirement: BENCH-04
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib baseline_from_compare"
        status: pass
    human_judgment: false

duration: 960min
completed: 2026-10-29
status: complete
plan_head_before: 23cf6e227e84cf8348c6189ec6c0905c786c6001
plan_head_after: e2297d5ea
commits: 4
---

# Phase 321 Plan 02: Metal scoring wave Summary

**Real Metal before/after scores for 0.6B and 1.7B: compared reasoning on three slices, heldout_bench pass@1 on base and adapter (52 items), and collateral baselines derived from compare JSON.**

## Performance

- **Duration:** ~16h wall (dominated by Metal inference; checkpoint wall_secs below)
- **0.6B reasoning_compare wall_secs:** 20177 (~5.6h)
- **1.7B reasoning_compare wall_secs:** 30569 (~8.5h)
- **0.6B heldout_base / adapter:** 908s / 875s
- **1.7B heldout_base / adapter:** 1366s / ~1260s (adapter finished 14:51–15:09 UTC)

## Accomplishments

- Extended `scripts/mens-ladder-before-after.vox` with sequential legs, timeout wrapper, checkpoint resume, and `--size` filter.
- Added `baseline_json_from_compare` + `vox mens baseline-from-compare` for collateral pre-score shape.
- Produced compared reports and heldout evidence under `mens/runs/` for both trained sizes.

## Artifact paths

| Artifact | Path |
|----------|------|
| 0.6B compare | `mens/runs/reasoning_compare/Qwen3-0.6B-c1899de289a0.json` |
| 1.7B compare | `mens/runs/reasoning_compare/Qwen3-1.7B-70d244cc86cc.json` |
| 0.6B heldout | `mens/runs/qwen3-0.6b/evidence/heldout_{base,adapter}.json` |
| 1.7B heldout | `mens/runs/qwen3-1.7b/evidence/heldout_{base,adapter}.json` |
| Baselines | `mens/runs/qwen3-{0.6b,1.7b}/evidence/collateral_baseline.json` |
| Checkpoint | `.planning/phases/321-harness-before-after-harn-01/evidence/checkpoint-scoring.json` |
| Leg logs | `.planning/phases/321-harness-before-after-harn-01/evidence/score-*-*.log` |

## Commits

| SHA | Message |
|-----|---------|
| 7f36ee727 | feat(321-02): baseline JSON from reasoning compare report |
| 3ec31a00f | feat(321-02): Metal scoring orchestration in ladder script |
| 730e83dc3 | feat(321-02): complete 0.6B Metal scoring checkpoint |
| e2297d5ea | feat(321-02): complete 1.7B Metal scoring checkpoint |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Native `vox run` script lane compile failure**
- **Found during:** Task 2 script invocation for 1.7B
- **Issue:** `vox run scripts/mens-ladder-before-after.vox` (Auto/native) failed in `vox-script` json render types; heldout_adapter for 1.7B was already complete via direct `eval-local`.
- **Fix:** Completed 1.7B collateral baseline via direct `vox mens baseline-from-compare`; checkpoint updated manually. Use `vox run --interp` for orchestration until native lane is repaired.
- **Files modified:** checkpoint only

**2. [Rule 2 - Critical] Duplicate Metal load prevention**
- **Found during:** Task 1 tracer
- **Issue:** Overlapping compare/heldout subprocesses from parallel automation.
- **Fix:** Stopped duplicate probe/script PIDs; kept single `vox-ml-cli` GPU child per leg.

## Self-Check: PASSED

- Compared JSON: both sizes `status=compared`
- Heldout adapter: `.results|length` = 52 both sizes
- `collateral_baseline.json`: present both sizes with six keys
- Commits 7f36ee727, 3ec31a00f, 730e83dc3, e2297d5ea on branch `gsd/mens-compare-316`
