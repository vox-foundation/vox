---
title: "T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure (2026)"
description: "Adversarial code-first review of Vox's T3 Waves 1–4 implementation against T3 reference repositories and architecture specs, cataloging every remaining bug, dead island, and parity gap for Milestone v5.1 (Phases 283–288)."
category: "Architecture SSOTs"
status: "current"
topic: "agent-skills"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# T3 Ecosystem Wave 5 Deep Review & End-to-End Parity Closure (2026)

<!-- kb:summary -->
This document records the Wave 5 adversarial code-first review comparing Vox's T3 implementation (`vox-orchestrator`, `vox-orchestrator-mcp`, `vox-code-audit`, and `vox-gui`) against the six upstream T3 reference repositories (`t3code`, `t3router`, `pi-t3chat`, `NotT3Chat`, `t3-chat-tomdoestech`, and `t3-cloneathon`) and our T3 architecture specifications. It identifies six concrete defect clusters—prompt/sanitizer bugs and missing loop guards, split-brain resumable streams and stranded follow-up queues, async worktree lock drops and colliding checkpoint refs, UTF-8 byte-slice panics and synthetic tool fallbacks, frontend-backend message ID mismatches and unwired RPCs, and 43 live `vox-code-audit` violations—and defines the 24 requirements of Milestone `v5.1` (`Phases 283–288`).
<!-- /kb:summary -->

## 1. Executive Summary & Audit Methodology

Following the completion of Milestone `v4.8` (`Phases 265–270`), we conducted a three-axis adversarial audit comparing:

1. **Backend Orchestration & MCP Pipeline:** `crates/vox-orchestrator/src/` and `crates/vox-orchestrator-mcp/src/` vs. `/Users/brbrainerd/dev/t3code`, `/Users/brbrainerd/dev/t3router`, and `/Users/brbrainerd/dev/pi-t3chat`.
2. **Frontend GUI & Rust Tauri Bridge:** `crates/vox-gui/ui/src/` and `crates/vox-gui/src/commands/` vs. `/Users/brbrainerd/dev/t3code/apps/web`, `/Users/brbrainerd/dev/NotT3Chat`, `/Users/brbrainerd/dev/t3-chat-tomdoestech`, and `/Users/brbrainerd/dev/t3-cloneathon`.
3. **Static Enforcement (`vox-code-audit`):** `crates/vox-code-audit/src/detectors/` and `diagnostics/catalog.rs` vs. `/Users/brbrainerd/dev/t3code/oxlint-plugin-t3code` and [`t3-vox-toestub-rules-and-enforcement-research-2026.md`](t3-vox-toestub-rules-and-enforcement-research-2026.md).

While Waves 2–4 established the core domain structures and initial MCP/UI wiring, the audit uncovered **critical subtle bugs, split-brain data stores, un-wired domain engines, and 43 live static-analysis violations** that prevent full production parity.

---

## 2. Cluster-by-Cluster Findings (`file:line` Grounded)

### Cluster 1: Prompt Variable Target Bug, Sanitizer Tag Gaps, `ToolRefusalDetector` & Lazy MCP Meta-Tools (`Phase 277`)

1. **`$variable` Interpolation Applied to User Prompt Instead of `system_prompt` (`preamble_and_retrieval.rs:395-407` & `message.rs:283`):**
   - `interpolate_system_prompt_variables` is called on `expanded_prompt` and `turn_user_prompt` (the **user prompt**), and is **never called on `system_prompt`** (`message.rs:283`). Placeholders like `$workspace_root`, `$git_branch`, `$session_id`, and `$current_datetime` in `system_prompt` remain literal strings.
   - `current_datetime` (`preamble_and_retrieval.rs:402`) calls `ts_to_date_str`, which formats only `YYYY-MM-DD`, omitting `HH:MM:SSZ`.
   - `detect_workspace_git_branch` (`system_prompt_vars.rs:36`) reads `workspace_root.join(".git").join("HEAD")` directly. In linked Git worktrees (`DelegationWorktree` or `git worktree add`), `.git` is a regular file containing `gitdir: /path/to/repo/.git/worktrees/<name>`, so `.join("HEAD")` fails with `ENOTDIR` and falls back to `"unknown"`.
2. **Incomplete `context_sanitizer` Special-Token Escaping, `<WebFetch>` Compaction & Active-Turn Bypass (`context_sanitizer.rs` & `conversation.rs`):**
   - `escape_user_prompt_special_tokens` (`context_sanitizer.rs:109-120`) only escapes `<|im_start|>`, `<|im_end|>`, `<|endoftext|>`, and `<|system|>`, missing `<tool_call>`, `</tool_call>`, `<tool_response>`, `</tool_response>`, `<think>`, and `</think>` (`NotT3Chat` `Program.cs:467`), which allows user input to forge tool call blocks or reasoning tags.
   - `compact_historical_web_search` (`context_sanitizer.rs:20-39`) strips `<WebSearch>` but ignores `<WebFetch>...</WebFetch>` (`NotT3Chat` `Program.cs:573-586`).
   - Active-turn user messages (`turn_user_prompt` pushed at `agent_loop.rs:146` and `message.rs:367`) bypass `escape_user_prompt_special_tokens`, and `persist_and_envelope.rs:63, 75` hardcodes `thought_signature: None` instead of persisting `parse_special_entity_tags(&response_text).extracted_reasoning`.
3. **Missing `pi-t3chat` `ToolRefusalDetector` (`[SYSTEM TOOL CORRECTION]`) & `NotT3Chat` Search Circuit Breaker (`totalSearches >= 3`) (`agent_loop.rs`):**
   - `agent_loop.rs:235-290` breaks out immediately on the first response with zero tool calls (`if tool_calls.is_empty()`), even when a model hallucinates a capability refusal (`"I don't have access to your filesystem or terminal"`). `pi-t3chat` (`tools.ts:270-278, 461-473`) detects `REFUSAL_RE && CAPABILITY_RE` when tools are registered and re-prompts with `[SYSTEM TOOL CORRECTION]` (`max_corrections = 2`).
   - `NotT3Chat` (`Program.cs:554-560`) tracks per-turn search tool invocations and strips search/retrieval tools once `total_searches >= 3` so the model synthesizes an answer instead of looping until `max_iterations`.
4. **Dead `LazyMcpGroupRegistry` & `sanitize_mcp_tool_search_env` (`external_mcp.rs:74, 118-327`):**
   - `LazyMcpGroupRegistry` (`list_mcp_servers`, `list_mcp_tools`, `resolve_call_mcp`) and `sanitize_mcp_tool_search_env` have zero production callers outside `#[cfg(test)]`, while `external_mcp.rs:74` still hardcodes `cmd.env("ENABLE_TOOL_SEARCH", "true")`.

---

### Cluster 2: Sequenced `ResumableTurnStreamRegistry`, `ThreadRunQueue` Follow-Up Draining, `ModelSpec` Cost Ledger & `SessionPrivacyPosture` (`Phase 278`)

1. **Split-Brain `ServerState::resumable_streams` vs. `ResumableTurnStreamRegistry` (`server_state.rs:154`, `salvage_and_stream.rs:33-37`, `message.rs:366`):**
   - `vox_orchestrator::models::spec::ResumableTurnStreamRegistry` (`SequencedTurnChunk`, `TurnStreamEventKind`, `subscribe_or_replay`, `replay_after`, `cancel_turn`) has zero production callers.
   - `ServerState::resumable_streams` uses a raw `HashMap<String, (String, bool)>` that resets on every `stream_final_answer` iteration (`salvage_and_stream.rs:33-37`), wiping iteration 1's output on iteration 2+, and is never updated on non-agent-loop turns (`message.rs:366`).
2. **Stranded `FollowUpTurn` Prompts & Dead Background Settlement in `ThreadRunQueue` (`thread_run_queue.rs:105-169`, `persist_and_envelope.rs:356`):**
   - `pop_next_follow_up`, `set_limit_recovery`, `take_ready_limit_recovery`, `register_background_work`, `complete_background_work`, and `can_settle_turn` are never called in production. Prompts enqueued with `promote_to_steer: false` (`QueueDisposition::FollowUpTurn`) sit in `state.thread_run_queue` forever after `complete_active_turn` runs.
3. **Hardcoded `$0.001 / $0.002` Cost Rates, `"auto"` Preflight False Rejections & Missing 4-Hour Burst Window (`message.rs:259`, `persist_and_envelope.rs:346-352`, `budget/tracker.rs`):**
   - `message.rs:259` calls `check_preflight_balance("local_user", req.model_override.as_deref().unwrap_or("auto"))` before model resolution (`L354`), so `"auto"` returns `is_zero_cost_local_model("auto") == false` and blocks local Ollama/MENS models when platform credits are 0.
   - `persist_and_envelope.rs:346-352` passes hardcoded `0.001` and `0.002` USD/1k rates instead of calling `ModelRegistry::compute_turn_cost_delta` (`registry.rs:326`, currently dead outside unit tests).
   - `BatteryAndCostLedger` lacks `pi-t3chat`'s dual-window tracking (`four_hour_burst_used_bu` / `four_hour_reset_ms` + `monthly_used_bu`) and has no `vox_battery_status` MCP tool.
4. **`SessionPrivacyPosture::Ephemeral` Ignored During SQLite Persistence (`persist_and_envelope.rs:43-185`):**
   - `persist_and_build_chat_envelope` unconditionally writes user/assistant messages, workspace turns, telemetry events, and memories to `state.db` (`VoxDb`) even when `privacy_posture == SessionPrivacyPosture::Ephemeral` (`!privacy_posture.allows_disk_persistence()`).

---

### Cluster 3: Worktree Async Lock Drop, Colliding Sparse Checkpoint Refs, Real `MultiVariantWorktreePool` & Compact Snapshot Fix (`Phase 279`)

1. **`DelegationWorktree::create` Drops Sync Mutex Before `.await` & `WebhookDedupeAndRateGate` Resets Every 250ms (`agy_worktree.rs:163-171`, `webhook_intake.rs:190`):**
   - In `agy_worktree.rs:163-171`, `GitExec::new(repo_root).run(...)` returns an unpolled `Future` inside `{ let _guard = lease.lock()... }`, dropping `_guard` at line 170 before `worktree_res.await?` at line 171! Concurrent `git worktree add` calls execute completely unserialized.
   - In `webhook_intake.rs:190`, `submit_webhook_events` creates a fresh local `WebhookDedupeAndRateGate::default()` on every 250ms poll tick, resetting delivery deduplication and rate limits every 250ms.
2. **Sparse Git Checkpoint Turn Collision (`ckpt_turn = 1`), NTFS Colon Failure, Orphan Commits & Dirty Restore (`batch_and_eval_gate.rs:189`, `checkpoint_engine.rs:72, 86, 125, 155`):**
   - `batch_and_eval_gate.rs:189` computes `let ckpt_turn = (seq.len() as u32) + 1;` from the local per-iteration `seq` vector, overwriting `refs/vox/checkpoints/<session_id>/turn/1` on every turn!
   - `checkpoint_engine.rs:72` only replaces `/` in `thread_id` (`vox_ckpt_index_{}_{}`), leaving `:` and `%` from `VoxIdAllocatorV2` IDs, which fails on Windows paths.
   - `checkpoint_engine.rs:86` omits `-p HEAD` on `git commit-tree`, `restore_checkpoint` (`L155`) leaves behind newly created untracked files not present in `<target_ref>`, and `diff_checkpoints` (`L125`) is never called by `handle_vcs_checkpoint_diff_summary` (`t3_wave3_tools.rs:896`).
3. **In-Memory Mirage in `vox_muse_pool_spawn` & `vox_muse_pool_adopt` (`t3_wave3_tools.rs:442-514` & `muse_pool.rs`):**
   - `handle_muse_pool_spawn` only constructs `VariantWorktreeSlot` path structs in memory without persisting the pool on `ServerState` or materializing worktrees on disk, and `handle_muse_pool_adopt` re-plans a fresh pool from scratch without applying or pruning worktrees.
4. **Double `ToolResult` Envelope & Subagent Filter Bug in `orchestrator_status_compact` (`orchestrator_snapshot.rs:312-344`, `dispatch.rs:815-825`):**
   - `orchestrator_status_compact` compresses the outer `{"success": true, "data": ...}` wrapper from `orchestrator_status`, which `dispatch.rs:822` wraps a second time, and filters idle subagents only by `name.contains("subagent")` instead of checking `parent_agent_id` (`LiveBodyDeduper` is also un-wired).

---

### Cluster 4: UTF-8 Byte-Slice Panics, Multi-Provider Consensus Synthesizer, Recursive Session Discovery & Real Diagnostics (`Phase 280`)

1. **UTF-8 Boundary Panics in `context_handoff.rs:84` & `mcp_tool_access.rs:165, 172` + Stub `HandoffStrategy::Summarized` (`context_handoff.rs:56-62`):**
   - `body.truncate(HANDOFF_BYTE_CAP - SUFFIX.len())` (`context_handoff.rs:84`) and `&l[..160]` / `&pass2[..MAX_DOM_SNAPSHOT_BYTES - 24]` (`mcp_tool_access.rs:165, 172`) slice at raw byte offsets that panic whenever a multi-byte UTF-8 character (`—`, `…`, `•`, emoji, non-ASCII text) straddles the boundary.
   - `HandoffStrategy::Summarized` executes the exact same 4-turn tail slice as `CustomPrompt` without summarizing earlier turns, and `t3_wave3_tools.rs:306` persists handoff entries with `message_id: None`.
2. **`handle_consensus_run` OpenRouter-Only Check & Missing Synthesizer Pass (`t3_wave3_tools.rs:101-245`, `consensus.rs:163-209`):**
   - Checks only `SecretId::OpenRouterApiKey` (`L143`), ignoring Gemini, Anthropic, OpenAI, Groq, and local Ollama/MENS providers, and `ConsensusOrchestrator::evaluate_legs` only picks a single leg by heuristic regex score instead of synthesizing a combined consensus response across completed legs.
3. **1-Level `discover_latest_external_jsonl` Bug (`~/.claude/projects/<slug>/<uuid>.jsonl`) & Fake Guest Moderation (`t3_wave3_tools.rs:545-724`):**
   - `discover_latest_external_jsonl` (`L704`) only scans depth-1 files in `~/.claude/projects` and `~/.codex/sessions`, whereas Claude Code and Codex store `.jsonl` transcripts 2–3 levels deep!
   - `handle_guest_share_moderate` (`L582-589`) fabricates a fake invite and fake guest prompt whenever `pending_messages` is empty because `vox_guest_share_redeem` and `vox_guest_share_submit` were never exposed as MCP tools.
4. **Caller-Controlled Privilege Escalation in `authorize_wave3_call`, Ephemeral `PullRequestReviewGuideService` & Empty Diagnostics (`t3_wave3_tools.rs:49-69, 869, 964`):**
   - `authorize_wave3_call` trusts `runtime_mode` and `granted_capabilities` from caller arguments, `handle_pr_review_guide` recreates `PullRequestReviewGuideService::default()` on every call, and `handle_file_diff_and_diagnostics` hardcodes `"diagnostics": []` instead of running `vox-code-audit` / workspace diagnostics on the file.

---

### Cluster 5: Frontend-Backend `message_id` Unification, Live Transcript Actions, Mid-Turn Steer Composer & Stream Watchdog (`Phase 281`)

1. **Split-Brain `message_id` Between `chat_history:{session_id}` (`usr:...`/`ast:...`) and GUI SQLite (`"1"`, `"2"`, `"gui-run-1:asst"`) (`chat.rs:335-383`, `chat_turn.rs:317-337`, `history.rs:193, 247`):**
   - `parse_chat_message_envelope` in `crates/vox-gui/src/commands/chat.rs` discards `data.message_id` (`ast:...`), `battery_spent_bu`, `remaining_balance_bu`, `quality_score`, `flesch_kincaid_grade`, `compaction_audit`, and `posture_audit`.
   - `history.rs` (`chat_edit_message_in_place`, `chat_regenerate`, `chat_compact_up_to`) only matches exact `e.message_id == Some(message_id)` and never syncs mutations to `state.db`.
2. **Broken `ChatTranscript.tsx` Turn Actions (`✎ Edit`, `↻ Regenerate`, `✂ Compact Here`, `⑂ Compare Branches`) & `/compact` Slash Command:**
   - `ChatTranscript.tsx:186` hides `✎ Edit` on user messages (`!isUser`), `handleRegenerateTurn` (`L410-422`) passes `execute: false` and only displays a banner without updating messages or generating a new reply, `✂ Compact Here` (`compactChatUpTo`) is missing from message bubbles, `/compact` in `Loquela.tsx` is unhandled in `App.tsx:1500-1664`, and `handleOpenBranchComparison` (`L311-331`) fabricates a hardcoded node instead of comparing forked sessions.
3. **`Loquela.tsx` Enter-Key Task Abort Bug & Missing `[⚡ Steer]` / `[+ Queue Next]` Controls (`Loquela.tsx:1216-1224, 1338-1367`, `App.tsx:1316-1324`):**
   - Pressing `Enter` in `Loquela.tsx:1216-1224` while `taskInProgress === true` calls `onInterrupt(currentTaskId)` — aborting the active run instead of steering or queueing the typed prompt! And `steerChatQueue` (`t3Wave3Rpc.ts:765`) has zero `.tsx` callers.
4. **Missing `streamWatchdog.ts`, Unwired `resumeChatStream` & Incomplete `ChatExecutionRail` / `SessionInteropModal` / `ConsensusGridView` (`t3Wave3Rpc.ts`):**
   - `crates/vox-gui/ui/src/lib/chat/streamWatchdog.ts` (`30s` first-token / `120s` idle timeout) does not exist on disk, `resumeChatStream` has zero `.tsx` callers, `ChatSurface.tsx:695-703` never passes `compactionAudit`/`postureAudit`/`batterySpentBu`/`qualityScore` to `ChatExecutionRail` or `ContextWindowMeter`, `SessionInteropModal` is mounted without `messages` or `onSessionImported` (and lacks `sessionImportBundle` + external session list discovery), and `ConsensusGridView`'s `onAdoptVariant` (`vcsMusePoolAdopt`) is unwired.

---

### Cluster 6: `vox-code-audit` T3 Detector Catalog Registration, Rule Completeness & 43 Live Workspace Violations (`Phase 282`)

1. **Missing `diagnostics/catalog.rs` Registration & `diagnostic_id: None` on All 4 T3 Detectors:**
   - `vox/lint/unreasoned-suppression`, `vox/test/flaky-wait-and-unreasoned-skip`, `vox/mcp/access-and-error-safety`, and `vox/ui/layout-and-perf-hazards` are missing from `ALL_KNOWN_IDS` in `crates/vox-code-audit/src/diagnostics/catalog.rs:291-350` and emit `diagnostic_id: None`.
2. **Incomplete Rule Coverage & False Positive in `unreasoned_suppression.rs` and `test_clock_and_skip.rs`:**
   - `unreasoned_suppression.rs` ignores `.vox` files, ignores Rust `#[expect(`, fails to enforce `-- <reason>` on `// toestub-ignore(...)`, ignores preceding-line explanation comments (`explainedAbove`), and falsely flags valid `// @ts-expect-error <explanation>` comments lacking `-- `.
   - `test_clock_and_skip.rs` skips inline Rust `#[cfg(test)]` modules, omits unreasoned `.skip(` / `.todo(` / `xit(` / `xdescribe(`, omits sleep-polling loops (`tokio::time::sleep` / `setTimeout` inside `while`/`loop`/`for`), and omits `requestAnimationFrame` browser waits.
3. **Incomplete `mcp_access_and_error_safety.rs` & `ui_perf_hazards.rs` + Raw `Command::new` in `t3_wave3_tools.rs`:**
   - `mcp_access_and_error_safety.rs` checks `CallToolResult::error` instead of `ToolResult::err` and omits thick-handler subprocess detection (`std::process::Command::new` directly inside `t3_wave3_tools.rs:895, 940, 985, 1010`).
   - `ui_perf_hazards.rs` omits `.css` `:has(...)` (`vox/ui/unscoped-has`) and `scrollbar-gutter: stable` + `mx-auto` without `both-edges` (`vox/ui/centered-scroll-gutter`).
4. **43 Live Workspace Violations Across `crates/vox-gui/ui/src/`, `crates/vox-cli/`, and `crates/vox-orchestrator-mcp/`:**
   - 15 unreasoned lint suppressions in `crates/vox-gui/ui/src/` + 4 bare `toestub-ignore(scaling)` comments in `crates/vox-cli/`.
   - 1 `tracing::warn!(error = ?e, ...)` violation in `crates/vox-orchestrator-mcp/src/chat_tools/chat/harness_issue_judge.rs:137`.
   - 21 `transition-all` violations and 5 `new ResizeObserver(` violations across `crates/vox-gui/ui/src/`.

---

## 3. Milestone `v5.1` Phase Architecture (`Phases 283–288`, `24` Requirements)

| Phase | ID Prefix | Scope |
|---|---|---|
| **Phase 283** | `T3-PROMPT-LOOP-01..04` | Fix `system_prompt` `$variable` interpolation + worktree `.git` file branch resolution + ISO-8601 `$current_datetime`, expand `context_sanitizer` (`<tool_call>`, `<tool_response>`, `<think>`, `<WebFetch>`, active-turn user prompt escaping, `thought_signature` extraction), wire `pi-t3chat` `ToolRefusalDetector` (`[SYSTEM TOOL CORRECTION]`) + `NotT3Chat` search circuit breaker (`total_searches >= 3`), and wire `LazyMcpGroupRegistry` (`vox_list_mcp_servers`, `vox_list_mcp_tools`, `vox_call_mcp_lazy`) + `sanitize_mcp_tool_search_env` |
| **Phase 284** | `T3-STREAM-QUEUE-01..04` | Replace primitive `ServerState::resumable_streams` with `ResumableTurnStreamRegistry` (`SequencedTurnChunk`, multi-iteration accumulation, `vox_chat_stream_resume` + `vox_chat_stream_cancel`), wire `ThreadRunQueue` (`pop_next_follow_up`, `register_background_work`/`can_settle_turn`, 429 `LimitRecoveryState`), wire `ModelRegistry::compute_turn_cost_delta` + post-resolution preflight check + 4-hour/monthly quota windows + `vox_battery_status`, and enforce `SessionPrivacyPosture::Ephemeral` disk-persistence gating |
| **Phase 285** | `T3-WORKTREE-CKPT-01..04` | Fix `DelegationWorktree::create` async `tokio::sync::Mutex` lock across `.await` + persistent `ServerState::webhook_gate` (`WebhookDedupeAndRateGate` + HMAC/filter helpers), fix monotonic per-session sparse Git checkpoint turn counter + Windows NTFS temp-index sanitization + `-p HEAD` parent lineage + clean untracked-file restore + `diff_checkpoints`, materialize real worktrees in `MultiVariantWorktreePool` (`vox_muse_pool_spawn` / `vox_muse_pool_adopt`), and fix `orchestrator_status_compact` double-envelope + `parent_agent_id` subagent filtering + `LiveBodyDeduper` |
| **Phase 286** | `T3-TOOLS-HARDENING-01..04` | Fix UTF-8 char-boundary truncation panics in `context_handoff.rs` & `mcp_tool_access.rs` + real multi-turn digest in `HandoffStrategy::Summarized`, multi-provider secret check + synthesizer pass in `handle_consensus_run`, recursive depth-4 external session discovery (`~/.claude/projects`, `~/.codex/sessions`, `~/.cursor/chats` + `vox_session_list_external`) + real `vox_guest_share_redeem` / `vox_guest_share_submit`, and server-enforced `authorize_wave3_call` + persistent `PullRequestReviewGuideService` + real diagnostics in `handle_file_diff_and_diagnostics` |
| **Phase 287** | `T3-GUI-PIPELINE-01..04` | Unify `message_id` + telemetry fields across `vox_chat_message`, `vox-gui` (`chat.rs`/`chat_turn.rs`), `history.rs` (with turn-index fallback & SQLite sync), wire live user+assistant `✎ Edit`, executing `↻ Regenerate` (`execute: true`), `✂ Compact Here` (`compactChatUpTo`), `/compact` slash command, and real branch comparison in `ChatTranscript.tsx`, wire mid-turn `[⚡ Steer]` & `[+ Queue Next]` (`steerChatQueue`) in `Loquela.tsx` (fixing the Enter-key task abort bug), and implement `streamWatchdog.ts` + `resumeChatStream` + full `ChatExecutionRail` / `SessionInteropModal` / `ConsensusGridView` / `GuestShare` wiring |
| **Phase 288** | `T3-AUDIT-PARITY-01..04` | Register all T3 detector IDs in `diagnostics/catalog.rs` (`ALL_KNOWN_IDS`), upgrade `unreasoned_suppression.rs`, `test_clock_and_skip.rs`, `mcp_access_and_error_safety.rs`, and `ui_perf_hazards.rs` to full `oxlint-plugin-t3code` parity, refactor raw `Command::new` calls in `t3_wave3_tools.rs`, and remediate all 43 live workspace violations across `vox-gui`, `vox-cli`, and `vox-orchestrator-mcp` |
