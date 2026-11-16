---
name: gsd-extract-learnings
description: "Extract architectural decisions, failure modes, and workflow episodes from completed GSD phases for docs and MENS."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Knowledge & MENS"
---

# GSD Extract Learnings

> **Workflow Stage:** Knowledge & MENS  
> **CLI:** `vox corpus extract-gsd`  
> **MCP:** `vox_gsd_list_documents`

## Objective

Extract architectural decisions, failure modes, and workflow episodes from completed GSD phases for docs and MENS.

## Artifacts Managed

- `docs/src/architecture/`
- `mens/data/`

## Execution Protocol

1. Read completed `PLAN.md`, `SUMMARY.md`, and `VERIFICATION.md` artifacts.
2. Distill durable lessons into canonical architecture SSOT docs under `docs/src/`.
3. Run `vox corpus extract-gsd` so GSD workflow trajectories feed MENS training mixes.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-complete-milestone`
