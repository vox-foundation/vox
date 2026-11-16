---
phase: 445
slug: terminal-agentic-mode-and-mens-flywheel
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-15"
validated: "2026-11-16"
---

# Phase 445 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution of `445-terminal-agentic-mode-and-mens-flywheel`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-terminal-core`, `vox-orchestrator-mcp`, `vox-orchestrator-d`, `vox-gui --bin vox-gui`, `vox-code-audit`) + Vitest 4.1 + Testing Library (`crates/vox-gui/ui`) |
| **Config file** | `crates/vox-gui/ui/vitest.config.ts` |
| **Quick run command** | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalApprovalCard.test.tsx src/components/surfaces/Console/AgentTurnContent.test.tsx src/components/surfaces/Console/SlashMetaContent.test.tsx` |
| **Full suite command** | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core` · `... cargo test -p vox-orchestrator-d` · `... cargo test -p vox-orchestrator-mcp pending_approvals` · `... cargo test -p vox-orchestrator-mcp terminal` · `... cargo test -p vox-orchestrator-mcp daemon_extra` · `... cargo test -p vox-gui --bin vox-gui pty` · `... cargo test -p vox-gui --bin vox-gui remote_gate_fixture_matches_real_gate` · `timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` · `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=2 src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts` |
| **Estimated runtime** | ~69 seconds (36-file Vitest gate suite) + scoped Rust unit/integration suites + static hygiene gates |

---

## Sampling Rate

- **After every task:** Run the task-specific `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p <crate> ...` or `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run <target>` command.
- **After every plan wave:** Run the wave's combined Rust + Vitest + `vox-code-audit` (`llm_provider_call`) verification commands and verify `< 450` non-blank LoC on every touched `.rs`, `.ts`, and `.tsx` file.
- **Before `/gsd-verify-work`:** Full suite must be green, including `src/guards/zeroEmojiAndDingbat.test.ts`, `src/guards/noRawNativeControlsGuard.test.ts`, `src/guards/toastBodyGuard.test.ts`, `Console.test.tsx`, `ChatTerminalBlockCard.test.tsx`, and `vox audit code --path crates/vox-gui`.
- **Max feedback latency:** 45 seconds per task test run.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 445-01-01 | 01 | 1 | TERM-AGENT-01, TERM-AGENT-03, TERM-AGENT-04 | T-445-02 | `Session` custom block streaming (`begin_custom_block`, `append_block_chunk`, `finish_custom_block` without touching `open_id`), `/clear` & `/compact` `SnapshotResync`, `control_epoch`, and sorted `/clear` + `/compact` `CommandRegistry` entries | Rust unit | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core session commands` | ✅ | ✅ green |
| 445-01-02 | 01 | 1 | TERM-AGENT-04 | T-445-01 | `CorpusWriter` + `RedactingTranscriptSink` 3-stage redaction across `Accepted` & `Rejected` blocks, unconditional `credential_prompt` drop, and `DefaultPolicy` retaining non-empty `Rejected` blocks | Rust unit | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core corpus transcript` | ✅ | ✅ green |
| 445-01-03 | 01 | 1 | TERM-AGENT-01, TERM-AGENT-04 | T-445-01, T-445-03 | `SessionHost` (`LocalHost` + `DaemonHost::route("snapshot")`), `TermDispatch` RPC parity + `refuse_agent_origin`, `JournalSink` under `.vox/terminal-journal/`, and `crate-api-sync` | Rust unit + daemon integration | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core` · `... cargo test -p vox-orchestrator-d` | ✅ | ✅ green |
| 445-02-01 | 02 | 1 | TERM-AGENT-03 | T-445-05, T-445-06 | `PendingApprovals` metadata (`PendingApprovalMeta`), dual-path `PendingResolution` across `tasks_core.rs` (`vox_resolve_approval` + `floor_gate`) and `daemon_extra.rs` (`RESOLVE_APPROVAL`), and boundary-safe `matches_leaf_command` | Rust unit | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp pending_approvals` · `... daemon_extra` · `--lib floor_gate` · `--test pending_approvals_tests` · `--test pending_approvals_floor_tests` | ✅ | ✅ green |
| 445-02-02 | 02 | 1 | TERM-AGENT-03 | T-445-04, T-445-05, T-445-07 | `terminal_approvals.rs` + `terminal_tools.rs`: compound-safe `can_auto_approve_in_session` (`extract_effective_commands`), `Edit & Run` AST/tier re-validation, `Reject + Steer` note scrubbing, and `Autopilot -> Manual` preemption | Rust unit | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp terminal` | ✅ | ✅ green |
| 445-03-01 | 03 | 2 | TERM-AGENT-01, TERM-AGENT-02 | T-445-08, T-445-09 | `pty/agent.rs` (`term_agent_prompt_impl`, `term_agent_explain_fix_impl`, `term_agent_cancel_impl`) via `vox_actor_runtime::llm::llm_stream_activity` + `futures_util::StreamExt`, Clavis/local-MENS discovery, 3-stage scrubbing, and `credential_prompt` refusal | Rust unit + `toestub` | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui pty` · `./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/src` | ✅ | ✅ green |
| 445-03-02 | 03 | 2 | TERM-AGENT-01, TERM-AGENT-04 | T-445-08, T-445-10 | `pty/slash.rs` (`term_slash_exec_impl` live `/budget`, `/model`, `/skills`, `/memory`, `/context`, `/clear`, `/compact`), `pty/approvals.rs` (`term_block_feedback_impl`), `pty.rs` + `main.rs` registration, and `remote_gate_fixture_matches_real_gate` | Rust unit | `timeout 600s env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui pty` · `... remote_gate_fixture_matches_real_gate` | ✅ | ✅ green |
| 445-03-03 | 03 | 2 | TERM-AGENT-01, TERM-AGENT-02, TERM-AGENT-03, TERM-AGENT-04 | T-445-10 | `mcpToolResult.ts` (`PendingApprovalRow` terminal metadata) + `transport.ts` (`termAgentPrompt`, `termAgentExplainFix`, `termAgentCancel`, `termSlashExec`, `termBlockFeedback`, `listTerminalPendingApprovals`, `resolveTerminalApproval`) | Vitest + `tsc` | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts` · `timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` | ✅ | ✅ green |
| 445-04-01 | 04 | 3 | TERM-AGENT-03 | T-445-12, T-445-13 | `useTerminalPendingApprovals.ts` + semantic `<Icon.waxSeal>` `TerminalApprovalCard.tsx` (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`, lazy-mounted inputs) across `Console` and `ChatTerminalBlockCard` / `TurnTrace` | Vitest + RTL | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/TerminalApprovalCard.test.tsx src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx` | ✅ | ✅ green |
| 445-04-02 | 04 | 3 | TERM-AGENT-01, TERM-AGENT-02, TERM-AGENT-04 | T-445-11, T-445-13 | `AgentTurnContent.tsx`, `SlashMetaContent.tsx` (with lazy-mounted `TermBlockFeedbackStrip`), `BlockCard.tsx` (`Explain / Fix Failed Block` `term-explain-fix-block` hidden on `credential_prompt`), and `BlockStreamView.tsx` | Vitest + RTL | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Console/AgentTurnContent.test.tsx src/components/surfaces/Console/SlashMetaContent.test.tsx src/components/surfaces/Console/BlockStreamView.test.tsx` | ✅ | ✅ green |
| 445-04-03 | 04 | 3 | TERM-AGENT-01, TERM-AGENT-02, TERM-AGENT-03, TERM-AGENT-04 | T-445-11, T-445-12, T-445-13 | `TerminalSessionPane.tsx` + `Console.tsx` (`?` / `/slash` routing, inline approvals, `Autopilot` preemption banner, unique `getByRole('textbox')` on initial mount) + full Rust/Vitest/guard/audit verification | Full Gate Suite | `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=2 src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements:
- `vox-terminal-core`, `vox-orchestrator-mcp`, `vox-orchestrator-d`, `vox-gui`, and `vox-code-audit` unit/integration test runners are active and compile cleanly.
- `crates/vox-gui/ui/vitest.config.ts`, `@testing-library/react`, `Console.test.tsx`, `ChatTerminalBlockCard.test.tsx`, and `zeroEmojiAndDingbat.test.ts` are active.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Live cloud/local SSE token arrival cadence & Carrara/brass wax-seal visual polish on `<TerminalApprovalCard>` | TERM-AGENT-01, TERM-AGENT-03 | Headless Vitest verifies streaming state reducer transitions and DOM attributes, while live provider token latency and gold/crimson wax-seal sheen are visual | Launch Axis Desktop -> open `Console`, submit `?explain how cargo check works` (observe live streaming `AgentTurn`), run `false` and click **Explain / Fix**, then trigger an agent terminal command in `Co-Pilot` mode to inspect the wax-seal approval card in both `Console` and `Chat`. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (`11/11` verification rows green; `141/141` `vox-terminal-core`, `50/50` `vox-orchestrator-d`, `4/4` `pending_approvals` + `8/8` `terminal` + `3/3` `daemon_extra` + `15/15` `pending_approvals_tests` + `8/8` `pending_approvals_floor_tests` + `9/9` `floor_gate` in `vox-orchestrator-mcp`, `35/35` `vox-gui --bin vox-gui pty` + `1/1` `remote_gate_fixture_matches_real_gate`, `36/36` Vitest files / `213/213` tests, `tsc --noEmit` exit `0`, `toestub --rules vox/llm-provider-call` `0` violations, `vox ci crate-api-sync` OK `131 crates`, `vox-arch-check` exit `0`)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 45s per scoped task check (`69.11s` across full 36-file Vitest gate suite)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-11-16
