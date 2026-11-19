---
phase: 265
phase_name: "salvage-rules-and-prompt-vars"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 265 Learnings: salvage-rules-and-prompt-vars

## Decisions

### T3-SALVAGE-RULES-01
**`T3-SALVAGE-RULES-01`:** Added Tier 1.5 `TOOL_COLON_FENCE_RE` (`r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\r?\n(.*?)\s*```"`) in `salvage_tool_calls_from_text` (`salvage_and_stream.rs`) so `pi-t3chat` tool fences suppressed during token streaming (`looks_like_streamed_tool_call_envelope`) are salvaged into canonical `{"id", "name", "arguments"}` payloads.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 265-01-SUMMARY.md

---

### T3-SALVAGE-RULES-02
**`T3-SALVAGE-RULES-02`:** Wired `resolve_user_rule_mentions` into `resolve_mentions_with_rules` (`mentions.rs`) and `chat_message` (`message.rs`), loading `gui.user_rules` from `state.db` + `.vox/rules.json` + built-in defaults and stripping `@rule:<name>` before `MENTION_RE` file lookup runs.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 265-01-SUMMARY.md

---

### T3-SALVAGE-RULES-03
**`T3-SALVAGE-RULES-03`:** Added `detect_workspace_git_branch` and `build_live_runtime_vars` in `system_prompt_vars.rs` and wired `interpolate_system_prompt_variables` into `build_system_prompt_with_skill` (`chat_tools/mod.rs`) and `prepare_turn_context_and_retrieval` (`preamble_and_retrieval.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 265-01-SUMMARY.md

---

### T3-SALVAGE-RULES-04
**`T3-SALVAGE-RULES-04`:** Verified with unit tests (`salvages_pi_t3chat_tool_colon_fence_matching_stream_suppression_envelope`, `resolve_mentions_with_rules_strips_rule_tag_before_file_lookup_and_injects_block`, `interpolates_all_8_runtime_variables`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 265-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

