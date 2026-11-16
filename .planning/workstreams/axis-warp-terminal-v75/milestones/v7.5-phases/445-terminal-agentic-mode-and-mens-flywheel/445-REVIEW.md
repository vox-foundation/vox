---
phase: 445-terminal-agentic-mode-and-mens-flywheel
reviewed: 2026-11-16T00:28:00Z
depth: deep
files_reviewed: 58
files_reviewed_list:
  - crates/vox-terminal-core/src/session_agent.rs
  - crates/vox-terminal-core/src/host_agent.rs
  - crates/vox-terminal-core/src/daemon_host_ops.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/commands.rs
  - crates/vox-terminal-core/src/transcript.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-terminal-core/src/block.rs
  - crates/vox-terminal-core/src/agent.rs
  - crates/vox-terminal-core/src/corpus/redact.rs
  - crates/vox-terminal-core/src/corpus/writer.rs
  - crates/vox-terminal-core/src/corpus/curate.rs
  - crates/vox-orchestrator-d/src/term_session_ops.rs
  - crates/vox-orchestrator-d/src/term_dispatch.rs
  - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
  - crates/vox-orchestrator-mcp/src/pending_approvals.rs
  - crates/vox-orchestrator-mcp/src/floor_gate.rs
  - crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs
  - crates/vox-orchestrator-mcp/src/input_schemas/mens_and_ops.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/models.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs
  - crates/vox-orchestrator-mcp/src/daemon_extra.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs
  - crates/vox-gui/src/main.rs
  - crates/vox-gui/src/commands/mens_serve.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/src/commands/pty/warp.rs
  - crates/vox-gui/src/commands/pty/agent.rs
  - crates/vox-gui/src/commands/pty/slash.rs
  - crates/vox-gui/src/commands/pty/approvals.rs
  - crates/vox-gui/src/commands/pty/tests.rs
  - crates/vox-gui/src/commands/pty/agent_tests.rs
  - crates/vox-gui/ui/src/lib/mcpToolResult.ts
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-gui/ui/src/transport.console.test.ts
  - crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
findings:
  critical: 5
  warning: 6
  info: 2
  total: 13
status: issues_found
---

# Phase 445: Code Review Report

**Reviewed:** 2026-11-16T00:28:00Z
**Depth:** deep
**Files Reviewed:** 58
**Status:** issues_found

## Summary

Read-only review of Phase 445 (`445-terminal-agentic-mode-and-mens-flywheel`, Plans `445-01..04`, `TERM-AGENT-01..04`) across `vox-terminal-core`, `vox-orchestrator-d`, `vox-orchestrator-mcp`, `vox-gui` Rust commands, and `vox-gui/ui` React/TypeScript surfaces.

**Verified Strengths & Clean Checks:**
- **Model-agnostic LLM boundary:** `toestub --rules vox/llm-provider-call` passed with exit code `0` (`No issues found — code is clean!`) across `crates/vox-gui/src`, `crates/vox-gui/ui/src`, and `crates/vox-terminal-core/src`; `rg` found zero hardcoded vendor LLM URLs across the Phase 445 files. All terminal LLM calls go through `vox_actor_runtime::llm::{llm_chat, llm_stream_activity}` and Clavis/local MENS discovery.
- **LoC discipline & UI guardrails:** Every new/split Phase 445 file measured via `rg -c "\S"` is under `430` non-blank LoC (largest: `Console.tsx` `415`, `TerminalApprovalCard.tsx` `413`, `agent_tests.rs` `408`, `agent.rs` `398`, `TerminalSessionPane.tsx` `393`, `terminal_approvals.rs` `392`). Static UI guardrail grep across the Phase 445 UI components returned exit code `1` (zero `.5` Tailwind spacing utilities, `font-bold`/`font-medium`, raw `<select>`, or `dangerouslySetInnerHTML`).
- **Safety floor & compound allowlist defenses:** `floor_gate::refuse_agent_floor_resolution` (`crates/vox-orchestrator-mcp/src/floor_gate.rs:25-52`) blocks agent resolution for `[floor:` summaries, `floor_category.is_some()`, `credential_prompt == Some(true)`, and floored tool names. `can_auto_approve_in_session` (`crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs:90-125`) enforces exec-policy AST checks, `L1ReadOnly` blocking, redirect/substitution/env-prefix checks, and `extract_effective_commands` leaf checking.
- **MENS corpus redaction & `credential_prompt` isolation:** `corpus/{redact,writer,curate}.rs`, `build_explain_fix_messages` (`agent.rs:185-187`), and `record_block_feedback` (`host_agent.rs:144-146`) block `credential_prompt` blocks and apply 3-stage redaction.

**Key Risks Requiring Fix Before Phase Sign-off:**
5 Critical and 6 Warning defects remain in `Autopilot -> Manual` preemption (`host.rs::input`, `agent.rs`), pre-resolution validation of `Edit & Run` + `allow_pattern` (`pending_approvals.rs` / `tasks_core.rs` / `daemon_extra.rs`), MENS `Corrected` trajectory `from` pairing (`useTerminalPendingApprovals.ts` / `host_agent.rs`), `/budget` MCP tool contract mismatch (`pty/slash.rs`), streaming secret chunk-splitting (`FLUSH_MIN = 48` in `pty/agent.rs`), `/compact` window & `SnapshotResync` resets, and Console/Chat approval UI state edges.

*(Note: Read-only audit — `cargo test` and `vitest` were not executed.)*

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `LocalHost::input` flips `Autopilot -> Manual` without `bump_control_epoch()` or interrupting open agent execution

**File:** `crates/vox-terminal-core/src/host.rs:813-835` (also `crates/vox-terminal-core/src/host.rs:550-569`, `crates/vox-gui/src/commands/pty/agent.rs:335-365`)
**Issue:** When `LocalHost::input` receives non-empty raw bytes while `*entry.control_mode == TermControlMode::Autopilot` (`host.rs:815-830`), it flips `*mode = TermControlMode::Manual` and emits `SessionEvent::ControlModeChanged { control_mode: TermControlMode::Manual }`, **but** unlike `LocalHost::set_control_mode` (`host.rs:553-568`) and `LocalHost::submit_with_origin` (`host.rs:645-661`), it does not call `s.bump_control_epoch()`, does not send `TermSignal::Interrupt` to an active `BlockOrigin::Agent` PTY block, and does not notify `entry.settled_cv`. Compounding the bug, once `input()` sets `*mode = TermControlMode::Manual`, a subsequent call to `set_control_mode(sid, Manual)` (for example from `TerminalSessionPane.tsx:179` `preemptAutopilot`) sees `prev == TermControlMode::Manual` at `host.rs:553` and **also** skips `bump_control_epoch()` and `TermSignal::Interrupt`. In-flight `execute_agent_turn` loops (`agent.rs:336,363`) rely on `h.control_epoch(sid)` to cancel and therefore keep streaming after operator Manual takeover.
**Fix:**
```rust
            if flipped {
                let interrupt_agent_pty = {
                    let mut s = lock(&entry.session);
                    let _ = s.bump_control_epoch();
                    s.emit_event(SessionEvent::ControlModeChanged {
                        control_mode: TermControlMode::Manual,
                    });
                    s.open_block_id().is_some_and(|open_id| {
                        s.blocks()
                            .iter()
                            .any(|b| b.id == open_id && matches!(b.origin, BlockOrigin::Agent { .. }))
                    })
                };
                if interrupt_agent_pty && let Some(ref mut pty) = *lock(&entry.pty) {
                    let _ = pty.signal(TermSignal::Interrupt);
                }
                entry.settled_cv.notify_all();
            }
```

### CR-02: `Edit & Run` (`Modified`) mutates `session_allowlist` and deletes the pending approval before command validation

**File:** `crates/vox-orchestrator-mcp/src/pending_approvals.rs:240-269` (also `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs:139-154`, `crates/vox-orchestrator-mcp/src/daemon_extra.rs:286-300`, `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs:221-235`, `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts:186-218`)
**Issue:** Both `vox_resolve_approval` (`tasks_core.rs:139-148`) and daemon `RESOLVE_APPROVAL` (`daemon_extra.rs:286-296`) call `pending_approvals.resolve_with_detail(approval_id, resolution)` without validating `modified_command`. Inside `resolve_with_detail` (`pending_approvals.rs:247-257`), the approval is removed from `g.pending`, **`allow_pattern` is inserted into `g.session_allowlist`**, and `{ "resolved": true }` is returned to the caller. Only after waking `park_terminal_hitl` (`terminal_approvals.rs:221-234`) do `validate_edited_terminal_command` and the floor-escalation check run. If the edited command fails AST, `L1ReadOnly`, or `classify_terminal_floor` checks:
1. `allow_pattern` has already been committed in both MCP `session_allowlist` (`pending_approvals.rs:256`) and GUI `sessionPatterns` (`useTerminalPendingApprovals.ts:207`);
2. The original pending approval is deleted and cannot be re-acted on by the operator;
3. The GUI (`useTerminalPendingApprovals.ts:186-218`) receives `{ ok: true }`, displays no validation error, and logs the rejected `modifiedCommand` into the MENS `Corrected` trajectory.
In addition, `pending_approvals.rs:252-253` checks `entry.floor_category.is_none()` without checking `entry.summary.starts_with("[floor:")` (which `floor_gate.rs:38` treats as floored).
**Fix:** Validate `modified_command` (against `validate_edited_terminal_command` and non-escalating `classify_terminal_floor` / session tier) in a shared pre-resolution check invoked by `tasks_core.rs` and `daemon_extra.rs` *before* calling `resolve_with_detail` (returning an error and keeping the approval in `g.pending` if invalid), and also guard `!entry.summary.starts_with("[floor:")` at `pending_approvals.rs:252`.

### CR-03: MENS `TranscriptKind::Corrected` records the wrong `from` command across MCP `Edit & Run` and Console `Co-Pilot`

**File:** `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts:170-177,211-218` (also `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:415`, `crates/vox-terminal-core/src/host_agent.rs:147-154`)
**Issue:** On `Modified` (`Edit & Run`), `useTerminalPendingApprovals.ts:175-177` (Co-Pilot) calls `recordCorrectionSignal(entry?.sourceBlockId ?? latestBlockId, nextCmd)` and `useTerminalPendingApprovals.ts:211-218` (daemon MCP) calls `recordCorrectionSignal(latestBlockId, decision.modifiedCommand.trim())`. `TerminalSessionPane.tsx:415` calls `stageOrRunProposedCommand(cmd, submitShellLine)` without supplying `sourceBlockId`. Meanwhile `host_agent.rs:150-153` ignores `approval.command` and writes `TranscriptKind::Corrected { from: block.input, to: corrected_command }` from whichever block matches `block_id`. Because a parked `vox_terminal_exec` call has not yet opened a `Block`, `latestBlockId` resolves to an unrelated prior command (or `null`, silently dropping the signal, or a `credential_prompt` block, returning `false`). For local Co-Pilot turns where `sourceBlockId` is omitted, `latestBlockId` points to the `AgentTurn` block whose `input` is the user's `?prompt`, not the proposed `original` command. This corrupts `(from, to)` pairs in the MENS trajectory log.
**Fix:**
```rust
// Pass explicit `original_command: Option<String>` from the approval row (`approval.command ?? approval.summary`)
// to `term_block_feedback` / `record_block_feedback` so `Corrected` records `from: original_command`
// directly when `block_id` is absent or refers to a parent `AgentTurn` block, and pass `block.id`
// through `onRunProposedCommand(cmd, block.id)` in BlockCard / BlockStreamView / TerminalSessionPane.
```

### CR-04: `/budget <usd>` calls `vox_models_preference_set` with a rejected key, swallows the error, and fabricates success

**File:** `crates/vox-gui/src/commands/pty/slash.rs:94-116` (also `crates/vox-orchestrator-mcp/src/gui_ops/models.rs:36-37,140-150`, `crates/vox-gui/src/commands/pty/agent_tests.rs:283-300`)
**Issue:** `handle_budget` (`slash.rs:107-109`) calls `b.call_tool("vox_models_preference_set", json!({ "key": "per_session_budget_usd", "value": val })).await`, discards the result with `let _ = ...`, and unconditionally reports `updated_per_session_budget_usd: {val}`. However, `gui_ops/models.rs:36-37,140-147` strictly restricts `MODELS_PREFERENCE_KEYS` to `&["active_model", "routing_priority", "selection_policy"]` and **always** returns `ToolResult::err_with_remediation("key 'per_session_budget_usd' is not a models preference", ...)`. `agent_tests.rs:293-300` mocks `vox_models_preference_set` as succeeding for `"per_session_budget_usd"`, hiding this contract failure. In addition, `slash.rs:111-116` masks backend failures via `.unwrap_or_default()`, printing `0 / 0` and `$0.0000` with exit code `0`.
**Fix:** Route `per_session_budget_usd` updates through a real config/budget mutation surface (matching `crates/vox-gui/src/commands/user_config.rs:261-263` / orchestrator live `VoxConfig`) or add an explicit MCP/HTTP budget setter, inspect `tool_data(...)` on the write result, return exit code `1` on failure, and handle `Err` from `context_budget_impl` / `get_llm_spend_impl` explicitly instead of `.unwrap_or_default()`.

### CR-05: Streaming secret redaction splits long secrets across 48-byte flush chunks

**File:** `crates/vox-gui/src/commands/pty/agent.rs:26,346-352`
**Issue:** `execute_agent_turn` flushes `buf` whenever `buf.contains('\n') || buf.len() >= FLUSH_MIN` (`FLUSH_MIN = 48` at `agent.rs:26`), calling `scrub_for_agent_inbox(&buf)` independently on each 48-byte slice (`agent.rs:348-351`). Any single-line secret longer than 48 bytes (such as `sk-proj-...` keys, JWTs, or long Clavis secret values) is split across two or more `scrub_for_agent_inbox` calls, breaking both exact-value Clavis matching (`vox_secrets::scrub_resolved_secrets`) and full-token regex redaction (`redact_owned`), and persisting raw secret halves into `Block.output` and `full`.
**Fix:**
```rust
// Hold back an un-terminated trailing token / lookback tail (e.g., since the last whitespace
// or max 256 bytes) while streaming, scrubbing the full buffer before emitting only the
// safe prefix and flushing the tail only at stream end (`!buf.is_empty()` at line 358).
```

## Warnings

### WR-01: `resolve_terminal_llm_config_with` overrides explicit cloud `active_model` with local endpoints and hardcodes `VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT`

**File:** `crates/vox-gui/src/commands/pty/agent.rs:116-158`
**Issue:** `resolve_terminal_llm_config_with` checks `port` (`119`), local URL secrets (`123-132`), and reachable local providers (`133-146`) before checking whether `act` (`active_model`) belongs to a cloud provider (`anthropic/...`, `gemini/...`, `openrouter/...`, etc.). When local `mens_serve` or Ollama is present, an explicit cloud `active_model` set via `/model` or Models settings is routed as `model = loc_m` to `127.0.0.1:{port}` or Ollama. Additionally, lines `142-144` hardcode `VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT` (`11435`) for `VoxLocal` instead of reading `VOX_LOCAL_ENDPOINT` (`SecretId::VoxLocalEndpoint`).
**Fix:** Inspect `active_model` provider prefixes (or `ModelRegistry` provider metadata) first when `act` is explicitly set to a cloud provider with a configured Clavis key, and resolve `VoxLocalEndpoint` before falling back to `VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT`.

### WR-02: `execute_agent_turn` cannot be cancelled or preempted while `llm_stream_activity` connects or stalls

**File:** `crates/vox-gui/src/commands/pty/agent.rs:320,335-356`
**Issue:** `llm_stream_activity(&opts, msgs, cfg).await` (`agent.rs:320`) is awaited directly without `tokio::select!` on `rx.changed()` or `control_epoch`, so user Cancel or `Autopilot -> Manual` cannot abort an in-flight connection/TTFT wait until the 90s timeout (`319`). Inside the streaming loop (`335-356`), `h.control_epoch(sid)` is only polled when `stream.next()` or `rx.changed()` resolves (`340-355`), so a stalled stream ignores `control_epoch` bumps until the next chunk arrives.
**Fix:** Wrap `llm_stream_activity` in `tokio::select!` with `rx.changed()` and an epoch check interval/notification, and notify `AgentTurnTracker::cancel` (or watch `SessionEvent::ControlModeChanged`) when `Autopilot -> Manual` bumps `control_epoch`.

### WR-03: `/compact` candidate window mismatches `Session::compact_blocks`, and `/clear` & `/compact` force double xterm/forwarder resets

**File:** `crates/vox-gui/src/commands/pty/slash.rs:202-243` (also `crates/vox-terminal-core/src/session_agent.rs:115-128,130-180`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:158-163,210-225`, `crates/vox-gui/src/commands/pty/warp.rs:193-199`)
**Issue:** Three coupled inconsistencies in `/clear` and `/compact`:
1. `slash.rs:207-213` slices `att.blocks[..len - 3]` before filtering `!pinned && !credential_prompt && !Running/AwaitingInput`, whereas `session_agent.rs:135-148` collects all `!pinned && !Running/AwaitingInput` indices across the whole vector, keeps the last `3` eligible blocks, and omits `!b.credential_prompt`. Consequently, `/compact` can call the LLM to summarize candidate blocks and then have `compact_blocks` return `(None, [])`, or `slash.rs` can report "Nothing to compact" when a `credential_prompt` block precedes 3 eligible blocks (while direct `compact_blocks` callers would compact a `credential_prompt` block).
2. `session_agent.rs:121,173` sets `resync_required: true` on `/clear` and `/compact` `SnapshotResync` events, causing `warp.rs:193-195` to reset `Utf8StreamDecoder` and `TerminalSessionPane.tsx:224` to clear the Raw VT xterm buffer (`text: ''`, `append: false`) even though no PTY bytes were dropped.
3. `TerminalSessionPane.tsx:158-163` (`onSlashMutatedBlocks`) calls `termSessionAttach(sid)` with `forceFullReset = true` after `/clear` and `/compact`, aborting/respawning `spawn_term_forwarder` and replaying up to 2 MiB of raw PTY output even when `SnapshotResync` already updated `paneState.blocks`.
**Fix:** Exclude `!b.credential_prompt` in `session_agent.rs:139-141` and select the exact pre-compaction blocks in `slash.rs` using the same rule; set `resync_required: false` on block-list-only `SnapshotResync` events so Raw VT state is preserved; and skip `termSessionAttach(sid)` in `onSlashMutatedBlocks` unless `listenTermEvent` is unsubscribed.

### WR-04: Cancelled blocks (`status: 'Cancelled'`, `exit_code: 130`) are misclassified as `Failed` in `BlockCard.tsx`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx:158-172` (also `crates/vox-terminal-core/src/block.rs:112-115`)
**Issue:** `Block::cancel()` (`block.rs:112-115`) sets `status = BlockStatus::Cancelled` and `exit_code = Some(130)`. However, `BlockCard.tsx:158` computes `const isFailed = block.status === 'Failed' || (block.exit_code !== null && block.exit_code !== undefined && block.exit_code !== 0);` without excluding `block.status === 'Cancelled'`. As a result, `canExplainFix` (`159`) becomes `true` (rendering `Explain / Fix Failed Block` on user-cancelled shell commands) and `isFailed` precedes `block.status === 'Cancelled'` in `cardTone` (`164` vs `166`), making the `Cancelled` tone branch unreachable whenever `exit_code === 130`.
**Fix:**
```tsx
const isFailed =
  block.status !== 'Cancelled' &&
  (block.status === 'Failed' || (block.exit_code !== null && block.exit_code !== undefined && block.exit_code !== 0));
```

### WR-05: `TermBlockFeedbackStrip` marks verdicts as active even when `termBlockFeedback` fails or returns `recorded: false`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx:212-236` (also `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts:332-353`)
**Issue:** `submitBlockFeedback` (`useTerminalPendingApprovals.ts:340-350`) returns early silently when `!sid || block.credential_prompt || !fn`, catches thrown errors into a toast, resolves `undefined`, and ignores `recorded === false`. `SlashMetaContent.tsx:216-218,229-232` unconditionally invokes `setVerdict(...)` and `setSavedCorrected(...)` immediately after `await Promise.resolve(onFeedback(...))`, leaving the UI visibly selected (`Accepted` / `Rejected` / `Corrected`) even when feedback was not persisted.
**Fix:** Return `Promise<boolean>` from `submitBlockFeedback` (resolving `true` only when `res?.recorded !== false` and no error was thrown) and guard `setVerdict` / `setSavedCorrected` in `TermBlockFeedbackStrip` on `ok !== false` (`if (ok === true)` when boolean returned).

### WR-06: `ChatTerminalBlockCard` & `TurnTrace` approval matching, missing React `key`, and silent fallback resolution

**File:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx:143-164,205` (also `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx:96-114`, `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:230`)
**Issue:** Three issues in Chat inline terminal block/approval rendering:
1. `ChatTerminalBlockCard.tsx:147` matches `matchingApproval` whenever `command === 'vox_terminal_exec' || command === 'vox_terminal_send_input'`, which causes already-completed cards whose `command` fell back to `'vox_terminal_exec'` to attach unrelated pending approvals in the same session.
2. `ChatTerminalBlockCard.tsx:205` and `TurnTrace.tsx:107` render `<TerminalApprovalCard>` without `key={approval.approval_id}`, leaking local `useState` (`activeMode`, `editedCommand`, `steeringNote`) between consecutive approvals.
3. The fallback `handleResolveInline` (`ChatTerminalBlockCard.tsx:155-160`) ignores `typeof fn !== 'function'`, `res.is_error`, and `unwrapMcpEnvelope(res.result)?.resolved === false`, always returning `{ ok: true }`; additionally, `ChatTranscript.tsx:230` renders `<ChatTerminalBlockCard liveBlock={activeAgentBlock} onNavigate={onNavigate} />` without `pendingApprovals` or `onResolveApproval`.
**Fix:** Add `key={matchingApproval.approval_id}` (`ChatTerminalBlockCard.tsx:205`) and `key={approvals[0].approval_id}` (`TurnTrace.tsx:107`); restrict line `147` fallback matching so terminal states (`Ok`/`Failed`/`Cancelled`) never match unrelated approvals; validate `typeof fn === 'function'`, `!res?.is_error`, and `envelope?.resolved !== false` in `handleResolveInline`; and wire `pendingApprovals`/`onResolveApproval` into `ChatTranscript.tsx:230` if live inline approvals are supported there.

## Info

### IN-01: `/context` silently prints `0 / 0 usable` when backend budget RPC fails

**File:** `crates/vox-gui/src/commands/pty/slash.rs:192-198`
**Issue:** `handle_context` uses `.unwrap_or_default()` on `context_budget_impl`, displaying `token_budget: 0 / 0 usable` instead of `unavailable` when the daemon/MCP backend is unreachable.
**Fix:** Match `Result` from `context_budget_impl` and render `token_budget: unavailable` on `Err`.

### IN-02: `build_explain_fix_messages` truncates stdout/stderr from the front, risking loss of error tails

**File:** `crates/vox-gui/src/commands/pty/agent.rs:205-208`
**Issue:** `cap()` truncates `Stdout`/`Stderr` at `&s[..s.floor_char_boundary(PRE_SCRUB)]` and then `&sc[..sc.floor_char_boundary(MAX_SLICE)]`, keeping only the first 8 KiB. Verbose compiler/test failures exceeding 8 KiB drop the summary/failure tail at the end of output.
**Fix:** Keep a head+tail split (or tail window) inside `cap()` after `scrub_for_agent_inbox` so the final error lines remain visible to the model.

---

_Reviewed: 2026-11-16T00:28:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
