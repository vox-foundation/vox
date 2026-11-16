---
name: gsd-add-tests
description: "Generate unit, integration, and E2E tests for a completed phase based on UAT and requirement specs."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Testing"
---

# GSD Add Tests

> **Workflow Stage:** Testing  
> **CLI:** `vox gsd verify-work <phase>`  
> **MCP:** `vox_gsd_verify_work`

## Objective

Generate unit, integration, and E2E tests for a completed phase based on UAT and requirement specs.

## Artifacts Managed

- `.planning/phases/<phase>/VERIFICATION.md`

## Execution Protocol

1. Map each requirement ID in `.planning/REQUIREMENTS.md` to concrete test cases.
2. Implement deterministic unit/integration or Playwright E2E tests using real behavioral assertions.
3. Run the full scoped test suite and store passing verification results.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-verify-work`
