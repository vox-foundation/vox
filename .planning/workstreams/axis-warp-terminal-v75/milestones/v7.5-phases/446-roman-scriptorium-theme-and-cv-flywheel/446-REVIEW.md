---
phase: 446-roman-scriptorium-theme-and-cv-flywheel
reviewed: 2026-11-16T02:58:17Z
depth: deep
files_reviewed: 54
files_reviewed_list:
  - crates/vox-gui/ui/tokens/primitive.json
  - crates/vox-gui/ui/src/styles/tokens.generated.css
  - crates/vox-gui/ui/src/styles/tokens.slate.generated.css
  - crates/vox-gui/ui/src/styles/tokens.contrast.generated.css
  - crates/vox-gui/ui/src/styles/tokens.travertine.generated.css
  - crates/vox-gui/ui/src/styles/tokens.generated.ts
  - crates/vox-gui/ui/src/styles/fonts.css
  - contracts/gui/theme-contract.v1.json
  - crates/vox-gui/ui/src/lib/themeContract.ts
  - crates/vox-gui/ui/src/lib/themeContract.test.ts
  - crates/vox-gui/ui/src/lib/cvFlywheel.ts
  - crates/vox-gui/ui/src/index.css
  - crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentStrip.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTab.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SendToAgent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ScriptoriumPromptEditor.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TerminalApprovalCard.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/SlashMetaContent.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/DiscoveryRail.test.tsx
  - crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx
  - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx
  - crates/vox-gui/ui/src/hooks/useTerminalPendingApprovals.ts
  - crates/vox-gui/ui/e2e/review/terminalAudits.ts
  - crates/vox-gui/ui/e2e/review/terminalAudits.test.ts
  - crates/vox-gui/ui/e2e/review/audits.ts
  - crates/vox-gui/ui/e2e/review/audits.test.ts
  - crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts
  - crates/vox-gui/ui/e2e/review/states.ts
  - crates/vox-gui/ui/e2e/review/capture.spec.ts
  - crates/vox-gui/ui/e2e/review/cv-flywheel.spec.ts
  - crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts
  - crates/vox-gui/ui/e2e/lib/terminalRichFixtures.ts
  - crates/vox-gui/ui/e2e/lib/tauriMockRich.ts
  - crates/vox-gui/ui/e2e/lib/tauriMock.ts
  - crates/vox-gui/ui/e2e/console-workbench.spec.ts
  - docs/src/architecture/warp-roman-terminal-v75-research-2026.md
findings:
  critical: 2
  warning: 7
  info: 4
  total: 13
status: issues_found
---

# Phase 446: Code Review Report

**Reviewed:** 2026-11-16T02:58:17Z
**Depth:** deep
**Files Reviewed:** 54
**Status:** issues_found

## Summary

Deep read-only review of Phase 446 (`446-roman-scriptorium-theme-and-cv-flywheel`, Plans `446-01..04`, `TERM-ROMA-CV-01..04`) covering Style Dictionary tokens, 3-way theme contract parity, `index.css` Dual-Surface isolation, Console & Chat terminal components, E2E fixtures/mocks, CV/DOM audit harnesses, captured `ledger-after.json` screenshots, and promoted SSOT documentation.

**What passed cleanly (directly verified):**
- **3-way contract & token parity:** `contracts/gui/theme-contract.v1.json`, `crates/vox-gui/ui/src/lib/themeContract.ts`, `tokens/primitive.json`, and all 4 `tokens*.generated.css` bundles define all `23` required semantic variables (`--font-terminal-mono` + `--terminal-well-*`). Zero banned font literals (`Courier New`, `Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`) appear anywhere in `crates/vox-gui/ui/src/` outside `themeContract.ts:108-113` and `themeContract.test.ts:147`.
- **64-color ANSI WCAG compliance & indirection:** All 4×16 ANSI palettes in `terminalTheme.ts` meet WCAG contrast requirements (`arcane` min `black=#8b7e6d` `4.74:1`, `travertine` min `black=#8e8477` `4.96:1`, `slate` min `black=#7c8b9e` `5.57:1`, `high-contrast` min `black=#a3b1c6` `9.66:1 >= 7.0`). All terminal wells use `--terminal-well-*` variables (`0` `bg-[#...]` / `zinc-` / `slate-` / `bg-black` / `border-cyan` / `.text-brass` in production Console/Chat JSX).
- **XSS/URL sanitization & `.toString()` containment:** `sgrToSpans.ts` restricts external URLs to `http://`/`https://`, rejects flag/scheme/traversal/drive-letter injections via `hasUnsafeSchemePrefix` + `sanitizeWorkspaceFileLocator`, and avoids `dangerouslySetInnerHTML`. `collectTerminalAuditMetrics`, `auditDomQualityInPage`, `buildTerminalRichFixtures`, and `installTerminalRichMock` are 100% self-contained on `Function.prototype.toString()`.
- **Static & unit verification (post-`446-04`):** `tsc --noEmit` exited `0`; Vitest ran `37` files / `215` tests passing (`9.31s`); `toestub --rules vox/ui/theme-contract-drift` and `toestub --rules vox/llm-provider-call` returned clean; `vox-doc-pipeline --lint-only` exited `0`; `vox-arch-check` exited `0`; all Phase 446-created/focused source files are `< 435` non-blank LoC.

**Why `status: issues_found` (`2` Critical, `7` Warning, `4` Info):**
Inspecting the actual `ledger-after.json` PNG screenshots alongside source & audit code uncovered two blocking gaps that the CV audit harness reported as `0` defects:
1. **`CR-01`:** In `auto-tui-alt-screen`, the real `@xterm/xterm` (`^6.0.0`) viewport renders **zero visible text characters** across `arcane`, `slate`, and `high-contrast` (only an empty dark well with the cursor stranded at row ~7 col 0), while `terminalAudits.ts:31` skips `.xterm-rows` because `@xterm/xterm` sets `aria-hidden="true"` on `.xterm-rows`.
2. **`CR-02`:** At `900x600` in `split-panes`, `Close Terminal Pane` is horizontally sliced mid-text (`X C`) by `ConsoleSplitGrid`'s `overflow-hidden` leaf container, and at `1440x900` in `runbooks-open`, `Insert into Prompt` is sliced (`ERT INTO PROMPT`) inside `DiscoveryRail` (`w-64`), while `audits.ts` misses ancestor-clipped buttons outside `chat-execution-rail`.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `auto-tui-alt-screen` `.xterm` viewport renders zero visible text glyphs while `terminalAudits.ts` skips `.xterm-rows` via `[aria-hidden="true"]`

**File:** `crates/vox-gui/ui/e2e/review/terminalAudits.ts:27-35, 56-65, 108-130, 241-271`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:143-176, 300-324`, `crates/vox-gui/ui/test-results/cv-flywheel-terminal/screenshots/auto-tui-alt-screen--arcane--1440x900--after.png`
**Issue:**
Visual and pixel-level inspection of the captured `CV_STAGE=after` screenshots (`auto-tui-alt-screen--arcane--1440x900--after.png`, `auto-tui-alt-screen--slate--1440x900--after.png`, `auto-tui-alt-screen--high-contrast--1440x900--after.png`) shows the `TUI Active - Return on exit` banner and a cursor parked on row ~7 col 0 (matching the 6-line `replayUtf8` payload in `terminalRichFixtures.ts:34-42`), but **zero visible text glyphs** inside the terminal well. Direct PNG RGB inspection of the well interior (`x=260..1050, y=400..700` on `1440x900`) finds only `#15110d` (`236,210` px) and `#71591a` (`790` px) — `0` foreground text pixels.

This severe rendering regression went undetected (`terminalStartupSpamCount=0`, `terminalFontProportionalLeakCount=0`, `terminalContrastViolationCount=0`, `112/112` pass) because `@xterm/xterm@6.0.0`'s `DomRenderer` sets `aria-hidden="true"` directly on `.xterm-rows` (`node_modules/@xterm/xterm/lib/xterm.js`), and `terminalAudits.ts:31` excludes any element matching `el.closest('.sr-only, [hidden], [aria-hidden="true"], details:not([open])')`. Consequently, `terminalAudits.ts` skips `.xterm-rows` and every child `span` in real xterm mounts (note `terminalAudits.test.ts:64` mocked `<div class="xterm-rows">` without `aria-hidden="true"`).
Candidate driver in `TerminalTab.tsx`: xterm opens into `terminal-vt-layer` while `opacity-0 -z-10`, `WebglTerminalAddon` only probes an offscreen `webgl2` canvas and immediately calls `loseContext()`, and `term.reset()` + `term.write(replaySnapshot.text)` runs synchronously alongside `fit()` on visibility transition without verifying non-empty rendered `.xterm-rows` rows.
**Fix:**
1. Update `terminalAudits.ts` so `isVisible` checks host visibility when auditing `.xterm` / `.xterm-rows` instead of rejecting xterm's own `aria-hidden="true"` on `.xterm-rows`, and assert non-empty rendered text inside `.xterm-rows` whenever `[data-testid="terminal-viewport"] .xterm` is visible:
```ts
const isXtermRowContent = (el: HTMLElement): boolean =>
  Boolean(el.closest('.xterm-rows')) && !el.closest('[hidden], .sr-only');

// Inside isVisible(el): allow .xterm-rows unless the ancestor .xterm host itself is aria-hidden/hidden
const ariaHiddenAncestor = el.closest('[aria-hidden="true"]');
if (ariaHiddenAncestor && !(isXtermRowContent(el) && ariaHiddenAncestor.classList.contains('xterm-rows'))) {
  return false;
}
```
2. Reproduce `auto-tui-alt-screen` in Chromium (`TerminalTab.tsx`), ensure `term.write(replaySnapshot.text)` paints visible rows (check `term.reset()` ordering / `term.refresh(0, term.rows - 1)` after `fit()` when `active` flips `true`, and `states.ts:144` waiting for non-empty `.xterm-rows` text).

---

### CR-02: Split panes at `900x600` (`Close Terminal Pane`) and Runbooks (`Insert into Prompt`) clip interactive buttons horizontally while `audits.ts` misses ancestor-clipped controls

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx:353-370`, `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx:141-143`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:238-245`, `crates/vox-gui/ui/e2e/review/audits.ts:116-134`
**Issue:**
Captured `CV_STAGE=after` screenshots show two interactive buttons visibly clipped mid-label, contradicting `clippedInteractiveCount === 0` in `ledger-after.json`:
1. In `split-panes--arcane--900x600--after.png`, each split leaf inside `ConsoleSplitGrid` (`overflow-hidden` at `ConsoleSplitGrid.tsx:141`) is ~`370px` wide, while `TerminalSessionPane`'s header row (`flex h-8 shrink-0 items-center justify-between` at `TerminalSessionPane.tsx:353`) holds `View Blocks | View Raw VT | Search Output | Close Terminal Pane` inside `shrink-0`. On both left and right panes, `Close Terminal Pane` overflows the right edge of its pane container and is sliced to `x C`.
2. In `runbooks-open--arcane--1440x900--after.png`, inside `DiscoveryRail` (`w-64`, `256px`), `RunbookPanel.tsx:238` renders `Insert into Prompt` and `Run Runbook` in `flex items-center justify-end gap-2` without `flex-wrap`, overflowing left past the card edge and slicing `Insert into Prompt` to `ERT INTO PROMPT`. (At `900x600`, `DiscoveryRail` overlays a ~`175px`-tall middle region, pushing both runbook cards off-screen below the scroll fold, which `states.ts:112` bypassed by waiting for `state: 'attached'` instead of `'visible'`.)

`audits.ts:116-134` missed both clippings because `clippedInteractiveCount` only tests self-overflow on the button (`el.scrollWidth > el.clientWidth + 4`), `[role="dialog"]` overflow, or right-edge overflow against `[data-testid="chat-execution-rail"]` / `[data-testid="context-window-meter"]` — never checking general `overflow-x: hidden|auto|scroll|clip` ancestors such as split-pane wrappers or rail cards.
**Fix:**
1. Wrap or compact split-pane header actions (`flex-wrap` / `h-auto min-h-8` or icon-only close button when split/compact) in `TerminalSessionPane.tsx:353-370`, and add `flex-wrap` to `RunbookPanel.tsx:238`:
```tsx
// RunbookPanel.tsx:238
<div className="flex flex-wrap items-center justify-end gap-2">
```
2. Extend `clippedInteractiveCount` in `crates/vox-gui/ui/e2e/review/audits.ts:117-134` to check whether a visible button extends outside any `overflow-x: hidden|clip|auto|scroll` ancestor (`r.left < ancRect.left - 2 || r.right > ancRect.right + 2` when the ancestor itself is not horizontally scrollable to reveal it or cuts off the control).

---

## Warnings

### WR-01: Silent `catch` in `collectCombinedMetrics` converts `AxeBuilder` failures into `0` violations

**File:** `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts:137-151`
**Issue:**
```ts
  try {
    const axe = await new AxeBuilder({ page })
      .withTags(['wcag2a', 'wcag2aa'])
      .options({ resultTypes: ['violations'] })
      .analyze();
    ...
  } catch {
    axeViolations = 0;
  }
```
If `@axe-core/playwright` fails to inject, times out, or throws, the `catch` block silently zeroes `axeViolations = 0` and records nothing in `axeDetails`, turning an accessibility audit failure into a green ledger cell.
**Fix:** Propagate the error or record an explicit audit failure (`axeViolations = 1`, push an `AxeNodeSummary` entry `{ id: 'axe-runner-error', impact: 'critical', targets: [], summary: String(err) }`) so a broken Axe run cannot pass `findEntryFailures`.

---

### WR-02: `SURFACE_STATES['console']` / `['chat']` strip `initFlags` and `approval-card-pending` swallows timeout via `.catch(() => {})`

**File:** `crates/vox-gui/ui/e2e/review/states.ts:122-129, 134-145, 154-159, 191`, `crates/vox-gui/ui/e2e/review/capture.spec.ts:58-74`
**Issue:**
`SURFACE_STATES['console']` (`states.ts:154-159`) maps `CONSOLE_CV_STATES` while omitting `initFlags`, and `SURFACE_STATES['chat']` (`states.ts:191`) omits `CHAT_TERMINAL_DOCK_STATE.initFlags` (`{ __MOCK_CHAT_TERM_DOCK__: true }`). `capture.spec.ts:58-74` also has no `initFlags` pre-load injection before `page.goto('/')`. Because `TerminalSessionPane` / `TerminalTab` / `useTerminalPendingApprovals` / `ChatSurface` invoke `term_session_attach`, `vox_pending_approvals`, and `chat_get_messages` immediately on mount, `capture.spec.ts` executes those IPC calls before `setup()` sets the `window.__MOCK_*` / `window.__VOX_E2E_REAL_XTERM__` flags. Additionally, `states.ts:128` ignores timeout failures on `term-approval-card` with `.catch(() => {})`.
**Fix:** Preserve `initFlags` on `ReviewState` / `SURFACE_STATES` and inject them in `capture.spec.ts` via `page.addInitScript` prior to `page.goto('/')`, and drop `.catch(() => {})` on `states.ts:128` (once `initFlags` seeds `__MOCK_TERM_APPROVAL_ACTIVE__` pre-mount, `term-approval-card` appears on the initial poll without needing a fallback swallow).

---

### WR-03: `index.css:817, 1274` injects inner `#FAF5E8`/border boxes onto bordered `input[type="search"]` wrappers

**File:** `crates/vox-gui/ui/src/index.css:817, 1274`, `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:295-311`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:138-145`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:182-189`
**Issue:**
`index.css:817` applies `background-color: #FAF5E8 !important; border: 1.5px solid rgba(197, 155, 39, 0.72) !important; border-radius: 8px;` to `:is(main, [role="dialog"]) input[type="search"]:not([data-slot]):not(.xterm-helper-textarea)...:not([data-terminal-mono="true"])...`, and `index.css:1274` forces `background-color: var(--color-bg-base) !important` in dark/high-contrast themes. Three `Console` search inputs (`term-history-input`, `term-runbook-filter`, `term-ledger-filter`) use `type="search"` with `bg-transparent` inside an already-bordered flex container and lack both `data-terminal-mono="true"` and `data-slot`, producing visible double-bordered inner boxes in `history-search-open--arcane--1440x900--after.png`, `runbooks-open--arcane--1440x900--after.png`, and `json-ledger-table--arcane--1440x900--after.png`.
**Fix:** Tag those borderless inner inputs with `data-slot="inline-search-input"` (or `data-terminal-mono="true"` where monospace is intended, as on `HistorySearchPopover.tsx:299` and `JsonLedgerTable.tsx:184`):
```tsx
<input
  type="search"
  data-slot="inline-search-input"
  data-terminal-mono="true"
  ...
```

---

### WR-04: `RunbookPanel.tsx:235` and `JsonLedgerTable.tsx:271` `<pre>` blocks omit `data-terminal-mono="true"` and are overridden to `Palatino` serif

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:235`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:271`, `crates/vox-gui/ui/src/index.css:569-580`
**Issue:**
`index.css:570-576` applies `font-family: var(--font-mono, 'Palatino'...) !important;` to every `<pre>` element not marked `.xterm`, `[data-terminal-mono="true"]`, or inside `[data-terminal-well="true"]`. Because `index.css:576` uses `!important`, it overrides `RunbookPanel.tsx:235`'s inline `style={RUNBOOK_MONO_STYLE}`, rendering interpolated runbook commands in proportional `Palatino` serif (visible in `runbooks-open--arcane--1440x900--after.png`) and excluding both `RunbookPanel.tsx:235` and `JsonLedgerTable.tsx:271` from `terminalAudits.ts` (`monoTargets` only scans `[data-terminal-mono="true"]`).
**Fix:** Add `data-terminal-mono="true"` and `style={TERMINAL_MONO_STYLE}` to both `<pre>` elements (`RunbookPanel.tsx:235` and `JsonLedgerTable.tsx:271`), importing `TERMINAL_MONO_STYLE` from `./terminalTheme`.

---

### WR-05: `vlmVisionFlywheel.ts` terminal-well floors (`0.92` / `0.88`) and `decodePngRgbaInPage` zero-context fallback mask visual regressions

**File:** `crates/vox-gui/ui/e2e/review/vlmVisionFlywheel.ts:363-375, 401-407`, `crates/vox-gui/ui/e2e/review/terminal-cv-flywheel.spec.ts:119-127`, `crates/vox-gui/ui/src/lib/cvFlywheel.ts:134-141`
**Issue:**
In `vlmVisionFlywheel.ts:363-366`, whenever `visionSignals.hasTerminalWell` is true, `baseQ2` floors at `0.92+` and `q2Answer` (`L373`) treats `hasTerminalWell` as an alternative to `hasVisibleMarbleFrieze`. Similarly, `L401-404` clamps `conceptArtHistogramSimilarity` up to `>= 0.88` whenever a terminal well is present. Combined, these floors caused `auto-tui-alt-screen` (whose `.xterm` rendered empty in `CR-01`) to still receive `vlmOverallScore = 0.9670` (`q5 = 0.9292`). Meanwhile, if `canvas.getContext('2d')` returns `null` in `decodePngRgbaInPage` (`terminal-cv-flywheel.spec.ts:120`), it returns `{ w: 0, h: 0, bytes: [] }`, causing `computeRgbaFrameCvMetrics` (`cvFlywheel.ts:134-141`) to return synthetic `{ 50, 50, 20 }` scores rather than failing.
**Fix:** Throw explicitly in `decodePngRgbaInPage` when `getContext('2d')` is null, and in `vlmVisionFlywheel.ts` avoid substituting `hasTerminalWell` for missing `<RomanSurfaceHeader>` checks or clamping `effectiveHistSim` above measured similarity without factoring rendered terminal text density.

---

### WR-06: Dim ANSI (`opacity-75`) / search `<mark>` contrast drift and unparsed ANSI escapes in mock `TerminalTab` `<pre>`

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts:217, 371`, `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:366-368`, `crates/vox-gui/ui/e2e/review/terminalAudits.ts:254-262`
**Issue:**
1. `sgrToSpans.ts:371` applies Tailwind `opacity-75` for `\x1b[2m` dim spans; on borderline ANSI colors such as `arcane` `black` (`#8b7e6d`, `4.74:1`), `0.75` alpha compositing against `#15110d` lowers contrast below WCAG `4.5:1` AA, and `terminalAudits.ts:254-262` evaluates `getComputedStyle(el).color` without multiplying element `opacity`. Likewise `sgrToSpans.ts:217` wraps search matches in `bg-brass/30`, shifting background luminance behind colored spans.
2. When `isMock` is `true` (`__VOX_E2E_REAL_XTERM__` unset) and `mockReplay` is populated (`terminalRichFixtures.ts:38-40` includes `\x1b[32mREADY\x1b[0m`), `TerminalTab.tsx:367` renders `{mockReplay}` verbatim inside `<pre>` without `sgrToSpans(...)`, leaking raw ESC sequences if View Raw VT is opened in non-real-xterm mock mode.
**Fix:** Parse `mockReplay` through `sgrToSpans(mockReplay)` inside `TerminalTab.tsx:367`, fold element `opacity` into `fg` compositing inside `terminalAudits.ts:254-262`, and ensure `\x1b[2m` dim / search `<mark>` maintain `>= 4.5:1` (`>= 7.0:1` in `high-contrast`, e.g., high-contrast `<mark>` fill/fg).

---

### WR-07: Promoted Architecture SSOT doc claims npm `@xterm/addon-*` packages are installed when local stubs are used

**File:** `docs/src/architecture/warp-roman-terminal-v75-research-2026.md:27, 62`, `crates/vox-gui/ui/package.json:31-32`
**Issue:**
`docs/src/architecture/warp-roman-terminal-v75-research-2026.md` (promoted to `status: "current"`) states at lines 27 and 62: `Installed @xterm/addon-{webgl,search,web-links,unicode11,serialize}` and `Real @xterm/addon-webgl`. However, `crates/vox-gui/ui/package.json` only installs `"@xterm/xterm": "^6.0.0"` and `"@xterm/addon-fit": "^0.11.0"`. The other 5 addons (`webglAddon.ts`, `xtermAddons.ts`) are custom local TypeScript implementations, and `WebglTerminalAddon` (`webglAddon.ts:26-47`) only probes `webgl2` on a throwaway canvas and immediately releases it via `loseContext()`, so xterm actually renders through its DOM renderer (`DomRenderer`).
**Fix:** Update lines 27 and 62 in `docs/src/architecture/warp-roman-terminal-v75-research-2026.md` to state accurately that `@xterm/xterm` + `@xterm/addon-fit` are installed alongside in-repo addon adapters (`webglAddon.ts`, `xtermAddons.ts`) — or actually install and wire `@xterm/addon-webgl`.

---

## Info

### IN-01: Four remaining `.5` (`2px` / `14px`) Tailwind utility classes outside the `4px` spacing grid

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/HistorySearchPopover.tsx:149`, `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx:65, 69, 73`
**Issue:** `HistorySearchPopover.tsx:149` uses `mx-0.5` on the `[REDACTED]` badge and `TermSearchBar.tsx:65, 69, 73` use `size-3.5` on search icons instead of `4px`-grid utilities (`mx-1`, `size-3` or `size-4`).
**Fix:** Replace `mx-0.5` with `mx-1` and `size-3.5` with `size-3` (matching `ConsoleSessionBar.tsx:127`).

### IN-02: Local `TERMINAL_MONO_STYLE` duplication and minor Phase 446 planning artifact drift

**File:** `crates/vox-gui/ui/src/components/surfaces/Console/AgentTurnContent.tsx:7-12`, `crates/vox-gui/ui/src/components/surfaces/Console/RunbookPanel.tsx:13-18`, `crates/vox-gui/ui/src/components/surfaces/Console/JsonLedgerTable.tsx:21-26`
**Issue:** All three files redefine identical `TERMINAL_MONO_STYLE` objects locally (`*_MONO_STYLE`) instead of importing `TERMINAL_MONO_STYLE` from `./terminalTheme`. Also note minor historical doc drift in Phase 446 planning docs: `446-UI-SPEC.md` records superseded ANSI hex values before `446-02-PLAN.md` calibration, and `446-03-SUMMARY.md:46` still cites `\x1b[30;47m` after `446-04` updated `terminalRichFixtures.ts:12` to `\x1b[30m`.
**Fix:** Import `TERMINAL_MONO_STYLE` from `./terminalTheme` in `AgentTurnContent.tsx`, `RunbookPanel.tsx`, and `JsonLedgerTable.tsx`.

### IN-03: Latent CSS exclusions missing on `index.css` tab/table-header/landmark blocks & unbundled `JetBrains Mono` `@font-face`

**File:** `crates/vox-gui/ui/src/index.css:376, 387, 395-410, 466-510, 1073-1091`, `crates/vox-gui/ui/src/styles/fonts.css:1-32`
**Issue:**
1. `index.css` omits `:not(.xterm):not(.xterm *):not([data-terminal-well="true"]):not([data-terminal-well="true"] *)` on `[role="tab"]` (`L376, 387`), `thead` (`L395-410`), `section > header` / `[role="tablist"]` (`L466-510`), and dark-theme `aside, header, footer, main [role="tablist"]` (`L1073-1091`). Currently benign because no `[role="tab"]`, `<table>`, `<header>`, or `<aside>` sits inside `[data-terminal-well="true"]`, though any future inline tab/table inside a well would collide.
2. `styles/fonts.css` bundles `Cinzel`, `EB Garamond`, and `Inter`, while `--font-terminal-mono` relies on system/OS `JetBrains Mono`, `ui-monospace`, or `SFMono-Regular`.
**Fix:** Add the Dual-Surface `:not(...)` exclusions to the remaining `index.css` selectors for completeness, and optionally bundle `JetBrainsMono` `.woff2` in `fonts.css` if deterministic cross-OS font metrics are desired.

### IN-04: Pre-existing LoC budget exceptions (`themeContract.ts`, `tauriMock.ts`) and 1 pre-existing `toestub` `ResizeObserver` warning

**File:** `crates/vox-gui/ui/src/lib/themeContract.ts:1-576`, `crates/vox-gui/ui/e2e/lib/tauriMock.ts:1-2315`, `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx:92`
**Issue:** All Phase 446-created and Console-focused files stay strictly `< 435` non-blank LoC (highest: `cvFlywheel.ts: 431`, `TerminalApprovalCard.tsx: 428`, `Console.tsx: 414`, `AgentTurnContent.tsx: 412`). Pre-existing `themeContract.ts` (`543` non-blank LoC, `+2` lines in 446-01) and `tauriMock.ts` (`2296` non-blank LoC, `+32` net lines in 446-04) exceed 435 LoC. `toestub --rules vox/ui` also emits `1` pre-existing warning inside `Console/` (`BlockStreamView.tsx:92` `new ResizeObserver(`).
**Fix:** No action required for Phase 446; consider splitting `tauriMock.ts` terminal handlers and migrating `BlockStreamView.tsx:92` to a shared `useResizeObserver` helper in future cleanup.

---

_Reviewed: 2026-11-16T02:58:17Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
