---
name: gsd-ship
description: "Run final GSD quality gates, verify documentation/SSOT sync, and prepare a phase or milestone branch for merge."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Delivery"
---

# GSD Ship

> **Workflow Stage:** Delivery  
> **CLI:** `vox gsd complete-milestone`  
> **MCP:** `vox_gsd_complete_milestone`

## Objective

Run final GSD quality gates, verify documentation/SSOT sync, and prepare a phase or milestone branch for merge.

## Artifacts Managed

- `.planning/VERIFICATION.md`
- `.planning/STATE.md`

## Execution Protocol

1. Confirm zero failing tests, clean `vox audit arch` / `vox audit code`, and SSOT parity.
2. Confirm all `REQUIREMENTS.md` boxes for shipped scope are checked with evidence.
3. Prepare clean release summary for human merge approval.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-complete-milestone`
