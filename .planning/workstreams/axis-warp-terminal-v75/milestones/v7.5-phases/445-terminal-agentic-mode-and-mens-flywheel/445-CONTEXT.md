# Phase 445: Terminal-Native Agentic Mode (`vox_actor_runtime::llm`), Live Slash Commands, Co-Pilot Approvals & MENS Flywheel - Context

**Gathered:** 2026-11-15
**Status:** Ready for UI spec & planning
**Mode:** Autonomous Smart Discuss (`--ws axis-warp-terminal-v75`)

<domain>
## Phase Boundary

Enable full agentic control directly inside Vox Axis `Console` (and compact `TerminalDockPanel` + `ChatTranscript` terminal approval cards) backed by live model-agnostic LLM streaming, live orchestrator slash commands, inline Roman wax-seal Co-Pilot approvals, and MENS trajectory curation:

1. **Terminal-Native Agentic Turn Streaming (`TERM-AGENT-01`)**: Submitting `?prompt` or natural-language prose (`intentRouter.ts` / `InputIntent::Agent`) opens an in-band `BlockKind::AgentTurn` block in the active terminal session, routes model selection via `vox_terminal_core::agent::route_agent_prompt` (`vox_orchestrator::models::select_with_default_registry`), and streams a real multi-step response + runnable command plan exclusively through `vox_actor_runtime::llm` (`llm_stream` / `llm_chat` / `infer_with_retry`) — zero hardcoded vendor hostnames or vendor SDKs (`vox audit code` `llm_provider_call` = `0` errors), with graceful offline/no-key diagnostic guidance via Clavis (`vox_secrets::resolve_secret`).
2. **1-Click *Explain / Fix Failed Block* (`TERM-AGENT-02`)**: Every failed command block (`exit_code != 0`, `BlockStatus::Failed`) exposes a 1-click **Explain / Fix Failed Block** CTA (`data-testid="term-explain-fix-block"`) that attaches the 3-stage scrubbed (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact`) failed command + bounded output + live `cwd` / `git_branch` context, streams a concise root-cause diagnosis + proposed repair command into an inline Agent block, and offers 1-click **Run Fix** / **Edit in Prompt**.
3. **Inline Roman Wax-Seal Co-Pilot Command Approval Cards & Autopilot Preemption (`TERM-AGENT-03`)**:
   - Commands parked by `vox_terminal_exec` (`PendingApprovals` / `ExecPolicyV1` `Ask` or `Co-Pilot` mode) and in-terminal proposed agent commands under `Co-Pilot` render inline Roman Scriptorium wax-seal **Command Approval Cards** (`data-testid="term-approval-card"`) in **both** `Console` (`BlockStreamView` / `TerminalSessionPane`) and `Chat` (`ChatTerminalBlockCard` / `TurnTrace`), with 4 actions:
     - **Approve** (`ApprovalOutcome::Approved`)
     - **Edit & Run** (inline command edit -> `ApprovalOutcome::Modified` + executes edited command)
     - **Allow Pattern in Session** (session-scoped prefix/executable allowlist stored in backend session state — never weakening `Deny` or `CREDENTIAL_PROMPT_USER_ONLY`!)
     - **Reject + Steer** (`ApprovalOutcome::Rejected` + optional steering note routed back to the agent turn / A2A inbox)
   - Any operator keystroke, VT `onData`, stdin submit, or prompt submit while a session is in `Autopilot` immediately preempts control mode to `Manual` (`term_session_set_control("manual")` + `ControlModeChanged` event) on both frontend and backend.
4. **Live Orchestrator Slash Commands & MENS Trajectory Flywheel (`TERM-AGENT-04`)**:
   - Replace static placeholder strings in `vox_terminal_core::commands` / GUI dispatch so `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, and `/compact` query and mutate **live** orchestrator / session state (`vox_budget_status` / budget override, `model_active` / `route_agent_prompt`, `vox_skill_list` / `vox_skill_search`, `vox_memory_search` / recent facts, live context-window usage, `/clear` clearing session viewport blocks locally with journal preservation, `/compact` summarizing prior terminal blocks via `vox_actor_runtime::llm` / deterministic fallback into a compact summary block).
   - Wire live terminal sessions + per-block **Accept** / **Reject** / **Correct** feedback buttons (`data-testid="term-feedback-accept"`, `term-feedback-reject`, `term-feedback-correct`) through 3-stage Clavis + `vox-redact` scrubbing (including redacting `Block.input` and `Block.output` inside `TranscriptKind::Accepted` and `TranscriptKind::Rejected` in `CorpusWriter`!) -> `vox-journal` (`JournalSink`) -> `curate` (`DefaultPolicy`) -> `CorpusWriter` JSONL for MENS training.

Covers requirements: `TERM-AGENT-01`, `TERM-AGENT-02`, `TERM-AGENT-03`, `TERM-AGENT-04`.

</domain>

<decisions>
## Implementation Decisions

### 1. Model-Agnostic Terminal Agent Streaming (`TERM-AGENT-01`)
- **D-01 (`vox_actor_runtime::llm` Exclusive Facade):** All LLM invocations for terminal `?prompt`, natural-language prose turns, failed-block diagnosis, and `/compact` summarization MUST call `vox_actor_runtime::llm` (`llm_stream_events` / `llm_stream` / `infer_with_retry`) from `vox-gui` (which already depends on `vox-actor-runtime`, `vox-orchestrator`, `vox-secrets`, and `vox-terminal-core` — zero new Cargo crate edges!). Never instantiate vendor SDKs or hardcode provider URLs (`vox audit code` `llm_provider_call` must exit `0`).
- **D-02 (Structured Agent Block + Proposed Runnable Steps):** Terminal agent turns stream incremental delta chunks into an in-band `BlockKind::AgentTurn` block via `SessionHost` / `TERM_EVENT_EVENT` (`AgentStreamDelta` / `OutputAppended`) and parse fenced runnable shell/vox proposals (` ```sh ` / ` ```vox ` or structured JSON/plan footer) into interactive proposed-command chips (`Run Command`, `Insert into Prompt`, or Co-Pilot approval gating). When no LLM provider key or local MENS endpoint is configured in Clavis, return an actionable diagnostic block naming `vox secrets doctor` / Settings without panicking or hanging.

### 2. 1-Click *Explain / Fix Failed Block* (`TERM-AGENT-02`)
- **D-03 (Scrubbed Failed-Block Context Envelope):** On any block with `status === 'Failed'` or `(exit_code ?? 0) !== 0`, `BlockCard` renders an explicit **Explain / Fix Failed Block** button (`data-testid="term-explain-fix-block"`). Clicking it invokes `term_agent_explain_fix(session_id, block_id)` on the backend, which reads the authoritative block from `SessionHost`, applies 3-stage scrubbing (`strip_ansi_and_normalize` -> `vox_secrets::scrub_resolved_secrets` -> `vox_redact::redact_owned`) + `truncate_terminal_block` (`<= 16 KiB`), attaches live `cwd`, `git_branch`, and shell kind, streams the diagnosis via `vox_actor_runtime::llm`, and surfaces a runnable repair proposal in-band (`data-testid="term-proposed-fix-run"`, `data-testid="term-proposed-fix-edit"`).

### 3. Roman Wax-Seal Co-Pilot Approval Cards & Autopilot Preemption (`TERM-AGENT-03`)
- **D-04 (Shared `TerminalApprovalCard` Across `Console` and `Chat`):** Build a shared Roman Scriptorium wax-seal `<TerminalApprovalCard>` (`crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`) mounted both inline inside `Console` (`TerminalSessionPane` / `BlockStreamView`) and inside `Chat` (`ChatTerminalBlockCard` / `TurnTrace`) whenever a `vox_terminal_exec` HITL approval (`PendingApprovals` + session `approval_id`) or session Co-Pilot command gate is pending.
- **D-05 (4 First-Class Approval Actions + Guarded Session Pattern Allowlist):**
  1. `Approve`: resolves `PendingApprovals` with `ApprovalOutcome::Approved` (or executes the gated terminal command).
  2. `Edit & Run`: opens an inline `<textarea>` / `<input>` pre-filled with the proposed command, re-evaluates the edited command through `evaluate_command_ast` + `evaluate_security_tier` (so editing cannot smuggle a `Deny` command!), resolves `ApprovalOutcome::Modified` (or executes the edited command), and records a `TranscriptKind::Corrected { from, to }` MENS signal!
  3. `Allow Pattern in Session`: records a session-scoped command prefix pattern (e.g. `cargo test`, `pnpm --dir crates/vox-gui/ui`) in backend session state so matching `Ask`-tier commands auto-approve for the remainder of that session — **strictly forbidden** from overriding `ExecPolicy` `Deny` rules, AST subshell/redirection violations, or `CREDENTIAL_PROMPT_USER_ONLY` gates.
  4. `Reject + Steer`: resolves `ApprovalOutcome::Rejected`, records `TranscriptKind::Rejected`, and if the operator entered a steering note, forwards the scrubbed note to the active agent turn / `send_to_agent`.
- **D-06 (Immediate `Autopilot -> Manual` Preemption):** Reinforce end-to-end `Autopilot -> Manual` preemption on both backend (`SessionHost` / `PtyManager`: human-originated `write_stdin` / `submit` while `control_mode == Autopilot` flips `control_mode = Manual` and emits `SessionEvent::ControlModeChanged`) and frontend (`Console` / `TerminalSessionPane` / `ScriptoriumPromptEditor`).

### 4. Live Slash Commands & MENS Trajectory Curation (`TERM-AGENT-04`)
- **D-07 (7 Live Slash Commands):** Implement backend Tauri/MCP-backed execution (`term_slash_exec(session_id, raw_slash)`) for:
  - `/budget [limit]`: reads live budget/spend state (`context_budget` / `llm_spend` / orchestrator budget) and updates session/orchestrator token budget when an argument is supplied.
  - `/model [model_id | route <prompt>]`: lists/reads active model (`model_active` / `ModelRegistry`), switches active session/GUI model override when given `<model_id>`, or explains routing axes via `route_agent_prompt`.
  - `/skills [query]`: queries live `vox-skills` / `vox_skill_list` / `vox_skill_search` registry (never a hardcoded hint string).
  - `/memory [query]`: queries live orchestrator/DB memory (`memory` / `vox_memory_search`) and returns matching entries.
  - `/context`: reports live session context envelope (block count, pinned blocks, `cwd`, `git_branch`, active model, security level, control mode, token estimate).
  - `/clear`: flushes curated events to `JournalSink` before clearing in-memory viewport blocks (`SessionEvent::SnapshotResync { blocks: vec![], ... }`).
  - `/compact`: summarizes completed session blocks into one compact `BlockKind::AgentTurn` summary block (via `vox_actor_runtime::llm` with deterministic local block-digest fallback when offline) and replaces older unpinned blocks while preserving pinned blocks!
- **D-08 (3-Stage Scrubbed `vox-journal` -> `curate` -> `CorpusWriter` MENS Pipeline):**
  - Fix `CorpusWriter::redact_event` (`crates/vox-terminal-core/src/corpus/writer.rs`) so `TranscriptKind::Accepted { block }` and `TranscriptKind::Rejected { block }` scrub `block.input`, `block.output`, and `block.cwd` (plus optional Clavis scrubber callback) before serialization, and update `DefaultPolicy` (or `FeedbackPolicy`) so explicit operator `Rejected` / `Accepted` / `Corrected` preference signals are preserved for DPO/preference curation alongside `JournalSink` persistence under `.vox/terminal-journal/` (workspace-local gitignored / temp-testable).
  - Expose `term_block_feedback(session_id, block_id, verdict, corrected_command?)` (`verdict`: `"accepted" | "rejected" | "corrected"`) and surface compact Roman feedback controls (`Accept`, `Reject`, `Correct`) on agent/completed blocks.

### 5. Non-Negotiable Repo Guardrails
- **D-09 (Zero New Dependencies or Crate Edges):** Keep `Cargo.lock`, `pnpm-lock.yaml`, and `layers.toml` untouched (`./target/debug/vox-arch-check` = `0` errors; run `./target/debug/vox ci crate-api-sync --write` if `vox-terminal-core/src/lib.rs` public items change).
- **D-10 (Submodule & LoC Hygiene `< 500` LoC):** Keep every Rust and TypeScript file `< 450` non-blank LoC. Place new `vox-gui` terminal agent/slash/approval helpers in submodules under `crates/vox-gui/src/commands/pty/` (`pty/agent.rs`, `pty/slash.rs`, `pty/approvals.rs`) with thin `#[tauri::command]` wrappers in `crates/vox-gui/src/commands/pty.rs` so `gui_command_backends` stays 100% green without touching `command-backends.v1.yaml`.
- **D-11 (Static UI & Accessibility Guards):** Zero emojis/dingbats/geometric shapes (`zeroEmojiAndDingbat.test.ts`), zero raw `<select>` (`<VoxSelect>` only), all Tauri IPC in `src/transport.ts` (`transportIpcGuard.test.ts`), `sanitizeErrorForToast` on toasts, strictly 4 font sizes (`11px`, `12px`, `14px`, `18px`) & 2 weights (`400`, `600`), zero `.5` fractional Tailwind classes, and unique `screen.getByRole('textbox')` on initial `Console` mount!

</decisions>

<canonical_refs>
## Canonical References

- `.planning/workstreams/axis-warp-terminal-v75/ROADMAP.md` — Phase 445 goal & 4 success criteria
- `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` — `TERM-AGENT-01..04`
- `crates/vox-actor-runtime/src/llm/mod.rs` — Model-agnostic LLM facade (`llm_chat`, `llm_stream`, `llm_stream_events`, `infer_with_retry`)
- `crates/vox-terminal-core/src/{agent.rs,commands.rs,transcript.rs,session.rs,host.rs,corpus/curate.rs,corpus/writer.rs,corpus/redact.rs}` — Existing routing, slash registry, transcript journal, and MENS corpus writer
- `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_tools.rs` — `PendingApprovals` HITL registration & `ExecPolicyV1` evaluation
- `crates/vox-gui/src/commands/{pty.rs,console_a2a.rs,mission_control.rs,model_active.rs,context_budget.rs,llm_spend.rs,memory.rs}` — Live GUI daemon/DB/host bridges
- `crates/vox-gui/ui/src/components/surfaces/Console/{BlockCard.tsx,BlockStreamView.tsx,TerminalSessionPane.tsx,ScriptoriumPromptEditor.tsx,Console.tsx}` — Terminal UI integration points
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` — Chat inline terminal card

</canonical_refs>

<deferred>
## Deferred Ideas

- Global `index.css` `--font-terminal-mono` `:not(.xterm):not(.xterm *)` CSS containment, 4-theme Dual-Surface Roman Scriptorium token audit across all surfaces, deterministic terminal CV invariants in `audits.ts`/`cvFlywheel.ts`, and multi-pass `900×600`..`1920×1080` Playwright + VLM convergence -> Phase 446 (`TERM-ROMA-CV-01..04`).
- Background speculative next-command ghost synthesis after every block exit -> `v2` (`TERM-FUT-01`).

</deferred>
