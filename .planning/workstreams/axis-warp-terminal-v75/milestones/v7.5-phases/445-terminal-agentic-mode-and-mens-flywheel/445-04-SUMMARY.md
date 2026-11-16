---
phase: 445-terminal-agentic-mode-and-mens-flywheel
plan: 04
subsystem: gui-console-and-chat-terminal-bridge
tags:
  - axis-gui
  - terminal-agentic-mode
  - pending-approvals
  - mens-flywheel
  - phase-445
requirements:
  - TERM-AGENT-01
  - TERM-AGENT-02
  - TERM-AGENT-03
  - TERM-AGENT-04
key_files:
  created:
    - crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.test.tsx
  modified:
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
    - crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx
status: complete
---

# Phase 445 Plan 04 Summary: Console Agentic Mode, Approval Cards, Slash Meta & MENS Flywheel UI

Implemented the Phase 445 Plan 04 UI surface across the Axis Console and Chat terminal bridge (`TERM-AGENT-01`, `TERM-AGENT-02`, `TERM-AGENT-03`, `TERM-AGENT-04`) within the exact 16-file boundary and under the `430` non-blank LoC budget on every file.

## What Was Built

1. **Shared Pending Approvals & Agentic Hook (`crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`)**
   - Added `deriveSessionAllowPattern` (token-boundary safe prefix matching `PendingApprovals::matches_leaf_command` semantics without trailing `*`) and `matchesSessionPattern` (rejecting compound shell operators).
   - Implemented `useTerminalPendingApprovals`: polls `listTerminalPendingApprovals`, supports local Co-Pilot command gating (`triggerPreemptionNotice`, `resumeCoPilot`, `dismissPreemptedBanner`), handles dual local/daemon `resolveApproval` with inline `Edit & Run` validation errors and `'corrected'` MENS trajectory feedback, dispatches `?` / `/ai` / `/ask` / `agent` prompts (`termAgentPrompt`), `/slash` commands (`termSlashExec`), and shell/`!nu`/`.vox` inputs (`termSessionSubmit`), plus `runExplainFix`, `submitBlockFeedback`, `cancelAgentTurn`, and `try/catch` partial-mock-safe `getTransportFn` helpers.
   - Normalized backend error toasts through `sanitizeErrorForToast(err)`.

2. **Terminal Approval Card & Preemption Banner (`TerminalApprovalCard.tsx`, `TerminalApprovalCard.test.tsx`)**
   - Rendered `CO-PILOT GATE`, `SAFETY FLOOR GATE`, and `CREDENTIAL GATE` with semantic wax-seal signet pill (`data-roman-wax-seal="true"` + `<Icon.waxSeal className="size-3 shrink-0" />`), pure 7-bit ASCII copy, 4px spacing grid, and valid tokens (`--color-status-pass`, `--color-status-warn`, `--color-status-fail`, `text-accent`).
   - Implemented 4 approval actions (`Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`) while keeping `term-approval-edit-input` and `term-approval-steer-input` lazy-mounted as `null` when idle so `screen.getByRole('textbox')` stays unique on initial `<Console />` mount.
   - Added `<TerminalPaneApprovalBanner>` with `term-autopilot-preempted-banner` (`Resume Co-Pilot`, `Dismiss Notice`) and `term-approval-tray` (`1 of N pending`).

3. **Agent Turn, Slash Meta, Explain/Fix & MENS Feedback (`AgentTurnContent.tsx`, `SlashMetaContent.tsx`, `BlockCard.tsx`, `BlockStreamView.tsx`)**
   - `<AgentTurnContent>` parses `[agent-model: ...]` headers (`ollama - mens-local` normalized to `ollama/mens-local`), renders streaming pulse (`Routing prompt through model registry...`), extracts fenced commands or `VOX_PROPOSED_COMMAND:` into `Run Fix` / `Run Command`, `Edit in Prompt`, and `Copy`, handles `[clavis-unconfigured]` diagnostics (`Open Keys & Secrets`, `Run Secrets Doctor`), and offers `Retry Agent Turn` on `[stream-error]`.
   - `<SlashMetaContent>` renders `/compact` (`term-compact-digest-card`), `/budget`, `/model`, `/skills`, `/memory`, and `/context` meta blocks, plus `<TermBlockFeedbackStrip>` (hidden when `credential_prompt === true`, WCAG 2.5.3 `aria-label` matching, lazy-mounted `term-feedback-correct-input`).
   - Updated `BlockCard.tsx` and `BlockStreamView.tsx` to delegate `AgentTurn` and `SlashCommand`/`Meta` blocks, add `Explain / Fix Failed Block` (`term-explain-fix-block` / `Diagnosing Failed Block...`), wire `<TermBlockFeedbackStrip>`, handle `/clear` empty state (`clearedEmpty`), and preserve Phase 444 `JsonLedgerTable`.

4. **Console & Chat Unification (`TerminalSessionPane.tsx`, `Console.tsx`, `ChatTerminalBlockCard.tsx`, `TurnTrace.tsx`)**
   - Wired `TerminalSessionPane.tsx` and `Console.tsx` to `useTerminalPendingApprovals`, rendering `<TerminalPaneApprovalBanner>`, Explain/Fix and feedback handlers, Co-Pilot gating, and preserving all Phase 444 Warp features (`ScriptoriumPromptEditor`, `useConsoleWarpActions`, `JsonLedgerTable`, `DiscoveryRail`, `RunbookPanel`, `HistorySearchPopover`, `term-subshell-voxify-banner`, `classifyPromptIntent`).
   - Wired `ChatTerminalBlockCard.tsx` and `TurnTrace.tsx` to share `useTerminalPendingApprovals` (when terminal tools are active) and render `<TerminalApprovalCard compact />` inline in Chat while safely handling partially mocked transport modules.

## Non-Blank LoC Check (`rg -c '\S'`)

All 16 files verified strictly under `430` non-blank lines after final edit:

- `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`: `373`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`: `413`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.test.tsx`: `164`
- `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx`: `358`
- `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.test.tsx`: `154`
- `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx`: `343`
- `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.test.tsx`: `120`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`: `358`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`: `202`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx`: `385`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`: `393`
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`: `415`
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx`: `417`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx`: `213`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx`: `370`
- `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx`: `157`

## Verification

### UI Checks (run after final edit)

- TypeScript check: `timeout 240s pnpm --dir crates/vox-gui/ui exec tsc --noEmit` -> exit `0`
- Vitest suite: `timeout 240s pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts`
  - Result: `Test Files 36 passed (36), Tests 204 passed (204)` (`3.68s`)
  - Includes: `TerminalApprovalCard.test.tsx` (`6` tests), `AgentTurnContent.test.tsx` (`5` tests), `SlashMetaContent.test.tsx` (`3` tests), `BlockStreamView.test.tsx` (`10` tests), `TerminalSessionPane.test.tsx` (`6` tests), `Console.test.tsx` (`17` tests), `ChatTerminalBlockCard.test.tsx` (`8` tests), `TurnTrace.test.tsx` (`9` tests), `src/guards/*`, `src/lib/themeContract.test.ts`, and `src/transport.console.test.ts`.
- UI token/spacing/icon/ASCII guard grep across all 16 files -> exit `0` with zero `.5` spacing, zero `status-ok`/`status-error`/`text-brass`, zero `font-bold`/`font-medium`, zero `RomanWaxSealBadge`, zero `<select`, zero `<Button>` `icon=`/`trailingIcon=`, and zero new non-ASCII characters (only pre-existing characters in `TurnTrace.tsx`, `ChatTerminalBlockCard.test.tsx`, and `Console.tsx`).

### Rust, Crate Sync & Audit Checks (no Rust files modified in Plan 04)

- `./target/debug/vox ci crate-api-sync` -> `crate-api-sync OK (131 crates)` (exit `0`)
- `./target/debug/vox-arch-check` -> exit `0` (pre-existing LoC delta warnings only)
- `./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/src` -> `No issues found — code is clean!` (exit `0`)
- `./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/ui/src` -> `No issues found — code is clean!` (exit `0`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core` -> exit `0` (lib `82`, `agent` `3`, `block` `5`, `commands` `2`, `host` `1`, `input` `10`, `osc633` `5`, `pty` `5`, `session` `7`, `shell_init_and_stateful_pty` `6`, `smoke` `1`, `streaming_utf8_and_alt_screen` `9`, `transcript` `2`, `vox_interp` `1`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-d` -> exit `0` (lib `37`, bin `10`, `boot_profiles` `3`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp pending_approvals` -> exit `0` (lib `3`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp terminal_approvals` -> exit `0` (lib `3`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-orchestrator-mcp terminal_tools` -> exit `0` (lib `4`)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui pty` -> exit `0` (`35` passed, `366` filtered out)
- `env RUSTC_WRAPPER="" CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-gui --bin vox-gui remote_gate_fixture_matches_real_gate` -> exit `0` (`1` passed, `400` filtered out)
