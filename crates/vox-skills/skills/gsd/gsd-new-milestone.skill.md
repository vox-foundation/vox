---
name: gsd-new-milestone
description: "Start a new milestone cycle — update PROJECT.md, scope REQUIREMENTS.md, append phases to ROADMAP.md, and reset STATE.md."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Milestone Initialization"
---

# GSD New Milestone

> **Workflow Stage:** Milestone Initialization  
> **CLI:** `vox gsd new-milestone <version> "<title>"`  
> **MCP:** `vox_gsd_new_milestone`

## Objective

Start a new milestone cycle — update PROJECT.md, scope REQUIREMENTS.md, append phases to ROADMAP.md, and reset STATE.md.

## Artifacts Managed

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/state.json`

## Execution Protocol

1. Inspect existing `.planning/PROJECT.md`, `.planning/STATE.md`, and completed milestones.
2. Use `brainstorming` if the milestone introduces unfamiliar subsystem boundaries.
3. Prepend the active milestone summary, constraints, and continuous phase numbers across `PROJECT.md`, `REQUIREMENTS.md`, and `ROADMAP.md`.
4. Sync milestone, phase, requirement, and document rows into VoxDb (`vox_gsd_new_milestone`).
5. Set `.planning/state.json` active phase to the first phase of the new milestone.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-spec-phase <N>` or `/gsd-plan-phase <N>`
