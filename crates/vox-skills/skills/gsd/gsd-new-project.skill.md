---
name: gsd-new-project
description: "Initialize a new GSD project with deep context gathering and PROJECT.md, REQUIREMENTS.md, ROADMAP.md, and STATE.md."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Project Initialization"
---

# GSD New Project

> **Workflow Stage:** Project Initialization  
> **CLI:** `vox gsd new-milestone <version> "<title>"`  
> **MCP:** `vox_gsd_new_milestone`

## Objective

Initialize a new GSD project with deep context gathering and PROJECT.md, REQUIREMENTS.md, ROADMAP.md, and STATE.md.

## Artifacts Managed

- `.planning/PROJECT.md`
- `.planning/REQUIREMENTS.md`
- `.planning/ROADMAP.md`
- `.planning/STATE.md`
- `.planning/state.json`

## Execution Protocol

1. Run `brainstorming` first when core product boundaries or target users remain ambiguous.
2. Capture project vision, non-goals, architectural invariants, and constraints in `.planning/PROJECT.md`.
3. Derive testable v1 requirements with unique IDs in `.planning/REQUIREMENTS.md`.
4. Partition requirements into numbered phases inside `.planning/ROADMAP.md` and persist `.planning/STATE.md` / `state.json`.
5. Index initialized documents via `vox_gsd_store_document` so VoxDb holds CAS snapshots.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-discuss-phase 1` or `/gsd-plan-phase 1`
