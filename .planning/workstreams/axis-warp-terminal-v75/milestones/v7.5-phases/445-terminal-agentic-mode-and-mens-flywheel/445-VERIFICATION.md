---
phase: 445-terminal-agentic-mode-and-mens-flywheel
verified: 2026-11-16T01:53:00Z
status: passed
score: 14/14 must-haves verified
behavior_unverified: 0
overrides_applied: 0
covered_files:
  - .planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md
  - .planning/workstreams/axis-warp-terminal-v75/ROADMAP.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-01-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-01-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-02-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-02-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-03-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-03-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-04-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-04-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-CONTEXT.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-PATTERNS.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-RESEARCH.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-REVIEW-FIX.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-REVIEW.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-SECURITY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-UI-REVIEW.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-UI-SPEC.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-VALIDATION.md
  - crates/vox-gui/src/commands/mens_serve.rs
  - crates/vox-gui/src/commands/pty.rs
  - crates/vox-gui/src/commands/pty/agent.rs
  - crates/vox-gui/src/commands/pty/agent_tests.rs
  - crates/vox-gui/src/commands/pty/approvals.rs
  - crates/vox-gui/src/commands/pty/slash.rs
  - crates/vox-gui/src/commands/pty/tests.rs
  - crates/vox-gui/src/commands/pty/warp.rs
  - crates/vox-gui/src/main.rs
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts
  - crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts
  - crates/vox-gui/ui/src/lib/mcpToolResult.ts
  - crates/vox-gui/ui/src/transport.console.test.ts
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs
  - crates/vox-orchestrator-d/src/term_session_ops.rs
  - crates/vox-orchestrator-mcp/src/daemon_extra.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/mod.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs
  - crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools_tests.rs
  - crates/vox-orchestrator-mcp/src/floor_gate.rs
  - crates/vox-orchestrator-mcp/src/input_schemas/core_and_tasks.rs
  - crates/vox-orchestrator-mcp/src/pending_approvals.rs
  - crates/vox-terminal-core/src/commands.rs
  - crates/vox-terminal-core/src/corpus/curate.rs
  - crates/vox-terminal-core/src/corpus/mod.rs
  - crates/vox-terminal-core/src/corpus/redact.rs
  - crates/vox-terminal-core/src/corpus/writer.rs
  - crates/vox-terminal-core/src/daemon_host.rs
  - crates/vox-terminal-core/src/daemon_host_ops.rs
  - crates/vox-terminal-core/src/host.rs
  - crates/vox-terminal-core/src/host_agent.rs
  - crates/vox-terminal-core/src/lib.rs
  - crates/vox-terminal-core/src/session.rs
  - crates/vox-terminal-core/src/session_agent.rs
  - crates/vox-terminal-core/src/transcript.rs
  - docs/src/api/vox-terminal-core.generated.md
covered_digest: "v1:sha256:0eb494aedb016ee0a154506a6e16fa5c28a4e9dc02f1001c81a581299e09ac9b"
---

# Phase 445: Terminal-Native Agentic Mode, Live Slash Commands, Co-Pilot Approvals & MENS Flywheel Verification Report

**Phase Goal:** Enable full agentic control directly from the terminal via `vox_actor_runtime::llm` streaming turns, 1-click failed-block diagnosis/fix, inline Co-Pilot Command Approval Cards, live orchestrator slash commands, and redacted MENS trajectory feedback.
**Verified:** 2026-11-16T01:53:00Z
**Status:** passed
**Re-verification:** No — initial phase verification (executed after post-review remediation `445-REVIEW-FIX.md`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | **ROADMAP SC-1 (`TERM-AGENT-01`):** Submitting `?prompt` or natural-language prose in the terminal streams a real multi-step plan and response through `vox_actor_runtime::llm` (`0` vendor SDK violations in `toestub` / `vox audit code`). | ✓ VERIFIED | `intentRouter.ts:13` classifies `?` / prose into `ask` / `slash` intents routed by `Console.tsx` + `TerminalSessionPane.tsx` -> `transport.ts` `termAgentPrompt` -> `pty.rs:347-452` -> `pty/agent.rs:324` (`term_agent_prompt_impl`) calling `vox_actor_runtime::llm::{LlmConfig, llm_chat, llm_stream_activity}` via `futures_util::StreamExt` and streaming `AgentTurn` chunks into `Session` by `BlockId`. Verified by `streaming_agent_prompt_and_explain_fix_via_llm_facade`, `AgentTurnContent.test.tsx` (`6/6`), `Console.test.tsx` (`19/19`), and `./target/debug/toestub --rules vox/llm-provider-call` exiting `0` (`0` issues) on both `crates/vox-gui/src` and `crates/vox-gui/ui/src`. |
| 2 | **ROADMAP SC-2 (`TERM-AGENT-02`):** Every failed block (`exit_code != 0`) exposes a working 1-click *Explain / Fix Failed Block* flow that proposes an executable repair command in-band (while refusing/hiding on `credential_prompt`). | ✓ VERIFIED | `BlockCard.tsx:154-157` exposes `Explain / Fix Failed Block` (`term-explain-fix-block`) on non-running, non-cancelled failed blocks where `!block.credential_prompt`; `useTerminalPendingApprovals.ts:338` guards `credential_prompt` and invokes `transport.termAgentExplainFix` -> `pty/agent.rs:342` (`term_agent_explain_fix_impl`), attaching head+tail `<= 8 KiB` scrubbed stdout/stderr + `cwd` + `git_branch` and emitting an `AgentTurn` card (`AgentTurnContent.tsx`) with `Execute Proposed Command` & `Copy`. Verified by `explain_fix_scrubs_caps_and_refuses_credential_blocks`, `BlockStreamView.test.tsx` (`11/11`), and `AgentTurnContent.test.tsx` (`6/6`). |
| 3 | **ROADMAP SC-3 (`TERM-AGENT-03`):** Co-Pilot gated commands render inline Roman wax-seal Command Approval Cards (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`) in both `Console` and `Chat`, and any user keystroke/submit in `Autopilot` immediately switches control to `Manual`. | ✓ VERIFIED | `TerminalApprovalCard.tsx:48-57,128` renders semantic `<Icon.waxSeal />` cards with `Approve`, `Edit & Run`, `Allow Pattern in Session` (disabled/hidden on `floor_category` / `credential_prompt`), and `Reject + Steer`, reused identically across `Console` (`TerminalSessionPane.tsx`) and `Chat` (`ChatTerminalBlockCard.tsx:140-172,216` & `TurnTrace.tsx:107-116`). On preemption, `LocalHost::input` (`host.rs:833-848`) and `set_control_mode` (`L585-589`) call `preempt_agent_turns_and_pty` (`L464-481`), while `Console.tsx:115,175,427-428` and `TerminalSessionPane.tsx:173,273,415-432` preempt UI state and surface the preemption banner (`TerminalApprovalCard.tsx:399`). Verified by `TerminalApprovalCard.test.tsx` (`6/6`), `ChatTerminalBlockCard.test.tsx` (`9/9`), `TurnTrace.test.tsx` (`9/9`), `agent_turn_cancel_and_control_epoch_preemption_close_block_cancelled`, and `edit_and_run_allow_pattern_reject_steer_and_autopilot_preemption_e2e`. |
| 4 | **ROADMAP SC-4 (`TERM-AGENT-04`):** `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, and `/compact` execute against live orchestrator state, and per-block Accept/Reject/Correct actions emit redacted `vox-journal` -> `CorpusWriter` records. | ✓ VERIFIED | `pty/slash.rs:94-169,386` (`term_slash_exec_impl`) queries/mutates live config, preferences (`set_user_config` + `vox_preference_set`, returning exit `1` / `unavailable` on failure), skills, memory, context, `/clear`, and `/compact`. `SlashMetaContent.tsx:264-296` & `useTerminalPendingApprovals.ts:164-242,365` drive `transport.termBlockFeedback` -> `pty/approvals.rs:228` (`term_block_feedback_impl`) -> `LocalHost::record_block_feedback` (`host_agent.rs`, supporting `BlockId(0)` with `original_command`) -> `RedactingTranscriptSink` -> `CorpusWriter::record_with` (`writer.rs`) & `.vox/terminal-journal/` (`JournalSink`). Verified by `live_slash_commands_and_compact_digest_execute_end_to_end`, `block_feedback_persists_redacted_journal_and_refuses_credential_blocks`, `vox-terminal-core` (`141/141`), and `SlashMetaContent.test.tsx` (`3/3`). |
| 5 | **Plan 445-01 Truth 1 (Custom Block Isolation & Resync):** `Session` streams `AgentTurn` and `SlashCommand` blocks strictly by `BlockId` without touching `open_id`, `/clear` and `/compact` preserve pinned/active/credential blocks via `SnapshotResync`, and `CommandRegistry` includes sorted `/clear` + `/compact`. | ✓ VERIFIED | `session_agent.rs` (`begin_custom_block`, `append_block_chunk` UTF-8 char-boundary 256 KiB cap, `finish_custom_block`, `cancel_custom_block`, `clear_unpinned_blocks`, `compact_blocks` with `!b.pinned && !b.credential_prompt && !Running/AwaitingInput` and `resync_required: false`, `bump_control_epoch`) + `commands.rs` (`/clear`, `/compact` sorted) + `TerminalSessionPane.tsx` `clearedEmpty` snapshot preservation. Verified by `vox-terminal-core` (`141/141`) and `TerminalSessionPane.test.tsx` (`6/6`). |
| 6 | **Plan 445-01 Truth 2 (3-Stage Redaction & Negative Retention):** `CorpusWriter` + `RedactingTranscriptSink` scrub both `Accepted` and `Rejected` blocks, unconditionally drop `credential_prompt == true`, retain non-empty `Rejected` / non-identical `Corrected` samples in `DefaultPolicy`, and flush `JournalSink` before clears/compactions. | ✓ VERIFIED | `corpus/redact.rs` (`scrub_ansi_and_redact_with`, `redact_block_with`, `redact_block` covering `input`, `output`, `cwd`, `git_branch`), `corpus/writer.rs` (`redact_event_with` drops `credential_prompt == true` and scrubs `Accepted` & `Rejected`), `corpus/curate.rs` (`DefaultPolicy::decide`), `transcript.rs` (`TranscriptSink::flush` -> `journal.sync()`), and `host_agent.rs` (`open_default_journal_sink`, pre-flush in `clear_unpinned_blocks`/`compact_blocks`). Verified by `vox-terminal-core` `corpus` & `transcript` suites inside `141/141` pass. |
| 7 | **Plan 445-01 Truth 3 (Daemon RPC Parity & Origin Guards):** `DaemonHost::route` handles `"snapshot"`, `vox-orchestrator-d` dispatches `term.clear`/`term.compact`/`term.feedback` with `refuse_agent_origin`, initializes `.vox/terminal-journal/`, and matches `crate-api-sync`. | ✓ VERIFIED | `daemon_host.rs:312-338,675-735,809-831` + `daemon_host_ops.rs` + `term_session_ops.rs` (`refuse_agent_origin(p)?` on `CLEAR`/`COMPACT`/`FEEDBACK` and `block_origin` for custom open) + `vox_orchestrator_d.rs:291-310`. Verified by `vox-orchestrator-d` (`50/50` incl. `custom_blocks_clear_compact_feedback_and_origin_guards_work_over_daemon`) and `./target/debug/vox ci crate-api-sync` (`crate-api-sync OK (131 crates)`). |
| 8 | **Plan 445-02 Truth 1 (Dual-Path MCP Approval Validation & Floor Gate):** Both `vox_resolve_approval` (`tasks_core.rs`) and `RESOLVE_APPROVAL` (`daemon_extra.rs`) run `validate_terminal_resolution_before_commit` BEFORE resolving, keeping invalid edits parked; `floor_gate` blocks agent resolution on floor/credential items. | ✓ VERIFIED | `tasks_core.rs:146-158` and `daemon_extra.rs:293-306` invoke `terminal_approvals::validate_terminal_resolution_before_commit` (`terminal_approvals.rs:174-206`) prior to `resolve_with_detail`; `floor_gate.rs:38-41` (`refuse_agent_floor_resolution`) blocks `[floor:` OR `info.floor_category.is_some() || info.credential_prompt == Some(true)`. Verified by `invalid_edit_and_run_keeps_approval_parked_and_session_allowlist_untouched`, `dual_path_resolve_approval_forwards_detail_and_validates_modified`, `floor_gate` (`9/9`), `pending_approvals_tests` (`15/15`), and `pending_approvals_floor_tests` (`8/8`). |
| 9 | **Plan 445-02 Truth 2 (Compound-Safe Allowlists & `Edit & Run`):** `can_auto_approve_in_session` validates every leaf in `extract_effective_commands`, blocks `credential_prompt`/`L1ReadOnly`/`classify_floor`/redirects/substitutions/`ENV=`, and `Edit & Run` re-checks `evaluate_command_ast` + `evaluate_security_tier`. | ✓ VERIFIED | `pending_approvals.rs:253,292` (`BAD_PATTERN_CHARS`, `!entry.summary.starts_with("[floor:")`, token-boundary `matches_leaf_command`) + `terminal_approvals.rs:83` (`can_auto_approve_in_session`) + `core_and_tasks.rs:157-159` schema (`modified_command`, `steering_note`, `allow_pattern`). Verified by `vox-orchestrator-mcp terminal` (`8/8`) and `pending_approvals` (`4/4`). |
| 10 | **Plan 445-02 Truth 3 (`Reject + Steer` & Wait Scrubbing):** `steering_note` and background wait/list outputs strip ECMA-48 controls and scrub Clavis + redacted secrets, preserving multi-byte UTF-8 and pinned blocks before slicing. | ✓ VERIFIED | `terminal_tools.rs:24` (`scrub_terminal_mcp_str` -> `strip_ecma48_and_normalize_cr` + `vox_secrets::scrub_resolved_secrets` + `vox_redact::redact_owned`), `terminal_tools_tests.rs`. Verified by `scrub_utf8_ecma48_background_wait_and_pinned_filter_before_slice`, `park_hitl_reject_steer_scrubs_secrets_and_autopilot_preempts`, and `terminal_mcp_schemas_scrubbing_sticky_hitl_and_credential_guard`. |
| 11 | **Plan 445-03 Truth 1 (Model Discovery, Cross-Chunk Scrub & Cancel):** `resolve_terminal_llm_config_with` checks cloud Clavis slugs (including `"gemini"`) + local MENS serve port, `take_safe_stream_flush` buffers split secret prefixes before scrubbing, and `term_agent_cancel` + `control_epoch` halt in-flight streams. | ✓ VERIFIED | `mens_serve.rs:88-90` (`running_port`), `pty/approvals.rs:72` (`resolve_terminal_llm_config_with`), `pty/agent.rs:202-222,259-321,363` (`take_safe_stream_flush`, `tokio::select!` + 100ms `control_epoch` polling during connect & stream). Verified by `terminal_llm_config_resolution_and_clavis_slugs`, `unconfigured_agent_turn_records_diagnostic_block`, and `agent_turn_cancel_and_control_epoch_preemption_close_block_cancelled`. |
| 12 | **Plan 445-03 Truth 2 (`commands::pty` Host Gate & Frontend Transport Contracts):** All 5 new Tauri commands live under `commands::pty` (`ModuleClass::Host`) in `main.rs`, and `mcpToolResult.ts` + `transport.ts` parse terminal approval metadata and wrap all 7 IPC endpoints cleanly. | ✓ VERIFIED | `main.rs:217-221` registers `term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`, `term_slash_exec`, `term_block_feedback`; `mcpToolResult.ts` parses `PendingApprovalRow` terminal fields; `transport.ts` exposes `termAgentPrompt`, `termAgentExplainFix`, `termAgentCancel`, `termSlashExec`, `termBlockFeedback`, `listTerminalPendingApprovals`, `resolveTerminalApproval`. Verified by `remote_gate_fixture_matches_real_gate` (`1/1`), `commandBackendsMigration.test.ts` (`11/11`), `transport.console.test.ts` (`8/8`), and `tsc --noEmit` exit `0`. |
| 13 | **Plan 445-04 Truth 1 (Approval UX & Error Guarding):** `TerminalApprovalCard` lazy-mounts edit/steer inputs with focus + `Escape`, suppresses allowlisting on floor/credential items, guards against optimistic commit on rejection, and sanitizes all IPC error toasts. | ✓ VERIFIED | `useTerminalPendingApprovals.ts:88,164-246,326,346,382` checks `extractApprovalResolutionError` before removing approvals/saving feedback and passes all catch paths through `sanitizeErrorForToast`; `TerminalApprovalCard.tsx` passes `TerminalApprovalCard.test.tsx` (`6/6`), `toastBodyGuard.test.ts` (`1/1`), and `transportIpcGuard.test.ts` (`2/2`). |
| 14 | **Plan 445-04 Truth 2 (Accessibility, Design Guards, Theme Contract & LoC Budget):** `Console` keeps unique initial `textbox`, passes all 36 Vitest suites (`213/213` tests) including `zeroEmojiAndDingbat`, `noRawNativeControlsGuard`, and `themeContract.test.ts`, and every new/split Phase 445 file stays `< 435` non-blank LoC. | ✓ VERIFIED | Full Vitest gate suite (`36/36` files, `213/213` tests in `69.11s`), `tsc --noEmit` (`0`), `vox-arch-check` (`0`), `rg -n "TBD\|FIXME\|XXX\|PLACEHOLDER\|todo!\(\|unimplemented!\(\|dangerouslySetInnerHTML\|innerHTML"` (`0` matches), and `rg -c '\S'` confirms every Phase 445-created/split file is `<= 427` non-blank LoC (`< 435`). |

**Score:** 14/14 truths verified (`4/4` ROADMAP Success Criteria + `10/10` Cross-Plan Must-Have Truths)

### Plan Prohibitions Audit (`445-01` .. `445-04`)

| Plan | Prohibition | Status | Evidence |
|------|-------------|--------|----------|
| 445-01 #1 | Do NOT add any new crate dependencies or edit `Cargo.toml` / `Cargo.lock` / `docs/src/architecture/layers.toml`. | ✓ HELD | Working-tree `git diff --stat` against `Cargo.toml`, `Cargo.lock`, `crates/vox-terminal-core/Cargo.toml`, `crates/vox-orchestrator-d/Cargo.toml`, and `layers.toml` is empty; `./target/debug/vox-arch-check` exited `0`. |
| 445-01 #2 | Do NOT touch `Session.open_id` during custom `AgentTurn`/`SlashCommand` block streaming or allow agent-origin calls to invoke `term.clear`/`term.compact`/`term.feedback`. | ✓ HELD | `session_agent.rs` mutates custom blocks exclusively by `BlockId`; `term_session_ops.rs` enforces `refuse_agent_origin(p)?` on clear/compact/feedback. |
| 445-01 #3 | Do NOT persist `credential_prompt == true` blocks or unredacted `Rejected` blocks into `JournalSink` / `CorpusWriter`. | ✓ HELD | `CorpusWriter::redact_event_with` drops `credential_prompt == true` and runs 3-stage redaction across both `Accepted` and `Rejected` blocks. |
| 445-02 #1 | Do NOT let agents resolve human `[floor:*]`, `floor_category`, or `credential_prompt` approvals, and do NOT run `Edit & Run` / `Allow Pattern in Session` without AST + `L1ReadOnly` + compound leaf checks. | ✓ HELD | `floor_gate.rs:38-41`, `pending_approvals.rs:253,292`, `terminal_approvals.rs:83,174-206`, `tasks_core.rs:146-158`, and `daemon_extra.rs:293-306` enforce pre-commit checks across both MCP and HTTP/daemon paths (`15/15` + `8/8` + `9/9` + `8/8` + `3/3` tests passing). |
| 445-03 #1 | Do NOT hardcode cloud LLM vendor hostnames/SDKs anywhere in `crates/vox-gui/` — route 100% through `vox_actor_runtime::llm`. | ✓ HELD | `./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/src` and `crates/vox-gui/ui/src` exited `0` (`0` issues); `pty/agent.rs` uses `vox_actor_runtime::llm::{LlmConfig, llm_chat, llm_stream_activity}`. |
| 445-03 #2 | Do NOT place new Tauri commands outside `commands::pty` (`ModuleClass::Host`) or edit `contracts/gui/command-backends.v1.yaml`. | ✓ HELD | All 5 new commands live in `crates/vox-gui/src/commands/pty.rs` (`main.rs:217-221`), working-tree `git diff --stat -- contracts/gui/command-backends.v1.yaml` is empty, `remote_gate_fixture_matches_real_gate` (`1/1`) and `commandBackendsMigration.test.ts` (`11/11`) pass. |
| 445-04 #1 | Do NOT allow `BlockCard` Explain/Fix or MENS feedback strip on `credential_prompt` blocks, do NOT allow pattern allowlisting on floor/credential cards, and do NOT use `dangerouslySetInnerHTML`, raw `<select>`, or emoji/dingbat glyphs. | ✓ HELD | `BlockCard.tsx:157`, `SlashMetaContent.tsx:264`, `TerminalApprovalCard.tsx:48-57`, `useTerminalPendingApprovals.ts:192,338,365`, `0` HTML injection matches, and all UI guards (`zeroEmojiAndDingbat`, `noRawNativeControlsGuard`, `toastBodyGuard`, `transportIpcGuard`) pass. |
| 445-04 #2 | Keep all Phase 445-created and split production files `< 435` non-blank LoC. | ✓ HELD | Measured via `rg -c '\S'`: max new/split Rust file is `427` (`terminal_tools_tests.rs`, max prod `426` `pending_approvals.rs`) and max new/split UI prod file is `424` (`TerminalApprovalCard.tsx`), all `< 435`. |

### Post-Review Remediations Verified (`445-REVIEW-FIX.md`)

All **22/22** findings from `445-REVIEW.md` (`13` findings: `CR-01..05`, `WR-01..06`, `IN-01..02`) and `445-UI-REVIEW.md` (`UI-BL-01..02`, `UI-WR-01..07`; lifted from `16/24` pre-fix) were directly verified in source and tests:
- **`CR-01` (`Autopilot` PTY preemption):** `LocalHost::input` (`host.rs:833-848`) & `set_control_mode` (`L585-589`) call `preempt_agent_turns_and_pty` (`L464-481`), cancelling running `AgentTurn` blocks, sending `TermSignal::Interrupt` to agent PTYs, bumping `control_epoch`, and notifying `settled_cv`.
- **`CR-02` (Dual-path MCP pre-commit validation & `[floor:` block):** `validate_terminal_resolution_before_commit` (`terminal_approvals.rs:174-206`) runs BEFORE `resolve_with_detail` in `tasks_core.rs:146-158` and `daemon_extra.rs:293-306`; `pending_approvals.rs:253` guards `!entry.summary.starts_with("[floor:")`; GUI `extractApprovalResolutionError` prevents optimistic removal on error.
- **`CR-03` (`BlockId(0)` `original_command` feedback):** `BlockFeedbackVerdict::Corrected { corrected_command, original_command }` propagates from `BlockCard.tsx:303` -> `BlockStreamView.tsx:164` -> `TerminalSessionPane.tsx:420` -> `useTerminalPendingApprovals.ts:164-242` -> `pty/approvals.rs:228` -> `host_agent.rs`.
- **`CR-04` (Live `/budget` fidelity):** `handle_budget_with` (`pty/slash.rs:94-169`) writes `user_config::set_user_config`, mirrors `vox_preference_set`, returns exit `1` on config error or total backend failure, and reports `unavailable` rather than fabricating `0`.
- **`CR-05` (Cross-chunk secret prefix scrubbing):** `take_safe_stream_flush` (`pty/agent.rs:202-222`) holds back trailing partial tokens and prefixes (`bearer `, `key= `, `key: `, `token= `, `token: `) across chunk boundaries prior to `scrub_for_agent_inbox`.
- **`WR-01..06`, `IN-01..02`, `UI-BL-01..02`, `UI-WR-01..07`:** Verified in `approvals.rs:72-128` (Clavis `"gemini"` + local port), `agent.rs:100-118,259-321` (100ms connect polling & explicit local diagnostics), `session_agent.rs` (`!b.credential_prompt`, `resync_required: false`), `BlockCard.tsx:154-156` (`Cancelled` exclusion), `SlashMetaContent.tsx:268-296` (`ok === true` guard), `ChatTerminalBlockCard.tsx:140-172,216` & `TurnTrace.tsx:107-116`, `TerminalApprovalCard.tsx` (`bg-(--color-accent-default) text-bg-base` / `bg-bg-base` contrast, button DOM order, auto-focus, `Escape`), `TerminalSessionPane.tsx` (`clearedEmpty` empty snapshot handling), `intentRouter.ts:13` (`ASK_SLASH_RE`), and dynamic `/budget` alert state.

### Required Artifacts & Non-Blank LoC Budget (`< 435` non-blank LoC for created/split files)

| Artifact | Expected | Status | Non-Blank LoC (`rg -c '\S'`) |
|----------|----------|--------|------------------------------|
| `crates/vox-terminal-core/src/{session_agent.rs,host_agent.rs,daemon_host_ops.rs,commands.rs,transcript.rs}` | Custom blocks, snapshot/compaction, `control_epoch`, `JournalSink` flush, `/clear` + `/compact` | ✓ VERIFIED | `324` / `364` / `154` / `240` / `85` (all `< 435`) |
| `crates/vox-terminal-core/src/corpus/{mod.rs,redact.rs,writer.rs,curate.rs}` | 3-stage redaction over `Accepted` + `Rejected`, `credential_prompt` drop, negative curation | ✓ VERIFIED | `9` / `282` / `286` / `201` (all `< 435`) |
| `crates/vox-orchestrator-d/src/{term_session_ops.rs,bin/vox_orchestrator_d.rs}` | Daemon RPC custom blocks, `refuse_agent_origin`, loopback `.vox/terminal-journal/` | ✓ VERIFIED | `272` / `421` (all `< 435`) |
| `crates/vox-orchestrator-mcp/src/{pending_approvals.rs,floor_gate.rs,daemon_extra.rs,input_schemas/core_and_tasks.rs}` | Terminal approval metadata, floor/credential guard, schema & HTTP/daemon resolve parity | ✓ VERIFIED | `426` / `409` / `414` / `195` (all `< 435`) |
| `crates/vox-orchestrator-mcp/src/dispatch_domains/{tasks_core.rs,terminal_approvals.rs,terminal_tools.rs,terminal_tools_tests.rs}` | Dual-path pre-commit validation, `Edit & Run`, compound leaf allowlist, note/output scrubbing | ✓ VERIFIED | `414` / `420` / `317` / `427` (all `< 435`) |
| `crates/vox-gui/src/commands/{mens_serve.rs,pty.rs,pty/{agent.rs,slash.rs,approvals.rs,warp.rs,tests.rs,agent_tests.rs}}` | Model-agnostic LLM stream, Explain/Fix, live slash, feedback, Clavis/MENS resolution, tests | ✓ VERIFIED | `382` / `413` / `353` / `387` / `416` / `398` / `402` / `417` (all `< 435`) |
| `crates/vox-gui/ui/src/{lib/mcpToolResult.ts,hooks/useTerminalPendingApprovals.ts}` | Typed `PendingApprovalRow` metadata + approval/agent/slash/feedback React state & guards | ✓ VERIFIED | `114` / `408` (all `< 435`) |
| `crates/vox-gui/ui/src/components/surfaces/Console/{TerminalApprovalCard.tsx,AgentTurnContent.tsx,SlashMetaContent.tsx,BlockCard.tsx,BlockStreamView.tsx,TerminalSessionPane.tsx,Console.tsx,intentRouter.ts}` | Wax-seal approval cards, streaming turns, feedback strip, Explain/Fix, preemption, `?`/`/slash` routing | ✓ VERIFIED | `424` / `411` / `402` / `362` / `218` / `398` / `414` / `126` (all `< 435`) |
| `crates/vox-gui/ui/src/components/surfaces/Chat/{ChatTerminalBlockCard.tsx,TurnTrace.tsx}` | Shared Co-Pilot wax-seal approval cards inside Chat trace & terminal cards | ✓ VERIFIED | `225` / `163` (all `< 435`) |
| Test files (`transport.console.test.ts` & Console/Chat `.test.tsx`) | Transport, ApprovalCard, AgentTurn, SlashMeta, BlockStreamView, SessionPane, Console, Chat tests | ✓ VERIFIED | `390` (`transport.console`), `172` (`Approval`), `179` (`AgentTurn`), `144` (`SlashMeta`), `336` (`SessionPane`), `425` (`ChatTerminalBlockCard`), `114` (`TurnTrace`); pre-existing `BlockStreamView.test.tsx` `439`, `Console.test.tsx` `559` |
| `docs/src/api/vox-terminal-core.generated.md` | Synced crate API documentation generated by `vox ci crate-api-sync` | ✓ VERIFIED | `crate-api-sync OK (131 crates)` |

*(Pre-existing large files modified incrementally without split targets: `host.rs` 1166, `daemon_host.rs` 895, `transport.ts` 1848 non-blank LoC.)*

### Key Link & Data-Flow Verification (Levels 3–4)

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `Console.tsx` / `intentRouter.ts` | `pty/agent.rs` (`vox_actor_runtime::llm`) | `TerminalSessionPane` -> `transport.termAgentPrompt` / `termAgentExplainFix` -> `commands::pty` | ✓ WIRED & FLOWING | Streams `llm_stream_activity` deltas through `take_safe_stream_flush` into `Session` custom `AgentTurn` blocks (`BlockId`-isolated) with `control_epoch` cancellation. |
| `TerminalApprovalCard.tsx` (`Console` & `Chat`) | `PendingApprovals` (`vox-orchestrator-mcp`) | `useTerminalPendingApprovals` -> `transport.resolveTerminalApproval` -> `tasks_core.rs` / `daemon_extra.rs` | ✓ WIRED & FLOWING | Both `vox_resolve_approval` and `RESOLVE_APPROVAL` run `validate_terminal_resolution_before_commit` + `floor_gate` before waking agent tool calls; `Autopilot` keystrokes preempt via UI + `LocalHost::input`. |
| `ScriptoriumPromptEditor` (`/slash`) | `pty/slash.rs` | `transport.termSlashExec` -> `term_slash_exec_impl` | ✓ WIRED & FLOWING | Queries/mutates live orchestrator config, skills, memory, context, `/clear`, and `/compact`, rendering `SlashMetaContent` cards. |
| `SlashMetaContent.tsx` (`TermBlockFeedbackStrip`) | `JournalSink` & `CorpusWriter` | `useTerminalPendingApprovals.submitFeedback` -> `term_block_feedback` -> `host_agent::record_block_feedback` | ✓ WIRED & FLOWING | Supports `BlockId(0)` via `original_command`, enforces `credential_prompt` + `refuse_agent_origin`, scrubs via `vox-redact` + `vox-secrets`, and curates `Rejected`/`Corrected` trajectories. |

### Live Verification Runs Executed in This Session

Every build, test, lint, and audit check below ran live in this verification session with **0 failures** and **0 skipped assertions**:

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| `vox-terminal-core` full suite | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core` | `141 passed; 0 failed` (lib `84`, `agent` `3`, `block` `5`, `commands` `2`, `host` `1`, `input` `10`, `osc633` `5`, `pty` `5`, `session` `7`, `shell_init_and_stateful_pty` `6`, `smoke` `1`, `streaming_utf8_and_alt_screen` `9`, `transcript` `2`, `vox_interp` `1`) | ✓ PASS |
| `vox-orchestrator-d` full suite | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-d` | `50 passed; 0 failed` (lib `37`, bin `10`, `boot_profiles` `3`) | ✓ PASS |
| `vox-orchestrator-mcp` `pending_approvals` | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp pending_approvals` | `4 passed; 0 failed` (lib `3`, `daemon_extra_tests` `1`) | ✓ PASS |
| `vox-orchestrator-mcp` `terminal` | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp terminal` | `8 passed; 0 failed` (`terminal_approvals` + `terminal_tools` + `terminal_tools_tests`) | ✓ PASS |
| `vox-orchestrator-mcp` `daemon_extra` | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp daemon_extra` | `3 passed; 0 failed` | ✓ PASS |
| `vox-orchestrator-mcp` integration & floor suites | `--test pending_approvals_tests`, `--test pending_approvals_floor_tests`, `--lib floor_gate` | `15 passed; 0 failed`, `8 passed; 0 failed`, `9 passed; 0 failed` | ✓ PASS |
| `vox-gui` `pty` suite | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui pty` | `35 passed; 0 failed` (`366 filtered out`) | ✓ PASS |
| `vox-gui` remote IPC gate | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui remote_gate_fixture_matches_real_gate` | `1 passed; 0 failed` (`400 filtered out`) | ✓ PASS |
| Frontend TypeScript check | `timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` | Exit `0`, `0` diagnostics | ✓ PASS |
| Frontend Console, Chat, Guards & Theme Vitest suite | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=2 src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts` | `Test Files 36 passed (36)`, `Tests 213 passed (213)` in `69.11s` | ✓ PASS |
| `vox/llm-provider-call` policy audit | `./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/src` & `crates/vox-gui/ui/src` | Exit `0` (`No issues found — code is clean!`) on both | ✓ PASS |
| Crate API doc sync | `timeout 60s ./target/debug/vox ci crate-api-sync` | Exit `0` (`crate-api-sync OK (131 crates)`) | ✓ PASS |
| Architecture layer & boundary check | `timeout 60s ./target/debug/vox-arch-check` | Exit `0` (`0` violations; advisory `LoC delta >15%` release warnings only) | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| `TERM-AGENT-01` | `445-01`, `445-03`, `445-04` | Submitting `?prompt` or natural-language prose in the terminal streams a real multi-step agent turn and command plan through the model-agnostic `vox_actor_runtime::llm` facade (never vendor SDKs or static stubs). | ✓ SATISFIED | `pty/agent.rs`, `pty/approvals.rs`, `session_agent.rs`, `AgentTurnContent.tsx`, `intentRouter.ts`, `Console.tsx`; `toestub` `0` issues + `streaming_agent_prompt_and_explain_fix_via_llm_facade` + `AgentTurnContent.test.tsx` (`6/6`). |
| `TERM-AGENT-02` | `445-03`, `445-04` | Failed command blocks (`exit_code != 0`) display a 1-click *Explain / Fix Failed Block* action that attaches the redacted block + `cwd` context and streams a targeted diagnosis and runnable fix proposal. | ✓ SATISFIED | `pty/agent.rs:342`, `BlockCard.tsx:154-157`, `useTerminalPendingApprovals.ts:338`, `AgentTurnContent.tsx`; `explain_fix_scrubs_caps_and_refuses_credential_blocks` + `BlockStreamView.test.tsx` (`11/11`). |
| `TERM-AGENT-03` | `445-01`, `445-02`, `445-03`, `445-04` | Co-Pilot mode renders inline Roman wax-seal Command Approval Cards (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`) in both `Console` and `Chat`, while any user keystroke or submit in `Autopilot` immediately transitions control mode to `Manual`. | ✓ SATISFIED | `pending_approvals.rs`, `floor_gate.rs`, `terminal_approvals.rs`, `tasks_core.rs`, `daemon_extra.rs`, `host.rs:464-481,833-848`, `TerminalApprovalCard.tsx`, `ChatTerminalBlockCard.tsx`, `TurnTrace.tsx`; all MCP & UI approval/preemption suites green. |
| `TERM-AGENT-04` | `445-01`, `445-03`, `445-04` | Terminal `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, and `/compact` slash commands query/mutate live orchestrator state, and completed sessions record redacted `vox-journal` -> `curate` -> `CorpusWriter` trajectories with per-block Accept/Reject/Correct feedback signals for MENS. | ✓ SATISFIED | `pty/slash.rs`, `pty/approvals.rs`, `corpus/{redact,writer,curate}.rs`, `transcript.rs`, `host_agent.rs`, `term_session_ops.rs`, `SlashMetaContent.tsx`; `vox-terminal-core` (`141/141`), `vox-orchestrator-d` (`50/50`), `live_slash_commands_and_compact_digest_execute_end_to_end`, `block_feedback_persists_redacted_journal_and_refuses_credential_blocks`, `SlashMetaContent.test.tsx` (`3/3`). |

*Orphaned Phase 445 requirements in `REQUIREMENTS.md`: **0** (`TERM-AGENT-01..04` are the exact 4 requirements mapped to Phase 445 in `REQUIREMENTS.md:101-104`).*

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| None | — | Zero `TBD`/`FIXME`/`XXX`/`PLACEHOLDER`/`todo!()`/`unimplemented!()`, zero `dangerouslySetInnerHTML`/`innerHTML`, zero vendor SDK calls (`toestub` `0`), zero UI guard violations | — | None |

### Human Verification Required

None required to gate Phase 445 completion. Live multi-theme Playwright + computer-vision screenshot convergence across all 4 Vox Roma themes (`arcane`, `slate`, `travertine`, `high-contrast`) and live visual SSE token streaming cadence / Carrara & burnished-brass wax-seal polish are tracked in **Phase 446 (`TERM-ROMA-CV-01..04`)** and noted in `445-VALIDATION.md` Manual-Only Verifications.

### Gaps Summary

No gaps found. All 4 Phase 445 requirements (`TERM-AGENT-01..04`), all 4 ROADMAP Success Criteria, all 10 Cross-Plan Must-Have Truths, all 8 Plan Prohibitions, all 22 Code & UI Review remediations (`445-REVIEW-FIX.md`), all 14 STRIDE threat mitigations (`445-SECURITY.md`), and all 11 Nyquist task gates (`445-VALIDATION.md`) are verified and passing.

---

_Verified: 2026-11-16T01:53:00Z_
_Verifier: Antigravity (gsd-verifier)_
