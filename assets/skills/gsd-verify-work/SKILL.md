---
name: gsd-verify-work
description: "Validate built features against phase requirements and UAT criteria before marking a phase complete."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Verification"
---

# GSD Verify Work

> **Workflow Stage:** Phase Verification  
> **CLI:** `vox gsd verify-work <phase>`  
> **MCP:** `vox_gsd_verify_work`

## Objective

Validate built features against phase requirements and UAT criteria before marking a phase complete.

## Artifacts Managed

- `.planning/phases/<phase>/VERIFICATION.md`
- `.planning/REQUIREMENTS.md`

## Execution Protocol

1. Re-run unit, integration, guard, and UI/E2E tests covering every requirement ID in `<phase>`.
2. Check requirements coverage row-by-row with file paths and command output evidence.
3. Record pass/fail findings in `VERIFICATION.md` via `vox_gsd_verify_work`.
4. Update requirement checkboxes in `.planning/REQUIREMENTS.md` and phase completion in `ROADMAP.md`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-progress` or `/gsd-complete-milestone`
