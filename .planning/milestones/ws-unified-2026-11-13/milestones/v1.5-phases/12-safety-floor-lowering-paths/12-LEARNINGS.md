---
phase: 12
phase_name: "safety-floor-lowering-paths"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 12 Learnings: safety-floor-lowering-paths

## Decisions

### Build the typed, level-independent safety floor (FLOOR-01, FLOOR-02) and...
Build the typed, level-independent safety floor (FLOOR-01, FLOOR-02) and enforce it at the single tool-call chokepoint. Purpose: Every MCP client, the chat agent loop, and the autonomous runtime converge on `dispatch_tool_call`. Today the dangerous-tool gate keys only on the canonical tool name and four of six gated names are phantom; the real mutators are ungated. This plan adds `vox_orchestrator::oversight::floor` (pure classifier over the action only) and a small `floor_gate` hook so floored calls always park, bypassing `mode_auto_approves` and the allowlist. Output: new `oversight` module, new `floor_gate.rs`, a ~6-line change to the gate block in `dispatch.rs`, a one-line registry-probe skip, two integration tests updated to approve their now-parked installs, and a where-things-live row.

**Rationale:** Primary phase objective established in plan.
**Source:** 12-01-PLAN.md

---

### Close the two in-band lowering paths for oversight on the task-completio...
Close the two in-band lowering paths for oversight on the task-completion and task-submit sides (FLOOR-03, roadmap SC-3). Purpose: `effective_risk_for_task` returns `task.risk_posture.or(category).or(source)`, so a self-assigned `RiskPosture::High` hint turns grounding and Socrates off and loosens the approval tier (research Pattern 4). Separately, `trust_relax_gates` silently skips both requeue arms, including the `bypass_disallowed` arm (Pitfall 2). This plan makes strictest-wins the rule and narrows trust relax to hint-added enforcement only. Output: new strictest-wins helpers in `mode.rs`, a reshaped resolver plus gate conditions in `socrates.rs`, the tier input change in `attention_fields.rs`, and inverted plus new call-site tests.

**Rationale:** Primary phase objective established in plan.
**Source:** 12-02-PLAN.md

## Lessons

## Patterns

## Surprises

