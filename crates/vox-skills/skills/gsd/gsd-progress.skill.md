---
name: gsd-progress
description: "Check milestone/phase completion status, active documents, and route cleanly to the next GSD step."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Navigation & Status"
---

# GSD Progress

> **Workflow Stage:** Navigation & Status  
> **CLI:** `vox gsd progress`  
> **MCP:** `vox_gsd_progress`

## Objective

Check milestone/phase completion status, active documents, and route cleanly to the next GSD step.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/state.json`

## Execution Protocol

1. Inspect `.planning/STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`, and VoxDb GSD state via `vox_gsd_progress`.
2. Report completed vs pending phases, requirement counts, and CAS-backed document status.
3. Recommend the exact next GSD command for the active phase.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-next`
