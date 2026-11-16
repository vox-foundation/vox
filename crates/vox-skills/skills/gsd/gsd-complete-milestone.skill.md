---
name: gsd-complete-milestone
description: "Archive a verified milestone cleanly into `.planning/milestones/`, update `PROJECT.md`, and prepare the next cycle."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Milestone Completion"
---

# GSD Complete Milestone

> **Workflow Stage:** Milestone Completion  
> **CLI:** `vox gsd complete-milestone <version>`  
> **MCP:** `vox_gsd_complete_milestone`

## Objective

Archive a verified milestone cleanly into `.planning/milestones/`, update `PROJECT.md`, and prepare the next cycle.

## Artifacts Managed

- `.planning/PROJECT.md`
- `.planning/STATE.md`

## Execution Protocol

1. Confirm all phases in the milestone pass `VERIFICATION.md` and `REQUIREMENTS.md` criteria.
2. Snapshot milestone summary and requirement outcomes into `.planning/milestones/<version>-archive.md`.
3. Mark milestone `completed` in VoxDb via `vox_gsd_complete_milestone`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-new-milestone`
