---
name: gsd-plan-phase
description: "Create detailed wave-ordered PLAN.md artifacts with verification assertions and PlanNode DAG mapping."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Planning"
---

# GSD Plan Phase

> **Workflow Stage:** Phase Planning  
> **CLI:** `vox gsd plan-phase <phase>`  
> **MCP:** `vox_gsd_plan_phase`

## Objective

Create detailed wave-ordered PLAN.md artifacts with verification assertions and PlanNode DAG mapping.

## Artifacts Managed

- `.planning/phases/<phase>/<phase>-01-PLAN.md`

## Execution Protocol

1. Load `REQUIREMENTS.md`, `ROADMAP.md`, and `CONTEXT.md` for `<phase>`.
2. Decompose work into atomic, dependency-ordered waves (`wave: 1`, `wave: 2`, ...) referencing exact verified paths.
3. Ensure each task defines explicit build/test verification commands and file ownership.
4. Validate and persist waves into `vox-orchestrator` `PlanNode` DAG via `vox_gsd_plan_phase`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-execute-phase <N>`
