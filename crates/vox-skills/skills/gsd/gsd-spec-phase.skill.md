---
name: gsd-spec-phase
description: "Clarify WHAT a phase delivers with ambiguity scoring and produce SPEC.md before discuss-phase or plan-phase."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Phase Specification"
---

# GSD Spec Phase

> **Workflow Stage:** Phase Specification  
> **CLI:** `vox gsd docs <phase>`  
> **MCP:** `vox_gsd_store_document`

## Objective

Clarify WHAT a phase delivers with ambiguity scoring and produce SPEC.md before discuss-phase or plan-phase.

## Artifacts Managed

- `.planning/phases/<phase>/SPEC.md`

## Execution Protocol

1. Read target phase goals and requirement IDs from `.planning/ROADMAP.md` and `.planning/REQUIREMENTS.md`.
2. Evaluate scope ambiguity; invoke `brainstorming` to resolve open architectural trade-offs.
3. Author `SPEC.md` with exact inputs, outputs, invariants, and non-goals.
4. Persist `SPEC.md` via `vox_gsd_store_document` with kind `spec`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-discuss-phase <N>`
