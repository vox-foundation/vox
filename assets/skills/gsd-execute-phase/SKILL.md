---
name: gsd-execute-phase
description: "Execute all plans in a phase with dependency-aware wave parallelization and post-wave verification."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Execution"
---

# GSD Execute Phase

> **Workflow Stage:** Phase Execution  
> **CLI:** `vox gsd execute-phase <phase>`  
> **MCP:** `vox_gsd_execute_phase`

## Objective

Execute all plans in a phase with dependency-aware wave parallelization and post-wave verification.

## Artifacts Managed

- `.planning/phases/<phase>/<phase>-01-SUMMARY.md`
- `.planning/STATE.md`

## Execution Protocol

1. Dispatch wave-1 tasks cleanly (parallelizable only when file scopes are disjoint).
2. Run scoped compile, lint, and test gates at the end of every wave before unlocking downstream waves.
3. Write concrete verification outputs into `SUMMARY.md` without fabricated claims.
4. Update phase status and progress in `.planning/STATE.md` and `vox_gsd_execute_phase`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-verify-work <N>`
