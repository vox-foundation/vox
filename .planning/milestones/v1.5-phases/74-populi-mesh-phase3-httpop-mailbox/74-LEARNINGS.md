---
phase: 74
phase_name: "populi-mesh-phase3-httpop-mailbox"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 74 Learnings: populi-mesh-phase3-httpop-mailbox

## Decisions

### Integrate the Populi Mesh Phase 3 branches (mesh-phase3-populi-httpop 1c...
Integrate the Populi Mesh Phase 3 branches (`mesh-phase3-populi-httpop` `1cd2b31a4`, `mesh-phase3-a2a-mailbox` `00313ae50`, and `mesh-phase3-queue-stats` `9a903c94f`): port `PopuliHttpOp` (`Noop`, `Snapshot`, `Join`, `Heartbeat`) in `crates/vox-workflow-runtime/src/workflow/populi.rs` onto `vox-mesh-transport` (iroh QUIC), make unconfigured `Dispatch` and `Wait` fail closed instead of returning fake `local_registry_only` success envelopes, and verify A2A mailbox + queue stats over iroh QUIC.

**Rationale:** Primary phase objective established in plan.
**Source:** 74-01-PLAN.md

## Lessons

## Patterns

## Surprises

