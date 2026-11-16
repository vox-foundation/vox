---
name: gsd-validate-phase
description: "Retroactively audit and fill test/validation coverage gaps for a completed phase."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Verification Audit"
---

# GSD Validate Phase

> **Workflow Stage:** Verification Audit  
> **CLI:** `vox gsd verify-work <phase> --audit`  
> **MCP:** `vox_gsd_verify_work`

## Objective

Retroactively audit and fill test/validation coverage gaps for a completed phase.

## Artifacts Managed

- `.planning/phases/<phase>/VERIFICATION.md`

## Execution Protocol

1. Inspect implemented artifacts against requirement claims in `REQUIREMENTS.md`.
2. Identify untested edge cases or skipped assertions and add missing deterministic tests.
3. Re-run verification and update `VERIFICATION.md`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-code-review <N>`
