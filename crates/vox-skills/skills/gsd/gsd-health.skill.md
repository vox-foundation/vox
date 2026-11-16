---
name: gsd-health
description: "Diagnose `.planning/` directory integrity, frontmatter consistency, and VoxDb/CAS synchronization health."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Maintenance"
---

# GSD Health

> **Workflow Stage:** Maintenance  
> **CLI:** `vox gsd health`  
> **MCP:** `vox_gsd_status`

## Objective

Diagnose `.planning/` directory integrity, frontmatter consistency, and VoxDb/CAS synchronization health.

## Artifacts Managed

- `.planning/STATE.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`

## Execution Protocol

1. Run `vox gsd health` to validate `.planning/` markdown schema, milestone linkage, and CAS hash integrity.
2. Repair orphaned phase links or missing `state.json` fields.
3. Confirm `vox_gsd_status` reports clean health before proceeding.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-progress`
