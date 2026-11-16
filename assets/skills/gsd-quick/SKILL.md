---
name: gsd-quick
description: "Execute a bounded single task with GSD state tracking and atomic verification while skipping full multi-phase scaffolding."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Fast Execution"
---

# GSD Quick

> **Workflow Stage:** Fast Execution  
> **CLI:** `vox gsd progress`  
> **MCP:** `vox_gsd_store_document`

## Objective

Execute a bounded single task with GSD state tracking and atomic verification while skipping full multi-phase scaffolding.

## Artifacts Managed

- `.planning/quick/`

## Execution Protocol

1. Confirm scope fits a single cohesive slice with clear pass/fail verification.
2. Log the quick task summary in `.planning/STATE.md` / VoxDb.
3. Implement cleanly, run scoped tests/lints, and record verified outcome.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-progress`
