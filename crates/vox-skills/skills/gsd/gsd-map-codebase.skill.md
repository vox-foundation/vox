---
name: gsd-map-codebase
description: "Analyze workspace structure, crate dependencies, and hot modules to populate `.planning/codebase/` maps."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Discovery"
---

# GSD Map Codebase

> **Workflow Stage:** Discovery  
> **CLI:** `vox gsd docs`  
> **MCP:** `vox_gsd_list_documents`

## Objective

Analyze workspace structure, crate dependencies, and hot modules to populate `.planning/codebase/` maps.

## Artifacts Managed

- `.planning/codebase/ARCHITECTURE.md`
- `.planning/codebase/STACK.md`

## Execution Protocol

1. Inspect workspace manifests, `where-things-live.md`, and `layers.toml` boundaries.
2. Document subsystem entrypoints, data stores, IPC bridges, and test harnesses.
3. Store updated codebase maps via `vox_gsd_store_document`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-plan-phase`
