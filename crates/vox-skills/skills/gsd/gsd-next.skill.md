---
name: gsd-next
description: "Smart entry — inspect project state and automatically route to the highest-priority next GSD workflow."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Navigation & Status"
---

# GSD Next

> **Workflow Stage:** Navigation & Status  
> **CLI:** `vox gsd progress`  
> **MCP:** `vox_gsd_status`

## Objective

Smart entry — inspect project state and automatically route to the highest-priority next GSD workflow.

## Artifacts Managed

- `.planning/STATE.md`

## Execution Protocol

1. Query `vox_gsd_status` for active milestone, phase state, and document completeness.
2. Select `/gsd-discuss-phase`, `/gsd-plan-phase`, `/gsd-execute-phase`, `/gsd-verify-work`, or `/gsd-complete-milestone` based on missing artifacts.
3. Execute or present the recommended step.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** Selected GSD workflow
