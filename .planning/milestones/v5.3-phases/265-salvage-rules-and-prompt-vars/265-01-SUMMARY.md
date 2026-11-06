---
phase: 265-salvage-rules-and-prompt-vars
plan: 01
subsystem: vox-orchestrator-mcp
tags: [t3, salvage, user-rules, system-prompt-vars]
requires: []
provides:
  - Tier 1.5 `tool:<name>` fence salvage in `salvage_tool_calls_from_text`
  - Live `@rule:<name>` and `Always` user rule resolution in `resolve_mentions_with_rules`
  - Live `$variable` system prompt interpolation in `build_system_prompt_with_skill` and `prepare_turn_context_and_retrieval`
affects:
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/mod.rs
completed: 2026-10-25
requirements:
  - T3-SALVAGE-RULES-01
  - T3-SALVAGE-RULES-02
  - T3-SALVAGE-RULES-03
  - T3-SALVAGE-RULES-04
---

# Phase 265 Plan 01 Summary

## Accomplishments
- **`T3-SALVAGE-RULES-01`:** Added Tier 1.5 `TOOL_COLON_FENCE_RE` (`r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\r?\n(.*?)\s*```"`) in `salvage_tool_calls_from_text` (`salvage_and_stream.rs`) so `pi-t3chat` tool fences suppressed during token streaming (`looks_like_streamed_tool_call_envelope`) are salvaged into canonical `{"id", "name", "arguments"}` payloads.
- **`T3-SALVAGE-RULES-02`:** Wired `resolve_user_rule_mentions` into `resolve_mentions_with_rules` (`mentions.rs`) and `chat_message` (`message.rs`), loading `gui.user_rules` from `state.db` + `.vox/rules.json` + built-in defaults and stripping `@rule:<name>` before `MENTION_RE` file lookup runs.
- **`T3-SALVAGE-RULES-03`:** Added `detect_workspace_git_branch` and `build_live_runtime_vars` in `system_prompt_vars.rs` and wired `interpolate_system_prompt_variables` into `build_system_prompt_with_skill` (`chat_tools/mod.rs`) and `prepare_turn_context_and_retrieval` (`preamble_and_retrieval.rs`).
- **`T3-SALVAGE-RULES-04`:** Verified with unit tests (`salvages_pi_t3chat_tool_colon_fence_matching_stream_suppression_envelope`, `resolve_mentions_with_rules_strips_rule_tag_before_file_lookup_and_injects_block`, `interpolates_all_8_runtime_variables`).
