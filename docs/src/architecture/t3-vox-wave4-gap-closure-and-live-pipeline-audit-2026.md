---
title: "T3 Wave 4 Gap Closure & Live Pipeline Audit (2026)"
description: "Forensic code-vs-spec audit of T3 Wave 1-3 implementations across vox-orchestrator, vox-orchestrator-mcp, vox-llm-egress, vox-search, and vox-gui, specifying Wave 4 (Milestone v4.8) end-to-end live-pipeline integration."
category: "Architecture SSOTs"
status: "current"
sort_order: 24
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
---

# T3 Wave 4 Gap Closure & Live Pipeline Audit (2026)

<!-- kb:summary -->
Forensic code-to-specification audit comparing the T3 Wave 1–3 architecture blueprints against the live `vox_chat_message`, `agent_loop`, `t3_wave3_tools`, `agy_worktree`, `webhook_intake`, and Tauri IPC execution paths. Identifies ten concrete integration gaps where domain primitives were unit-tested in isolation but bypassed, stubbed, or broken on the live request path, and defines the six-phase Wave 4 (`v4.8`, Phases 265–270) remediation specification.
<!-- /kb:summary -->

## 1. Executive Summary

Across Milestones `v4.5` (`Phases 247–252`), `v4.6` (`Phases 253–258`), and `v4.7` (`Phases 259–264`), Vox implemented the core domain primitives from the T3 research corpus (`t3code`, `t3router`, `pi-t3chat`, `thom-chat`, `helioschat`):
- **Typed IDs, Outbox & Compaction:** `VoxIdAllocatorV2`, `OutboxQueue`, `context_sanitizer`, `CompactionConfig`.
- **Router, Consensus, Battery & Stream Guards:** `ConsensusOrchestrator`, `BatteryCostLedger`, `ResponseQualityAnalyzer`, `StreamIdleWatchdog`, `ToolRefusalDetector`, `AtomicResumableStreamBuffer`.
- **Worktrees, Checkpoints, MCP & Local Sync:** `GitPlumbingCheckpointStore`, `WorkspaceLeaseMap`, `provision_untracked_env_files`, `McpOAuthPKCEFlow`, `CommandOutputHub`, `ChatTranscriptCache`.
- **Agentic UX, Rules, Webhooks & Session Interop:** `UserRuleRegistry`, `resolve_user_rule_mentions`, `interpolate_system_prompt_variables`, `WebhookDedupeAndRateGate`, `GuestShareService`, `ExternalSessionImporter`.
- **TOESTUB / `vox-code-audit` Static Detectors:** 9 detectors (`wire_unbounded_collection`, `sql_unbounded_query`, `v8_Map_Set_hot_path_clone`, `css_unscoped_transition_all`, `child_process_shell_true`, `worktree_git_index_race`, `git_log_unbounded_all`, `react_provider_inline_value`, `llm_stream_missing_idle_watchdog`).

However, a line-by-line call-graph audit of `crates/vox-orchestrator-mcp/src/chat_tools/chat/` and `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts` revealed **10 high-impact integration gaps (`GAP-01` through `GAP-10`)** where these modules are either broken on edge cases or bypassed on the live production execution path.

---

## 2. Forensic Gap Catalog (`GAP-01` – `GAP-10`)

### `GAP-01`: `tool:<name>` Fence Stream Suppression Drops Tool Calls Without Salvaging (`salvage_and_stream.rs`)

- **Location:** [`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs`](../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs) (`L211` vs `L377–390`).
- **Root Cause:**
  1. `looks_like_streamed_tool_call_envelope` (`L211`) checks `trimmed.starts_with("```tool:")` and returns `true`, suppressing `TokenStreamed` events so raw tool fences are not leaked to the user transcript.
  2. When the stream finishes and native `tool_calls` is empty, `salvage_tool_calls_from_text` (`L377`) runs `r"(?s)```(?:json|tool_call|xml)?\s*(.*?)\s*```"`.
  3. For a `pi-t3chat`-style `tool:vox_read_file` code fence containing `{"path": "Cargo.toml"}`, `inner` captures `"tool:vox_read_file\n{\"path\": \"Cargo.toml\"}"`, which fails `serde_json::from_str`. Tier 4 (`extract_balanced_json_objects`) extracts `{"path": "Cargo.toml"}`, which lacks a `"name"` field (`normalize_salvaged_tool_call_item` returns `None`).
  4. **Impact:** Both the streamed text AND the salvaged tool call are silently dropped.
- **Remediation (`T3-SALVAGE-RULES-01`):**
  - Add Tier 1.5 in `salvage_tool_calls_from_text` matching `r"(?s)```tool:([A-Za-z0-9_.:-]+)\s*\n(.*?)\s*```"`. Extract `cap[1]` as `name` and parse `cap[2]` (or balanced JSON object within `cap[2]`) as `arguments`, then normalize via `normalize_salvaged_tool_call_item`.

### `GAP-02`: `@rule:<name>` Mentions & `$variable` System Prompt Interpolation Bypassed on Live Chat Turn (`mentions.rs`, `preamble_and_retrieval.rs`)

- **Location:**
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/mentions.rs) (`L36`, `L308`)
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs) (`L36`)
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs) (`L17–58`)
- **Root Cause:**
  1. `resolve_user_rule_mentions` (`mentions.rs:36`) is never called by `resolve_mentions` (`mentions.rs:308`) or `prepare_turn_context_and_retrieval`. Worse, `MENTION_RE` (`@([A-Za-z0-9_.:/\\-]+)`) matches `@rule:rust-strict` and attempts to resolve `rule:rust-strict` as a workspace file path, emitting `[ Mention @rule:rust-strict not found in workspace ]`!
  2. `interpolate_system_prompt_variables` (`system_prompt_vars.rs:36`) is never called when constructing the system prompt in `build_system_prompt_with_skill` or `prepare_turn_context_and_retrieval`.
- **Remediation (`T3-SALVAGE-RULES-02..04`):**
  - In `resolve_mentions_with_rules` / `resolve_mentions`, strip and resolve `@rule:<name>` tags (plus `Always`-mode rules) via `resolve_user_rule_mentions` **before** running `MENTION_RE` file resolution, loading persisted user rules from `state.db` (`get_user_preference("local_user", "gui.user_rules")`) with default fallback.
  - In `prepare_turn_context_and_retrieval` (`preamble_and_retrieval.rs`), construct `SystemPromptVarContext` from `state.repository` (`workspace_root`, git branch from `.git/HEAD`), `params.session_id`, `params.active_file`, `model_used_pre`, and `params.cognitive_profile`, and pass `system_prompt` through `interpolate_system_prompt_variables`.

### `GAP-03`: `context_sanitizer` & `UpToMessageId` Compaction Not Wired into `conversation.rs`

- **Location:** [`crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/conversation.rs) (`L428–454`).
- **Root Cause:**
  - `transcript_entry_to_llm_message` (`L428`) pushes raw `entry.content` directly into `ChatMessage` without calling `vox_orchestrator::context_sanitizer::sanitize_assistant_for_history` on `"assistant"` turns or `escape_prompt_injection_tokens` on `"user"` / `"tool"` turns.
  - Closed `<think>...</think>` blocks and `<WebSearch>...</WebSearch>` blocks from earlier turns remain in context on every subsequent turn.
  - Additionally, `vox_chat_compact_up_to` (`CompactionDirection::UpToMessageId`) is not exposed in `chat_tools/chat/history.rs`.
- **Remediation (`T3-HIST-BATTERY-01..02`):**
  - Wire `sanitize_assistant_for_history(&entry.content)` for `"assistant"` messages and `escape_prompt_injection_tokens(&entry.content)` for `"user"` messages inside `transcript_entry_to_llm_message`.
  - Add `chat_compact_up_to` (`vox_chat_compact_up_to`) to `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` and `dispatch.rs` so the GUI and agents can compact transcript history up to or from a specific anchor `message_id`.

### `GAP-04`: `BatteryCostLedger`, `VoxIdAllocatorV2` & Real `ResponseQualityAnalyzer` Scores Bypassed in `message.rs` & `persist_and_envelope.rs`

- **Location:**
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs) (`L57–115`)
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs) (`L44`, `L53`, `L348`)
- **Root Cause:**
  1. `BatteryCostLedger` (`crates/vox-orchestrator/src/budget/tracker.rs`) is never instantiated on `ServerState` or checked/debited during `chat_message`.
  2. `persist_and_envelope.rs:44,53` mints message IDs via `format!("usr-{}", now_ts())` and `format!("ast-{}", now_ts())` (which collide whenever two turns complete within the same second) instead of `VoxIdAllocatorV2::derive`.
  3. `persist_and_envelope.rs:348` hardcodes `quality_score: Some(1.0)` in `LlmSurfaceTelemetry` instead of computing the actual `ResponseQualityAnalyzer::analyze_response_quality(&params.prompt, &response_text)` score.
- **Remediation (`T3-HIST-BATTERY-03..04`):**
  - Add `battery_ledger: Arc<BatteryCostLedger>` and `id_allocator: VoxIdAllocatorV2` to `ServerState`.
  - Run `state.battery_ledger.check_preflight_balance("local_user", 1.0)` before turn execution and `state.battery_ledger.record_turn_usage("local_user", &session_id, &model_used, prompt_tokens, completion_tokens)` in `persist_and_build_chat_envelope`.
  - Mint deterministic collision-free `usr-*` and `ast-*` IDs via `state.id_allocator.derive(...)`.
  - Compute `analyze_response_quality(&params.prompt, &response_text)` in `persist_and_build_chat_envelope`, feed `(quality.quality_score / 100.0)` into `LlmSurfaceTelemetry::quality_score`, and include `quality_score` and `battery_spent_bu` in the response envelope's `posture_audit`.

### `GAP-05`: `CommandOutputHub` `\r` Folding, `bound_snapshot_metadata` DOM Shedding, Pre-Write Sparse Checkpoints, Resumable Stream Buffer & Mid-Turn Steer Draining Not Wired in `agent_loop.rs`

- **Location:**
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs) (`L279`, `L521–545`)
  - [`crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/salvage_and_stream.rs) (`L65–78`)
- **Root Cause:**
  1. Tool outputs in `agent_loop.rs:542` bypass `vox_orchestrator::command_output_hub::normalize_terminal_output` (`\r` progress bar folding) and `vox_orchestrator::snapshot::bound_snapshot_metadata` (`20 KiB` DOM snapshot shedding).
  2. Write/mutation tools (`should_sparse_checkpoint(&call.name)`) execute at `L524` without capturing a pre-mutation sparse Git checkpoint (`GitPlumbingCheckpointStore::capture_checkpoint`).
  3. `stream_llm_with_pref_and_events` (`salvage_and_stream.rs:69`) emits `AgentEventKind::TokenStreamed` to `EventBus` without appending chunks to a `ResumableTurnStreamRegistry` (`AtomicResumableStreamBuffer`) on `ServerState`.
  4. Between tool iterations (`agent_loop.rs:279`), promoted steering prompts from `ThreadRunQueue` (`ServerState::thread_run_queue`) are never drained into `messages`.
- **Remediation (`T3-LOOP-CHECKPOINT-01..04`):**
  - Normalize tool output strings with `normalize_terminal_output` and `bound_snapshot_metadata` before `cap_and_summarize_tool_output`.
  - When `should_sparse_checkpoint(&call.name)` is true, automatically record a pre-tool sparse checkpoint on `state.checkpoint_store`.
  - Hold `resumable_streams: Arc<ResumableTurnStreamRegistry>` and `thread_run_queue: Arc<RwLock<ThreadRunQueue>>` on `ServerState`; append streamed chunks in `stream_llm_with_pref_and_events` and expose `vox_chat_stream_resume`; drain promoted steering items (`QueueDisposition::Steer`) into `messages` at the start of each `agent_loop` iteration.

### `GAP-06`: Synthetic/Stateless Handlers in `t3_wave3_tools.rs`

- **Location:** [`crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs`](../../../../crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs) (`L141–231`, `L406–564`).
- **Root Cause:**
  1. `handle_consensus_run` (`L141`) generates synthetic string templates when `legs` is omitted instead of fanning out concurrent `call_llm_with_pref` calls across the requested `models` via `futures_util::future::join_all` (with graceful fallback when no upstream API key is configured).
  2. `handle_context_handoff` (`L194`) ignores `state` (`_state`) when `turns` is omitted instead of hydrating `source_thread_id`'s live conversation history via `context_history_or_hydrate(state, &format!("chat_history:{source_thread_id}"))` and writing the handoff brief into `chat_history:{new_thread_id}`.
  3. `handle_guest_share_mint` & `handle_guest_share_moderate` (`L406–474`) instantiate a fresh local `GuestShareService::new_from_clavis()` on every call instead of sharing a persistent `Arc<RwLock<GuestShareService>>` on `ServerState` and appending approved guest prompts into `chat_history:{thread_id}`.
  4. `handle_session_import_external`, `handle_session_export_bundle`, and `handle_session_import_bundle` (`L477–564`) ignore `state`:
     - `import_external` does not read real `file_path` / `~/.claude/projects`, `~/.codex/sessions`, or `~/.cursor/chats` files when `raw_payload` is omitted, nor does it persist imported turns into `chat_history:{session_id}`.
     - `export_bundle` does not hydrate `chat_history:{session_id}` from `state` when `messages` is omitted.
     - `import_bundle` does not write the verified bundle's messages into `chat_history:{session_id}` and `state.db`.
- **Remediation (`T3-STATEFUL-TOOLS-01..04`):**
  - Wire all six handlers in `t3_wave3_tools.rs` to live `ServerState` (`context_history_or_hydrate`, `persist_context_Async`, `state.guest_share_service`, and concurrent `join_all` LLM fan-out in `handle_consensus_run`).

### `GAP-07`: Missing `vox_chat_regenerate` & `vox_chat_edit_message_in_place` Backend MCP Tools (`history.rs`, `dispatch.rs`)

- **Location:** [`crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs`](../../../../crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs).
- **Root Cause:**
  - `t3-vox-agentic-ux-rules-webhooks-and-session-interop-spec-2026.md` §1.3 specified two distinct message mutation operations:
    1. `vox_chat_edit_message_in_place`: mutates an assistant (or user) message's content in-place in `chat_history:{session_id}` **without** truncating subsequent messages (`[Edited]` badge).
    2. `vox_chat_regenerate`: truncates history from the target message onward and re-triggers `chat_message` with the stored `GenerationPostureSnapshot` or an explicit `model_override` / `reasoning_effort`.
  - Currently only `vox_chat_slash` `/retry` and `vox_chat_fork` exist.
- **Remediation (`T3-REGEN-WORKTREE-01`):**
  - Implement `chat_edit_message_in_place` and `chat_regenerate` in `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` and register `vox_chat_edit_message_in_place`, `vox_chat_regenerate`, `vox_chat_compact_up_to`, `vox_chat_stream_resume`, and `vox_chat_queue_steer` in `tool_registry.rs`, `input_schemas.rs`, and `dispatch.rs`.

### `GAP-08`: `DelegationWorktree::create` & `submit_webhook_events` Internal Bypass (`agy_worktree.rs`, `webhook_intake.rs`)

- **Location:**
  - [`crates/vox-orchestrator-mcp/src/agy_worktree.rs`](../../../../crates/vox-orchestrator-mcp/src/agy_worktree.rs) (`L94`, `L157–170`)
  - [`crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs`](../../../../crates/vox-orchestrator-mcp/src/server_state/webhook_intake.rs) (`L94`, `L188–209`)
- **Root Cause:**
  1. `DelegationWorktree::create` spawns `git worktree add` without acquiring `WorkspaceLeaseMap::global().lease_for(repo_root)` and without calling `provision_untracked_env_files(repo_root, &wt_path)` (`L94`) after worktree creation.
  2. `submit_webhook_events` (`webhook_intake.rs:188`) validates the HMAC signature and immediately pushes events into `webhook_inbox` without calling `WebhookDedupeAndRateGate::admit_delivery` (`L94`) to reject replayed `delivery_id`s or `> 60/min` bursts.
- **Remediation (`T3-REGEN-WORKTREE-02..03`):**
  - Acquire the `WorkspaceLeaseMap` mutex guard across `git worktree add` and call `provision_untracked_env_files(repo_root, &wt_path)` in `DelegationWorktree::create`.
  - Hold a `WebhookDedupeAndRateGate` on `ServerState` (or global singleton) and gate every `WebhookEventItem` in `submit_webhook_events` via `admit_delivery`.

### `GAP-09`: `vox_orchestrator_status` Ignores `compact` Argument & `transfer_budget.rs` Uses Synthetic Fixtures

- **Location:**
  - [`crates/vox-orchestrator-mcp/src/dispatch.rs`](../../../../crates/vox-orchestrator-mcp/src/dispatch.rs) (`L806`)
  - [`crates/vox-orchestrator-mcp/tests/transfer_budget.rs`](../../../../crates/vox-orchestrator-mcp/tests/transfer_budget.rs) (`L9–33`)
- **Root Cause:**
  - `dispatch.rs:806` calls `dei_tools::orchestrator_status(state).await` unconditionally, ignoring `args["compact"]` and `args["omit_idle_subagents"]` instead of routing to `dei_tools::orchestrator_status_compact(state, omit_idle_subagents)`.
  - `transfer_budget.rs` tests a hardcoded 5-field `serde_json::json!` literal instead of building a 900-row / 1.1 MiB payload and asserting that `orchestrator_status_compact` + `cap_and_summarize_tool_output` keep the wire payload under `256 KiB` and `4` DB round-trips.
- **Remediation (`T3-REGEN-WORKTREE-04`):**
  - Route `vox_orchestrator_status` in `dispatch.rs` to `orchestrator_status_compact` when `compact == true` (defaulting `omit_idle_subagents` to `true`), and upgrade `crates/vox-orchestrator-mcp/tests/transfer_budget.rs` to exercise the real compact projection and 1.1 MiB tool-output capping functions.

### `GAP-10`: `t3Wave3Rpc.ts` `unwrapMcpData` Fails on Tauri `invoke_mcp_tool` `{ is_error, result }` Wrapper (`t3Wave3Rpc.ts`)

- **Location:** [`crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts`](../../../../crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts) (`L88–104`).
- **Root Cause:**
  - In Tauri desktop mode, `voxTransport.invokeMcpTool` invokes Tauri command `invoke_mcp_tool` ([`crates/vox-gui/src/commands/mod.rs:146`](../../../../crates/vox-gui/src/commands/mod.rs)), which returns `{ is_error: false, result: { success: true, data: ... } }` (or a JSON string inside `result`).
  - `unwrapMcpData` in `t3Wave3Rpc.ts:96` only checks `if ('data' in obj && obj.data !== undefined) return obj.data as T;` at the outer object level. Because `data` is nested inside `obj.result.data`, `unwrapMcpData` returns the outer `{ is_error: false, result: ... }` wrapper, which fails every caller's field check (`'winning_response' in res`, `'bundle_json' in res`, `'share_token' in res`, etc.) and causes the GUI to fall through to hardcoded mock objects even when connected to the live Rust backend!
- **Remediation (`T3-GUI-ENVELOPE-01..04`):**
  - Fix `unwrapMcpData` in `t3Wave3Rpc.ts` to recursively unwrap `{ is_error, result }`, stringified JSON payloads, MCP `{ content: [{ type: 'text', text: '...' }] }` blocks, and `{ success, data }` envelopes (throwing when `is_error === true` or `success === false`).
  - Add typed client wrappers `regenerateChatTurn`, `editChatMessageInPlace`, `compactChatUpTo`, `resumeChatStream`, and `steerChatQueue` in `t3Wave3Rpc.ts`, wire in-place assistant message editing and posture-aware regeneration into `ChatTranscript.tsx`, and add Vitest coverage verifying real Tauri `{ is_error: false, result: { success: true, data: ... } }` envelope unwrapping.

---

## 3. Milestone `v4.8` Phase Mapping (`Phases 265–270`, `24` Requirements)

| Phase | Requirement IDs | Scope |
|---|---|---|
| **Phase 265** | `T3-SALVAGE-RULES-01..04` | Fix `tool:<name>` fence salvage in `salvage_and_stream.rs`, wire `@rule:<name>` + `Always` rules into `resolve_mentions` (`mentions.rs`), and wire `interpolate_system_prompt_variables` into `preamble_and_retrieval.rs`. |
| **Phase 266** | `T3-HIST-BATTERY-01..04` | Wire `sanitize_assistant_for_history` & `escape_prompt_injection_tokens` into `conversation.rs`, expose `vox_chat_compact_up_to`, and wire live `BatteryCostLedger`, `VoxIdAllocatorV2`, and `ResponseQualityAnalyzer` into `message.rs` & `persist_and_envelope.rs`. |
| **Phase 267** | `T3-LOOP-CHECKPOINT-01..04` | Wire `normalize_terminal_output` (`\r`), `bound_snapshot_metadata` (`20 KiB` DOM cap), automatic pre-write sparse Git checkpoints (`should_sparse_checkpoint`), `ResumableTurnStreamRegistry` (`vox_chat_stream_resume`), and mid-turn `ThreadRunQueue` steer draining into `agent_loop.rs`. |
| **Phase 268** | `T3-STATEFUL-TOOLS-01..04` | Upgrade `t3_wave3_tools.rs` to dispatch concurrent `join_all` LLM calls in `vox_consensus_run`, hydrate/persist real `chat_history:{session_id}` transcripts in `vox_context_handoff` & `vox_session_*`, and persist `GuestShareService` on `ServerState`. |
| **Phase 269** | `T3-REGEN-WORKTREE-01..04` | Implement `vox_chat_regenerate` & `vox_chat_edit_message_in_place` in `history.rs`, wire `WorkspaceLeaseMap` + `provision_untracked_env_files` in `DelegationWorktree::create`, wire `WebhookDedupeAndRateGate` in `submit_webhook_events`, and wire `compact` in `vox_orchestrator_status` + `transfer_budget.rs`. |
| **Phase 270** | `T3-GUI-ENVELOPE-01..04` | Fix `unwrapMcpData` in `t3Wave3Rpc.ts` for nested Tauri `{ is_error, result: { success, data } }` & MCP content blocks, wire `regenerateChatTurn` & `editChatMessageInPlace` in `ChatTranscript.tsx`, and verify all Rust, Vitest, TypeScript, and audit gates. |
