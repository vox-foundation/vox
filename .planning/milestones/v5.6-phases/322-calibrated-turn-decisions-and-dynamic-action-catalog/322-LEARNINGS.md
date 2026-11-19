---
phase: 322
phase_name: "calibrated-turn-decisions-and-dynamic-action-catalog"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 322 Learnings: calibrated-turn-decisions-and-dynamic-action-catalog

## Decisions

### Created [turn_decision_pack.rs](../../../crates/vox-orchestrator-mcp/src...
Created [`turn_decision_pack.rs`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_decision_pack.rs) defining `TURN_DECISION_PACK_SCHEMA_V1` (`"turn_decision_pack.v1"`), `TurnDecisionPack` (`#[serde(deny_unknown_fields)]`), `TurnDecisionPack::from_json_rejecting_model_ids`, `CallerExplicitRouteFlags`, and `apply_turn_decision_pack_upstream`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 322-01-SUMMARY.md

---

### Wired [build_selection_request_with_decision_pack](../../../crates/vox-o...
Wired [`build_selection_request_with_decision_pack`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs) in [`resolve.rs`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs), patching `task`, `complexity`, and `prefer_local` only when not explicitly set by the caller, and allowing `allow_free_in_performance_mode = true` **only when `clutch.is_none()`** without ever accepting or emitting model IDs.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 322-01-SUMMARY.md

---

### Added PostToolDecisionPack (post_tool_decision_pack.v1) and PostToolDire...
Added `PostToolDecisionPack` (`post_tool_decision_pack.v1`) and `PostToolDirective` in [`lifecycle_hooks.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/lifecycle_hooks.rs), plus `evaluate_post_tool_batch_with_decision_pack`, `evaluate_stop_hook_with_decision_pack`, and `apply_post_tool_confidence_escalation` in [`batch_and_eval_gate.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/batch_and_eval_gate.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 322-01-SUMMARY.md

---

### Low model_still_ok_prob (< confidence_threshold) re-enters apply_repair_...
Low `model_still_ok_prob` (`< confidence_threshold`) re-enters `apply_repair_escalation_to_config` (`SelectionIntent::repair_loop()` with `prefer_local = true` and `allow_free_in_performance_mode = true`), and `PostToolDirective::DraftPr` is rejected unless `shortlist_has_pr_tool` is `true`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 322-01-SUMMARY.md

---

### Added unit tests proving pack = None produces byte-for-byte identical ou...
Added unit tests proving `pack = None` produces byte-for-byte identical outputs across `build_selection_request`, `resolve_mens_fast_apply_target`, `consult_oracle_model_advisory`, and `apply_repair_escalation_to_config`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 322-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

