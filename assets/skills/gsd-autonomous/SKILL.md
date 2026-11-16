---
name: gsd-autonomous
description: "Drive remaining phases autonomously in wave order (discuss -> plan -> execute -> verify) with hard verification gates."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Autonomous Pipeline"
---

# GSD Autonomous

> **Workflow Stage:** Autonomous Pipeline  
> **CLI:** `vox gsd execute-phase <phase>`  
> **MCP:** `vox_gsd_execute_phase`

## Objective

Drive remaining phases autonomously in wave order (discuss -> plan -> execute -> verify) with hard verification gates.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/ROADMAP.md`

## Execution Protocol

1. Iterate active milestone phases sequentially from `.planning/ROADMAP.md`.
2. Ensure each phase completes `SPEC`/`CONTEXT` -> wave `PLAN` -> wave execution -> `VERIFICATION.md` before advancing.
3. Stop immediately and escalate if a verification gate fails repeatedly or requires human secret/policy authorization.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-complete-milestone`
