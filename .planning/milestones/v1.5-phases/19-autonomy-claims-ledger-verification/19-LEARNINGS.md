---
phase: 19
phase_name: "autonomy-claims-ledger-verification"
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

# Phase 19 Learnings: autonomy-claims-ledger-verification

## Decisions

### Ship contracts/orchestration/autonomy-claims.v1.yaml mapping every overs...
Ship `contracts/orchestration/autonomy-claims.v1.yaml` mapping every oversight level effect string (`EffectRow.value` for all 5 levels × 3 effects) and all 4 `hard_floors` labels in `contracts/orchestration/oversight-levels.v1.yaml` to executable behavioral tests in `crates/vox-orchestrator/src/oversight/`, and enforce the absence of retired claims (`safety_tokens`, `delegate_to_free`).

**Rationale:** Primary phase objective established in plan.
**Source:** 19-01-PLAN.md

## Lessons

## Patterns

## Surprises

