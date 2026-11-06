# Phase 284 Plan 01 Summary: Resumable Stream Registry, Thread Run Queue, Battery Ledger & Privacy Posture

**Status:** Complete
**Date:** 2026-10-25
**Requirements:** `T3-STREAM-QUEUE-01`, `T3-STREAM-QUEUE-02`, `T3-STREAM-QUEUE-03`, `T3-STREAM-QUEUE-04`

## Changes Delivered
1. **Live `ResumableTurnStreamRegistry` & Multi-Iteration Stream Preservation (`T3-STREAM-QUEUE-01`):**
   - Wired `ResumableTurnStreamRegistry` into `ServerState` ([`server_state.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/server_state.rs)) and [`salvage_and_stream.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs) so sequenced `TextDelta`, `ToolCallStart`, and `Finish` chunks accumulate across multi-iteration tool turns without wiping prior iterations.
   - Upgraded `vox_chat_stream_resume` and added `vox_chat_stream_cancel` in [`history.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs), [`dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs), and [`input_schemas.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/input_schemas.rs).
2. **ThreadRunQueue FIFO Drain, `LimitRecoveryState` & `awaited: bool` Settlement Gate (`T3-STREAM-QUEUE-02`):**
   - Wired `register_background_work` and `complete_background_work` around subagent/task tool calls in [`batch_and_eval_gate.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/batch_and_eval_gate.rs), `set_limit_recovery` on 429/rate-limit errors in [`message.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs), and `pop_next_follow_up` (`next_queued_follow_up`) in [`persist_and_envelope.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs).
3. **Model-Resolved Battery Gate, Dynamic `ModelSpec` Rates & 4-Hour/Monthly Quota Windows (`T3-STREAM-QUEUE-03`):**
   - Upgraded [`tracker.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/budget/tracker.rs) with rolling 4-hour and monthly quota windows (`four_hour_spent_bu`, `four_hour_pct`, `monthly_spent_bu`, `monthly_pct`, `total_cache_read_tokens`), wired live `ModelSpec` rates in `persist_and_envelope.rs`, allowed `"auto"` to fall back to zero-cost local tier when BU is exhausted in `message.rs`, and exposed `vox_battery_status`.
4. **Live `SessionPrivacyPosture` Enforcement (`T3-STREAM-QUEUE-04`):**
   - Enforced `SessionPrivacyPosture` (`Standard`, `Ephemeral`, `LocalPrivate`) in `persist_and_envelope.rs`, skipping all `VoxDb` disk writes (`persisted_to_disk: false`) when `Ephemeral` is active.
