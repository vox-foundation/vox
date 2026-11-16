---
phase: 444-warp-editor-completions-and-runbooks
fixed_at: 2026-11-15T21:26:00Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/444-warp-editor-completions-and-runbooks/444-REVIEW.md
ui_review_path: .planning/workstreams/axis-warp-terminal-v75/phases/444-warp-editor-completions-and-runbooks/444-UI-REVIEW.md
iteration: 1
findings_in_scope: 26
fixed: 26
skipped: 0
status: all_fixed
---

# Phase 444: Code & UI Review Fix Report

**Fixed at:** 2026-11-15T21:26:00Z
**Source reviews:**
- `.planning/workstreams/axis-warp-terminal-v75/phases/444-warp-editor-completions-and-runbooks/444-REVIEW.md`
- `.planning/workstreams/axis-warp-terminal-v75/phases/444-warp-editor-completions-and-runbooks/444-UI-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 26 (`13` code-review items `CR-01..03`, `WR-01..08`, `IN-01..02` + `13` UI-review blockers/warnings `UI-BL-01..03`, `UI-WR-01..10`)
- Fixed: 26
- Skipped: 0
- Commit mode: `uncommitted in-tree` (per orchestrator workspace override — zero `git` index/working-tree/worktree mutations performed; orchestrator owns staging & commits).

## Fixed Issues (`444-REVIEW.md`)

### CR-01: UTF-16 vs UTF-8 offset mismatch corrupts completions after multi-byte characters

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.test.ts`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Added `utf16IndexToUtf8ByteOffset` (`TextEncoder().encode(line.slice(0, clamped)).byteLength`) before dispatching `termComplete` IPC requests and `utf8ByteOffsetToUtf16Index` (`TextDecoder` over byte slice) when mapping `dto.replace_start` and `dto.replace_end` back to JavaScript UTF-16 code-unit offsets. Added unit test with surrogate-pair emoji (`🚀`) and multi-byte UTF-8 (`é`) before the cursor.

### CR-02: `HistorySearchPopover` (`Ctrl+R`) `Enter` inserts into draft instead of executing immediately

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Wired `HistorySearchPopover` `onSelect={(cmd, action) => ...}` in `ScriptoriumPromptEditor.tsx` so `action === 'accept'` (`Enter` or primary row click) executes immediately via `executeCommandLine(cmd)` (recording history and routing intent before calling `onSubmit`), while `action === 'insert'` (`Tab` or `Insert Command`) populates the draft editor without submitting. Passed `initialQuery={value.includes('\n') ? '' : value}` so `Ctrl+R` seeds the search input with the current single-line draft.

### CR-03: `.vox` runbooks without `@cmd` drop `@param` arguments, and `{{name=default}}` / empty required params produce broken commands

**Files modified:** `crates/vox-terminal-core/src/runbooks.rs`, `crates/vox-terminal-core/src/runbooks_tests.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.test.tsx`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Updated `TEMPLATE_PARAM_RE` to `/\{\{\s*([A-Za-z_][A-Za-z0-9_]*)(?:\s*=\s*([^}]*?))?\s*\}\}/g`, broadened `SAFE_BARE_PARAM_RE` to `/^[A-Za-z0-9_./:@=-]+$/`, quoted empty strings as `"''"`, synthesized `vox run {rel_path} -- --<name> {{<name>}} ...` when `.vox` runbooks declare `@param` or `{{...}}` without an explicit `@cmd`, and added `getMissingRequiredParams` disabling `Run Runbook` / `Insert into Prompt` with an inline `role="alert"` message when required parameters have no value or default.

### WR-01: Completion score scale mismatch (`0..100` vs `0..1000`) and missing history-backed inline ghost completions

**Files modified:** `crates/vox-gui/src/commands/pty/warp.rs`, `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.test.ts`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Scaled Rust `0..100` scores to `0..1000` (`dto.score <= 120 ? Math.min(990, dto.score * 10) : Math.min(990, dto.score)`) in `terminalCompletions.ts`, wired `HistoryStore::query` prefix-matching recent commands into `complete_impl` (`TermCompletionKind::History`) in `warp.rs`, and passed local `historyCommands` fallback into `computeLocalTerminalCompletions` / `resolveTerminalCompletions` so inline ghost-text completions work both with and without Tauri IPC without hijacking `ArrowUp` recall when only `HIST` items match.

### WR-02: Subshell voxify hooks contain unguarded Nushell hook appending, unescaped Fish `633;E` command text, and fragile PowerShell `$LASTEXITCODE` handling

**Files modified:** `crates/vox-terminal-core/src/voxify.rs`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Added `$env.__VOX_SUBSHELL_HOOKED` idempotency guard + `pre_execution` (`633;E` + `633;C`) + `clear` without unused `$env.__vox_b64` pollution in `NU_HOOK`; escaped `\`, `;`, and `\n` in `FISH_HOOK` `633;E`; and normalized POSIX/Windows `OSC 7` `$uriPath`, `$?`-aware exit status capture, and `Set-PSReadLineKeyHandler -Key Enter` `633;E`/`633;C` emission in `PWSH_HOOK`.

### WR-03: `OmnibarRunbooksSection` items are excluded from `Omnibar.tsx` keyboard navigation

**Files modified:** `crates/vox-gui/ui/src/components/layout/Omnibar.tsx`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`, `crates/vox-gui/ui/src/components/layout/Omnibar.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Extended `Omnibar.tsx` `ArrowDown`, `ArrowUp`, and `Enter` keyboard navigation across `rows.length + matchingRunbooks.length`, passing `startIndex={rows.length}`, `selectedIdx={selectedRowIdx}`, and `onFocusIndex` to `OmnibarRunbooksSection` with `data-row` and `aria-current={selected ? 'true' : undefined}`.

### WR-04: Inline `ArrowUp` history recall ignores non-empty prefix filtering, `Clear History` leaves stale cache, and `term-history-trigger` clobbers stashed draft

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Prefix-filtered `ArrowUp`/`ArrowDown` inline recall against `liveDraftRef.current` when non-empty, added `onCleared` callback on `HistorySearchPopover` clearing `ipcHistoryCacheRef.current` and resetting `historyRecallCursor`, and restored `stashedBeforeCtrlR.current` cleanly when toggling `term-history-trigger` closed.

### WR-05: Workspace runbook scanner skips nested subdirectories, truncates before filtering, and registers empty Markdown files

**Files modified:** `crates/vox-terminal-core/src/runbooks.rs`, `crates/vox-terminal-core/src/runbooks_tests.rs`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Added `"runbooks"` to `SCAN_DIRS`, walked subdirectories recursively up to depth `3` (skipping hidden dirs, symlinks, `node_modules`, `target`), filtered candidate files by extension/size/canonical workspace confinement before sorting deterministically and truncating to `MAX_SCANNED_FILES`, and rejected Markdown files whose fenced code block body is empty.

### WR-06: Frontend `detectSubshellShellHint` diverges from backend `voxify::detect_subshell_command`

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Aligned `detectSubshellShellHint` with `voxify.rs` across `env VAR=val`, `sudo` valued flags (`-u`, `-g`, `-C`, `-D`, `-R`, `-T`, `-h`, `-p`, `-r`, `-t`), and `docker`/`podman`/`kubectl exec` long flags (`--stdin`, `--tty`, `-c`, `-n`, `-u`, `-w`, `-e`, `--context`) + container/pod positional skipping. Added unit assertions in `Console.test.tsx`.

### WR-07: Path completion only fires on explicit path prefixes (`./`, `/`, `~/`) and ignores bare `~` or relative argument filenames

**Files modified:** `crates/vox-terminal-core/src/completions.rs`, `crates/vox-terminal-core/src/completions_tests.rs`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Enabled path completion on general argument tokens (`token_idx >= 1 && !prefix.starts_with(['-', '$'])`) when no specialized git/CLI subcommand matches, and handled bare `~` (`cd ~`) by emitting `"~/"` plus `$HOME` directory completions. Extracted `completions_tests.rs` and `runbooks_tests.rs` so every `.rs` file stays `< 445` non-blank LoC.

### WR-08: Minor UX/spec drift (`Voxify Subshell` permanent disable, `ScriptoriumPromptEditor` static CTA, `JsonLedgerTable` direct `lucide-react` import)

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/ui/Icons.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Gated `term-voxify-subshell` on subshell detection while keeping it re-clickable when `Subshell Voxified` (allowing hook re-injection if needed), made `term-prompt-submit` render `Ask Agent` in agent mode vs `Run Command` in shell/vox/nu/slash modes, and added `Icon.arrowUpDown` in `Icons.tsx` so `JsonLedgerTable.tsx` imports zero icons directly from `lucide-react`.

### IN-01: `JsonLedgerTable` rejects valid empty JSON arrays `[]` and objects `{}`

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Accepted valid empty `[]` and `{}` payloads in `parseStructuredJsonOutput` and rendered an `Empty JSON collection` table state while retaining Raw/Table toggle and Copy JSON actions.

### IN-02: `OmnibarRunbookSheet` calls `termRunbooksList()` without active terminal `cwd`

**Files modified:** `crates/vox-gui/ui/src/lib/terminalBridge.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`, `crates/vox-gui/ui/src/components/layout/Omnibar.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Added `cwd?: string | null` to `TerminalBridgeState`, published `cwd: activeCwd ?? null` from `Console.tsx`, and passed `getTerminalBridgeState().cwd ?? undefined` into `loadWorkspaceRunbooks(cwd)` in `OmnibarRunbookSheet.tsx`.

---

## Fixed Issues (`444-UI-REVIEW.md`)

### UI-BL-01: `main textarea` global `!important` rules and `<pre>` vs `<textarea>` wrap/font-weight drift break syntax highlight & ghost-text overlay

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Replaced `<pre>` mirror with `<div ref={mirrorRef} aria-hidden="true" className="... overflow-y-auto px-2 py-1 text-xs leading-[1.45] font-normal whitespace-pre-wrap break-words">` using `TERMINAL_MONO_FONT_STACK` (`fontVariantLigatures: 'none'`, `scrollbarGutter: 'stable'`, `min-h-6` `24px` floor), normalized all `PROMPT_TOKEN_CLASSES` to `font-normal` (`400`) so glyph advance widths match `<textarea>` 1:1, and applied inline `style.setProperty(..., 'important')` overrides (`background-color: transparent`, `border: none`, `box-shadow: none`, `color`, `caret-color`, `font-family`) in `useLayoutEffect` on `textareaRef.current` without touching `src/index.css`.

### UI-BL-02: Undefined CSS custom properties (`--color-status-ok`, `--color-status-error`) and undefined `.text-brass` utility class

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts`, `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`, `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Replaced all `--color-status-ok` references with `--color-status-pass`, all `--color-status-error` references with `--color-status-fail`, and all `text-brass` classes with `text-accent` across every Phase 444 surface.

### UI-BL-03: `HistorySearchPopover` primary `Enter`/click semantics, fuzzy match spans, `[REDACTED]` badge, relative timestamp, and `KeycapIcon` footer

**Files modified:** `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.test.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:** Added subsequence/substring `highlightFuzzyMatch` (`text-accent font-semibold`), `[REDACTED]` amber badge (`term-history-redacted-badge`), relative timestamp chip (`formatRelativeTs`), `KeycapIcon` footer (`Enter` to run, `Tab` to insert, `Esc` to close), and `444-UI-SPEC.md` empty/confirmation copy (`No matching history commands`, `Type to filter recent commands in this workspace, or run a command in the terminal first.`, `Clear all recorded terminal command history on this machine?`).

### UI-WR-01..10: Copywriting, 7-bit ASCII, `TERMINAL_MONO_FONT_STACK`, 12-column / 200-row table caps, sanitized scan error state, tooltips, and spacing rhythm

**Files modified:** `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`, `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`
**Commit:** uncommitted (orchestrator-owned)
**Applied fix:**
- Applied `TERMINAL_MONO_FONT_STACK` on `IntelliSensePopover.tsx`, `HistorySearchPopover.tsx`, `RunbookPanel.tsx`, `OmnibarRunbookSheet.tsx`, and `JsonLedgerTable.tsx`.
- Capped `JsonLedgerTable.tsx` at `MAX_DISPLAY_ROWS = 200` and `MAX_DISPLAY_COLS = 12` with `X of Y rows` badge and column overflow notice.
- Added `sanitizeErrorForToast(err)` error alert + `Reload Runbooks` button and `No workspace runbooks discovered` empty state in `RunbookPanel.tsx`.
- Replaced `\u2014` (`—`) with 7-bit ASCII `-` in `TerminalSessionPane.tsx` and `Console.tsx`.
- Used `Icon.book` + `gap-2 py-2` in `OmnibarRunbookSheet.tsx`, `<Tooltip>` + `size-4` chevrons in `DiscoveryRail.tsx`, and `px-2` token alignment in `ConsoleSessionBar.tsx`.

---

## Verification Evidence (Main Checkout)

All verification gates were executed directly in the shared main checkout `/Users/brbrainerd/dev/vox` (no temporary worktree created; reproducible directly from current working tree):

1. **Non-blank LoC budget (`rg -c "\S"` — all Phase 444 files `< 445` non-blank LoC):**
   - `crates/vox-terminal-core/src/completions.rs`: `371`
   - `crates/vox-terminal-core/src/completions_tests.rs`: `110`
   - `crates/vox-terminal-core/src/runbooks.rs`: `432`
   - `crates/vox-terminal-core/src/runbooks_tests.rs`: `73`
   - `crates/vox-terminal-core/src/voxify.rs`: `397`
   - `crates/vox-gui/src/commands/pty/warp.rs`: `398`
   - `crates/vox-gui/ui/src/lib/terminalBridge.ts`: `159`
   - `crates/vox-gui/ui/src/components/ui/Icons.tsx`: `442`
   - `crates/vox-gui/ui/src/components/ui/IntelliSensePopover.tsx`: `166`
   - `crates/vox-gui/ui/src/components/layout/OmnibarRunbookSheet.tsx`: `216`
   - `crates/vox-gui/ui/src/components/surfaces/Console/terminalCompletions.ts`: `281` (`terminalCompletions.test.ts`: `192`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/promptHighlighter.ts`: `202`
   - `crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx`: `409` (`ScriptoriumPromptEditor.test.tsx`: `167`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx`: `366` (`HistorySearchPopover.test.tsx`: `128`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx`: `237` (`RunbookPanel.test.tsx`: `136`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx`: `176` (`DiscoveryRail.test.tsx`: `108`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/useConsoleWarpActions.ts`: `102`
   - `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`: `191`
   - `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`: `416`
   - `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`: `404` (`Console.test.tsx`: `393`)
   - `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx`: `283` (`JsonLedgerTable.test.tsx`: `85`)
   - *(Pre-existing shared `Omnibar.tsx` (`786`) and `Omnibar.test.tsx` (`616`) received minimal targeted keyboard-nav & test additions only.)*
2. **TypeScript (`pnpm --dir crates/vox-gui/ui exec tsc --noEmit`):** `EXIT 0` (zero errors).
3. **Vitest (`pnpm --dir crates/vox-gui/ui exec vitest run src/transport.console.test.ts src/components/surfaces/Console/ src/components/layout/Omnibar.test.tsx src/guards/ src/styles/themeContract.test.ts`):** `31 passed (31)` test files, `196 passed (196)` tests.
4. **Rust (`CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/target/ws-axis-warp-terminal-v75 cargo test -p vox-terminal-core`):** `137 passed; 0 failed` (`82` lib unit tests + `55` integration tests across 10 test binaries; zero compiler warnings in `vox-terminal-core`).
5. **Architecture & Crate API Sync (`./target/debug/vox ci crate-api-sync` & `./target/debug/vox-arch-check`):** `crate-api-sync OK (131 crates)` (`EXIT 0`) and `vox-arch-check` `EXIT 0`.

---

_Fixed: 2026-11-15T21:26:00Z_
_Fixer: Antigravity (`gsd-code-fixer`)_
_Iteration: 1_
