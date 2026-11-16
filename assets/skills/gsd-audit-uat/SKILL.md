---
name: gsd-audit-uat
description: "Cross-phase audit of all outstanding UAT criteria, E2E suites, and verification artifacts."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Verification Audit"
---

# GSD Audit UAT

> **Workflow Stage:** Verification Audit  
> **CLI:** `vox gsd verify-work <phase>`  
> **MCP:** `vox_gsd_verify_work`

## Objective

Cross-phase audit of all outstanding UAT criteria, E2E suites, and verification artifacts.

## Artifacts Managed

- `.planning/REQUIREMENTS.md`

## Execution Protocol

1. Collect all `VERIFICATION.md` docs across active milestone phases via `vox_gsd_list_documents`.
2. Identify pending manual/E2E checks or stale verification timestamps.
3. Run or schedule the remaining checks to close UAT.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-audit-fix`
