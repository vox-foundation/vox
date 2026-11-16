---
name: gsd-audit-fix
description: "Autonomous audit-to-fix loop — locate verification defects, classify root causes, repair, re-test, and log."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Remediation"
---

# GSD Audit Fix

> **Workflow Stage:** Remediation  
> **CLI:** `vox audit code`  
> **MCP:** `vox_gsd_verify_work`

## Objective

Autonomous audit-to-fix loop — locate verification defects, classify root causes, repair, re-test, and log.

## Artifacts Managed

- `.planning/phases/<phase>/VERIFICATION.md`

## Execution Protocol

1. Gather concrete failures from `vox audit code`, `vox audit arch`, and failing tests.
2. Isolate minimal fixes scoped to affected crates without relaxing gates or test assertions.
3. Re-run verification and update `VERIFICATION.md` with fresh passing output.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-verify-work`
