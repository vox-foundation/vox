---
phase: 322
reviewed: 2026-11-01
depth: standard
scope: phases 320-322 (plus 323 reviewed earlier)
status: issues_found
---

# Phase 322 Code Review

## Warnings

### WR-01: DEC/CAT code has no production caller (violates "no unused facade")
- `build_selection_request_with_decision_pack`, `apply_post_tool_confidence_escalation`, `evaluate_*_with_decision_pack`, `consult_oracle_model_advisory_with_pack`, `resolve_mens_fast_apply_target_with_pack`, `build_chatarc_shortlist`, and `apply_chatarc_choice_to_turn` are exercised only by unit tests.
- `resolve_mcp_chat_model_sync_inner` still calls `build_selection_request`; `run_agent_turn` does not build a ChatARC shortlist.
- Success criteria 1 and 4 in ROADMAP (REQUIREMENTS: "every decision type has a required agent_loop/select consumer") are therefore only met at library level.
- **Not auto-fixed:** needs a pack producer (a calibrated classifier) and a new `McpChatModelResolution` field. Six struct-literal sites (`try_agent_turn.rs`, `infer.rs`, `models_tools.rs`, `tests.rs`) would need edits, and that is outside the plan's file list. Recommend a follow-up phase.

### WR-02: `has_pr_tool` is a substring heuristic
- `skill_catalog.rs` `ChatArcShortlist::has_pr_tool` matches ids containing `pr_create`, `git_pr`, `gh_pr`, `pr_branch`. Could miss renamed PR tools or match unrelated ids. Low risk because it only gates `draft_pr` (fails closed).

## Info

- IN-01: `apply_repair_escalation_to_config` widened from `pub(super)` to `pub(crate)`; intended.
- IN-02: `CatalogEntry` and `skill_catalog` are now `pub`; no behavior change.
- No security findings: pack parsing is `deny_unknown_fields`, rejects model ids, and hosted Jev is gated on `local_only`.

## Fix status
No Critical findings. Warnings left open deliberately (see WR-01).
