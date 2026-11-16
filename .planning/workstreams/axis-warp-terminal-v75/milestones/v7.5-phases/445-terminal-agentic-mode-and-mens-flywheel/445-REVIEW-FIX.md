---
phase: 445-terminal-agentic-mode-and-mens-flywheel
fixed_at: 2026-11-16T01:36:00Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-REVIEW.md
ui_review_path: .planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-UI-REVIEW.md
iteration: 1
findings_in_scope: 22
fixed: 22
skipped: 0
status: all_fixed
---

# Phase 445: Code & UI Review Fix Report

**Fixed at:** 2026-11-16T01:36:00Z
**Source reviews:**
- `.planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-REVIEW.md`
- `.planning/workstreams/axis-warp-terminal-v75/phases/445-terminal-agentic-mode-and-mens-flywheel/445-UI-REVIEW.md`
**Iteration:** 1

**Summary:**
- Code-review findings (`445-REVIEW.md`): `13` (`CR-01..05`, `WR-01..06`, `IN-01..02`) — Rust fixes verified in-tree + all UI sides fixed.
- UI-review findings (`445-UI-REVIEW.md`): `2` blockers (`UI-BL-01`, `UI-BL-02`) + warnings across Copywriting, Visuals, Color, Typography, Spacing, and Experience Design (`UI-WR-01..07`) — fixed within strict Phase 445 TS/TSX allowlist while preserving 100% of Phase 446-02 dual-surface styling (`data-terminal-well="true"`, `data-terminal-mono="true"`, `TERMINAL_MONO_STYLE`, `--terminal-well-*` variables).
- Commit mode: `uncommitted in-tree` (orchestrator owns `git add` / `commit`).

---

## Verified Rust Fixes (`445-REVIEW.md`)

### CR-01: `LocalHost::input` preemption & open agent PTY/turn interruption
**Files verified:** `crates/vox-terminal-core/src/host.rs` (`L464-481`, `L585-589`, `L833-848`)
**Landed fix:** Extracted `preempt_agent_turns_and_pty(&entry, emit_mode, force_bump)` and invoked it from both `LocalHost::input` (`force_bump = true` when flipping `Autopilot -> Manual` on non-empty user bytes) and `LocalHost::set_control_mode`, calling `s.bump_control_epoch()`, emitting `SessionEvent::ControlModeChanged`, sending `TermSignal::Interrupt` to an open `BlockOrigin::Agent` PTY block, and waking `entry.settled_cv.notify_all()`.

### CR-02 (Rust): Pre-commit validation for `Edit & Run` (`Modified`) & `[floor:` prefix guard
**Files verified:** `crates/vox-orchestrator-mcp/src/dispatch_domains/terminal_approvals.rs` (`L174-206`), `crates/vox-orchestrator-mcp/src/dispatch_domains/tasks_core.rs` (`L146-158`), `crates/vox-orchestrator-mcp/src/daemon_extra.rs` (`L293-306`), `crates/vox-orchestrator-mcp/src/pending_approvals.rs` (`L240-258`)
**Landed fix:** Added `validate_terminal_resolution_before_commit` invoked by both `vox_resolve_approval` (`tasks_core.rs`) and daemon `RESOLVE_APPROVAL` (`daemon_extra.rs`) before `pending_approvals.resolve_with_detail`, keeping the pending entry intact and returning an error without mutating `session_allowlist` when `modified_command` fails AST or floor-escalation validation. `PendingApprovals::resolve_with_detail` also checks `!entry.summary.starts_with("[floor:")` alongside `entry.floor_category.is_none()` and `entry.credential_prompt != Some(true)`.

### CR-03 (Rust): Explicit `original_command` in `BlockFeedbackVerdict::Corrected` & `term_block_feedback`
**Files verified:** `crates/vox-terminal-core/src/session_agent.rs` (`L18-26`), `crates/vox-terminal-core/src/host_agent.rs` (`L135-191`, `L331-393`), `crates/vox-gui/src/commands/pty.rs` (`L432-452`), `crates/vox-gui/src/commands/pty/approvals.rs` (`L228-280`, `L378-444`)
**Landed fix:** Extended `BlockFeedbackVerdict::Corrected` with `original_command: Option<String>` and `term_block_feedback` / `term_block_feedback_impl` with `original_command: Option<String>` (scrubbed via `scrub_for_agent_inbox`). `LocalHost::record_block_feedback` resolves `TranscriptKind::Corrected { from, to }` from `original_command` when non-empty (supporting `BlockId(0)` for parked approvals without an open block) while preserving the `credential_prompt` block guard.

### CR-04: `/budget <usd>` real config mutation & explicit backend error handling
**Files verified:** `crates/vox-gui/src/commands/pty/slash.rs` (`L94-169`), `crates/vox-gui/src/commands/pty/agent_tests.rs` (`L320-369`)
**Landed fix:** Refactored `handle_budget` into `handle_budget_with`, persisting `per_session_budget_usd` via `crate::commands::user_config::set_user_config`, mirroring via `vox_preference_set` (with explicit warning on remote error), returning exit code `1` on config write failure or dual `context_budget_impl`/`get_llm_spend_impl` failure, and rendering explicit `unavailable` fields rather than `.unwrap_or_default()`.

### CR-05: Streaming secret redaction across split chunks
**Files verified:** `crates/vox-gui/src/commands/pty/agent.rs` (`L25`, `L202-222`, `L297-312`), `crates/vox-gui/src/commands/pty/agent_tests.rs` (`L68-119`)
**Landed fix:** Added `take_safe_stream_flush` (`STREAM_WH_MIN = 256`) holding back incomplete trailing non-whitespace tokens and sensitive prefixes (`bearer `, `key= `, `key: `, `token= `, `token: `) until newline/whitespace boundaries or stream end, scrubbing each flushed slice via `scrub_for_agent_inbox` and flushing the remaining scrubbed tail on completion.

### WR-01: Explicit cloud `active_model` & `VOX_LOCAL_ENDPOINT` resolution
**Files verified:** `crates/vox-gui/src/commands/pty/approvals.rs` (`L51-128`)
**Landed fix:** Added `explicit_cloud_provider` check at the top of `resolve_terminal_llm_config_with` so explicit cloud `active_model` selections with configured Clavis keys (`AnthropicApiKey`, `GeminiApiKey`, `OpenaiApiKey`, `OpenRouterApiKey`, `GroqApiKey`) route to the cloud provider rather than being overridden by running local MENS/Ollama servers, and resolved `VoxLocal` URLs from `vox_local_endpoint_probe_candidates()` before falling back to `VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT`.

### WR-02: Prompt cancellation & `Autopilot -> Manual` preemption during connect/stall
**Files verified:** `crates/vox-gui/src/commands/pty/agent.rs` (`L259-321`), `crates/vox-gui/src/commands/pty/agent_tests.rs` (`L146-193`)
**Landed fix:** Wrapped both the initial `llm_stream_activity` connection await and the `stream.next()` loop in `tokio::select!` against `rx.changed()` and a `100ms` `tick` polling `h.control_epoch(sid)`, closing cancelled turns with `BlockStatus::Cancelled` (`exit_code: 130`).

### WR-03 (Rust): `/compact` `credential_prompt` parity & `resync_required: false` on block-list resyncs
**Files verified:** `crates/vox-terminal-core/src/session_agent.rs` (`L115-182`), `crates/vox-gui/src/commands/pty/slash.rs` (`L244-289`), `crates/vox-gui/src/commands/pty/warp.rs` (`L193-199`)
**Landed fix:** Added `!b.credential_prompt` filtering to `Session::compact_blocks` matching `slash.rs::handle_compact`, and set `resync_required: false` on `/clear` (`clear_unpinned_blocks`) and `/compact` (`compact_blocks`) `SnapshotResync` events so Raw VT decoder and xterm viewport state remain intact.

### IN-01 & IN-02: `/context` error reporting & head+tail error slicing in `build_explain_fix_messages`
**Files verified:** `crates/vox-gui/src/commands/pty/slash.rs` (`L230-236`), `crates/vox-gui/src/commands/pty/agent.rs` (`L100-118`)
**Landed fix:** `handle_context` matches `context_budget_impl` `Err` to render `token_budget: unavailable`; `build_explain_fix_messages::cap` splits oversized `Stdout`/`Stderr` into head+tail windows (`PRE_SCRUB = 24_576`, `MAX_SLICE = 8_192`) so trailing compiler/test error lines stay visible to the model.

---

## Implemented UI Fixes (`445-REVIEW.md` & `445-UI-REVIEW.md`)

### 1. UI Blocker #1: WCAG AA Contrast in `travertine` & `arcane`
**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx`
**Applied fix:**
- Replaced `border-brass bg-brass ... text-primary-foreground` on primary buttons (`Approve`, `Run Edited Command`, `Run Fix`, `Run Command`, `Open Keys & Secrets`, `Save Correction`) with `border-(--color-accent-default) bg-(--color-accent-default) ... text-bg-base` (`>= 5.5:1` in `arcane`/`travertine`, `>= 9.5:1` in `slate`/`high-contrast`).
- Replaced `bg-overlay-subtle` with `bg-bg-base` on the wax-seal gate pill (`CO-PILOT GATE` / high-risk gates) and `Resume Co-Pilot` inside `TerminalApprovalCard.tsx` (eliminating `4.47:1` elevated composite contrast in `arcane`).

### 2. UI Blocker #2 + `WR-03` (UI): `/clear` `clearedEmpty` Race & Redundant `termSessionAttach`
**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx`
**Applied fix:**
- Separated `AltScreenChanged` and `SnapshotResync` handling in `listenTermEvent`: `SnapshotResync` only resets `setClearedEmpty(false)` when `next.blocks.length > 0`.
- Removed the unconditional post-dispatch `if (snapshotEventSeqRef.current > snapSeq) setClearedEmpty(false);` in `submitLine`, tracking `dispatchPreSnapSeqRef` instead.
- Updated `onSlashMutatedBlocks` to skip `termSessionAttach(sid)` whenever `SnapshotResync` from `listenTermEvent` already updated `paneState.blocks` (`snapshotEventSeqRef.current > dispatchPreSnapSeqRef.current`), using `applyAttachmentSnapshot(sid, snap, false)` in the offline/no-stream fallback path.
- Added test coverage in `Console.test.tsx` proving `/clear` preserves `'Session viewport cleared'` when `SnapshotResync { blocks: [] }` arrives mid-dispatch and avoids redundant `termSessionAttach` calls.

### 3. `CR-02` & `CR-03` (UI): Pre-Mutation Approval Guard, `originalCommand`, and `sourceBlockId`
**Files modified:** `crates/vox-gui/ui/src/transport.ts`, `crates/vox-gui/ui/src/transport.console.test.ts`, `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx`
**Applied fix:**
- Added optional `originalCommand?: string` to `termBlockFeedback` and `VoxTransport.termBlockFeedback` in `transport.ts` (`originalCommand: originalCommand ?? null` matching Tauri `pty.rs::term_block_feedback` `original_command`), verified in `transport.console.test.ts`.
- Extracted `extractApprovalResolutionError` in `useTerminalPendingApprovals.ts` checking `res.is_error`, `unwrapMcpEnvelope(res.result)?.resolved === false`, and `raw.success === false` before removing `daemonApprovals`, mutating `sessionPatterns`, or recording MENS corrections; surfaces sanitized backend `ExecPolicy` errors inline on `Edit & Run`.
- Passed `originalCommand = (approval.command ?? approval.summary).trim()` on `Modified` MENS feedback (skipping `credential_prompt` approvals, using `entry?.sourceBlockId ?? 0` for local Co-Pilot proposals and `0` for parked MCP approvals).
- Updated `onRunProposedCommand?: (cmd: string, sourceBlockId?: number) => void` across `BlockCard.tsx`, `BlockStreamView.tsx`, and `TerminalSessionPane.tsx` (`stageOrRunProposedCommand(cmd, submitShellLine, sourceBlockId)`).

### 4. `WR-04` + Repair Retry Context + `AgentTurnContent` / `BlockCard` Spec Alignment
**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.test.tsx`
**Applied fix:**
- Excluded `block.status === 'Cancelled'` from `isFailed` in `BlockCard.tsx` so cancelled blocks (`exit_code: 130`) retain `Cancelled` tone (`opacity-80`, `text-text-muted`) and do not show `Explain / Fix Failed Block`.
- Updated `BlockCard.tsx` origin badge text to `'agent'` / `'slash'` (CSS `uppercase`), and added `handleRetryAgentTurn` in `BlockStreamView.tsx` so retrying `? explain/fix block #N` re-invokes `onExplainFixBlock` with the matching target block in `blocks` when available instead of sending bare text.
- Updated `AgentTurnContent.tsx`: added `max-h-80 overflow-y-auto` on body container and `max-h-80 overflow-auto` on fenced `<pre>` blocks; kept `term-agent-streaming-status` visible during `Running` with `'Streaming response from {modelLabel}...'` once chunks arrive (preserving `'Routing prompt through model registry...'` at 0 bytes); updated Clavis copy to reference `VOX_LOCAL_ENDPOINT in Clavis`; added semantic `<ul>` / `<ol>` rendering (`space-y-1 pl-4`) in `renderProseBlock` and `data-terminal-mono="true"` on inline `<code>` and `<pre>` elements.

### 5. `WR-05` + Dynamic `/budget` Alert State & Compact Inline Slash Empty Cards
**Files modified:** `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.test.tsx`
**Applied fix:**
- Updated `submitBlockFeedback` (`useTerminalPendingApprovals.ts`) to return `Promise<boolean>`, resolving `true` only when `res?.recorded === true` and surfacing sanitized toast errors on failure.
- Guarded `setVerdict` and `setSavedCorrected` in `TermBlockFeedbackStrip` (`SlashMetaContent.tsx`) strictly on `ok === true`.
- Added `deriveBudgetAlert` in `SlashMetaContent.tsx` deriving `Ceiling Reached` (`text-(--color-status-fail)`), `Warning` (`>= 80%` or `>= threshold_tokens` in `text-(--color-status-warn)`), `Unavailable`, or `Healthy` (`text-(--color-status-pass)`) dynamically from parsed `context_tokens` and `session_spend_usd`.
- Replaced `<EmptyState>` (`py-16`) inside inline `/skills` and `/memory` cards with compact `p-4` status boxes preserving headings and quoting `{query}`; rendered structured `/skills` rows (`name` + `truncate` description), tightened `/compact` and `/budget` confirmation copy, and added `leading-[1.25]`/`leading-[1.3]` typography classes.

### 6. `WR-06` + Chat & `TurnTrace` Polish
**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx`, `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`
**Applied fix:**
- Added `key={matchingApproval.approval_id}` (`ChatTerminalBlockCard.tsx`) and `key={approvals[0].approval_id}` (`TurnTrace.tsx`) to `<TerminalApprovalCard>`.
- Restricted `ChatTerminalBlockCard.tsx` `matchingApproval` fallback so completed blocks (`Ok`/`Failed`/`Cancelled` or fulfilled receipts) never attach unrelated pending approvals by `'vox_terminal_exec'`/`'vox_terminal_send_input'` tool-name fallback, and active cards with explicit differing commands do not bind to unrelated session approvals.
- Updated `ChatTerminalBlockCard`'s fallback `handleResolveInline` to validate `typeof fn === 'function'` and `extractApprovalResolutionError(res, ...)` before returning `{ ok: true }`.
- Added `data-terminal-mono="true"` + `style={TERMINAL_MONO_STYLE}` on `.font-mono` elements in `TurnTrace.tsx`, replaced `—` in `TurnTrace.tsx:18` JSDoc with `-`, replaced `WORKSPACE · CONSOLE` with `WORKSPACE - CONSOLE` (`Console.tsx:355`), and removed redundant `onOpenSecrets` event re-dispatch in `Console.tsx`.
- *Intentional scope boundary notes:* Kept `{item.count}×` in `TurnTrace.tsx:153` per orchestrator confirmation so out-of-allowlist `TurnTrace.test.tsx:76` (`expect(step).toHaveTextContent('3×')`) stays intact and green; kept `ChatTranscript.tsx` untouched per scope boundary (`NEVER modify ... ChatTranscript.tsx`).

### 7. Keyboard DOM Order, `Escape` on Edit, & `/ai` Intent
**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts`, `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`
**Applied fix:**
- Moved `term-approval-edit-panel` and `term-approval-steer-panel` AFTER the Row 4 action button cluster in DOM order inside `TerminalApprovalCard.tsx`, added `useRef` + `useEffect` auto-focus when opening `'edit'`/`'steer'`, added `Escape` cancellation on `term-approval-edit-input`, and aligned Edit/Steer `<label>`, `aria-label`, and `placeholder` copy for WCAG 2.5.3 Label-in-Name parity.
- Updated `ASK_SLASH_RE` in `intentRouter.ts` to `/^\/(?:ask|ai)(?:\s|$)/i` so `/ai ...` displays the `agent` / `Ask Agent` pre-submit badge before submission, while guarding bare `/ai` / `/ask` in `useTerminalPendingApprovals.ts` so bare commands route to `/slash` usage help.

---

## Verification Evidence (Post-Final-Edit)

1. **TypeScript (`timeout 180s pnpm --dir crates/vox-gui/ui exec tsc --noEmit`):**
   - Exited with code `0` (zero output / zero errors).
2. **Vitest Bounded Workers (`timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run --maxWorkers=4 src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts`):**
   - `Test Files  36 passed (36)` | `Tests  213 passed (213)` | `Duration  40.03s` (exited with code `0`).
   - *(Note on unconstrained worker contention: default-worker `vitest` earlier timed out on `BlockStreamView.test.tsx` 100-card render at `5000ms` and `TerminalSessionPane.test.tsx:244` `68.5ms < 50ms` timing guard across 16 simultaneous jsdom workers; running `BlockStreamView.test.tsx` and `TerminalSessionPane.test.tsx` directly passed in `833ms` (`388ms` for the 100-block test) and `542ms` (`17/17` tests), and bounded `--maxWorkers=4` passed all `36/36` files / `213/213` tests.)*
3. **TOESTUB Theme Contract (`timeout 60s ./target/debug/toestub --rules vox/ui/theme-contract-drift crates/vox-gui/ui/src`):**
   - Exited with code `0`: `✅ No issues found — code is clean!`
4. **TOESTUB LLM Provider Boundary (`timeout 60s ./target/debug/toestub --rules vox/llm-provider-call crates/vox-gui/ui/src`):**
   - Exited with code `0`: `✅ No issues found — code is clean!`
5. **UI Guardrail Grep (`rg -n "font-bold|font-medium|bg-brass text-primary-foreground|\b(?:px|py|p|gap|size|mr|ml|mt|mb|pt|pb|space-y)-\d+\.5\b"` across modified Phase 445 UI files):**
   - Exited with code `1` (zero matches).
6. **Non-Blank LoC Budget (`rg -c "\S"` across every modified Phase 445 UI file — strictly `< 435`):**
   - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx`: `424`
   - `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`: `414`
   - `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx`: `411`
   - `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts`: `408`
   - `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx`: `402`
   - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`: `398`
   - `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`: `362`
   - `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx`: `225`
   - `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`: `218`
   - `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx`: `163`
   - `crates/vox-gui/ui/src/components/surfaces/Console/intentRouter.ts`: `126`

---

_Fixed: 2026-11-16T01:36:00Z_
_Fixer: Antigravity (UI Code & UI Review Fixer)_
_Iteration: 1_
