# Phase 209 Context: Extract T3 Harness & UX Capabilities into Vox Agent Harness (`vox-orchestrator`, `vox-orchestrator-mcp`)

## Goal
Implement the 4 Phase 209 agent harness, MCP discovery, and session UX capabilities (`VOX-HARNESS-T3-01..04`) extracted from `pingdotgg/t3code`, `vibheksoni/pi-t3chat`, `TGlide/thom-chat`, and `helioschat/app`:
1. **`VOX-HARNESS-T3-01`**: `ResumableTurnStreamRegistry`, `SequencedTurnChunk`, and `TurnStreamEventKind` in `crates/vox-orchestrator/src/models/spec.rs` supporting sequence-numbered reconnect replay (`replay_after(thread_id, last_seq)`) and out-of-band generation cancellation (`cancel_turn(thread_id)`).
2. **`VOX-HARNESS-T3-02`**: `LazyMcpGroupRegistry` (`list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`, `MAX_LAZY_MCP_WRAPPER_ROUNDS = 6`) in `crates/vox-orchestrator-mcp/src/external_mcp.rs` grouping `ext.<server>.<tool>` bridges into 3 lazy discovery primitives to prevent prompt token bloat.
3. **`VOX-HARNESS-T3-03`**: `ModelRegistry::enhance_user_prompt_for_spoke` + `EnhancedPromptReceipt` (extracting and preserving `@<rule>` tags while attaching spoke-specific directives) in `crates/vox-orchestrator/src/models/registry/conversational_routing.rs` and `SessionPrivacyPosture` (`Standard | Ephemeral | LocalPrivate`) in `crates/vox-orchestrator/src/models/spec.rs`.
4. **`VOX-HARNESS-T3-04`**: End-to-end verification tests and the machine-readable scorecard `contracts/reports/t3-vox-best-of-all-worlds-scorecard.v1.json`.
