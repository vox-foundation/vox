# Phase 445 — UI Review

**Audited:** 2026-11-15
**Baseline:** `445-UI-SPEC.md` (approved 2026-11-15)
**Screenshots:** not captured (code-only audit — no Vox GUI dev server running on ports `3000`, `5173`, `8080`, or `1420`; ports `3000` and `8080` serve unrelated local services)
**Verification Gates Observed:** `pnpm --dir crates/vox-gui/ui exec tsc --noEmit` -> exit `0`; `pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/ src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx src/components/surfaces/Chat/TurnTrace.test.tsx src/guards/ src/lib/themeContract.test.ts` -> `36` test files, `204` tests passed (`0` failed).

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Pure 7-bit ASCII and WCAG 2.5.3 `aria-label` prefixes across all new Phase 445 code, with minor spec copy drift (`AgentTurnContent.tsx`, `BlockCard.tsx`, `SlashMetaContent.tsx`) and 2 pre-existing non-ASCII chars (`Console.tsx`, `TurnTrace.tsx`). |
| 2. Visuals | 3/4 | Semantic `<span data-roman-wax-seal="true">` + `<Icon.waxSeal>` replaces `RomanWaxSealBadge` cleanly (`0` `<select>`, `0` `<Button icon=>`), but `SlashMetaContent.tsx` renders flat text instead of spec sub-views and `ChatTerminalBlockCard.tsx` can bind pending approvals to unrelated failed receipts. |
| 3. Color | 2/4 | Zero banned tokens (`status-ok`, `status-error`, `text-brass`, raw hex/Tailwind palette colors), but `bg-brass text-primary-foreground` computes to `3.61:1` (<4.5:1 AA) in `travertine`, `text-accent` on `bg-overlay-subtle` over `bg-bg-elevated` is `4.47:1` in `arcane`, and `/budget` Alert State is hardcoded green `Healthy`. |
| 4. Typography | 3/4 | Strictly 4 sizes (`11px`, `12px`, `14px`, `18px` Display), 2 weights (`400`, `600`; zero `font-bold`/`font-medium`), and `TERMINAL_MONO_FONT_STACK` across Phase 445 code; minor `leading-[1.25]`/`leading-[1.3]` omissions in `SlashMetaContent.tsx` and un-isolated `.font-mono` in `TurnTrace.tsx`. |
| 5. Spacing | 3/4 | Zero fractional `.5` Tailwind classes (`px-2.5`, `gap-1.5`, `size-3.5`, etc.) and 100% 4px-grid alignment across all 10 files; `AgentTurnContent.tsx` omits `max-h-80 overflow-y-auto` height caps and `<EmptyState>` adds `py-16` inside inline slash cards. |
| 6. Experience Design | 2/4 | Single `role="textbox"` invariant, `credential_prompt` safety guards, toast sanitization, and `<430` LoC budgets hold, but `/clear` `clearedEmpty` is overwritten by `SnapshotResync`, `TerminalApprovalCard` Edit/Steer panels mount above toggles without focus or `Escape` on Edit, and `Retry Agent Turn` drops Explain/Fix block context. |

**Overall: 16/24**

---

## Top 3 Priority Fixes

1. **[BLOCKER] Fix `travertine` WCAG AA contrast failure (`3.61:1`) on `bg-brass text-primary-foreground` CTAs and `arcane` `4.47:1` elevated badge contrast (`TerminalApprovalCard.tsx`, `AgentTurnContent.tsx`, `SlashMetaContent.tsx`)** — Primary buttons (`Approve`, `Run Edited Command`, `Run Command`, `Run Fix`, `Open Keys & Secrets`, `Save Correction`) fail WCAG 2.1 AA (`3.61:1 < 4.5:1`) in `travertine`/`glacier` (`#1C140C` on `#8a6a26`), and `CO-PILOT GATE` / `Resume Co-Pilot` sit at `4.47:1` in `arcane` (`#7d5f20` on `bg-overlay-subtle` over `#EFE6D5`) — Use `bg-(--color-accent-default) text-bg-base` or update `travertine` `--vox-shadcn-primary-fg` / `--brass` token pairing and remove `bg-overlay-subtle` behind `text-accent` on `bg-bg-elevated`.
2. **[BLOCKER] Fix `/clear` `clearedEmpty` overwrite race in `TerminalSessionPane.tsx:271-273`** — When a live daemon emits `SnapshotResync` during `await dispatchTerminalLine(sid, '/clear', ...)`, `snapshotEventSeqRef.current > snapSeq` evaluates true and immediately sets `clearedEmpty(false)`, suppressing `'Session viewport cleared'` in `BlockStreamView.tsx:168` — Only reset `setClearedEmpty(false)` on `SnapshotResync` when `blocks.length > 0` or when `line` is not `/clear` (and never after `onClearedSession` fired for the active dispatch).
3. **[WARNING] Fix `TerminalApprovalCard.tsx` keyboard DOM order / missing `Escape` on Edit & Run (`TerminalApprovalCard.tsx:171-346`) and dynamic `/budget` Alert State (`SlashMetaContent.tsx:79-82`)** — Row 3 (`Edit & Run` / `Reject + Steer` inline panels) renders before Row 4 trigger buttons in DOM order without focusing the mounted field, and `<textarea data-testid="term-approval-edit-input">` lacks an `Escape` cancel handler; meanwhile `/budget` hardcodes green `Healthy` even over ceiling — Move Row 3 sub-panels below Row 4 (or auto-focus the input on toggle), add `Escape` handling to `term-approval-edit-input`, and derive `/budget` Alert State from `used_tokens >= threshold_tokens` / spend vs budget.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

- **PASS — Pure 7-bit ASCII across all new Phase 445 code:** Zero occurrences of `·`, `—`, `…`, emojis, or dingbats in `useTerminalPendingApprovals.ts`, `TerminalApprovalCard.tsx`, `AgentTurnContent.tsx`, `SlashMetaContent.tsx`, `BlockCard.tsx`, `BlockStreamView.tsx`, `TerminalSessionPane.tsx`, and `ChatTerminalBlockCard.tsx`.
- **PASS — WCAG 2.5.3 Label-in-Name & Verb+Noun CTAs:** All Phase 445 buttons with both visible text and `aria-label` begin their `aria-label` with the exact visible label in both idle and active states (`Approve` -> `"Approve proposed command"`, `Accept`/`Accepted` -> `"Accept..."`/`"Accepted..."`, `Reject`/`Rejected`, `Correct`/`Corrected`, `Explain / Fix Failed Block` / `Diagnosing Failed Block...`, `Run Command {cmd}`, `Run Fix {cmd}`, `Edit in Prompt {cmd}`, `Open Keys & Secrets in Settings`, `Run Secrets Doctor command`, `Resume Co-Pilot mode`, `Dismiss Notice`).
- **WARNING — Pre-existing non-ASCII characters in modified parent files (`Console.tsx:355`, `TurnTrace.tsx:17, 148`):** `Console.tsx:355` retains `domainTag="WORKSPACE · CONSOLE"` (`U+00B7`) and `TurnTrace.tsx:148` renders `{item.count}×` (`U+00D7`; line 17 JSDoc has `—`). Replace `·` with `-` in `Console.tsx:355` and `×` with `x` in `TurnTrace.tsx:148` to satisfy strict repo-wide 7-bit ASCII hygiene.
- **WARNING — Streaming status copy divergence (`AgentTurnContent.tsx:271-292`):** Renders `'Routing prompt through model registry...'` instead of spec `'Routing prompt and awaiting first token...'` (`445-UI-SPEC.md:131, 178`), and suppresses the streaming status row as soon as `bodyText.trim().length > 0`, omitting `'Streaming response from {provider}/{model_id}...'`.
- **WARNING — Clavis diagnostic copy divergence (`AgentTurnContent.tsx:302, 305`):** Heading `'No LLM Provider Configured in Clavis'` and body `'...or local MENS in Clavis...'` differ from spec `'No LLM provider credentials configured in Clavis'` and `'...or VOX_LOCAL_ENDPOINT in Clavis...'` (`445-UI-SPEC.md:132`).
- **WARNING — Origin badge & repair heading copy divergence (`BlockCard.tsx:217, 272-276`):** Renders `'AGENT TURN'` / `'SLASH META'` instead of spec `'agent'` / `'slash'` (`445-UI-SPEC.md:170, 299`), and renders raw `block.input` (e.g. `'? fix block #7'`) rather than `'Diagnose Block #{id}: {cmd}'` (`445-UI-SPEC.md:171`).
- **WARNING — Minor label/placeholder deltas (`TerminalApprovalCard.tsx:176, 183, 233, 240, 252`, `SlashMetaContent.tsx:43, 63, 115, 139`):**
  - `TerminalApprovalCard.tsx:176, 183` and `233, 240`: `<label>` text (`'Edit command before execution (re-checked against ExecPolicy AST)'`, `'Optional steering note for the agent'`) differs from the control's `aria-label` (`'Edit proposed command before running'`, `'Steer agent after rejecting command'`); line 252 placeholder uses `'Explain why or steer the agent toward another approach...'` vs spec `'Steer the agent toward an alternative approach (optional)...'`.
  - `SlashMetaContent.tsx:43, 63, 115, 139`: `/compact` renders raw `'[compact digest - N blocks]'` instead of `'Compacted {N} prior blocks (pinned blocks preserved)'`; `/budget` confirmation reads `'Updated session token ceiling / budget to $...'`; `/skills` and `/memory` empty-state descriptions omit the `{query}` quote.

### Pillar 2: Visuals (3/4)

- **PASS — Semantic wax-seal signet & icon hygiene (`TerminalApprovalCard.tsx:120-131`):** Uses `<span data-roman-wax-seal="true" data-seal-tone={isHighRisk ? 'crimson' : 'gold'}>` with `<Icon.waxSeal className="size-3 shrink-0" aria-hidden="true" />`. Zero `RomanWaxSealBadge` imports, zero `<select>` elements, zero `<Button icon=...>` or `trailingIcon=` props, and zero `dangerouslySetInnerHTML` across all 10 files.
- **PASS — High-risk gate differentiation (`TerminalApprovalCard.tsx:49-62, 115, 122-127, 150-159`):** `credential_prompt` and `floor_category` elevate card border, seal tone, and `Single-command approval only` pill while disabling `Allow Pattern in Session`.
- **WARNING — `ChatTerminalBlockCard.tsx:142-148` overly permissive `matchingApproval` fallback:** On line 147, `return event?.fulfilled === false || correlatedBlock?.status === 'Running' || command === 'vox_terminal_exec' || command === 'vox_terminal_send_input'` runs even when `a.command?.trim()` exists and differs from `command`. In a multi-command Chat turn, an earlier unfulfilled tool receipt for `cargo check` can falsely mount a pending approval card belonging to a later `git reset --hard` call in the same session.
- **WARNING — Incomplete structured sub-renderers in `SlashMetaContent.tsx:88-167`:** `/model` omits Clavis readiness chips (`configured`/`unconfigured`) and `/model route <prompt>` falls back to plain `<pre>` (`SlashMetaContent.tsx:170`) instead of a 4-axis (`Latency`, `Cost/Token`, `Quality`, `Locality`) row; `/skills` (`121-127`) renders unformatted strings without bold `skill.name`, category badge, or 1-line `truncate`; `/memory` (`144-152`) renders raw text lines instead of `Total Entries`/`Backend Status`/`score` chips; `/context` (`158-165`) renders only the 5 keys emitted by backend `pty/slash.rs` (omitting `Security Level`, `Control Mode`, `Session Allowlist`).
- **WARNING — `term-agent-model-pill` placement & prose bullet semantics (`AgentTurnContent.tsx:146-163, 261-269`):** `term-agent-model-pill` renders inside the `AgentTurnContent` body rather than in the `BlockCard` header row (`445-UI-SPEC.md:172-173`), and `renderProseBlock` renders `- ` / `1. ` bullet items as `<p>` elements instead of `<ul>`/`<ol>`.

### Pillar 3: Color (2/4)

- **PASS — Token compliance across all 10 files:** Zero `--color-status-ok`, zero `--color-status-error`, zero `.text-brass`, zero raw hex/RGB values, and zero hardcoded Tailwind color utilities (`red-*`, `emerald-*`, `amber-*`, `slate-*`, `zinc-*`, `black/*`, `white/*`).
- **BLOCKER — WCAG AA contrast failure (`3.61:1 < 4.5:1`) in `travertine` (`glacier`) theme on primary brass CTAs:**
  `TerminalApprovalCard.tsx:219, 290`, `AgentTurnContent.tsx:221, 314`, and `SlashMetaContent.tsx:346` style `11px` primary buttons (`Approve`, `Run Edited Command`, `Run Fix`, `Run Command`, `Open Keys & Secrets`, `Save Correction`) with `bg-brass text-primary-foreground`. In `tokens.travertine.generated.css:3` (`--brass: 138 106 38`, `#8a6a26`) and `index.css:270` (`--vox-shadcn-primary-fg: #1C140C`), the foreground-to-background contrast ratio is **3.61:1** (fails WCAG 2.1 AA `4.5:1` for normal/small text; passes in `arcane` `5.59:1`, `slate` `7.54:1`, and `high-contrast` `9.99:1`).
- **WARNING — Borderline WCAG contrast (`4.47:1`) on `text-accent` + `bg-overlay-subtle` over `bg-bg-elevated` in `arcane` theme:**
  In `arcane`, `bg-overlay-subtle` (`rgba(184, 134, 11, 0.08)`) over `bg-bg-elevated` (`#EFE6D5`) composites to `#EBDEC5`; `text-accent` (`#7d5f20`) against `#EBDEC5` yields **4.47:1** (just below `4.50:1`) on `CO-PILOT GATE` (`TerminalApprovalCard.tsx:126`) and `Resume Co-Pilot` (`TerminalApprovalCard.tsx:398`). (Other semantic pairings pass AA across all 4 themes: `bg-bg-base` on `bg-(--color-status-fail)` is `5.93:1`–`7.59:1`; `text-(--color-status-pass)`/`warn`/`fail` on `bg-bg-elevated` are `5.28:1`–`11.86:1`.)
- **WARNING — Hardcoded green `Healthy` state in `/budget` (`SlashMetaContent.tsx:79-82`):** Line 81 hardcodes `<div className="text-xs font-semibold text-(--color-status-pass)">Healthy</div>` without checking context threshold or spend ceiling exceedance (`445-UI-SPEC.md:301`).

### Pillar 4: Typography (3/4)

- **PASS — Scale, weight, and font-stack discipline across all 10 files:**
  - Sizes used in the 10 files: `text-[11px]` (`11px`), `text-xs` (`12px`), and `text-sm` (`14px`), plus `18px` Display on `RomanSurfaceHeader`. Zero `text-[9px]`, `text-[10px]`, `text-[13px]`, or `text-base` classes appear in Phase 445 code.
  - Weights used: `font-normal` (`400`) and `font-semibold` (`600`) only — zero `font-bold` or `font-medium` anywhere in the 10 files, and every `<Button size="xs">` overrides `Button.tsx`'s `text-[10px] font-medium` via `cn()`/`twMerge`.
  - Monospace stack: `TERMINAL_MONO_FONT_STACK` (`'JetBrains Mono', ui-monospace, 'SFMono-Regular', monospace`) via `TERMINAL_MONO_STYLE`, with zero banned font literals (`Menlo`, `Monaco`, `Consolas`, `Courier`, `Arial`).
- **WARNING — Minor line-height omissions and un-isolated `.font-mono` in `TurnTrace.tsx`:**
  - `SlashMetaContent.tsx:94` omits `leading-[1.3]` on `text-sm font-semibold`, and lines 68, 72, 76, 80, 161 omit `leading-[1.25]` on `text-[11px]` labels (inheriting parent `leading-[1.45]`).
  - `TurnTrace.tsx:123, 148, 155` use Tailwind `font-mono` without `TERMINAL_MONO_STYLE` or `data-terminal-mono="true"`, which `index.css:569` maps to `Palatino` in `arcane`/`glacier`.
  - Pre-existing `RomanSurfaceHeader.tsx:244` (shared UI primitive outside the 10 files) retains `text-sm sm:text-base font-bold`.

### Pillar 5: Spacing (3/4)

- **PASS — Zero fractional `.5` spacing/sizing & strict 4px grid across all 10 files:**
  - Static scan confirms **0** matches for fractional `.5` utilities (`p-0.5`, `px-1.5`, `py-0.5`, `px-2.5`, `py-1.5`, `gap-0.5`, `gap-1.5`, `gap-2.5`, `size-1.5`, `size-3.5`, `h-6.5`, etc.) across all 10 Phase 445 files. Legacy `size-3.5` in `BlockCard.tsx` and `TerminalSessionPane.tsx` (`Icon.search` -> `size-4`) was normalized cleanly.
  - Zero arbitrary `[...px]` / `[...rem]` spacing values. Every spacing token sits on the 4px scale (`4px`, `8px`, `12px`, `16px`, `24px`, `32px`, `48px`, `128px`, `160px`, `240px`, `320px`), and every `<Button size="xs">` passes `py-1` (`4px`) to override `Button.tsx:22`'s `py-0.5`.
- **WARNING — Missing scroll height cap on `AgentTurnContent` (`445-UI-SPEC.md:383`):** `BlockCard.tsx:319` bounds Shell output at `max-h-80 overflow-y-auto`, `TerminalApprovalCard.tsx:166` bounds command previews at `max-h-32 overflow-auto`, and `SlashMetaContent.tsx:121` bounds `/skills` at `max-h-60 overflow-y-auto`, whereas `AgentTurnContent.tsx:259` and `344` lack `max-h-80 overflow-y-auto`, allowing long streamed LLM turns or fenced code blocks to grow unboundedly.
- **WARNING — Inline `<EmptyState>` vertical padding in `SlashMetaContent.tsx:111, 135`:** `EmptyState.tsx:44` applies `py-16` (`64px` top/bottom = `128px` padding) inside inline `/skills` and `/memory` block cards, noticeably inflating empty slash card height compared to `p-4` (`16px`).

### Pillar 6: Experience Design (2/4)

- **PASS — Single-textbox invariant, credential-prompt exclusions, error sanitization, and LoC budgets:**
  - `term-approval-edit-input`, `term-approval-steer-input`, and `term-feedback-correct-input` return `null` when idle (`TerminalApprovalCard.tsx:171, 228`, `SlashMetaContent.tsx:315`), preserving unique `screen.getByRole('textbox')` on initial `Console` mount (`Console.test.tsx` `17/17` pass).
  - `BlockCard.tsx:159`, `SlashMetaContent.tsx:208`, and `TerminalApprovalCard.tsx:49, 321` block `Explain / Fix Failed Block`, MENS feedback, and `Allow Pattern in Session` when `credential_prompt` (or `floor_category`) is set.
  - All error toasts/states sanitize via `sanitizeErrorForToast(err)` (`toastBodyGuard.test.ts` passes).
  - All 10 files are `< 430` non-blank LoC.
- **BLOCKER — `/clear` `clearedEmpty` overwritten by `SnapshotResync` in live daemon sessions (`TerminalSessionPane.tsx:215, 271-273`):**
  Line 271 snapshots `snapshotEventSeqRef.current` before `await dispatchTerminalLine(...)`. When `/clear` runs, `term_slash_exec` emits `SessionEvent::SnapshotResync { blocks: [], ... }`, incrementing `snapshotEventSeqRef.current` at line 215 while `await` is in flight; `onClearedSession` calls `setClearedEmpty(true)`, and line 273 (`if (snapshotEventSeqRef.current > snapSeq) setClearedEmpty(false);`) immediately flips `clearedEmpty` back to `false`. Live `/clear` therefore renders `'Terminal session ready'` instead of `'Session viewport cleared'`.
- **WARNING — Keyboard navigation & `Escape` gaps in `TerminalApprovalCard.tsx:171-346`:**
  - Row 3 (`term-approval-edit-panel` / `term-approval-steer-panel`) mounts above Row 4 (`Approve` / `Edit & Run` / `Allow Pattern in Session` / `Reject + Steer`) in DOM order, without focusing the newly mounted input on toggle; keyboard users activating `Edit & Run` or `Reject + Steer` must `Shift+Tab` backwards to enter the field.
  - `term-approval-edit-input` (`TerminalApprovalCard.tsx:179-188`) has no `Escape` key handler to cancel back to `'idle'` (unlike `term-approval-steer-input` at line 247 and `term-feedback-correct-input` at `SlashMetaContent.tsx:331`).
- **WARNING — `Retry Agent Turn` on failed Explain/Fix turns loses block context (`BlockCard.tsx:309`):**
  `onRetryAgentTurn={onRerun}` invokes `submitLine(block.input)`. For an Explain/Fix turn (`block.input === '? fix block #7'` from `pty/agent.rs:409`), retrying calls `termAgentPrompt(sid, 'fix block #7')` without the failed block's scrubbed stdout/stderr instead of re-invoking `termAgentExplainFix(sid, 7, 'fix')`.
- **WARNING — Unkeyed `<TerminalApprovalCard>` in Chat & missing `sourceBlockId` on Co-Pilot proposal staging:**
  - `ChatTerminalBlockCard.tsx:205` and `TurnTrace.tsx:107` omit `key={approval.approval_id}` (unlike `TerminalApprovalCard.tsx:419`), risking stale `activeMode`/`editError`/`steeringNote` state when a subsequent approval replaces the active one.
  - `TerminalSessionPane.tsx:415` calls `stageOrRunProposedCommand(cmd, submitShellLine)` without passing the proposing `AgentTurn` `block.id`, attributing `Edit & Run` corrections to `latestBlockId` instead of the originating proposal block (and dropping them in `TurnTrace` where `latestBlockId` is `null`).
- **WARNING — Pre-submit `/ai` mode badge mismatch (`intentRouter.ts:13` vs `useTerminalPendingApprovals.ts:28`) & redundant `onOpenSecrets` dispatch (`Console.tsx:419`):**
  - `useTerminalPendingApprovals.ts:270` dispatches `/ai <prompt>` to `termAgentPrompt`, whereas `intentRouter.ts:13` only recognizes `/ask`, causing `ScriptoriumPromptEditor` to display `SLASH` / `Run Command` while typing `/ai ...`.
  - `Console.tsx:419` re-dispatches `vox-settings-seed` and `vox://navigate-surface` already fired by `AgentTurnContent.tsx:194-201`.

---

## Files Audited

All 10 Phase 445 UI files verified strictly under `< 430` non-blank LoC (`rg -c '\S'`):

1. `crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts` — `373` non-blank LoC
2. `crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx` — `413` non-blank LoC
3. `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx` — `358` non-blank LoC
4. `crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx` — `343` non-blank LoC
5. `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` — `358` non-blank LoC
6. `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` — `202` non-blank LoC
7. `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` — `393` non-blank LoC
8. `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` — `415` non-blank LoC
9. `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` — `213` non-blank LoC
10. `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` — `157` non-blank LoC
