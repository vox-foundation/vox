---
name: gsd-discuss-phase
description: "Gather phase implementation context and operator decisions through adaptive questioning before planning."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Discovery"
---

# GSD Discuss Phase

> **Workflow Stage:** Phase Discovery  
> **CLI:** `vox gsd discuss-phase <phase>`  
> **MCP:** `vox_gsd_discuss_phase`

## Objective

Gather phase implementation context and operator decisions through adaptive questioning before planning.

## Artifacts Managed

- `.planning/phases/<phase>/CONTEXT.md`

## Execution Protocol

1. Inspect codebase crates/files touched by the phase using `vox_search_query` or `rg`.
2. Identify gray areas (API boundaries, persistence schema, error handling, GUI wiring).
3. Record concrete locked decisions and deferred items in `.planning/phases/<phase>/CONTEXT.md`.
4. Sync `CONTEXT.md` through `vox_gsd_discuss_phase`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-plan-phase <N>`
