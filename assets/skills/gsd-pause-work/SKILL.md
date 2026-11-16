---
name: gsd-pause-work
description: "Create an accurate handoff state in `.planning/STATE.md` and `state.json` when pausing mid-phase."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Session Handoff"
---

# GSD Pause Work

> **Workflow Stage:** Session Handoff  
> **CLI:** `vox gsd progress`  
> **MCP:** `vox_gsd_progress`

## Objective

Create an accurate handoff state in `.planning/STATE.md` and `state.json` when pausing mid-phase.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/state.json`

## Execution Protocol

1. Record completed files, pending tasks, active blockers, and exact next command in `.planning/STATE.md`.
2. Snapshot modified planning documents to VoxDb.
3. Verify `.planning/state.json` reflects the exact active phase and pause reason.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-resume-work`
