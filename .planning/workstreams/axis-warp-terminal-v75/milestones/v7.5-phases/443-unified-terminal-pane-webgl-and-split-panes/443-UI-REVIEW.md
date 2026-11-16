# Phase 443 — UI Review

**Audited:** 2026-11-15
**Baseline:** `443-UI-SPEC.md` (approved `2026-11-15T10:36:30Z`)
**Screenshots:** not captured (code-only audit mode; live Playwright + VLM CV Flywheel scheduled in Phase 446)

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Declared CTAs, empty/error copy, and `sanitizeErrorForToast` match spec; 2 buttons violate WCAG 2.5.3 Label-in-Name (`aria-label` diverges from visible text). |
| 2. Visuals | 3/4 | Unified 3-layer `<TerminalSessionPane>` + `ConsoleSplitGrid` eliminates double-echo layout, but `ConsoleSessionBar.tsx` imports `lucide-react` directly and `term-view-mode-toggle` is a swap button rather than a segmented active-mode toggle. |
| 3. Color | 3/4 | Zero dark-hex backgrounds and 16/16 WCAG >= 4.5:1 ANSI slots verified across all 4 profiles, but `Confirm Close Session` renders hardcoded `bg-red-500` (`variant="danger"`) and repeated block action buttons overuse `text-accent`. |
| 4. Typography | 3/4 | Zero banned font literals and clean `JetBrains Mono` stack on Console blocks, but `<pre>` in `ChatTerminalBlockCard.tsx` omits `TERMINAL_CARD_MONO_STYLE` (falling back to UA `monospace`) and default `<Button>` in `Console.tsx` leaks `12.5px` / `font-medium` (`500`). |
| 5. Spacing | 3/4 | Core `4px`/`8px`/`16px`/`24px`/`32px` grid respected across new components, with minor `.5` fractional leaks (`py-0.5`, `px-3.5 py-1.5`, `gap-1.5`) and missing `compact` `p-2` card density wiring. |
| 6. Experience Design | 2/4 | Strong virtualization, XSS safety, `axe` `nested-interactive` cleanliness, and unique `textbox` invariants, but unmodified global `p`/`f`/`a`/`r` key listeners hijack `Cmd/Ctrl+A` and `Cmd/Ctrl+R` when blocks exist, and VT search zero-match counts fall through to hidden blocks. |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **[BLOCKER] Unmodified global single-key shortcuts (`p`, `f`, `a`, `r`) hijack `Cmd/Ctrl+A` (Select All) and `Cmd/Ctrl+R` (`Console.tsx:130, 155-164`)** — Whenever at least one command block exists and focus is outside an `<input>`/`<textarea>` (e.g., selecting `<pre>` output, interacting with `<VoxSelect>` `[role="combobox"]`, or focusing any button), pressing `Cmd/Ctrl+A` prevents selection and opens `SendToAgent`, while `Cmd/Ctrl+R` re-submits the last shell command to the PTY — Guard `Console.tsx:155` with `!isTyping && !e.metaKey && !e.ctrlKey && !e.altKey && consoleRootRef.current?.contains(e.target as Node)` and expand `isTyping` to exclude `HTMLElement` targets matching `button, [role="combobox"], [role="listbox"], [role="option"], [role="dialog"], [contenteditable="true"]`.
2. **[WARNING] VT search zero-match count leaks hidden block counts & closing search drops keyboard focus (`TerminalSessionPane.tsx:240-243, 285`)** — In `Raw VT` / `ALT_SCREEN` mode, when a query has `0` matches in the active VT screen (`vtSearchStats.totalMatches === 0`), `showVt && vtSearchStats.totalMatches > 0` evaluates `false` and displays phantom match counts from hidden background blocks instead of `0 of 0 matches`; pressing `Escape` in `TermSearchBar` also unmounts the input without restoring focus to the active pane (`443-UI-SPEC.md:220`) — Change `TerminalSessionPane.tsx:240, 242` to branch strictly on `showVt ? vtSearchStats.totalMatches : blockSearchMatches.length` and restore focus to the active VT terminal or Console container inside `onClose`.
3. **[WARNING] `ChatTerminalBlockCard` `<pre>` drops `JetBrains Mono` stack & `Confirm Close Session` renders hardcoded `bg-red-500` (`ChatTerminalBlockCard.tsx:144`, `ConsoleSessionBar.tsx:152`)** — Browser user-agent CSS `pre { font-family: monospace }` overrides the parent `div`'s `TERMINAL_CARD_MONO_STYLE` on `chat-term-block-output` (losing `JetBrains Mono` and ligature suppression), and `<Button variant="danger">` applies `bg-red-500` (`Button.tsx:16`) instead of Roman Scriptorium `var(--color-status-fail)` (`443-UI-SPEC.md:97, 142`) — Add `style={TERMINAL_CARD_MONO_STYLE}` directly on `<pre data-testid="chat-term-block-output">` (`ChatTerminalBlockCard.tsx:144`) and style `Confirm Close Session` (`ConsoleSessionBar.tsx:152`) with `bg-(--color-status-fail) text-bg-base hover:opacity-90` plus an `Escape` key dismissal handler on `confirmCloseSid`.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)

**Passing contract checks:**
- Primary navigation CTA `Open in Console` (`data-testid="chat-term-open-console"`), split toolbar labels (`Split Pane Right`, `Split Pane Down`, `Close Terminal Pane`, `Search Output`), stdin/background CTAs (`Send Stdin Line`, `Promote to Background`, `Interrupt Process`), `Pinned` badge vs `Pin`/`Unpin` toggle disambiguation, empty state (`Terminal session ready` / `No command blocks recorded yet. Enter a shell command below or press Cmd+J to focus the prompt.`), search zero-match state (`0 of 0 matches — try another pattern or press Esc to clear`), WebGL fallback notice, disconnect alert, and two-step close confirmation copy match `443-UI-SPEC.md:130-142` verbatim.
- Legacy `Console.test.tsx` lowercase strings (`+ new session`, `continue in vox term`, `copy last block`, `send to agent`) are preserved per `443-UI-SPEC.md:128`.
- Zero emojis or banned dingbat glyphs appear in any audited file, and every `pushToast` error path wraps exceptions in `sanitizeErrorForToast(...)` (`TerminalSessionPane.tsx:130, 169, 181, 193, 203`; `Console.tsx:192, 202, 223, 247, 288`).

**Findings:**
- **[WARNING] WCAG 2.5.3 Label-in-Name mismatch on `Interrupt Process` (`BlockCard.tsx:199-200`)** — Button renders visible text `Interrupt Process` but sets `aria-label="Interrupt running command"`, masking the visible label from speech-input / screen-reader activation. Remove `aria-label` (since `<Icon.stop aria-hidden="true" />` + `<span>Interrupt Process</span>` already provides an exact accessible name) or align `aria-label` to `"Interrupt Process"`.
- **[WARNING] WCAG 2.5.3 Label-in-Name mismatch on `term-view-mode-toggle` (`TerminalSessionPane.tsx:276`)** — Visible button label is `View Raw VT` / `View Blocks`, whereas `aria-label` is `"Switch to raw VT view"` / `"Switch to command block view"`. Align `aria-label` so it starts with or equals the visible text.
- **[WARNING] Unexempted lowercase CTA & seal fallback labels (`Console.tsx:295-296`, `ChatTerminalBlockCard.tsx:85-86`)** — `Console.tsx:295-296` renders lowercase `agent {openAgentId}` and `back to terminal` (not among the 4 legacy buttons exempted in `443-UI-SPEC.md:128`), and `ChatTerminalBlockCard.tsx:85-86` falls back to `verified` / `unverified` when `TurnEventDto.exit_code` is absent instead of `443-UI-SPEC.md:244`'s declared status seals (`running`, `awaiting input`, `exit 0`, `exit N`).

### Pillar 2: Visuals (3/4)

**Passing contract checks:**
- Replaces the legacy `50%/50%` stacked `<BlockStreamView>` (`max-h-[340px]`) + `<TerminalTab>` double-echo layout with the 3-layer Roman Scriptorium hierarchy (`Console.tsx` chrome -> `ConsoleSplitGrid.tsx` resizable tree -> unified `TerminalSessionPane.tsx`).
- Every Phase 443 authored `.ts`/`.tsx` module stays strictly under the `450` non-blank LoC budget (`Console.tsx` refactored to `330` total lines; `TerminalSessionPane.tsx` `331` total lines; `TerminalTab.tsx` `328` total lines; `BlockCard.tsx` `256` total lines).

**Findings:**
- **[WARNING] Direct `lucide-react` import bypasses exclusive `Icons.tsx` registry (`ConsoleSessionBar.tsx:2, 8, 114, 119`)** — `443-UI-SPEC.md:24, 41` designates `../../ui/Icons` (`Icon.*`, `RomanIcon.*`, `KeycapIcon`) as the exclusive icon source, but `ConsoleSessionBar.tsx:2` imports `Columns2` and `Rows2` directly from `lucide-react` via `<VoxIcon>`. Export split-horizontal / split-vertical glyphs from `Icons.tsx` instead.
- **[WARNING] View mode switch implemented as single swap button instead of segmented toggle (`TerminalSessionPane.tsx:276`)** — `443-UI-SPEC.md:103, 136` specifies a segmented toggle (`View Blocks` / `View Raw VT`) with accent styling on the active mode pill. Currently a single muted `variant="ghost"` button shows only the *other* mode's action (`View Raw VT` while in `Blocks` mode), obscuring which mode is currently active.
- **[WARNING] Per-pane split buttons omitted in `Console.tsx` & compact icon-only search button lacks `<Tooltip>` (`Console.tsx:305-318`, `TerminalSessionPane.tsx:277`)** — `443-UI-SPEC.md:159, 225` specifies split controls in both `term-session-bar` and the active pane header, but `Console.tsx:305-318` does not pass `onSplitRight`/`onSplitDown` to `TerminalSessionPane` (avoiding duplicate `term-split-right` testids). In compact dock mode (`TerminalSessionPane.tsx:277`), `term-search-trigger` hides `<span>Search Output</span>` without wrapping the icon-only button in `<Tooltip>` (`443-UI-SPEC.md:42, 252`).
- **[WARNING] Hand-rolled empty state, missing `280x180` pane pixel floor, missing `TUI Active` pulse, and synthetic WebGL probe (`BlockStreamView.tsx:138-145`, `ConsoleSplitGrid.tsx:136`, `TerminalSessionPane.tsx:289`, `webglAddon.ts:39-56`)** — `BlockStreamView.tsx:138-145` renders raw `<div>/<h3>/<p>` instead of `<EmptyState>` (`443-UI-SPEC.md:44`); `ConsoleSplitGrid.tsx:136` uses `min-w-0 min-h-0` without `min-w-[280px] min-h-[180px]` (`443-UI-SPEC.md:159, 305`); `TerminalSessionPane.tsx:289` omits `animate-pulse` on the `TUI Active` emerald dot (`443-UI-SPEC.md:206`); and `webglAddon.ts:39-56` probes WebGL2 context loss on a detached `<canvas>` (constrained by `00-core.md` Rule 5's ban on new npm packages) while xterm continues painting via its built-in DOM renderer.

### Pillar 3: Color (3/4)

**Class & token usage audit across Phase 443 files:**
- Hardcoded `zinc-*` / `bg-black/*` / `border-white/*` / dark-brown hex backgrounds (`#0a0805`..`#2a2011`) in new/refactored Phase 443 Console & Chat card modules: **0** (note: untouched legacy lines in `ChatSurface.tsx:642, 940` still contain pre-existing `ring-offset-zinc-950` and `hover:border-white/20`).
- Semantic surface/text/border classes (`bg-bg-base`, `bg-bg-surface`, `bg-bg-elevated`, `bg-overlay-subtle`, `text-text-primary`, `text-text-secondary`, `text-text-muted`, `border-border-subtle`, `border-border-strong`): **74+ occurrences**, cleanly establishing the 60% (`bg-bg-base` viewport/output wells) / 30% (`bg-bg-surface` & `bg-bg-elevated` chrome/cards) split.
- 4-profile ANSI 16-color palette (`terminalTheme.ts:29-65`, `sgrToSpans.ts:6-14`): **16/16 foreground slots** match `443-UI-SPEC.md:113-122` (`>= 4.5:1` contrast across `arcane`, `travertine`, `slate`, `high-contrast`), with `resolveTerminalTheme()` reading `--color-bg-base` and `--color-text-primary` via `getComputedStyle`.

**Findings:**
- **[WARNING] Destructive session close button renders Tailwind `bg-red-500` instead of `var(--color-status-fail)` (`ConsoleSessionBar.tsx:152`, `Button.tsx:16`)** — `443-UI-SPEC.md:97, 142` requires `var(--color-status-fail)` (`#9E2B25` in `arcane`/`travertine`, `#f87171` in `slate`/`high-contrast`) on `Confirm Close Session`, but `<Button variant="danger">` resolves to hardcoded `bg-red-500 hover:bg-red-600 active:bg-red-700`.
- **[WARNING] 10% Accent (`text-accent` / `border-brass`) diluted on repeated block badges and footer actions (`BlockCard.tsx:180, 247, 250`, `TerminalSessionPane.tsx:270`)** — `443-UI-SPEC.md:99-105` restricts accent tokens to 5 high-signal elements (Pinned card/badge, active pane/block focus ring, active session tab, active search match & active mode pill, and `Open in Console`). Applying `text-accent` to `structured json` (`BlockCard.tsx:180`), `Focus in Chat` (`BlockCard.tsx:247`), and `Ask Agent` (`BlockCard.tsx:250`) on every block card multiplies accent chrome across every row in the stream, while `term-view-mode-toggle` (`TerminalSessionPane.tsx:276`) lacks active accent styling. Restyle `Focus in Chat`, `Ask Agent`, `structured json`, and `term:<sid>` (`TerminalSessionPane.tsx:270`) to `text-text-secondary hover:text-text-primary`.
- **[WARNING] Warm `TerminalTab` xterm theme does not react to live theme switches & ANSI 16 arrays are duplicated (`TerminalTab.tsx:98`, `sgrToSpans.ts:6-14`)** — `TerminalTab.tsx:98` assigns `theme: resolveTerminalTheme()` only during initial `new Terminal(...)` mount; toggling `document.documentElement.dataset.theme` at runtime leaves `term.options.theme` stale until session remount. Observe `data-theme` attribute mutations on `document.documentElement` to update `term.options.theme = resolveTerminalTheme()`, and import the 16-color palettes in `sgrToSpans.ts` from `terminalTheme.ts` to eliminate duplication.

### Pillar 4: Typography (3/4)

**Size & weight distribution across Phase 443 modules:**
- Font sizes in new Phase 443 components: `text-[11px]` (`35` uses — badges, seals, compact buttons, counters), `text-xs` / `12px` (`18` uses — terminal cells, `<pre>` output, inputs, body), `text-sm` / `14px` (`3` uses — `$ command` headings & empty-state heading), `18px` (`RomanSurfaceHeader` display title). Zero `text-base`, `text-lg`, `text-xl`, or `BANNED_ROMANESQUE_FONT_LITERALS` (`Menlo`, `Monaco`, `Consolas`, `Courier`, `Arial`).
- Font weights in new Phase 443 components: `400` (default) and `font-semibold` (`600`, `37` uses).

**Findings:**
- **[WARNING] `<pre data-testid="chat-term-block-output">` omits `TERMINAL_CARD_MONO_STYLE`, letting UA `font-family: monospace` override `JetBrains Mono` (`ChatTerminalBlockCard.tsx:144`)** — Unlike `BlockCard.tsx:212` (`<pre style={TERMINAL_MONO_STYLE}>`), `ChatTerminalBlockCard.tsx` sets `style={TERMINAL_CARD_MONO_STYLE}` only on the parent `<div>` (`line 124`). Browser user-agent stylesheets declare `pre { font-family: monospace }`, overriding inherited `font-family` on `<pre>` unless `style={TERMINAL_CARD_MONO_STYLE}` is applied directly to `<pre>` (`line 144`).
- **[WARNING] Default `<Button>` in `Console.tsx` leaks off-scale `text-[12.5px]` and `font-medium` (`500`) (`Console.tsx:271-273, 296`)** — `Console.tsx:296` (`back to terminal`) uses default `<Button>` (`size="md"` in `Button.tsx:8, 24` -> `text-[12.5px] font-medium`), and `Console.tsx:271-273` (`continue in vox term`, `copy last block`, `send to agent`) overrides `text-[11px]` without overriding `font-medium` (`500`) to `font-semibold` (`600`), violating the strict 4-size (`11px`/`12px`/`14px`/`18px`) and 2-weight (`400`/`600`) contract (`443-UI-SPEC.md:69`).
- **[WARNING] Session tab size/weight & VT fixed-cell style gaps (`ConsoleSessionBar.tsx:76, 84`, `TerminalTab.tsx:98, 325`)** — `ConsoleSessionBar.tsx:76, 84` renders active session tab titles at `11px` instead of `14px` (`443-UI-SPEC.md:75`) and inactive tabs at weight `400` instead of `600` (`443-UI-SPEC.md:73`). `TerminalTab.tsx:98` omits `letterSpacing: 0` in `new Terminal({...})` options, and `TerminalTab.tsx:325` omits `fontVariantLigatures: 'none'`, `fontFeatureSettings: '"calt" 0, "liga" 0'`, and `letterSpacing: '0px'` (`443-UI-SPEC.md:82`).

### Pillar 5: Spacing (3/4)

**Spacing class analysis across Phase 443 modules:**
- Standard `4px`-grid tokens dominate: `gap-1` (`4px`), `gap-2` (`8px`), `p-2` / `px-2` / `py-1` / `py-2` (`4px`–`8px`), `p-4` (`16px`), `py-6` (`24px`), `h-6` (`24px`), `h-8` (`32px`), plus declared `12px` (`p-3` / `px-3`) frame and card gutters.

**Findings:**
- **[WARNING] Fractional `.5` spacing classes (`2px` / `6px` / `14px`) in Phase 443 modified files (`TerminalTab.tsx:300`, `Console.tsx:271-273, 296`, `TurnTrace.tsx:88, 120`)** — `TerminalTab.tsx:300` uses `py-0.5` (`2px`) on its mock WebGL badge; `Console.tsx:271-273, 296` inherits `px-3.5 py-1.5` (`14px` / `6px`) and `h-9` (`36px`) from default `<Button size="md">`; and `TurnTrace.tsx:88, 120` retains `gap-1.5` (`6px`) and `py-0.5` (`2px`) on `chat-trace-summary`. Replace with `py-1`, `size="xs"` + `h-6 px-2 py-1`, and `gap-1` / `py-1`.
- **[WARNING] Compact `p-2` (`8px`) block card density not threaded to `BlockStreamView` / `BlockCard` (`TerminalSessionPane.tsx:303`, `BlockCard.tsx:159`)** — `443-UI-SPEC.md:180` specifies `p-2` (`8px`) card padding in compact mode (`ChatSurface` `TerminalDockPanel`) vs `p-3` (`12px`) in full `Console` mode, but `BlockCard.tsx:159` hardcodes `p-3` unconditionally. Thread `compact` from `TerminalSessionPane` -> `BlockStreamView` -> `BlockCard` (`compact ? 'p-2' : 'p-3'`).
- **[WARNING] Sticky `BlockCard` header lacks top padding and `backdrop-blur-xs` (`BlockCard.tsx:161`)** — `BlockCard.tsx:161` uses `sticky top-0 z-10 ... bg-bg-surface/95 pb-2` instead of `px-3 py-2 backdrop-blur-xs` (`443-UI-SPEC.md:180`), leaving `0px` top clearance when the header sticks to the top of the scroll container during long-output scrolling.

### Pillar 6: Experience Design (2/4)

**State, accessibility & performance coverage verified:**
- **Unique textbox invariant (`getByRole('textbox')`)**: Preserved across all idle states (`TerminalTab.tsx:45-56` sets `tabIndex = -1` and `aria-hidden="true"` on `.xterm-helper-textarea` when `suppressHelperTextarea` is active; `TermSearchBar.tsx:26` unmounts to `null` when closed; `BlockCard.tsx:217-234` unmounts `term-block-stdin-bar` to `null` whenever `block.status !== 'AwaitingInput'`).
- **`axe` `nested-interactive` clean (`ConsoleSessionBar.tsx:71-105`)**: Session tabs use `<div role="group" aria-label="Session ${s.id}">` wrapping sibling `term-session-tab-${s.id}` and `term-session-close-${s.id}` `<button type="button">` elements.
- **XSS & URL scheme safety**: Zero `dangerouslySetInnerHTML` across all Console and Chat terminal modules; both `sgrToSpans.ts:197-225` and `xtermAddons.ts:86-93` strictly allowlist `http:` and `https:` URLs while rejecting `javascript:`, `data:`, `vbscript:`, and `file:`.
- **Virtualization, `<16 ms` `ALT_SCREEN` handoff, `afterSeq` delta reduction, & Dockview `closedPanelIds`**: `BlockStreamView.tsx:56-83` virtualizes `> 24` blocks when `clientHeight > 0` with deterministic zero-height jsdom fallback; `TerminalSessionPane.tsx:302, 318` toggles CSS visibility on a persistent warm `TerminalTab` with zero remounts and zero post-submit `replay_utf8` storms; `ChatSurface.tsx:274-289` mounts `<TerminalSessionPane compact>` without resurrecting closed Dockview panels.

**Findings:**
- **[BLOCKER] Unmodified single-key shortcuts (`p`, `f`, `a`, `r`) fire during `Cmd/Ctrl` chord shortcuts and non-input widget interaction (`Console.tsx:130, 155-164`)** — In `Console.tsx:155`, `else if (!isTyping && target)` omits `!e.metaKey && !e.ctrlKey && !e.altKey` and checks only `HTMLInputElement | HTMLTextAreaElement`. When `blocks.length > 0` and focus is on `<pre>` output, a `<button>`, `<VoxSelect>` (`[role="combobox"]` / `[role="option"]`), or `document.body`, pressing `Cmd/Ctrl+A` (Select All) calls `e.preventDefault()` and opens `SendToAgent`, `Cmd/Ctrl+R` re-executes `target.input` in the shell, and `Cmd/Ctrl+P` toggles block pinning.
- **[WARNING] VT search match counter falls through to hidden block matches & `Escape` drops focus (`TerminalSessionPane.tsx:240-243, 285`)** — Because line 240 checks `showVt && vtSearchStats.totalMatches > 0` instead of `showVt`, searching in `Raw VT` / `ALT_SCREEN` mode for a pattern absent from the VT buffer displays match counts from hidden `BlockStreamView` cards instead of `0 of 0 matches — try another pattern or press Esc to clear`. Closing `TermSearchBar` (`line 285`) also fails to restore focus to the active terminal pane (`443-UI-SPEC.md:220`).
- **[WARNING] Two-step session close confirmation lacks `Escape` dismissal and stale-session cleanup (`ConsoleSessionBar.tsx:54, 146-160`)** — `confirmCloseSid` has no `Escape` key handler (contrary to `443-03-SUMMARY.md:56`) and does not auto-clear if `confirmCloseSid` is no longer in `sessions`.
- **[WARNING] `Autopilot -> Manual` reset misses `Raw VT` keystrokes and inline `AwaitingInput` stdin (`TerminalTab.tsx:129-132`, `TerminalSessionPane.tsx:313`)** — `443-UI-SPEC.md:239` requires *any* user submit or keystroke in `Autopilot` to immediately reset control mode to `Manual` locally and via `termSessionSetControl(sid, 'manual')`, which currently runs only on `Console.tsx` `InputEditor` submit (`line 97`) and `Escape` (`line 132`), not on direct VT `term.onData` input or `onSendStdin`.
- **[WARNING] Session block count cannot reset to `0` (`Console.tsx:230`)** — `blocks: meta.blocksCount > 0 ? meta.blocksCount : s.blocks` preserves stale positive counts if a session's blocks are cleared to `0`.

---

## Files Audited

- `crates/vox-gui/ui/src/components/surfaces/Console/terminalTheme.ts`
- `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts`
- `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts`
- `crates/vox-gui/ui/src/components/surfaces/Console/sessionReducer.ts`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/TermSearchBar.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`
- `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx`
