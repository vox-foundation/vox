# Phase 446 — UI Review

**Audited:** 2026-11-16
**Baseline:** `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-UI-SPEC.md` (approved 2026-11-15)
**Screenshots:** captured via Playwright CV Flywheel (`crates/vox-gui/ui/test-results/cv-flywheel-terminal/screenshots/`, 112 `CV_STAGE=after` PNGs + `ledger-after.json`); no live Vite dev server on `3000`/`5173`/`8080`/`1420` (`curl` returned `000` on localhost) and `ledger-before.json` was not persisted.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 2/4 | Non-ASCII punctuation (`—`, `…`, `·`) in 4 files, generic `Send`/`Cancel` in `SendToAgent.tsx`, WCAG 2.5.3 `aria-label` mismatch in `AgentStrip.tsx`, and empty/error copy drift |
| 2. Visuals | 2/4 | Strong Dual-Surface framing on single-pane desktop, but PNGs expose clipped/overlapping toolbars in `TerminalSessionPane`, clipped `Insert into Prompt` in `RunbookPanel`, clipped `HistorySearchPopover` at `900x600`, and 3 `size-3.5` icons in `TermSearchBar.tsx` |
| 3. Color | 3/4 | Zero dark-island/neon/`.text-brass` leaks and WCAG AA/AAA compliance across all 64 ANSI slots (`4.74:1`..`9.66:1` floors), with hex drift vs `446-UI-SPEC.md` table and accent tokens used beyond the 6 declared elements |
| 4. Typography | 2/4 | Strict 4-size (`11px`/`12px`/`14px`/`18px`) & 2-weight (`400`/`600`) discipline and `23 / 23` token parity, but missing `data-terminal-mono="true"` on `RunbookPanel.tsx:235` & `JsonLedgerTable.tsx:271` forces `Palatino` via `index.css:570-576` `!important`, plus unbundled `JetBrains Mono` |
| 5. Spacing | 2/4 | Exact `4px` utility grid and `0px` outer horizontal overflow, but `900x600` header/bar stacking collapses `.xterm`/`BlockStreamView`/`TerminalApprovalCard` below the `64px` floor, `DiscoveryRail` uses `w-8`/`w-64` vs `w-10`/`w-60`/`xl:w-64`, and `.5` utilities remain in 2 files |
| 6. Experience Design | 3/4 | Comprehensive empty/loading/error/destructive states, Single-Textbox invariant, `0` Axe violations, and verified test/doc gates, with `<span role="presentation">` in `HistorySearchPopover.tsx:365`, missing `ledger-before.json`, and `audits.ts` clipping/overlap blind spots |

**Overall: 14/24**

---

## Top 3 Priority Fixes

1. **BLOCKER — Fix `900x600` vertical collapse and `HistorySearchPopover` / `DiscoveryRail` clipping (`Console.tsx`, `TerminalApprovalCard.tsx`, `DiscoveryRail.tsx`)** — At `900x600`, stacked `RomanSurfaceHeader` + wrapped `AgentStrip` + wrapped `ConsoleSessionBar` shrink the body row so `.xterm` collapses to ~12px (`auto-tui-alt-screen--arcane--900x600--after.png`), `BlockStreamView` shrinks to ~50px (`block-stream-ansi--travertine--900x600--after.png`), `TerminalApprovalCard` clips its command and all 4 buttons inside `max-h-28` (`approval-card-pending--arcane--900x600--after.png`), and `Console.tsx:383` `overflow-hidden` decapitates `HistorySearchPopover` (`history-search-open--arcane--900x600--after.png`) — Compact `RomanSurfaceHeader` / `AgentStrip` / `ConsoleSessionBar` height below `1100px`, enforce `min-h-16` (`64px`) / `>= 220px` terminal viewport budget, remove `overflow-hidden` from `Console.tsx:383` (keep `overflow-hidden` on leaf stream containers only), and size `DiscoveryRail` to `w-10` collapsed / `w-60 xl:w-64` expanded (`446-UI-SPEC.md:58, 237-241`).
2. **BLOCKER — Fix compact/split `TerminalSessionPane` header overlap and `RunbookPanel` button clipping (`TerminalSessionPane.tsx`, `RunbookPanel.tsx`)** — `TerminalSessionPane.tsx:353-370` uses `h-8 justify-between` with `shrink-0` on both the git branch chip (`L357`) and right toolbar (`L360`), causing `View Blocks`/`View Raw VT` to collide with `term:s-1`/`feat/roman-scriptorium` and clip `Open in Console` / `Close Terminal Pane` in `split-panes` (`900x600`, `1440x900`) and `chat-terminal-dock` (`900x600`, `1280x800`, `1440x900`); `RunbookPanel.tsx:238` lacks `flex-wrap` inside `w-64`, clipping `Insert into Prompt` at `1440x900` (`runbooks-open--arcane--1440x900--after.png`) — Add `flex-wrap min-h-8` (removing fixed `h-8` and `shrink-0` on the branch pill) in `TerminalSessionPane.tsx:353-360` and add `flex-wrap` in `RunbookPanel.tsx:238`.
3. **BLOCKER — Restore `--font-terminal-mono` on `RunbookPanel` & `JsonLedgerTable` `<pre>` blocks and normalize typography/copy/fractional spacing (`RunbookPanel.tsx`, `JsonLedgerTable.tsx`, `BlockStreamView.tsx`, `SendToAgent.tsx`, `AgentStrip.tsx`, `TermSearchBar.tsx`, `AgentTab.tsx`, `HistorySearchPopover.tsx`)** — `index.css:570-576` forces `pre:not([data-terminal-mono="true"])` to `var(--font-mono)` (`Palatino`) with `!important`, overriding `RunbookPanel.tsx:235`'s inline style (visible in `runbooks-open--arcane--1440x900--after.png`) and `JsonLedgerTable.tsx:271` — Add `data-terminal-mono="true"` to `RunbookPanel.tsx:235`, `JsonLedgerTable.tsx:271`, `JsonLedgerTable.tsx:180`, `TermSearchBar.tsx:58`, `TerminalSessionPane.tsx:432`, and `HistorySearchPopover.tsx:308, 345`; move `TERMINAL_MONO_STYLE` off `BlockStreamView.tsx:180` onto command/code elements only; normalize remaining `.5` utilities (`HistorySearchPopover.tsx:149`, `TermSearchBar.tsx:65,69,73`), non-ASCII strings, `SendToAgent` labels, and `AgentStrip` `aria-label` prefixes.

---

## Detailed Findings

### Pillar 1: Copywriting (2/4)

- **WARNING — Non-ASCII punctuation in 4 source files (`446-UI-SPEC.md:179` requires pure 7-bit ASCII):**
  - `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx:30`: `'0 of 0 matches — try another pattern or press Esc to clear'` uses Unicode em-dash `—` (`U+2014`) instead of ASCII `-`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/AgentTab.tsx:50`: `waiting for events…` uses Unicode ellipsis `…` (`U+2026`) instead of ASCII `...`.
  - `crates/vox-gui/ui/src/components/surfaces/Console/AgentStrip.tsx:47, 51` and `crates/vox-gui/ui/src/components/surfaces/Console/SendToAgent.tsx:46`: `${a.name} · ${a.state}` uses Unicode middle dot `·` (`U+00B7`) instead of ASCII `-` or `:`.
- **WARNING — Generic labels and copy mismatch in `SendToAgent.tsx` (`446-UI-SPEC.md:187`):**
  - `SendToAgent.tsx:61, 64` renders generic `Send` and `Cancel` buttons without `aria-label`, whereas `446-UI-SPEC.md:187` requires `Send Block to Agent` (`aria-label="Send Block to Agent"`) and `Dismiss Agent Sheet` (`aria-label="Dismiss Agent Sheet"`).
- **WARNING — WCAG 2.5.3 Label-in-Name failure in `AgentStrip.tsx` & missing `title` in `ConsoleSessionBar.tsx`:**
  - `AgentStrip.tsx:48, 51`: visible text is `{a.name} · {a.state}` while `aria-label` is `Open agent ${a.name} (${a.state})`. Per `446-UI-SPEC.md:179`, `aria-label` MUST start with the visible text string (`${a.name} - ${a.state} - open agent events` or matching visible prefix).
  - `ConsoleSessionBar.tsx:120-128`: `term-session-close-{id}` sets `aria-label="Close session ${s.id}"` without `title="Close session ${s.id}"` specified in `446-UI-SPEC.md:185`.
- **WARNING — Empty & error copy drift against `## Copywriting Contract` (`446-UI-SPEC.md:190-194`):**
  - `BlockStreamView.tsx:190`: `'No command blocks recorded yet. Enter a shell command below or press Cmd+J to focus the prompt.'` differs from spec (`'No command blocks recorded in this session yet. Enter a shell command, ?prompt, !nu pipeline, or /slash command in the prompt editor below.'`).
  - `DiscoveryRail.tsx:171-172`: `'Start typing a vox command to see live help and idioms.'` differs from spec (`'Start typing a vox command in the prompt editor to inspect live subcommand help, flags, and runnable examples.'`).
  - `Console.tsx:359` renders `{orchError}` from `useOrchestratorStatus.ts:38-48`, and `TerminalSessionPane.tsx:394` renders `'Terminal session disconnected: ...'` instead of `'Terminal session bridge unavailable: ...'`.
- **PASS:** `+ new session`, `Split Pane Right`, `Split Pane Down`, `Close Terminal Pane`, `Use Suggested Action`, `Open in Console`, `23 / 23 Verified` (`SettingsView.tsx:1918`), `Explain / Fix Failed Block`, `Approve`, `Edit & Run`, `Allow Pattern in Session`, `Reject + Steer`, `Confirm Close Session` / `Keep Session Open`, and `Return to Terminal` match spec and WCAG 2.5.3 Label-in-Name.

### Pillar 2: Visuals (2/4)

- **BLOCKER — Compact and split `TerminalSessionPane` header overlaps and clips controls (`TerminalSessionPane.tsx:353-370`):**
  - Left badge container (`L354`) has a `shrink-0` git branch pill (`L357`), while the right action cluster (`L360`) has `shrink-0` inside an `h-8` non-wrapping bar.
  - Confirmed in `split-panes--arcane--900x600--after.png` (`Close Terminal Pane` clipped to `× C`), `split-panes--slate--1440x900--after.png` (`Close Terminal Pan` clipped and left badges pushed off-screen), `chat-terminal-dock--arcane--900x600--after.png` (`View Blocks`/`View Raw VT` overlap `term:s-1 (5)` & `feat/roman-scriptorium`, `Open in Console` clipped to `OPEN IN CONSO`), and `chat-terminal-dock--slate--1280x800--after.png` / `chat-terminal-dock--arcane--1440x900--after.png` (`feat/roman-sc` colliding with `View Blocks`).
- **BLOCKER — `RunbookPanel.tsx:238` clips `Insert into Prompt` at `1440x900`:**
  - Confirmed in `runbooks-open--arcane--1440x900--after.png`: `flex items-center justify-end gap-2` without `flex-wrap` inside the `256px` `DiscoveryRail` pushes `Insert into Prompt` past the left border (`...ERT INTO PROMPT`).
- **BLOCKER — `Console.tsx:383` `overflow-hidden` clips `HistorySearchPopover` at `900x600` & `DiscoveryRail` overlay covers terminal controls:**
  - Confirmed in `history-search-open--arcane--900x600--after.png`: the `HistorySearchPopover` (`bottom-[calc(100%+8px)]`) is clipped mid-list by `Console.tsx:383`'s `overflow-hidden`, hiding the title, `Clear History`, `Dismiss History`, and searchbox `<input>`.
  - Confirmed in `runbooks-open--arcane--900x600--after.png`: expanded `DiscoveryRail` (`z-20`) covers `View Raw VT` / `Search Output` / `Run Command` and truncates the runbook list inside the short body container.
- **WARNING — `DiscoveryRail.tsx:96-114, 149` collapsed width overflow and chevron direction:**
  - Confirmed in `block-stream-ansi--travertine--900x600--after.png` and `json-ledger-table--travertine--900x600--after.png`: `<aside className="w-8">` (`32px`) wraps `<Glass padding="p-1">` (`border-2` = `4px` + `p-1` = `8px`), leaving `20px` inner width so the `size-6` (`24px`) button extends past the container border.
  - `DiscoveryRail.tsx:110` renders `<Icon.chevR>` (`>`) when collapsed on the right edge (instead of `<Icon.chevL>` `<` pointing inward to expand), and `L149` renders `<Icon.chevL>` when expanded.
- **WARNING — Fractional `size-3.5` icon classes remain in `TermSearchBar.tsx:65, 69, 73` (`3` occurrences):**
  - `Icon.arrowUp`, `Icon.arrowDown`, `Icon.close` use `size-3.5` (`14px`) instead of `size-3` (`12px`) or `size-4` (`16px`).
- **PASS:** `Icon.*` is used exclusively (`55` `size-3`, `18` `size-4`, `7` `size-6`, `5` `size-2`, `1` `size-5`), `0` emojis, `0` guillemets, `0` `iconIssueCount` across 112 cells, and single-pane `1100x720..1920x1080` captures (`block-stream-ansi--arcane--1440x900--after.png`, `auto-tui-alt-screen--slate--1280x800--after.png`) show clear Dual-Surface hierarchy.

### Pillar 3: Color (3/4)

- **Class/Token Usage Counts (`Console/*` non-test + `ChatTerminalBlockCard.tsx`):**
  - `text-primary`: 103 | `text-accent`: 39 | `border-brass`: 21 | `bg-brass`: 10 | `border-accent`: 1
  - Zero occurrences of `zinc-`, `slate-`, `bg-black`, `bg-white`, `text-white`, `text-black`, `border-cyan`, `text-brass`, or banned neon RGBs (`#34d399`, `#6ee7b7`, `#22d3ee`, `#06b6d4`, `#10b981`).
  - `ledger-after.json`: `terminalContrastViolationCount === 0`, `lightThemeContrastLeakCount === 0`, and `digitalColorLeakCount === 0` across all `112 / 112` cells.
- **WARNING — Exact hex mismatch between `terminalTheme.ts` and `446-UI-SPEC.md` 64-color ANSI table (`L152-167`):**
  - `terminalTheme.ts:45-119` calibrates all 64 colors and passes WCAG AA (`>= 4.5:1`) / AAA (`>= 7.0:1`): `ARCANE_ANSI_16` min `4.74:1` (`black #8b7e6d` on `#15110d`), `TRAVERTINE_ANSI_16` min `4.96:1` (`black #8e8477` on `#171512`), `SLATE_ANSI_16` min `5.57:1` (`black #7c8b9e` on `#0c0e10`), `HIGH_CONTRAST_ANSI_16` min `9.66:1` (`black #a3b1c6` on `#000000`).
  - However, all 16 `ARCANE_ANSI_16` hexes (e.g. `black: '#8b7e6d'` vs spec `#8E8374`, `brightCyan: '#8ad8db'` vs spec `#5EEAD4`), all 16 `TRAVERTINE_ANSI_16` hexes, `SLATE_ANSI_16.brightCyan` (`#7dd3fc` vs spec `#67E8F9`), and 4 `HIGH_CONTRAST_ANSI_16` hexes (`black`, `red`, `cyan`, `white`) diverge from `446-UI-SPEC.md:152-167` (and `446-UI-SPEC.md:168` cites `5.05:1`/`5.22:1`/`8.27:1` floors while shipped code achieves `4.74:1`/`4.96:1`/`9.66:1`).
- **WARNING — Accent token usage beyond the 6 declared elements & hardcoded `#D4AF37` ring in `Glass.tsx:53`:**
  - `text-accent` (39) and `border-brass`/`bg-brass` (31) are also applied to `AgentTurnContent` model pills (`L314`), `JsonLedgerTable` `Copied` status (`L200`) & nested cell toggles (`L266`), `HistorySearchPopover` fuzzy matches (`L136`) & clock icon (`L243`), `BlockCard` role/pin badges (`L209-221`) & `Correct` button (`L352`), `ConsoleSessionBar` `Voxify Subshell` (`L158`), and `ScriptoriumPromptEditor` `AI Assist` (`L357`) / `Run Command` (`L430`) / `promptHighlighter.ts` (`L19-26`).
  - `DiscoveryRail.tsx:97, 137` mounts `<Glass>` (`Glass.tsx:53`), which injects `ring-[#D4AF37]/10` across all themes.
- **PASS:** `index.css:258-317` `--terminal-well-*` values match `446-UI-SPEC.md` 100%, `:not(.xterm):not([data-terminal-well="true"])` guards prevent `#1C140C` espresso bleed, `wellBg`/`wellFg` indirection avoids `GENERAL_DARK_HEX_BG_REGEX` violations, and `Button` (`data-slot="button"`) safely bypasses light/dark global button overrides (`index.css:413, 1209`).

### Pillar 4: Typography (2/4)

- **Size & Weight Distribution (`Console/*` non-test + `ChatTerminalBlockCard.tsx`):**
  - Sizes: `text-[11px]` (`147`), `text-xs` (`67`), `text-sm` (`13`), plus `18px` in `RomanSurfaceHeader`. Zero non-spec sizes (`9px`, `10px`, `10.5px`, `11.5px`, `13px`, `15px+`).
  - Weights: `font-semibold` (`161`), `font-normal` (`44`). Zero `font-bold`, `font-medium`, `font-light`, or `font-extrabold`.
- **BLOCKER — `index.css:570-576` `!important` forces proportional `Palatino` onto `<pre>` elements missing `data-terminal-mono="true"`:**
  - `index.css:570-576` sets `font-family: var(--font-mono, 'Palatino', ...) !important` on `pre:not(.xterm):not([data-terminal-mono="true"]):not([data-terminal-well="true"])`.
  - `RunbookPanel.tsx:235` (`<pre style={RUNBOOK_MONO_STYLE}>`) and `JsonLedgerTable.tsx:271` (`<pre ...>`) omit `data-terminal-mono="true"`, so `!important` overrides the inline `style` in `arcane`/`travertine`/`high-contrast`. Visually confirmed in `runbooks-open--arcane--1440x900--after.png` where `vox run scripts/verify-terminal.vox --theme arcane` renders in proportional `Palatino` serif rather than fixed-width monospace.
  - Similarly, `JsonLedgerTable.tsx:180`, `TermSearchBar.tsx:58`, `TerminalSessionPane.tsx:432`, and `HistorySearchPopover.tsx:308, 345` set inline `fontFamily: TERMINAL_MONO_FONT_STACK` without `data-terminal-mono="true"`, bypassing `index.css:584-592` ligature and `letter-spacing: 0 !important` locks.
- **WARNING — Prose leakage from container-level `TERMINAL_MONO_STYLE` & unbundled `'JetBrains Mono'`:**
  - `BlockStreamView.tsx:180` applies `style={TERMINAL_MONO_STYLE}` on `<section>`, styling `<EmptyState>` proportional prose (`Terminal session ready`) in monospace (`split-panes--slate--1440x900--after.png`).
  - `crates/vox-gui/ui/public/fonts/` and `crates/vox-gui/ui/src/styles/fonts.css` bundle `Cinzel`, `EB Garamond`, and `Inter` only — `'JetBrains Mono'` is not self-hosted under Tauri `font-src 'self'`, falling back to OS `ui-monospace` / `'SFMono-Regular'`.
- **PASS:** 6-file `--font-terminal-mono` token lockstep (`primitive.json`, 4 `tokens*.generated.css`, `tokens.generated.ts`, `theme-contract.v1.json`, `themeContract.ts`, `themeContract.test.ts`, `cv-flywheel.spec.ts`) is verified (`23 / 23 Verified`, `0` banned font literals), `.xterm-helper-textarea` zero-box lock (`index.css:594-611`) passes with `xtermHelperLeakCount === 0`, and `terminalFontProportionalLeakCount === 0` across all audited `[data-terminal-mono="true"]` / `.xterm` / `<pre>` wells.

### Pillar 5: Spacing (2/4)

- **Spacing Utility Counts (`Console/*` non-test + `ChatTerminalBlockCard.tsx`):**
  - Top classes: `px-2` (132), `py-1` (125), `gap-2` (95), `gap-1` (76), `p-2` (28), `px-3` (14), `py-2` (9), `p-3` (7), `px-1` (5), `p-4` (5). `4px` grid adhered to on outer padding (`p-2 min-[1100px]:p-3` on `Console.tsx:354`, `p-2`/`p-3` on `BlockCard.tsx:196` & `TerminalApprovalCard.tsx:115`).
- **BLOCKER — `900x600` vertical layout starvation violates `64px` (`min-h-16`) / `220px` terminal viewport floor:**
  - Observed in `auto-tui-alt-screen--arcane--900x600--after.png` (`.xterm` viewport ~12px tall), `block-stream-ansi--travertine--900x600--after.png` (`BlockStreamView` ~50px tall), `split-panes--arcane--900x600--after.png` (split panes < 64px tall with `ConsoleSplitGrid.tsx:142` `min-h-[180px]` floor exceeding available height), and `approval-card-pending--arcane--900x600--after.png` (`TerminalApprovalCard` truncated inside `TerminalApprovalCard.tsx:392` `max-h-28` to ~48px hiding command + CTAs).
- **WARNING — Remaining fractional `.5` utilities & `DiscoveryRail` width deviation (`446-UI-SPEC.md:58, 64, 237-241`):**
  - `HistorySearchPopover.tsx:149`: `mx-0.5` (`2px` fractional margin).
  - `TermSearchBar.tsx:65, 69, 73`: `size-3.5` (`14px` fractional icon size).
  - `DiscoveryRail.tsx:96`: collapsed strip uses `w-8` (`32px`) instead of `w-10` (`40px`); `DiscoveryRail.tsx:134` uses fixed `w-64` (`256px`) across all breakpoints instead of `w-60` (`240px`) below `1280px` and `xl:w-64`.
- **PASS:** `outerOverflowPx === 0` and `viewportCoverageRatio` (`0.779..0.886` within `[0.45, 0.92]`) pass across all 112 cells in `ledger-after.json`.

### Pillar 6: Experience Design (3/4)

- **State & Guard Coverage:**
  - `empty`: `BlockStreamView.tsx:184-192`, `DiscoveryRail.tsx:169-183`, `RunbookPanel.tsx:169-179`, `JsonLedgerTable.tsx:206-214, 243-249`, `HistorySearchPopover.tsx:313-323`.
  - `loading`: `BlockCard.tsx:279` / `AgentTurnContent.tsx:321-348` streaming status (`aria-busy={isRunning}`, `aria-live="polite"`), `BlockCard.tsx:354-360` `Diagnosing Failed Block...` disabled state, `TerminalApprovalCard.tsx:47` `busy` guard.
  - `error`: `Console.tsx:359` `orchError` (`role="alert"`), `TerminalSessionPane.tsx:392-397` `disconnectError` + `Reconnect Session`, `RunbookPanel.tsx:153-168` `scanError` + `Reload Runbooks`, `AgentTurnContent.tsx:350-391, 411-432` Clavis diagnostic & `Retry Agent Turn`, `TerminalApprovalCard.tsx:265-273` `editError`, all sanitized via `sanitizeErrorForToast`.
  - `destructive confirmation`: `ConsoleSessionBar.tsx:187-201` (`Confirm Close Session` / `Keep Session Open` + `Esc`), `HistorySearchPopover.tsx:247-270` (`Confirm Clear History` / `Keep History`).
  - `a11y & contracts`: Single visible `role="textbox"` invariant preserved on initial mount; `0` raw `<select>`/`<input type="range">` (`<VoxSelect>` everywhere); `axeViolations === 0` across all 112 `ledger-after.json` cells; `warp-roman-terminal-v75-research-2026.md` promoted to `status: "current"` (`0` `vox-doc-pipeline` lint errors); Vitest (`36 files / 211 tests` + `cvFlywheel.test.ts`), `console-workbench.spec.ts`, `Phase 386`, `toestub`, and `vox-arch-check` reported passing in `446-04-SUMMARY.md`.
- **WARNING — Non-semantic interactive `<span>` in `HistorySearchPopover.tsx:365-376`:**
  - `Insert Command` uses `<span role="presentation" onClick={...}>` inside `role="option"`, making per-row insert unfocusable via `Tab` and invisible as a button to screen readers (`SendToAgent.tsx:30` also mounts an inline `role="dialog"` without focus trap/initial focus shift).
- **WARNING — CV Flywheel `before -> after` baseline missing & `audits.ts` detection blind spots:**
  - `crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-before.json` does not exist; all `112` entries in `ledger-after.json` show `visualBeforeScore === visualAfterScore` (`terminal-cv-flywheel.spec.ts:319-321` fallback), leaving `CV_STAGE=before` baseline deltas unrecorded.
  - `audits.ts:116-134, 198-242` checks clipping only on direct button `scrollWidth > clientWidth` and restricts overlap checks to `button[aria-expanded]` intersected with `overflow` ancestors, failing to flag ancestor-clipped buttons (`RunbookPanel` `Insert into Prompt`, `TerminalSessionPane` `Close Terminal Pane`, `HistorySearchPopover` at `900x600`) and non-`aria-expanded` toolbar overlaps (`TerminalSessionPane` `View Blocks` over `term:s-1` in `chat-terminal-dock`).

---

## Files Audited

- **Design Spec, Summaries & Architecture Docs:**
  - `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-UI-SPEC.md`
  - `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-01-SUMMARY.md`
  - `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-02-SUMMARY.md`
  - `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-03-SUMMARY.md`
  - `.planning/workstreams/axis-warp-terminal-v75/phases/446-roman-scriptorium-theme-and-cv-flywheel/446-04-SUMMARY.md`
  - `docs/src/architecture/warp-roman-terminal-v75-research-2026.md`
- **Theme Contract, CSS, Tokens & UI Primitives:**
  - `contracts/gui/theme-contract.v1.json`
  - `crates/vox-gui/ui/src/index.css`, `src/styles/fonts.css`, `src/styles/tokens.generated.css`, `src/styles/tokens.slate.generated.css`, `src/styles/tokens.contrast.generated.css`, `src/styles/tokens.travertine.generated.css`
  - `crates/vox-gui/ui/src/lib/themeContract.ts`, `src/lib/themeContract.test.ts`, `src/lib/cvFlywheel.ts`, `src/lib/cn.ts`
  - `crates/vox-gui/ui/src/components/ui/{Button.tsx,Glass.tsx,Icons.tsx,RomanSurfaceHeader.tsx,Select.tsx}`
- **Console & Chat Surfaces:**
  - `crates/vox-gui/ui/src/components/surfaces/Console/{terminalTheme.ts,TerminalTab.tsx,TerminalTab.test.tsx,sgrToSpans.ts,BlockCard.tsx,BlockStreamView.tsx,TerminalSessionPane.tsx,AgentStrip.tsx,AgentTab.tsx,SendToAgent.tsx,InputEditor.tsx,ScriptoriumPromptEditor.tsx,promptHighlighter.ts,TerminalApprovalCard.tsx,HistorySearchPopover.tsx,HistorySearchPopover.test.tsx,AgentTurnContent.tsx,ConsoleSessionBar.tsx,ConsoleSplitGrid.tsx,DiscoveryRail.tsx,DiscoveryRail.test.tsx,JsonLedgerTable.tsx,RunbookPanel.tsx,SlashMetaContent.tsx,TermSearchBar.tsx,useConsoleWarpActions.ts,webglAddon.ts,Console.tsx}`
  - `crates/vox-gui/ui/src/components/surfaces/Chat/{ChatTerminalBlockCard.tsx,ChatSurface.tsx}`
  - `crates/vox-gui/ui/src/components/surfaces/Settings/SettingsView.tsx`
- **CV Flywheel Specs, Ledgers & Captured PNG Screenshots:**
  - `crates/vox-gui/ui/e2e/review/{terminal-cv-flywheel.spec.ts,terminalAudits.ts,audits.ts,vlmVisionFlywheel.ts,states.ts}`
  - `crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts`
  - `crates/vox-gui/ui/test-results/cv-flywheel-terminal/ledger-after.json` (112 cells)
  - Captured PNGs inspected: `block-stream-ansi--arcane--1440x900--after.png`, `block-stream-ansi--arcane--1920x1080--after.png`, `block-stream-ansi--high-contrast--1100x720--after.png`, `block-stream-ansi--travertine--900x600--after.png`, `split-panes--arcane--900x600--after.png`, `split-panes--slate--1440x900--after.png`, `history-search-open--arcane--900x600--after.png`, `history-search-open--arcane--1440x900--after.png`, `json-ledger-table--arcane--1440x900--after.png`, `json-ledger-table--travertine--900x600--after.png`, `runbooks-open--arcane--900x600--after.png`, `runbooks-open--arcane--1440x900--after.png`, `approval-card-pending--arcane--900x600--after.png`, `approval-card-pending--high-contrast--1440x900--after.png`, `auto-tui-alt-screen--arcane--900x600--after.png`, `auto-tui-alt-screen--slate--1280x800--after.png`, `chat-terminal-dock--arcane--900x600--after.png`, `chat-terminal-dock--arcane--1440x900--after.png`, `chat-terminal-dock--slate--1280x800--after.png`, `chat-terminal-dock--high-contrast--1440x900--after.png`
