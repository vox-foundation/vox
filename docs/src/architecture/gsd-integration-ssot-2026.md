---
title: "GSD (Get Shit Done) Suite Integration SSoT"
description: "Single source of truth for Vox deep GSD integration across base skills, VoxDb schema v98, vox-orchestrator planning engine, CLI/MCP parity, Vox Axis GUI, and MENS training."
category: "Architecture SSOTs"
topic: "agent-skills"
domain: "orchestration"
status: "current"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
training_eligible: false
---

# GSD (Get Shit Done) Suite Integration SSoT

<!-- kb:summary -->
Vox adopts the open-source GSD (Get Shit Done) workflow suite as its canonical structured planning, execution, verification, and milestone lifecycle framework, superseding the legacy procedural `obra/superpowers` skill set while intentionally retaining `brainstorming` for pre-plan design discovery. GSD is integrated end-to-end across vendored `assets/skills/` (`30` MIT-licensed bundles), L0 `vox-orchestrator-types::gsd` models, `VoxDb` schema v98 Arca CAS persistence, `vox-orchestrator::planning::gsd` wave-to-`PlanNode` DAG scheduling, `vox gsd` CLI and `vox_gsd_*` MCP tools, the Vox Axis GUI `GsdStudioView` studio, and MENS corpus extraction (`extract_gsd`).
<!-- /kb:summary -->

## Lifecycle & Architecture

1. **Base Skills Layer (`assets/skills/gsd-*`, `crates/vox-skills/skills/gsd/`)**: `30` portable MIT-licensed skills replace the `13` retired procedural superpowers skills, while `brainstorming` remains active and transitions cleanly into `gsd-spec-phase` / `gsd-plan-phase`.
2. **L0 Domain Models (`crates/vox-orchestrator-types/src/gsd.rs`)**: `GsdMilestone`, `GsdPhase`, `GsdRequirement`, `GsdDocument`, `GsdPlanWave`, and `GsdProgressSnapshot`.
3. **Persistence (`crates/vox-db`, Baseline v98)**: Tables `gsd_milestones`, `gsd_phases`, `gsd_requirements`, and `gsd_documents` backed by `VoxDb::store` Arca CAS content hashes (`ops_gsd.rs`).
4. **Native Orchestrator (`crates/vox-orchestrator/src/planning/gsd/`)**: `.planning/` markdown parser/writer, milestone/phase/verification state machine, wave-to-`PlanNode` DAG translation, and `GSD_FRAMEWORK_PROMPT`.
5. **CLI & MCP (`vox plan` / `vox gsd` & `vox_gsd_*`)**: Registered in `contracts/operations/catalog.v1.yaml` and exposed identically over CLI, MCP (`crates/vox-orchestrator-mcp/src/gsd_tools.rs`), and daemon-class GUI IPC (`crates/vox-gui/src/commands/gsd.rs`).
6. **Vox Axis GUI (`GsdStudioView`)**: Promoted `view_key: gsd` (`live_backend`) supporting `/gsd` and `/gsd-*` slash commands in `Loquela`.
7. **MENS Awareness (`vox-corpus` & `vox-ml-cli`)**: `extract_gsd` extracts milestone, roadmap, requirement, plan-wave, and verification trajectories into ShareGPT/tool-workflow training mixes (`vox corpus extract-gsd`).
