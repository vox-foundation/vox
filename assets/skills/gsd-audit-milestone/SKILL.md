---
name: gsd-audit-milestone
description: "Audit milestone completion against original architectural intent and requirements before archiving."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Milestone Audit"
---

# GSD Audit Milestone

> **Workflow Stage:** Milestone Audit  
> **CLI:** `vox gsd status`  
> **MCP:** `vox_gsd_progress`

## Objective

Audit milestone completion against original architectural intent and requirements before archiving.

## Artifacts Managed

- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`

## Execution Protocol

1. Cross-check every requirement ID in the milestone against implemented code and recorded verification output.
2. Flag any unverified requirement, drift, or broken SSOT parity.
3. Block archiving until gaps are resolved or explicitly deferred by the operator.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-complete-milestone`
