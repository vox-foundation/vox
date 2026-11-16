---
name: gsd-explore
description: "Socratic ideation and architecture option analysis before committing a feature into `REQUIREMENTS.md`."
license: MIT
metadata:
  author: open-gsd
  version: "1.0.0"
  vox-category: gsd-workflow
  vox-stage: "Ideation"
---

# GSD Explore

> **Workflow Stage:** Ideation  
> **CLI:** `vox gsd status`  
> **MCP:** `vox_gsd_status`

## Objective

Socratic ideation and architecture option analysis before committing a feature into `REQUIREMENTS.md`.

## Artifacts Managed

- `.planning/seeds/`

## Execution Protocol

1. Combine with `brainstorming` when comparing 2-3 design alternatives.
2. Evaluate crate layer impact (`layers.toml`), database migrations, and GUI/MENS touchpoints.
3. Promote decided approaches to `/gsd-new-milestone` or `/gsd-discuss-phase`.

## Verification & Exit Criteria

- All referenced artifact paths exist and parse cleanly via `vox_orchestrator::planning::gsd`.
- Any `VoxDb` mutations persist CAS hashes via `ops_gsd` without inventing synthetic metrics.
- **Next GSD step:** `/gsd-new-milestone` or `/gsd-spec-phase`
