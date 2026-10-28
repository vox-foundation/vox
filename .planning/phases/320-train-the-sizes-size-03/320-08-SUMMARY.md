---
phase: 320-train-the-sizes-size-03
plan: 08
subsystem: mens-training
tags: [qlora, gpu-specs, size-ladder, floor-measured]

requires:
  - phase: 320-07
    provides: S4 train records and ladder evidence for 0.6B/1.7B peaks
provides:
  - Measured QLoRA floors for 0.6B/1.7B in gpu-specs.yaml
  - Raised QLoRA estimate floors for 4B/8B/14B/32B (user option 1)
  - MEASURED_QLORA_PINS contract and floor-changes.json for Plan 09
affects: [320-09]

actuals:
  tokens: 85000
  tasks: 3
  commits: 4
plan_head_before: 12434f92e6704648caf32bd9bac7ecae92312407
plan_head_after: 56d6996b0

tech-stack:
  added: []
  patterns: [exact measured-rung YAML contract, ladder-derived test free-MB margins]

key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/evidence/floor-changes.json
  modified:
    - mens/config/gpu-specs.yaml
    - crates/vox-populi/tests/training_presets_yaml_contract.rs
    - crates/vox-populi/src/mens/tensor/size_select.rs
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
    - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
    - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs

key-decisions:
  - "Only 0.6B and 1.7B are floor_measured; 4B/8B/14B/32B QLoRA floors raised as estimates (56000/62000/70000/85000) per user option 1"
  - "LoRA rungs (44000/100000) unchanged"

requirements-completed: [SIZE-03]

duration: 45min
completed: 2026-10-28
status: complete
---

# Phase 320 Plan 08: Measured QLoRA floors Summary

**Metal-measured 0.6B/1.7B QLoRA peaks and raised estimate floors for larger ladder rungs, with contract and floor tests aligned.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 3/3
- **Commits:** 4 (`8af223536`, `9aa3a18ab`, `bcf70c99f`, `56d6996b0`)

## M8 mutation proof

| Step | Value |
|------|-------|
| `floor_measured: true` count before (K) | 2 |
| After mutating 0.6B to `floor_measured: false` (K-1) | 1 |
| `gpu_specs_qwen3_code_ladder_pins_all_six_dense_sizes` | FAIL |
| Restored from backup | match |
| Count after restore | 2 |
| Contract target after restore | PASS |

## floor-changes.json (high level)

- **Measured:** 0.6B → 23675 MB; 1.7B → 54583 MB (`silicon-gpu-metal`)
- **Raised estimates (QLoRA only):** 4B 56000, 8B 62000, 14B 70000, 32B 85000
- **changed_picks (agentic_default):** 13926, 20890, 27853, 31334 → Qwen2.5-Coder-7B; 83558 → Qwen3-14B QLoRA

## Test results

- `training_presets_yaml_contract`: 5/5 passed
- `size_select` + `spoke_base_resolver` (lib filter): 55/55 passed
- vox-ml-cli filtered set: 75/75 passed

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Policy] Phase 320 raised estimates exempt from 2.0x–4.0x bare-weight band**
- **Found during:** Task 1
- **Issue:** User-option floors for 8B+ exceed the nominal 2–4× band
- **Fix:** Skip band check when `floor_mb > 54_583` on unmeasured Qwen3 QLoRA rungs
- **Files modified:** `training_presets_yaml_contract.rs`

**2. [Rule 1 - Test] agentic_default 83558 tier**
- **Found during:** Task 2
- **Issue:** 14B QLoRA at 70000 beats 14B LoRA at 44000 at 83558 MB
- **Fix:** Expect Qwen3-14B qlora at that tier
- **Commit:** 9aa3a18ab

## Self-Check: PASSED
