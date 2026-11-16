---
name: gsd-debug
description: "Systematic root-cause debugging with persistent hypothesis tracking across context resets."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Diagnostics"
---

# GSD Debug

> **Workflow Stage:** Diagnostics  
> **CLI:** `vox gsd health`  
> **MCP:** `vox_gsd_store_document`

## Objective

Systematic root-cause debugging with persistent hypothesis tracking across context resets.

## Artifacts Managed

- `.planning/phases/<phase>/DEBUG.md`

## Execution Protocol

1. Reproduce the exact failure deterministically and record observed output in `DEBUG.md`.
2. Form one falsifiable hypothesis at a time; inspect code and state directly.
3. Write a regression test capturing the root bug before applying the minimal fix.
4. Verify full recovery and update `DEBUG.md` resolution notes.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-verify-work`
