---
phase: 06-model-routing-transparency-ml-dependency-health
plan: 04
subsystem: docs
tags: [adr-034, closure, ml-health, model-01, ml-01]
requires: [06-01, 06-03]
provides:
  - ADR-034 accepted against measured resolved versions and hosted CUDA compile evidence
  - Runner contract documents the fail-closed hosted CUDA lane and its coverage limit
  - MODEL-01 and ML-01 closed in planning
affects: []
completed: 2026-10-01
status: complete
---

# Phase 6 Plan 04 Summary

## Accomplishments

- ADR-034 rewritten as an accepted closure record: Candle 0.10.2, peft-rs 1.0.3, qlora-rs 1.0.5 were already unified, so no speculative bump; cites the `ml_dependency_health` test, the repaired `vox-plugin-speech` / `vox-ml-cli` targets, and the successful hosted run.
- `docs/src/ci/runner-contract.md`: canonical CUDA package names corrected; `ml-cuda-health` (CUDA 12.6.3 toolkit, `CUDA_COMPUTE_CAP=80`) documented as fail-closed, compile-only evidence distinct from local skip behavior.
- `workspace-dependency-audit-2026.md` Candle/peft/qlora entry updated to resolved reality.
- Intel: ADR-034 classification and decisions entry locked with acceptance evidence. ADR-046 left unchanged.
- REQUIREMENTS/ROADMAP: MODEL-01 and ML-01 complete; Phase 5 progress row corrected to 7/7 (its execution commits exist; the row was stale).

## Evidence

- Hosted run: https://github.com/vox-foundation/vox/actions/runs/36821415478 (job 110237662285), head `bc3214504f981e31817da3d9b4dbeae1b6cb0892`, conclusion success. Log shows `Cuda compilation tools, release 12.6, V12.6.85`, `candle-kernels` compiled, `CUDA feature checks OK (vox-plugin-speech, vox-ml-cli)`, and `vox-ml-cli CUDA release build OK`.
- Compile-only: no CUDA device or runtime kernels were exercised.

## Deviations

- First dispatched run (36820872174) failed installing `cuda-12-4`, which has no Ubuntu 24.04 apt package; fixed by pinning the 12.6.3 toolkit sub-package (commit `bc3214504`).
- Observation, out of ML-01 scope: the CUDA graph resolves two `cudarc` versions (0.19.7 and 0.17.8).
