---
name: gsd-resume-work
description: "Restore full GSD context from `.planning/STATE.md`, `ROADMAP.md`, and VoxDb before continuing execution."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Session Handoff"
---

# GSD Resume Work

> **Workflow Stage:** Session Handoff  
> **CLI:** `vox gsd status`  
> **MCP:** `vox_gsd_status`

## Objective

Restore full GSD context from `.planning/STATE.md`, `ROADMAP.md`, and VoxDb before continuing execution.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/ROADMAP.md`

## Execution Protocol

1. Read `.planning/STATE.md`, `state.json`, and active phase `PLAN.md`/`SUMMARY.md`.
2. Verify current workspace files and test status before issuing new edits.
3. Continue execution cleanly from the next unverified step.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-execute-phase <N>`
