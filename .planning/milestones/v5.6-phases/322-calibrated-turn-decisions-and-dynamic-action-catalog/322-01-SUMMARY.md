---
phase: 322-calibrated-turn-decisions-and-dynamic-action-catalog
plan: 01
status: complete
completed: 2026-11-01
requirements:
  - DEC-01
  - DEC-02
  - DEC-03
  - DEC-04
  - CAT-01
  - CAT-02
---

# Phase 322-01 Summary: Calibrated Turn Decisions and Dynamic Action Catalog

## Accomplishments

- **`DEC-01` (Pre-Turn `TurnDecisionPack` Upstream of `build_selection_request`):**
  - Created [`turn_decision_pack.rs`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/turn_decision_pack.rs) defining `TURN_DECISION_PACK_SCHEMA_V1` (`"turn_decision_pack.v1"`), `TurnDecisionPack` (`#[serde(deny_unknown_fields)]`), `TurnDecisionPack::from_json_rejecting_model_ids`, `CallerExplicitRouteFlags`, and `apply_turn_decision_pack_upstream`.
  - Wired [`build_selection_request_with_decision_pack`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs) in [`resolve.rs`](../../../crates/vox-orchestrator-mcp/src/llm_bridge/model_route_policy/resolve.rs), patching `task`, `complexity`, and `prefer_local` only when not explicitly set by the caller, and allowing `allow_free_in_performance_mode = true` **only when `clutch.is_none()`** without ever accepting or emitting model IDs.
- **`DEC-02` (Post-Tool Confidence Escalation & `draft_pr` Shortlist Guard):**
  - Added `PostToolDecisionPack` (`post_tool_decision_pack.v1`) and `PostToolDirective` in [`lifecycle_hooks.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/lifecycle_hooks.rs), plus `evaluate_post_tool_batch_with_decision_pack`, `evaluate_stop_hook_with_decision_pack`, and `apply_post_tool_confidence_escalation` in [`batch_and_eval_gate.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/batch_and_eval_gate.rs).
  - Low `model_still_ok_prob` (`< confidence_threshold`) re-enters `apply_repair_escalation_to_config` (`SelectionIntent::repair_loop()` with `prefer_local = true` and `allow_free_in_performance_mode = true`), and `PostToolDirective::DraftPr` is rejected unless `shortlist_has_pr_tool` is `true`.
- **`DEC-03` (Flag-Off Byte-for-Byte Identity Tests):**
  - Added unit tests proving `pack = None` produces byte-for-byte identical outputs across `build_selection_request`, `resolve_mens_fast_apply_target`, `consult_oracle_model_advisory`, and `apply_repair_escalation_to_config`.
- **`DEC-04` (Selective-Prediction Calibration Eval & `local_only` Privacy Gate):**
  - Implemented `CalibrationSample`, `SelectivePredictionMetrics` (`accuracy`, `ece`, `coverage_at_error`), `evaluate_selective_prediction`, `hosted_jev_consultation_allowed` (gated behind `!vox_orchestrator::route_policy::inference_privacy_local_only_from_env()`), and `validate_jev_training_isolation` (`ALLOW_JEV_DISTILLATION_INTO_MENS_SFT = false`).
- **`CAT-01` & `CAT-02` (ChatARC Dynamic Action Catalog Shortlist & Calibrated Choice):**
  - Extended [`skill_catalog.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/skill_catalog.rs) with `build_chatarc_shortlist` (`MAX_CHATARC_SHORTLIST_K = 10`, plus `"none"` abstention) composing `CatalogEntry` skills (using `reliability: Option<f64>` as a ranking bonus), `vox_mcp_registry::TOOL_REGISTRY`, and `vox_capability_registry::bundled_index()` CLI rows without introducing a third registry.
  - Added `apply_chatarc_choice_to_turn` to activate a skill (`TurnContext.active_skill_id`), hoist an MCP tool (`TurnContext.pin_names`), or emit a CLI command hint when `confidence >= threshold`, failing open to lexical/deferred behavior below threshold, on `"none"`, or on outage.

## Verification

- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-orchestrator-mcp --lib turn_decision_pack -- --test-threads=8` -> `4 passed; 0 failed`
- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-orchestrator-mcp --lib lifecycle_hooks::tests -- --test-threads=8` -> `5 passed; 0 failed`
- `CARGO_INCREMENTAL=0 timeout 900s cargo test -p vox-orchestrator-mcp --lib skill_catalog::tests -- --test-threads=8` -> `11 passed; 0 failed`
