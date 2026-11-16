---
name: gsd-fast
description: "Execute a trivial deterministic edit inline with direct verification and minimal ceremony."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Fast Execution"
---

# GSD Fast

> **Workflow Stage:** Fast Execution  
> **CLI:** `vox gsd status`  
> **MCP:** `vox_gsd_status`

## Objective

Execute a trivial deterministic edit inline with direct verification and minimal ceremony.

## Artifacts Managed

- `.planning/STATE.md`

## Execution Protocol

1. Verify the change touches only 1-2 unambiguous files with zero architectural risk.
2. Apply the change directly and run the corresponding check immediately.
3. Report exact file paths and verification output.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-progress`
