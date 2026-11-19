---
phase: 209
phase_name: "extract-t3-harness-ux-capabilities-into-vox-agent-harness-vo"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 209 Learnings: extract-t3-harness-ux-capabilities-into-vox-agent-harness-vo

## Decisions

### Added TurnStreamEventKind, SequencedTurnChunk, and ResumableTurnStreamRe...
Added `TurnStreamEventKind`, `SequencedTurnChunk`, and `ResumableTurnStreamRegistry` in `crates/vox-orchestrator/src/models/spec.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 209-01-SUMMARY.md

---

### Supports sequence-numbered chunk append (append_chunk), reconnect delta ...
Supports sequence-numbered chunk append (`append_chunk`), reconnect delta replay (`replay_after(thread_id, last_seq)`), snapshot inspection (`accumulated_snapshot`), and out-of-band cancellation (`cancel_turn`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 209-01-SUMMARY.md

---

### Added LazyMcpGroupRegistry (from_prefixed_tools, list_mcp_servers, list_...
Added `LazyMcpGroupRegistry` (`from_prefixed_tools`, `list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`, `MAX_LAZY_MCP_WRAPPER_ROUNDS = 6`) in `crates/vox-orchestrator-mcp/src/external_mcp.rs` and re-exported in `crates/vox-orchestrator-mcp/src/lib.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 209-01-SUMMARY.md

---

### Added EnhancedPromptReceipt and ModelRegistry::enhance_user_prompt_for_s...
Added `EnhancedPromptReceipt` and `ModelRegistry::enhance_user_prompt_for_spoke` in `crates/vox-orchestrator/src/models/registry/conversational_routing.rs` (extracting and preserving `@<rule>` tags while attaching spoke-specific quality directives).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 209-01-SUMMARY.md

---

### Added SessionPrivacyPosture (Standard, Ephemeral, LocalPrivate) in crate...
Added `SessionPrivacyPosture` (`Standard`, `Ephemeral`, `LocalPrivate`) in `crates/vox-orchestrator/src/models/spec.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 209-01-SUMMARY.md

## Lessons

### t3_resumable_turn_stream_replays_after_cursor_and_supports_cancellation ...
`t3_resumable_turn_stream_replays_after_cursor_and_supports_cancellation` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).

**Context:** Verified in phase acceptance checks.
**Source:** 209-VERIFICATION.md

---

### lazy_mcp_group_registry_supports_list_servers_list_tools_and_call_resolu...
`lazy_mcp_group_registry_supports_list_servers_list_tools_and_call_resolution` in `crates/vox-orchestrator-mcp/src/external_mcp.rs`: PASSED (`ok`).

**Context:** Verified in phase acceptance checks.
**Source:** 209-VERIFICATION.md

## Patterns

## Surprises

