---
phase: 266-history-sanitizer-and-battery-telemetry
plan: 01
subsystem: vox-orchestrator-mcp
tags: [t3, context-sanitizer, partial-compaction, battery-ledger, quality-analyzer, id-allocator]
completed: 2026-10-25
---

# Phase 266 Plan 01 Summary: History Sanitizer, Partial Compaction, Battery Gate & Live Quality Telemetry

## Delivered Requirements (`T3-HIST-BATTERY-01..04`)

1. **`T3-HIST-BATTERY-01` — Historical Reasoning Strip, `<WebSearch>` Compaction & User Special-Token Escaping (`GAP-01`):**
   - Wired `vox_orchestrator::context_sanitizer::sanitize_historical_assistant_message` and `escape_user_prompt_special_tokens` directly into `transcript_entry_to_llm_message` in [`crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs).
   - Added unit test `transcript_entry_to_llm_message_sanitizes_assistant_reasoning_and_user_special_tokens`.

2. **`T3-HIST-BATTERY-02` — Directional `up_to`/`from` Partial Compaction & Gemini `thoughtSignature` Preservation (`GAP-01`):**
   - Added `thought_signature` and `edited_at` fields to [`ChatTranscriptEntry`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/params.rs).
   - Implemented `chat_compact_up_to` (`vox_chat_compact_up_to`) in [`crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs) backed by `CompactionEngine::compact_partial_with_signatures`.

3. **`T3-HIST-BATTERY-03` — Deterministic `VoxIdAllocatorV2` Message IDs & Live Flesch-Kincaid `quality_score` (`GAP-03`, `GAP-07`):**
   - Added `id_allocator: SystemRandomIdAllocator` and `battery_ledger: Arc<PrMutex<BatteryAndCostLedger>>` to [`ServerState`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/server_state.rs).
   - Replaced `now_ts()`-based `usr-`/`asst-` message IDs and hardcoded `quality_score: Some(1.0)` in [`crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs) with `state.id_allocator.derive` and `analyze_response_quality(&params.prompt, &response_text)`.

4. **`T3-HIST-BATTERY-04` — Live `BatteryAndCostLedger` Pre-Flight Gate & Dual-Unit Post-Turn Transaction (`GAP-02`):**
   - Wired `BatteryAndCostLedger::check_preflight_balance` into `chat_message` ([`crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs)) and `record_turn_usage` into `persist_and_build_chat_envelope`.

## Verification Evidence
- `cargo test -p vox-orchestrator-mcp --lib -- conversation history` → **PASS**
