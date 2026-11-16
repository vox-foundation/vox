---
name: gsd-capture
description: "Capture engineering ideas, follow-up tasks, bugs, and architectural seeds directly into `.planning/`."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Backlog & Seeds"
---

# GSD Capture

> **Workflow Stage:** Backlog & Seeds  
> **CLI:** `vox gsd docs`  
> **MCP:** `vox_gsd_store_document`

## Objective

Capture engineering ideas, follow-up tasks, bugs, and architectural seeds directly into `.planning/`.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/seeds/`

## Execution Protocol

1. Classify captured item as milestone requirement, phase backlog item, or quick task.
2. Write structured note with origin file references into `.planning/`.
3. Persist via `vox_gsd_store_document` for searchability in GUI and MENS.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-progress`
