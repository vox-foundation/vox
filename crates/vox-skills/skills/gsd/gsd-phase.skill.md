---
name: gsd-phase
description: "Add, insert, reorder, or update phases cleanly in ROADMAP.md and VoxDb."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Roadmap Management"
---

# GSD Phase Manager

> **Workflow Stage:** Roadmap Management  
> **CLI:** `vox gsd status`  
> **MCP:** `vox_gsd_status`

## Objective

Add, insert, reorder, or update phases cleanly in ROADMAP.md and VoxDb.

## Artifacts Managed

- `.planning/ROADMAP.md`
- `.planning/STATE.md`

## Execution Protocol

1. Inspect existing phase numbering and dependencies in `.planning/ROADMAP.md`.
2. Maintain strictly increasing phase numbers and update requirement traceability tables.
3. Sync phase metadata with VoxDb via `vox_gsd_status`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-plan-phase <N>`
