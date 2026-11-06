# Phase 283 Plan 01 Summary: System Prompt Vars, Context Sanitizer, Loop Guards & Lazy MCP Tools

**Status:** Complete
**Date:** 2026-10-25
**Requirements:** `T3-PROMPT-LOOP-01`, `T3-PROMPT-LOOP-02`, `T3-PROMPT-LOOP-03`, `T3-PROMPT-LOOP-04`

## Changes Delivered
1. **Worktree-Aware Git Branch, ISO-8601 UTC Date & 12-Variable System Prompt Interpolation (`T3-PROMPT-LOOP-01`):**
   - Upgraded `detect_workspace_git_branch` in [`system_prompt_vars.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/system_prompt_vars.rs) to resolve linked Git worktrees where `.git` is a file containing `gitdir: <path>`.
   - Normalized 10-character calendar dates (`YYYY-MM-DD`) to ISO-8601 UTC (`YYYY-MM-DDT00:00:00Z`) while preserving `SOTA-KVCACHE-01` day-stability.
   - Added `$session_id`, `$active_file`, `$model_id`, and `$cognitive_profile` to `SystemPromptRuntimeVars` and wired live interpolation in [`preamble_and_retrieval.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message/preamble_and_retrieval.rs) and [`message.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message.rs).
2. **Active-Turn Prompt Injection Escaping, `<WebFetch>` Compaction & `<think>` Signature Extraction (`T3-PROMPT-LOOP-02`):**
   - Extended [`context_sanitizer.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator/src/context_sanitizer.rs) to compact `<WebFetch>...</WebFetch>` alongside `<WebSearch>` and escape `<tool_call>`, `<tool_response>`, and `<think>` control tags in `escape_user_prompt_special_tokens`.
   - Applied `escape_user_prompt_special_tokens` on active-turn user prompts in `preamble_and_retrieval.rs` and `agent_loop.rs`, and persisted extracted `<think>` blocks into `ChatTranscriptEntry::thought_signature` in [`persist_and_envelope.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/message/persist_and_envelope.rs).
3. **2-Strike Refusal Cap, Active Tool Names & Search Circuit Breaker (`T3-PROMPT-LOOP-03`):**
   - Upgraded [`tool_refusal_detector.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/tool_refusal_detector.rs) and [`agent_loop.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop.rs) with `MAX_REFUSAL_CORRECTIONS = 2`, `build_tool_refusal_recovery_reminder(&active_names)`, and a 3-call search circuit breaker (`search_circuit_breaker_tripped`).
4. **Live `sanitize_mcp_tool_search_env` & 3-Tool Lazy MCP Discovery (`T3-PROMPT-LOOP-04`):**
   - Wired `sanitize_mcp_tool_search_env` into `connect()` in [`external_mcp.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/external_mcp.rs) and exposed `vox_list_mcp_servers`, `vox_list_mcp_tools`, and `vox_call_mcp_lazy` in [`t3_wave3_tools.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/t3_wave3_tools.rs), [`dispatch.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/dispatch.rs), and [`input_schemas.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/input_schemas.rs).
