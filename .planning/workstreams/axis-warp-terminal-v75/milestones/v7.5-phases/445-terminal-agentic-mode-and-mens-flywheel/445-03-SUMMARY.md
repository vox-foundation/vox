---
phase: 445-terminal-agentic-mode-and-mens-flywheel
plan: 03
subsystem: gui-terminal
tags: [vox-gui, pty, agent-mode, slash-commands, mens-flywheel, llm-facade, clavis, typescript-ipc]

requires:
  - phase: 445-terminal-agentic-mode-and-mens-flywheel
    plan: 01
    provides: SessionHost custom block streaming, /clear & /compact SnapshotResync, control_epoch preemption, and RedactingTranscriptSink / JournalSink
  - phase: 445-terminal-agentic-mode-and-mens-flywheel
    plan: 02
    provides: Enriched terminal PendingApprovalRow metadata and vox_terminal_resolve_approval MCP tool
  - phase: 444-warp-editor-completions-and-runbooks
    provides: PtyWarpState and term_history_*, term_complete, term_runbooks_list, term_session_voxify commands
provides:
  - Model-agnostic terminal LLM streaming (`term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`) via `vox_actor_runtime::llm::{LlmConfig::direct, llm_stream_activity, llm_chat}` + Clavis / local MENS resolution (`MensServeState::running_port`)
  - Live terminal slash dispatcher (`term_slash_exec`) covering `/ai`, `/sh`, `/nu`, `/clear`, `/compact`, `/help`, `/model`, `/budget`, `/skills`, `/memory`, and `/context`
  - Terminal MENS flywheel feedback IPC (`term_block_feedback`) with 3-stage `scrub_for_agent_inbox` redaction, `credential_prompt` exclusion, and durable `LocalHost` `.vox/terminal-journal` fallback
  - Typed TypeScript IPC surface (`TermAgentTurnDto`, `TermSlashResultDto`, `TermFeedbackResultDto`, `ResolveTerminalApprovalInput`, `termAgentPrompt`, `termAgentExplainFix`, `termAgentCancel`, `termSlashExec`, `termBlockFeedback`, `listTerminalPendingApprovals`, `resolveTerminalApproval`) and extended `PendingApprovalRow`
affects: [445-04, vox-gui]

actuals:
  tokens: 28000
  tasks: 3
  commits: 0

tech-stack:
  added: []
  patterns:
    - "100% model-agnostic terminal LLM routing via `vox_actor_runtime::llm::{LlmConfig::direct, llm_stream_activity, llm_chat}` + `futures_util::StreamExt::next` with Clavis `SecretId` resolution (`\"gemini\"` for `GeminiApiKey`) and local MENS discovery via `MensServeState::running_port` (`\"ollama\"` loopback `base_url`)"
    - "Dual cancellation (`CancellationToken` + `SessionHost::control_epoch` polling) guaranteeing cancelled/preempted `AgentTurn` blocks close as `BlockStatus::Cancelled`"
    - "Submodule decomposition (`pty/agent.rs`, `pty/slash.rs`, `pty/approvals.rs`, `pty/tests.rs`, `pty/agent_tests.rs`) keeping every `pty` Rust file strictly `< 420` non-blank LoC"

key-files:
  created:
    - crates/vox-gui/src/commands/pty/agent.rs
    - crates/vox-gui/src/commands/pty/slash.rs
    - crates/vox-gui/src/commands/pty/approvals.rs
    - crates/vox-gui/src/commands/pty/agent_tests.rs
  modified:
    - crates/vox-gui/src/commands/mens_serve.rs
    - crates/vox-gui/src/commands/pty.rs
    - crates/vox-gui/src/commands/pty/tests.rs
    - crates/vox-gui/src/main.rs
    - crates/vox-gui/ui/src/lib/mcpToolResult.ts
    - crates/vox-gui/ui/src/transport.ts
    - crates/vox-gui/ui/src/transport.console.test.ts

key-decisions:
  - "Exposed `pub(crate) fn running_port(&self) -> Option<u16>` on `MensServeState` (`mens_serve.rs`) and mapped local MENS HTTP servers to `LlmConfig::direct(\"ollama\", format!(\"vox-mens:{port}\"))` with `.with_base_url(format!(\"http://127.0.0.1:{port}\"))` instead of inventing a non-existent `SecretId::VoxLocalEndpoint`."
  - "Mapped `SecretId::GeminiApiKey` strictly to `vox_config::resolve_egress`'s canonical `\"gemini\"` provider slug (never `\"google\"`), with zero hardcoded vendor URLs anywhere in `vox-gui`."
  - "Preserved 100% of Phase 444 (`pub(crate) mod warp;`, `PtyWarpState`, `term_history_*`, `term_complete`, `term_runbooks_list`, `term_session_voxify`, and `pty/warp::tests`) while splitting new async agent/slash/feedback tests into `pty/agent_tests.rs` so every `pty` `.rs` file stays `< 420` non-blank LoC."
  - "Verified `State<'_, Arc<MensServeState>>`, `State<'_, Arc<BackendState>>`, `State<'_, Arc<PersistentDaemon>>`, and `State<'_, PtyManager>` against `crates/vox-gui/src/main.rs` `app.manage(...)` registrations so Tauri command extraction never panics at runtime."

patterns-established:
  - "Explain & Fix 3-stage context scrubbing: `scrub_for_agent_inbox` + 8 KiB / 80-line tail cap + unconditional `CREDENTIAL_PROMPT_EXPLAIN_DENIED` refusal on `credential_prompt == true` blocks before any LLM call."
  - "Pure ASCII `-` separators across all `pty/agent.rs` and `pty/slash.rs` user-facing strings (zero middle dots `·` and zero emojis/dingbats)."

requirements-completed: [TERM-AGENT-01, TERM-AGENT-02, TERM-AGENT-04]

coverage:
  - id: D1
    description: "`term_agent_prompt`, `term_agent_explain_fix`, and `term_agent_cancel` stream through `vox_actor_runtime::llm::llm_stream_activity` with Clavis/local-MENS discovery, 3-stage scrubbing, `credential_prompt` refusal, and `CancellationToken` + `control_epoch` preemption."
    requirement: TERM-AGENT-01
    verification:
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty/tests.rs#terminal_llm_config_resolution_and_clavis_slugs"
        status: pass
      - kind: unit
        ref: "crates/vox-gui/src/commands/pty/tests.rs#explain_fix_scrubs_caps_and_refuses_credential_blocks"
        status: pass
      - kind: integration
        ref: "crates/vox-gui/src/commands/pty/agent_tests.rs#streaming_agent_prompt_and_explain_fix_via_llm_facade"
        status: pass
      - kind: integration
        ref: "crates/vox-gui/src/commands/pty/agent_tests.rs#agent_turn_cancel_and_control_epoch_preemption_close_block_cancelled"
        status: pass
    human_judgment: false
  - id: D2
    description: "`term_slash_exec` executes all 11 live slash commands (`/ai`, `/sh`, `/nu`, `/clear`, `/compact`, `/help`, `/model`, `/budget`, `/skills`, `/memory`, `/context`) against `SessionHost` + `GuiBackend`, and `term_block_feedback` persists redacted verdicts to `.vox/terminal-journal` while refusing `credential_prompt` blocks."
    requirement: TERM-AGENT-04
    verification:
      - kind: integration
        ref: "crates/vox-gui/src/commands/pty/agent_tests.rs#live_slash_commands_and_compact_digest_execute_end_to_end"
        status: pass
      - kind: integration
        ref: "crates/vox-gui/src/commands/pty/agent_tests.rs#block_feedback_persists_redacted_journal_and_refuses_credential_blocks"
        status: pass
    human_judgment: false
  - id: D3
    description: "TypeScript IPC surface (`mcpToolResult.ts` + `transport.ts`) exposes typed terminal agent, slash, feedback, and HITL approval helpers with backward-compatible `parsePendingApprovals`."
    requirement: TERM-AGENT-02
    verification:
      - kind: unit
        ref: "crates/vox-gui/ui/src/transport.console.test.ts"
        status: pass
      - kind: typecheck
        ref: "crates/vox-gui/ui/tsconfig.json"
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-11-15
status: complete
---

# Phase 445 Plan 03: GUI Terminal Agentic Streaming, Live Slash Commands, MENS Feedback Persistence, and TypeScript IPC Summary

**Shipped model-agnostic terminal LLM streaming (`term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`), 11 live terminal slash commands (`term_slash_exec`), redacted MENS flywheel feedback persistence (`term_block_feedback`), and typed TypeScript IPC + HITL approval wrappers across `vox-gui` while preserving 100% of Phase 444.**

## Performance

- **Tasks:** 3
- **Files touched:** 11 source/test files (`4` created, `7` modified — left uncommitted in shared worktree per orchestrator Git rule)
- **Non-blank LoC audit (`rg -c "\S"` — all strictly `< 420`):**
  - `crates/vox-gui/src/commands/pty.rs`: `404`
  - `crates/vox-gui/src/commands/pty/agent.rs`: `398`
  - `crates/vox-gui/src/commands/pty/slash.rs`: `318`
  - `crates/vox-gui/src/commands/pty/approvals.rs`: `165`
  - `crates/vox-gui/src/commands/pty/tests.rs`: `343`
  - `crates/vox-gui/src/commands/pty/agent_tests.rs`: `408`

## Accomplishments

- **Task 1 (`mens_serve.rs`, `pty/agent.rs`, `pty/approvals.rs`):**
  - Added `MensServeState::running_port(&self) -> Option<u16>` in `crates/vox-gui/src/commands/mens_serve.rs` by probing the live child process state (`try_wait().ok().flatten().is_none()`).
  - Implemented `resolve_terminal_llm_config` / `resolve_terminal_llm_config_with` in `crates/vox-gui/src/commands/pty/agent.rs` honoring operator `model_get_active` preferences first, then active local MENS (`LlmConfig::direct("ollama", ...)` with loopback `.with_base_url("http://127.0.0.1:<port>")`), then Clavis `resolve_secret` (`AnthropicApiKey` -> `"anthropic"`, `OpenaiApiKey` -> `"openai"`, `GeminiApiKey` -> `"gemini"`, `OpenRouterApiKey` -> `"openrouter"`), and emitting `CLAVIS_UNCONFIGURED_DIAG` inside a closed `BlockKind::AgentTurn` block (`exit_code = 1`) when unconfigured.
  - Routed 100% of terminal LLM streaming and summarization through `vox_actor_runtime::llm::{llm_stream_activity, llm_chat}` + `futures_util::StreamExt::next` with dual cancellation (`CancellationToken` + `SessionHost::control_epoch` polling) and 3-stage `scrub_for_agent_inbox` + 8 KiB / 80-line tail capping + `CREDENTIAL_PROMPT_EXPLAIN_DENIED` enforcement on `term_agent_explain_fix`.
  - Implemented `build_local_host_with_journal` / `build_local_host_with_journal_at` and `term_block_feedback_impl` in `crates/vox-gui/src/commands/pty/approvals.rs` wiring `.vox/terminal-journal` (`transcript.journal` + `corpus.jsonl` with `vox_secrets::scrub_resolved_secrets`) on `PtyManager::local_host`.
- **Task 2 (`pty/slash.rs`, `pty.rs`, `main.rs`, `pty/tests.rs`, `pty/agent_tests.rs`):**
  - Implemented `term_slash_exec_impl` in `crates/vox-gui/src/commands/pty/slash.rs` handling `/ai`, `/sh`, `/nu` (`voxify_subshell(Some("nu"))`), `/clear` (`clear_unpinned_blocks`), `/compact` (digest synthesis via `summarize_blocks_with_llm` with deterministic scrubbed Markdown fallback + `compact_blocks(..., 4)`), `/help`, `/model`, `/budget`, `/skills`, `/memory`, `/context`, and unknown `/foo` (`exit_code = 1`).
  - Registered `term_agent_prompt`, `term_agent_explain_fix`, `term_agent_cancel`, `term_slash_exec`, and `term_block_feedback` in `crates/vox-gui/src/commands/pty.rs` and `crates/vox-gui/src/main.rs` (`axis_invoke_handler!`), preserving all Phase 444 `PtyWarpState` and `term_history_*` / `term_complete` / `term_runbooks_list` / `term_session_voxify` commands.
- **Task 3 (`mcpToolResult.ts`, `transport.ts`, `transport.console.test.ts`):**
  - Extended `PendingApprovalRow` and `parsePendingApprovals` in `crates/vox-gui/ui/src/lib/mcpToolResult.ts` with optional `floor_category`, `session_id`, `command`, `control_mode`, and `credential_prompt` fields while maintaining backward compatibility for 4-field rows.
  - Exported `TermAgentTurnDto`, `TermSlashResultDto`, `TermFeedbackResultDto`, `ResolveTerminalApprovalInput`, `termAgentPrompt`, `termAgentExplainFix`, `termAgentCancel`, `termSlashExec`, `termBlockFeedback`, `listTerminalPendingApprovals`, and `resolveTerminalApproval` (plus `VoxTransport` methods) in `crates/vox-gui/ui/src/transport.ts`, covered by unit tests in `crates/vox-gui/ui/src/transport.console.test.ts`.

## Verification Proof Outputs

### 1. Rust `pty` Unit & Integration Suite (`35 passed; 0 failed`)
Command:
`timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui pty`

```text
running 35 tests
...
test commands::pty::agent_tests::unconfigured_agent_turn_records_diagnostic_block ... ok
test commands::pty::tests::local_fallback_session_host_lifecycle ... ok
test commands::pty::tests::explain_fix_scrubs_caps_and_refuses_credential_blocks ... ok
test commands::pty::warp::tests::warp_helpers_scrub_history_complete_tokens_list_runbooks_and_voxify_session ... ok
test commands::pty::agent_tests::block_feedback_persists_redacted_journal_and_refuses_credential_blocks ... ok
test commands::pty::agent_tests::agent_turn_cancel_and_control_epoch_preemption_close_block_cancelled ... ok
test commands::pty::agent_tests::streaming_agent_prompt_and_explain_fix_via_llm_facade ... ok
test commands::pty::tests::terminal_llm_config_resolution_and_clavis_slugs ... ok
test commands::pty::agent_tests::live_slash_commands_and_compact_digest_execute_end_to_end ... ok

test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 366 filtered out; finished in 12.59s
```

### 2. Remote Manifest Gate Parity (`1 passed; 0 failed`)
Command:
`timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui remote_gate_fixture_matches_real_gate`

```text
running 1 test
test remote::manifest::tests::remote_gate_fixture_matches_real_gate ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 400 filtered out; finished in 0.00s
```

### 3. TypeScript Vitest Transport Suite (`8 passed`)
Command:
`timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts`

```text
 ✓ src/transport.console.test.ts (8 tests) 11ms
   ✓ console transport > discoverySuggest forwards typed + limit to invoke 4ms
   ✓ console transport > ptyWrite forwards tab id + data 0ms
   ✓ console transport > termSession* wrappers forward SessionHost IPC commands 1ms
   ✓ console transport > termSessionAttach preserves 1-arg payload and sends afterSeq only when non-null 0ms
   ✓ console transport > termHistory*, termComplete, termRunbooksList, and termSessionVoxify invoke expected IPC endpoints and keys 1ms
   ✓ console transport > terminalBridge publishes and consumes RunbookLaunchRequest payloads across Omnibar and Console 1ms
   ✓ console transport > parsePendingApprovals preserves optional terminal fields and stays backward-compatible 0ms
   ✓ console transport > termAgent*, termSlashExec, termBlockFeedback, and terminal approval helpers invoke exact contracts 2ms

 Test Files  1 passed (1)
      Tests  8 passed (8)
```

### 4. TypeScript Typecheck (`tsc --noEmit`)
Command:
`timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit`

```text
Command exited with code 0. (0 errors)
```

### 5. Static Policy Audit (Vendor URLs, `tokio_stream`, `VoxLocalEndpoint`, Middle Dots)
Command:
`rg -n "api\.anthropic|api\.openai|generativelanguage\.googleapis|openrouter\.ai|tokio_stream|VoxLocalEndpoint|·" crates/vox-gui/src/commands/pty/agent.rs crates/vox-gui/src/commands/pty/slash.rs crates/vox-gui/src/commands/pty/approvals.rs`

```text
0 matches (exit code 1)
```
