---
name: gsd-mvp-phase
description: "Plan a phase as a vertical MVP slice — user story, SPIDR splitting, then plan-phase."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Planning"
---

# GSD MVP Phase

> **Workflow Stage:** Phase Planning  
> **CLI:** `vox gsd plan-phase <phase> --mvp`  
> **MCP:** `vox_gsd_plan_phase`

## Objective

Plan a phase as a vertical MVP slice — user story, SPIDR splitting, then plan-phase.

## Artifacts Managed

- `.planning/phases/<phase>/<phase>-01-PLAN.md`

## Execution Protocol

1. Split broad requirements into the narrowest end-to-end verifiable vertical slice.
2. Keep interfaces stable while deferring non-critical polish into follow-up waves.
3. Generate wave-annotated `PLAN.md` and validate with `vox_gsd_plan_phase`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-execute-phase <N>`
